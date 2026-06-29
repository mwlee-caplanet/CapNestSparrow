//! JNI glue layer for the sparrow library.
//!
//! This module exposes the optimization functionality via JNI
//! so the library can be consumed from Java/Kotlin.
//!
//! Enabled with the `jni` feature flag.

use jni::JNIEnv;
use jni::JavaVM;
use jni::objects::{JClass, JString, JObject, JValueGen, GlobalRef};
use jni::sys::jobject;
use std::ffi::{CStr, c_void};
use std::sync::OnceLock;

use crate::ffi;

/// Cached JavaVM, captured in `JNI_OnLoad`. The progress-callback trampoline
/// uses it to attach the reporting thread and re-enter the JVM from native code.
static JVM: OnceLock<JavaVM> = OnceLock::new();

// ── JNI Entry Point ──

#[unsafe(no_mangle)]
pub extern "system" fn JNI_OnLoad(
    vm: *mut jni::sys::JavaVM,
    _reserved: *mut std::ffi::c_void,
) -> jni::sys::jint {
    // Cache the JavaVM so the progress-callback trampoline can re-enter the JVM.
    if let Ok(java_vm) = unsafe { JavaVM::from_raw(vm) } {
        let _ = JVM.set(java_vm);
    }
    jni::sys::JNI_VERSION_1_6
}

// ── JNI Methods ──

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_sparrow_SparrowOptimizer_optimize(
    mut env: JNIEnv,
    _class: JClass,
    input_path: JString,
    output_dir: JString,
    time_secs: jni::sys::jlong,
    rng_seed: jni::sys::jlong,
) -> jni::sys::jint {
    // Convert Java strings to C strings
    let input_path_c = java_string_to_rust(&mut env, &input_path);
    let output_dir_c = java_string_to_rust(&mut env, &output_dir);

    let (input_ptr, output_ptr) = match (input_path_c, output_dir_c) {
        (Some(i), Some(o)) => (i, o),
        _ => return -1,
    };

    // Call the FFI function directly
    ffi::sparrow_optimize(
        input_ptr.as_ptr() as *const std::os::raw::c_char,
        output_ptr.as_ptr() as *const std::os::raw::c_char,
        time_secs as u64,
        rng_seed as u64,
        1, // early_termination = true
    )
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_sparrow_SparrowOptimizer_optimizeEx(
    mut env: JNIEnv,
    _class: JClass,
    input_path: JString,
    output_dir: JString,
    config_obj: JObject,
) -> jobject {
    // Create result object
    let result_class = match env.find_class("com/sparrow/OptimizationResult") {
        Ok(c) => c,
        Err(_) => return std::ptr::null_mut(),
    };

    let result = match env.new_object(&result_class, "()V", &[]) {
        Ok(obj) => obj,
        Err(_) => return std::ptr::null_mut(),
    };

    let set_bool = |env: &mut JNIEnv, val: u8| {
        let _ = env.set_field(&result, "success", "Z", JValueGen::Bool(val));
    };

    // Convert Java strings
    let input_path_c = match java_string_to_rust(&mut env, &input_path) {
        Some(s) => s,
        None => {
            set_bool(&mut env, 0u8);
            return result.into_raw();
        }
    };

    let output_dir_c = match java_string_to_rust(&mut env, &output_dir) {
        Some(s) => s,
        None => {
            set_bool(&mut env, 0u8);
            return result.into_raw();
        }
    };

    // Read config from the Java SparrowConfig object (defaults when null)
    let config = build_config_from_java(&mut env, &config_obj);

    // Call optimization
    let mut result_struct: ffi::COptimizationResult = unsafe { std::mem::zeroed() };

    let ret_code = ffi::sparrow_optimize_ex(
        input_path_c.as_ptr() as *const std::os::raw::c_char,
        output_dir_c.as_ptr() as *const std::os::raw::c_char,
        &config,
        None,
        std::ptr::null_mut(),
        &mut result_struct,
        0, // emit_intermediate_json: optimizeEx never passes a callback
    );

    // Set result fields
    let _ = env.set_field(&result, "success", "Z", JValueGen::Bool(if ret_code == 0 { 1u8 } else { 0u8 }));
    let _ = env.set_field(&result, "stripWidth", "F", JValueGen::Float(result_struct.strip_width));
    let _ = env.set_field(&result, "density", "F", JValueGen::Float(result_struct.density));
    let _ = env.set_field(&result, "totalItems", "I", JValueGen::Int(result_struct.total_items));
    let _ = env.set_field(&result, "errorCode", "I", JValueGen::Int(result_struct.error_code));

    // Convert output JSON
    if !result_struct.output_json.is_null() {
        if let Ok(json_str) = unsafe { CStr::from_ptr(result_struct.output_json) }.to_str() {
            if let Ok(java_string) = env.new_string(json_str) {
                let _ = env.set_field(&result, "outputJson", "Ljava/lang/String;", JValueGen::Object(&java_string));
            }
        }
        ffi::sparrow_free_string(result_struct.output_json);
    }

    // Free error message if present
    if !result_struct.error_message.is_null() {
        ffi::sparrow_free_string(result_struct.error_message);
    }

    result.into_raw()
}

// ── Progress callback support ──

/// Context handed to the C callback as `user_data`. Holds a global ref to the
/// Java ProgressListener so it survives the (synchronous) native call.
struct ProgressCtx {
    listener: GlobalRef,
}

/// `extern "C"` trampoline matching `ffi::SolutionCallback`.
/// Bridges native progress reports into `ProgressListener.onProgress(...)`.
extern "C" fn progress_trampoline(
    report_type: i32,
    strip_width: f32,
    density: f32,
    solution_json: *const std::os::raw::c_char,
    user_data: *mut c_void,
) {
    if user_data.is_null() {
        return;
    }
    let ctx = unsafe { &*(user_data as *const ProgressCtx) };
    let Some(vm) = JVM.get() else {
        return;
    };
    // Attach the reporting thread (no-op if already attached); the guard does
    // not detach a thread that was already attached.
    if let Ok(mut env) = vm.attach_current_thread() {
        // Convert the (possibly null) solution JSON into a Java String (or null).
        let json_jstr = if solution_json.is_null() {
            None
        } else {
            unsafe { CStr::from_ptr(solution_json) }
                .to_str()
                .ok()
                .and_then(|s| env.new_string(s).ok())
        };
        let null_obj = JObject::null();
        let json_arg: &JObject = match json_jstr.as_ref() {
            Some(js) => js, // &JString -> &JObject via Deref coercion
            None => &null_obj,
        };
        let _ = env.call_method(
            &ctx.listener,
            "onProgress",
            "(IFFLjava/lang/String;)V",
            &[
                JValueGen::Int(report_type),
                JValueGen::Float(strip_width),
                JValueGen::Float(density),
                JValueGen::Object(json_arg),
            ],
        );
        // Clear any pending exception so subsequent JNI calls stay valid.
        if env.exception_check().unwrap_or(false) {
            let _ = env.exception_clear();
        }
    }
}

/// Extended optimization with a real-time progress callback.
///
/// Reads the Java `SparrowConfig` field-by-field (defaults when null) and wires
/// up the progress callback. Pass `null` for `listener` to run without a callback.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_sparrow_SparrowOptimizer_optimizeWithProgress(
    mut env: JNIEnv,
    _class: JClass,
    input_path: JString,
    output_dir: JString,
    config_obj: JObject,
    listener: JObject,
) -> jobject {
    // Create result object
    let result_class = match env.find_class("com/sparrow/OptimizationResult") {
        Ok(c) => c,
        Err(_) => return std::ptr::null_mut(),
    };
    let result = match env.new_object(&result_class, "()V", &[]) {
        Ok(obj) => obj,
        Err(_) => return std::ptr::null_mut(),
    };

    let set_fail = |env: &mut JNIEnv| {
        let _ = env.set_field(&result, "success", "Z", JValueGen::Bool(0u8));
    };

    // Convert Java strings
    let input_path_c = match java_string_to_rust(&mut env, &input_path) {
        Some(s) => s,
        None => {
            set_fail(&mut env);
            return result.into_raw();
        }
    };
    let output_dir_c = match java_string_to_rust(&mut env, &output_dir) {
        Some(s) => s,
        None => {
            set_fail(&mut env);
            return result.into_raw();
        }
    };

    // Build a global ref to the listener so the C callback can use it.
    // `null` listener -> run without a callback.
    let ctx: Option<ProgressCtx> = if listener.is_null() {
        None
    } else {
        env.new_global_ref(&listener)
            .ok()
            .map(|gref| ProgressCtx { listener: gref })
    };

    // Read config from the Java SparrowConfig object (defaults when null)
    let config = build_config_from_java(&mut env, &config_obj);

    let mut result_struct: ffi::COptimizationResult = unsafe { std::mem::zeroed() };

    let (cb, user_data): (Option<ffi::SolutionCallback>, *mut c_void) = match &ctx {
        Some(c) => (
            Some(progress_trampoline as ffi::SolutionCallback),
            c as *const ProgressCtx as *mut c_void,
        ),
        None => (None, std::ptr::null_mut()),
    };

    // Synchronous: the C callback only fires for the duration of this call,
    // so `ctx` (and its GlobalRef) staying alive on this stack is sufficient.
    let ret_code = ffi::sparrow_optimize_ex(
        input_path_c.as_ptr() as *const std::os::raw::c_char,
        output_dir_c.as_ptr() as *const std::os::raw::c_char,
        &config,
        cb,
        user_data,
        &mut result_struct,
        if cb.is_some() { 1 } else { 0 }, // emit intermediate JSON when a listener is present
    );

    drop(ctx);

    // Fill result fields
    let _ = env.set_field(
        &result,
        "success",
        "Z",
        JValueGen::Bool(if ret_code == 0 { 1u8 } else { 0u8 }),
    );
    let _ = env.set_field(
        &result,
        "stripWidth",
        "F",
        JValueGen::Float(result_struct.strip_width),
    );
    let _ = env.set_field(
        &result,
        "density",
        "F",
        JValueGen::Float(result_struct.density),
    );
    let _ = env.set_field(
        &result,
        "totalItems",
        "I",
        JValueGen::Int(result_struct.total_items),
    );
    let _ = env.set_field(
        &result,
        "errorCode",
        "I",
        JValueGen::Int(result_struct.error_code),
    );

    // Output JSON
    if !result_struct.output_json.is_null() {
        if let Ok(json_str) = unsafe { CStr::from_ptr(result_struct.output_json) }.to_str() {
            if let Ok(java_string) = env.new_string(json_str) {
                let _ = env.set_field(
                    &result,
                    "outputJson",
                    "Ljava/lang/String;",
                    JValueGen::Object(&java_string),
                );
            }
        }
        ffi::sparrow_free_string(result_struct.output_json);
    }
    if !result_struct.error_message.is_null() {
        ffi::sparrow_free_string(result_struct.error_message);
    }

    result.into_raw()
}

// ── Helpers ──

fn java_string_to_rust(env: &mut JNIEnv, js: &JString) -> Option<String> {
    match env.get_string(js) {
        Ok(s) => match s.to_str() {
            Ok(s) => Some(s.to_string()),
            Err(_) => None,
        },
        Err(_) => None,
    }
}

// ── Config marshalling (Java SparrowConfig -> CSparrowConfig) ──

fn jget_int(env: &mut JNIEnv, obj: &JObject, name: &str) -> i32 {
    env.get_field(obj, name, "I").and_then(|v| v.i()).unwrap_or(0)
}

fn jget_long(env: &mut JNIEnv, obj: &JObject, name: &str) -> i64 {
    env.get_field(obj, name, "J").and_then(|v| v.j()).unwrap_or(0)
}

fn jget_float(env: &mut JNIEnv, obj: &JObject, name: &str) -> f32 {
    env.get_field(obj, name, "F").and_then(|v| v.f()).unwrap_or(0.0)
}

fn jget_obj<'local>(
    env: &mut JNIEnv<'local>,
    obj: &JObject,
    name: &str,
    sig: &str,
) -> Option<JObject<'local>> {
    env.get_field(obj, name, sig).ok().and_then(|v| v.l().ok())
}

fn read_separator(env: &mut JNIEnv, sep: &JObject) -> ffi::CSeparatorConfig {
    ffi::CSeparatorConfig {
        iter_no_imprv_limit: jget_int(env, sep, "iterNoImprvLimit"),
        strike_limit: jget_int(env, sep, "strikeLimit"),
        n_workers: jget_int(env, sep, "nWorkers"),
        n_container_samples: jget_int(env, sep, "nContainerSamples"),
        n_focussed_samples: jget_int(env, sep, "nFocussedSamples"),
        n_coord_descents: jget_int(env, sep, "nCoordDescents"),
    }
}

/// Marshal a Java `com.sparrow.SparrowConfig` object into a native `CSparrowConfig`.
/// Returns the library default config when `config_obj` is null, and falls back
/// to defaults for any nested object that is unexpectedly null.
fn build_config_from_java<'local>(
    env: &mut JNIEnv<'local>,
    config_obj: &JObject,
) -> ffi::CSparrowConfig {
    let mut cfg: ffi::CSparrowConfig = unsafe { std::mem::zeroed() };
    ffi::sparrow_get_default_config(&mut cfg);

    if config_obj.is_null() {
        return cfg;
    }

    const SEP_SIG: &str = "Lcom/sparrow/SparrowConfig$SeparatorConfig;";
    const EXPL_SIG: &str = "Lcom/sparrow/SparrowConfig$ExplorationConfig;";
    const CMPR_SIG: &str = "Lcom/sparrow/SparrowConfig$CompressionConfig;";

    // Top-level scalars
    cfg.rng_seed = jget_long(env, config_obj, "rngSeed") as u64;
    cfg.poly_simpl_tolerance = jget_float(env, config_obj, "polySimplTolerance");
    cfg.min_item_separation = jget_float(env, config_obj, "minItemSeparation");
    cfg.narrow_concavity_cutoff_dist = jget_float(env, config_obj, "narrowConcavityCutoffDist");
    cfg.narrow_concavity_cutoff_area = jget_float(env, config_obj, "narrowConcavityCutoffArea");

    // Exploration phase
    if let Some(expl) = jget_obj(env, config_obj, "exploration", EXPL_SIG) {
        cfg.exploration.shrink_step = jget_float(env, &expl, "shrinkStep");
        cfg.exploration.time_limit_secs = jget_float(env, &expl, "timeLimitSecs");
        cfg.exploration.max_conseq_failed_attempts =
            jget_int(env, &expl, "maxConseqFailedAttempts");
        cfg.exploration.solution_pool_distribution_stddev =
            jget_float(env, &expl, "solutionPoolDistributionStdDev");
        cfg.exploration.large_item_ch_area_cutoff_percentile =
            jget_float(env, &expl, "largeItemChAreaCutoffPercentile");
        if let Some(sep) = jget_obj(env, &expl, "separator", SEP_SIG) {
            cfg.exploration.separator = read_separator(env, &sep);
        }
    }

    // Compression phase
    if let Some(cmpr) = jget_obj(env, config_obj, "compression", CMPR_SIG) {
        cfg.compression.shrink_range_min = jget_float(env, &cmpr, "shrinkRangeMin");
        cfg.compression.shrink_range_max = jget_float(env, &cmpr, "shrinkRangeMax");
        cfg.compression.time_limit_secs = jget_float(env, &cmpr, "timeLimitSecs");
        cfg.compression.shrink_decay_strategy = jget_int(env, &cmpr, "shrinkDecayStrategy");
        cfg.compression.shrink_decay_ratio = jget_float(env, &cmpr, "shrinkDecayRatio");
        if let Some(sep) = jget_obj(env, &cmpr, "separator", SEP_SIG) {
            cfg.compression.separator = read_separator(env, &sep);
        }
    }

    // Defensive: clear any pending field-access exception so later JNI calls stay valid.
    if env.exception_check().unwrap_or(false) {
        let _ = env.exception_clear();
    }

    cfg
}
