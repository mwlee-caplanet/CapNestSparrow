//! C-compatible FFI layer for the sparrow library.
//!
//! This module exposes the core optimization functionality via `extern "C"` functions
//! so that the library can be consumed from C/C++/C# or any other language that
//! supports the C ABI.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::time::{Duration, Instant};

use rand::rngs::Xoshiro256PlusPlus;
use rand::{Rng, SeedableRng};

use jagua_rs::io::import::Importer;

use crate::config::{DEFAULT_SPARROW_CONFIG, ExplorationConfig, CompressionConfig, ShrinkDecayStrategy};
use crate::consts::{DEFAULT_COMPRESS_TIME_RATIO, DEFAULT_EXPLORE_TIME_RATIO};
use crate::optimizer::optimize;
use crate::util::listener::{DummySolListener, ReportType, SolutionListener};
use crate::util::terminator::{BasicTerminator, Terminator};
use crate::util::io;
use crate::util::svg_exporter::SvgExporter;

// ── Opaque handle types ────────────────────────────────────────────

/// Opaque handle to a Sparrow optimizer instance.
pub struct SparrowHandle {
    // (reserved for future stateful usage)
    _private: (),
}

// ── Utility: read C string ─────────────────────────────────────────

unsafe fn cstr_to_str<'a>(ptr: *const c_char) -> &'a str {
    if ptr.is_null() {
        ""
    } else {
        unsafe { CStr::from_ptr(ptr) }.to_str().unwrap_or("")
    }
}

// ── C-compatible structs for configuration ─────────────────────────

/// C-compatible separator configuration
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CSeparatorConfig {
    pub iter_no_imprv_limit: i32,
    pub strike_limit: i32,
    pub n_workers: i32,
    pub n_container_samples: i32,
    pub n_focussed_samples: i32,
    pub n_coord_descents: i32,
}

/// C-compatible exploration configuration
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CExplorationConfig {
    pub shrink_step: f32,
    pub time_limit_secs: f32,
    pub max_conseq_failed_attempts: i32, // -1 = None
    pub solution_pool_distribution_stddev: f32,
    pub large_item_ch_area_cutoff_percentile: f32,
    pub separator: CSeparatorConfig,
}

/// C-compatible compression configuration
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CCompressionConfig {
    pub shrink_range_min: f32,
    pub shrink_range_max: f32,
    pub time_limit_secs: f32,
    pub shrink_decay_strategy: i32, // 0 = TimeBased, 1 = FailureBased
    pub shrink_decay_ratio: f32,    // only used for FailureBased
    pub separator: CSeparatorConfig,
}

/// C-compatible full sparrow configuration
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CSparrowConfig {
    pub rng_seed: u64, // 0 = random
    pub poly_simpl_tolerance: f32, // negative = None
    pub min_item_separation: f32,  // negative = None
    pub narrow_concavity_cutoff_dist: f32, // negative = None
    pub narrow_concavity_cutoff_area: f32, // negative = None
    pub exploration: CExplorationConfig,
    pub compression: CCompressionConfig,
}

/// C-compatible optimization result
#[repr(C)]
#[derive(Debug, Clone)]
pub struct COptimizationResult {
    /// Final strip width
    pub strip_width: f32,
    /// Final density (0.0 - 1.0)
    pub density: f32,
    /// Number of items placed
    pub total_items: i32,
    /// Success flag (0 = success, non-zero = error code)
    pub error_code: i32,
    /// Error message (must be freed with sparrow_free_string)
    pub error_message: *mut c_char,
    /// Output JSON string (must be freed with sparrow_free_string)
    pub output_json: *mut c_char,
}

/// Callback type for solution reporting.
/// - `report_type`: 0=ExplFeas, 1=ExplInfeas, 2=ExplImproving, 3=CmprFeas, 4=Final
/// - `strip_width`: current strip width
/// - `density`: current density (0.0 - 1.0)
/// - `solution_json`: solution-only JSON (UTF-8, null-terminated) for this report,
///   or NULL when none is attached. Valid ONLY for the duration of the callback;
///   the library frees it immediately after the callback returns — do not store the
///   pointer, copy the string if you need to keep it.
/// - `user_data`: user-provided pointer
pub type SolutionCallback = extern "C" fn(
    report_type: i32,
    strip_width: f32,
    density: f32,
    solution_json: *const c_char,
    user_data: *mut std::ffi::c_void,
);

// ── Callback-based solution listener ───────────────────────────────

struct CallbackListener {
    callback: SolutionCallback,
    user_data: *mut std::ffi::c_void,
    /// When true, serialize the solution to JSON on improving/final reports.
    emit_json: bool,
    /// Timestamp of the last JSON emission (for throttling).
    last_emit: Option<Instant>,
    /// Minimum interval between JSON emissions (Final always emits regardless).
    min_interval: Duration,
}

impl SolutionListener for CallbackListener {
    fn report(&mut self, report: ReportType, solution: &jagua_rs::probs::spp::entities::SPSolution, instance: &jagua_rs::probs::spp::entities::SPInstance) {
        let report_type = match report {
            ReportType::ExplFeas => 0,
            ReportType::ExplInfeas => 1,
            ReportType::ExplImproving => 2,
            ReportType::CmprFeas => 3,
            ReportType::Final => 4,
        };
        let density = solution.density(instance);
        let strip_width = solution.strip_width();

        // ① gate: only serialize on improving / final reports (feasibility probes are skipped)
        let improving = matches!(
            report,
            ReportType::ExplImproving | ReportType::CmprFeas | ReportType::Final
        );
        let is_final = matches!(report, ReportType::Final);
        // ④ throttle: skip JSON if within min_interval (Final always passes)
        let throttled = match self.last_emit {
            Some(t) => t.elapsed() < self.min_interval,
            None => false,
        };

        if self.emit_json && improving && (is_final || !throttled) {
            // ② solution-only payload (the static instance is NOT re-serialized)
            let ext_sol = jagua_rs::probs::spp::io::export(instance, solution, *crate::EPOCH);
            if let Ok(json) = serde_json::to_string(&ext_sol) {
                if let Ok(cs) = CString::new(json) {
                    // ⑤ borrowed string: valid only during the callback, freed right after
                    (self.callback)(report_type, strip_width, density, cs.as_ptr(), self.user_data);
                    self.last_emit = Some(Instant::now());
                    return;
                }
            }
        }
        // scalars only (no JSON for this report)
        (self.callback)(report_type, strip_width, density, std::ptr::null(), self.user_data);
    }
}

// ── Helper: build config from C struct ─────────────────────────────

fn build_config(cfg: &CSparrowConfig) -> crate::config::SparrowConfig {
    let mut config = DEFAULT_SPARROW_CONFIG;

    if cfg.rng_seed != 0 {
        config.rng_seed = Some(cfg.rng_seed as usize);
    }

    config.poly_simpl_tolerance = if cfg.poly_simpl_tolerance >= 0.0 {
        Some(cfg.poly_simpl_tolerance)
    } else {
        None
    };

    config.min_item_separation = if cfg.min_item_separation >= 0.0 {
        Some(cfg.min_item_separation)
    } else {
        None
    };

    config.narrow_concavity_cutoff_ratio = if cfg.narrow_concavity_cutoff_dist >= 0.0 && cfg.narrow_concavity_cutoff_area >= 0.0 {
        Some((cfg.narrow_concavity_cutoff_dist, cfg.narrow_concavity_cutoff_area))
    } else {
        None
    };

    // Exploration config
    config.expl_cfg = ExplorationConfig {
        shrink_step: cfg.exploration.shrink_step,
        time_limit: Duration::from_secs_f32(cfg.exploration.time_limit_secs),
        max_conseq_failed_attempts: if cfg.exploration.max_conseq_failed_attempts >= 0 {
            Some(cfg.exploration.max_conseq_failed_attempts as usize)
        } else {
            None
        },
        solution_pool_distribution_stddev: cfg.exploration.solution_pool_distribution_stddev,
        large_item_ch_area_cutoff_percentile: cfg.exploration.large_item_ch_area_cutoff_percentile,
        separator_config: crate::optimizer::separator::SeparatorConfig {
            iter_no_imprv_limit: cfg.exploration.separator.iter_no_imprv_limit as usize,
            strike_limit: cfg.exploration.separator.strike_limit as usize,
            n_workers: cfg.exploration.separator.n_workers as usize,
            log_level: log::Level::Info,
            sample_config: crate::sample::search::SampleConfig {
                n_container_samples: cfg.exploration.separator.n_container_samples as usize,
                n_focussed_samples: cfg.exploration.separator.n_focussed_samples as usize,
                n_coord_descents: cfg.exploration.separator.n_coord_descents as usize,
            },
        },
    };

    // Compression config
    config.cmpr_cfg = CompressionConfig {
        shrink_range: (cfg.compression.shrink_range_min, cfg.compression.shrink_range_max),
        time_limit: Duration::from_secs_f32(cfg.compression.time_limit_secs),
        shrink_decay: match cfg.compression.shrink_decay_strategy {
            1 => ShrinkDecayStrategy::FailureBased(cfg.compression.shrink_decay_ratio),
            _ => ShrinkDecayStrategy::TimeBased,
        },
        separator_config: crate::optimizer::separator::SeparatorConfig {
            iter_no_imprv_limit: cfg.compression.separator.iter_no_imprv_limit as usize,
            strike_limit: cfg.compression.separator.strike_limit as usize,
            n_workers: cfg.compression.separator.n_workers as usize,
            log_level: log::Level::Debug,
            sample_config: crate::sample::search::SampleConfig {
                n_container_samples: cfg.compression.separator.n_container_samples as usize,
                n_focussed_samples: cfg.compression.separator.n_focussed_samples as usize,
                n_coord_descents: cfg.compression.separator.n_coord_descents as usize,
            },
        },
    };

    config
}

// ── Exported functions ─────────────────────────────────────────────

/// Run the full sparrow optimization pipeline on a JSON input file.
///
/// # Parameters
/// - `input_path`: path to the input JSON file (instance or instance+solution).
/// - `output_dir`: directory where output files will be written.
/// - `global_time_secs`: total time budget in seconds (0 = use default 600s).
/// - `rng_seed`: seed for the RNG (0 = random).
/// - `early_termination`: if non-zero, enable early termination heuristics.
///
/// # Returns
/// 0 on success, non-zero on failure.
#[unsafe(no_mangle)]
pub extern "C" fn sparrow_optimize(
    input_path: *const c_char,
    output_dir: *const c_char,
    global_time_secs: u64,
    rng_seed: u64,
    early_termination: i32,
) -> i32 {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let input_path_str = unsafe { cstr_to_str(input_path) };
        let output_dir_str = unsafe { cstr_to_str(output_dir) };

        if input_path_str.is_empty() {
            eprintln!("[sparrow_ffi] input_path must not be null or empty");
            return 1;
        }

        let output_dir_str = if output_dir_str.is_empty() {
            "output"
        } else {
            output_dir_str
        };

        // Create output directory
        if let Err(e) = std::fs::create_dir_all(output_dir_str) {
            eprintln!("[sparrow_ffi] failed to create output dir: {e}");
            return 2;
        }

        // Build config
        let mut config = DEFAULT_SPARROW_CONFIG;

        let time_limit = if global_time_secs > 0 {
            Duration::from_secs(global_time_secs)
        } else {
            Duration::from_secs(600)
        };

        config.expl_cfg.time_limit = time_limit.mul_f32(DEFAULT_EXPLORE_TIME_RATIO);
        config.cmpr_cfg.time_limit = time_limit.mul_f32(DEFAULT_COMPRESS_TIME_RATIO);

        if early_termination != 0 {
            config.expl_cfg.max_conseq_failed_attempts =
                Some(crate::consts::DEFAULT_MAX_CONSEQ_FAILS_EXPL);
            config.cmpr_cfg.shrink_decay =
                crate::config::ShrinkDecayStrategy::FailureBased(
                    crate::consts::DEFAULT_FAIL_DECAY_RATIO_CMPR,
                );
        }

        if rng_seed != 0 {
            config.rng_seed = Some(rng_seed as usize);
        }

        // RNG
        let rng: Xoshiro256PlusPlus = match config.rng_seed {
            Some(seed) => Xoshiro256PlusPlus::seed_from_u64(seed as u64),
            None => Xoshiro256PlusPlus::seed_from_u64(rand::random()),
        };

        // Read input
        let input_path = std::path::Path::new(input_path_str);
        let (ext_instance, ext_solution) = match io::read_spp_input(input_path) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("[sparrow_ffi] failed to read input: {e}");
                return 3;
            }
        };

        // Import instance
        let importer = Importer::new(
            config.cde_config,
            config.poly_simpl_tolerance,
            config.min_item_separation,
            config.narrow_concavity_cutoff_ratio,
        );
        let instance = match jagua_rs::probs::spp::io::import_instance(&importer, &ext_instance) {
            Ok(inst) => inst,
            Err(e) => {
                eprintln!("[sparrow_ffi] failed to import instance: {e}");
                return 4;
            }
        };

        let initial_solution = ext_solution.map(|e| {
            jagua_rs::probs::spp::io::import_solution(&instance, &e)
        });

        // Run optimization
        let mut terminator = BasicTerminator::new();

        let final_svg_path = Some(format!("{output_dir_str}/final_{}.svg", ext_instance.name));
        let intermediate_svg_dir = match cfg!(feature = "only_final_svg") {
            true => None,
            false => Some(format!("{output_dir_str}/sols_{}", ext_instance.name)),
        };
        let live_svg_path = match cfg!(feature = "live_svg") {
            true => Some("data/live/.live_solution.svg".to_string()),
            false => None,
        };
        let mut svg_exporter = SvgExporter::new(final_svg_path, intermediate_svg_dir, live_svg_path);

        let solution = optimize(
            instance.clone(),
            rng,
            &mut svg_exporter,
            &mut terminator,
            &config.expl_cfg,
            &config.cmpr_cfg,
            initial_solution.as_ref(),
        );

        // Write output JSON
        let json_path = format!("{output_dir_str}/final_{}.json", ext_instance.name);
        let json_output = io::ExtSPOutput {
            instance: ext_instance,
            solution: jagua_rs::probs::spp::io::export(&instance, &solution, *crate::EPOCH),
        };
        if let Err(e) = io::write_json(&json_output, std::path::Path::new(&json_path), log::Level::Info) {
            eprintln!("[sparrow_ffi] failed to write output json: {e}");
            return 5;
        }

        0
    }));

    match result {
        Ok(code) => code,
        Err(_) => {
            eprintln!("[sparrow_ffi] panic during optimization");
            -1
        }
    }
}

/// Run optimization with full configuration control and callback support.
///
/// # Parameters
/// - `input_path`: path to the input JSON file.
/// - `output_dir`: directory where output files will be written.
/// - `config`: pointer to a CSparrowConfig struct with full configuration.
/// - `callback`: optional callback for solution reporting (null = no callback).
/// - `user_data`: user data pointer passed to the callback.
/// - `result`: pointer to a COptimizationResult that will be filled.
///
/// # Returns
/// 0 on success, non-zero on failure.
#[unsafe(no_mangle)]
pub extern "C" fn sparrow_optimize_ex(
    input_path: *const c_char,
    output_dir: *const c_char,
    config: *const CSparrowConfig,
    callback: Option<SolutionCallback>,
    user_data: *mut std::ffi::c_void,
    result: *mut COptimizationResult,
    emit_intermediate_json: i32,
) -> i32 {
    if result.is_null() {
        return -1;
    }

    let result_ref = unsafe { &mut *result };
    result_ref.error_code = 0;
    result_ref.error_message = std::ptr::null_mut();
    result_ref.output_json = std::ptr::null_mut();
    result_ref.strip_width = 0.0;
    result_ref.density = 0.0;
    result_ref.total_items = 0;

    let run = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let input_path_str = unsafe { cstr_to_str(input_path) };
        let output_dir_str = unsafe { cstr_to_str(output_dir) };

        if input_path_str.is_empty() {
            result_ref.error_code = 1;
            result_ref.error_message = to_cstring("input_path must not be null or empty");
            return;
        }

        let output_dir_str = if output_dir_str.is_empty() {
            "output"
        } else {
            output_dir_str
        };

        if let Err(e) = std::fs::create_dir_all(output_dir_str) {
            result_ref.error_code = 2;
            result_ref.error_message = to_cstring(&format!("failed to create output dir: {e}"));
            return;
        }

        // Build config
        let config = if config.is_null() {
            DEFAULT_SPARROW_CONFIG
        } else {
            build_config(unsafe { &*config })
        };

        // RNG
        let rng: Xoshiro256PlusPlus = match config.rng_seed {
            Some(seed) => Xoshiro256PlusPlus::seed_from_u64(seed as u64),
            None => Xoshiro256PlusPlus::seed_from_u64(rand::random()),
        };

        // Read input
        let input_path = std::path::Path::new(input_path_str);
        let (ext_instance, ext_solution) = match io::read_spp_input(input_path) {
            Ok(v) => v,
            Err(e) => {
                result_ref.error_code = 3;
                result_ref.error_message = to_cstring(&format!("failed to read input: {e}"));
                return;
            }
        };

        // Import instance
        let importer = Importer::new(
            config.cde_config,
            config.poly_simpl_tolerance,
            config.min_item_separation,
            config.narrow_concavity_cutoff_ratio,
        );
        let instance = match jagua_rs::probs::spp::io::import_instance(&importer, &ext_instance) {
            Ok(inst) => inst,
            Err(e) => {
                result_ref.error_code = 4;
                result_ref.error_message = to_cstring(&format!("failed to import instance: {e}"));
                return;
            }
        };

        let initial_solution = ext_solution.map(|e| {
            jagua_rs::probs::spp::io::import_solution(&instance, &e)
        });

        // Run optimization with or without callback
        let mut terminator = BasicTerminator::new();
        let solution = match callback {
            Some(cb) => {
                let mut listener = CallbackListener {
                    callback: cb,
                    user_data,
                    emit_json: emit_intermediate_json != 0,
                    last_emit: None,
                    min_interval: Duration::from_millis(150),
                };
                optimize(
                    instance.clone(),
                    rng,
                    &mut listener,
                    &mut terminator,
                    &config.expl_cfg,
                    &config.cmpr_cfg,
                    initial_solution.as_ref(),
                )
            }
            None => {
                let mut dummy_listener = DummySolListener;
                optimize(
                    instance.clone(),
                    rng,
                    &mut dummy_listener,
                    &mut terminator,
                    &config.expl_cfg,
                    &config.cmpr_cfg,
                    initial_solution.as_ref(),
                )
            }
        };

        // Fill result
        result_ref.strip_width = solution.strip_width();
        result_ref.density = solution.density(&instance);
        result_ref.total_items = instance.total_item_qty() as i32;

        // Write output JSON
        let json_output = io::ExtSPOutput {
            instance: ext_instance,
            solution: jagua_rs::probs::spp::io::export(&instance, &solution, *crate::EPOCH),
        };
        match serde_json::to_string_pretty(&json_output) {
            Ok(json_str) => {
                result_ref.output_json = to_cstring(&json_str);
            }
            Err(e) => {
                result_ref.error_code = 5;
                result_ref.error_message = to_cstring(&format!("failed to serialize output: {e}"));
            }
        }
    }));

    match run {
        Ok(()) => result_ref.error_code,
        Err(_) => {
            if result_ref.error_code == 0 {
                result_ref.error_code = -1;
                result_ref.error_message = to_cstring("panic during optimization");
            }
            result_ref.error_code
        }
    }
}

/// Get the default configuration as a CSparrowConfig struct.
/// The caller provides a pointer to a CSparrowConfig that will be filled.
#[unsafe(no_mangle)]
pub extern "C" fn sparrow_get_default_config(config: *mut CSparrowConfig) {
    if config.is_null() {
        return;
    }
    let cfg = unsafe { &mut *config };
    let def = DEFAULT_SPARROW_CONFIG;

    cfg.rng_seed = 0;
    cfg.poly_simpl_tolerance = def.poly_simpl_tolerance.unwrap_or(-1.0);
    cfg.min_item_separation = def.min_item_separation.unwrap_or(-1.0);
    cfg.narrow_concavity_cutoff_dist = def.narrow_concavity_cutoff_ratio.map(|(d, _)| d).unwrap_or(-1.0);
    cfg.narrow_concavity_cutoff_area = def.narrow_concavity_cutoff_ratio.map(|(_, a)| a).unwrap_or(-1.0);

    cfg.exploration = CExplorationConfig {
        shrink_step: def.expl_cfg.shrink_step,
        time_limit_secs: def.expl_cfg.time_limit.as_secs_f32(),
        max_conseq_failed_attempts: def.expl_cfg.max_conseq_failed_attempts.map(|v| v as i32).unwrap_or(-1),
        solution_pool_distribution_stddev: def.expl_cfg.solution_pool_distribution_stddev,
        large_item_ch_area_cutoff_percentile: def.expl_cfg.large_item_ch_area_cutoff_percentile,
        separator: CSeparatorConfig {
            iter_no_imprv_limit: def.expl_cfg.separator_config.iter_no_imprv_limit as i32,
            strike_limit: def.expl_cfg.separator_config.strike_limit as i32,
            n_workers: def.expl_cfg.separator_config.n_workers as i32,
            n_container_samples: def.expl_cfg.separator_config.sample_config.n_container_samples as i32,
            n_focussed_samples: def.expl_cfg.separator_config.sample_config.n_focussed_samples as i32,
            n_coord_descents: def.expl_cfg.separator_config.sample_config.n_coord_descents as i32,
        },
    };

    cfg.compression = CCompressionConfig {
        shrink_range_min: def.cmpr_cfg.shrink_range.0,
        shrink_range_max: def.cmpr_cfg.shrink_range.1,
        time_limit_secs: def.cmpr_cfg.time_limit.as_secs_f32(),
        shrink_decay_strategy: match def.cmpr_cfg.shrink_decay {
            ShrinkDecayStrategy::TimeBased => 0,
            ShrinkDecayStrategy::FailureBased(_) => 1,
        },
        shrink_decay_ratio: match def.cmpr_cfg.shrink_decay {
            ShrinkDecayStrategy::FailureBased(r) => r,
            _ => 0.9,
        },
        separator: CSeparatorConfig {
            iter_no_imprv_limit: def.cmpr_cfg.separator_config.iter_no_imprv_limit as i32,
            strike_limit: def.cmpr_cfg.separator_config.strike_limit as i32,
            n_workers: def.cmpr_cfg.separator_config.n_workers as i32,
            n_container_samples: def.cmpr_cfg.separator_config.sample_config.n_container_samples as i32,
            n_focussed_samples: def.cmpr_cfg.separator_config.sample_config.n_focussed_samples as i32,
            n_coord_descents: def.cmpr_cfg.separator_config.sample_config.n_coord_descents as i32,
        },
    };
}

/// Free a string previously returned by the library.
///
/// # Safety
/// `ptr` must have been returned by a sparrow FFI function that allocates strings.
#[unsafe(no_mangle)]
pub extern "C" fn sparrow_free_string(ptr: *mut c_char) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        let _ = CString::from_raw(ptr);
    }
}

/// Free a COptimizationResult's allocated strings.
///
/// # Safety
/// `result` must point to a valid COptimizationResult that was filled by sparrow_optimize_ex.
#[unsafe(no_mangle)]
pub extern "C" fn sparrow_free_result(result: *mut COptimizationResult) {
    if result.is_null() {
        return;
    }
    let r = unsafe { &mut *result };
    if !r.error_message.is_null() {
        unsafe { let _ = CString::from_raw(r.error_message); }
        r.error_message = std::ptr::null_mut();
    }
    if !r.output_json.is_null() {
        unsafe { let _ = CString::from_raw(r.output_json); }
        r.output_json = std::ptr::null_mut();
    }
}

/// Return the library version as a static C string.
/// The returned pointer must NOT be freed.
#[unsafe(no_mangle)]
pub extern "C" fn sparrow_version() -> *const c_char {
    concat!(env!("CARGO_PKG_VERSION"), "\0").as_ptr() as *const c_char
}

// ── Internal helpers ───────────────────────────────────────────────

fn to_cstring(s: &str) -> *mut c_char {
    CString::new(s).unwrap_or_else(|_| CString::new("string conversion error").unwrap()).into_raw()
}

