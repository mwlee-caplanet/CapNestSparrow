//! Step-by-step FFI API for the Sparrow optimization pipeline.
//!
//! Instead of a single `sparrow_optimize` call, this module lets C# (or any
//! C-ABI consumer) drive the optimization **phase-by-phase**:
//!
//!   h = sparrow_init(input_path, output_dir)
//!   sparrow_explore_phase(h)       // run the full exploration phase
//!   sparrow_compress_phase(h)      // run the full compression phase
//!   json = sparrow_get_solution_json(h)
//!   sparrow_free_string(json)      // reuse ffi::sparrow_free_string
//!   sparrow_free(h)
//!
//! SVG files (final / intermediate / live) are written automatically
//! via `SvgExporter` during each phase.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use jagua_rs::io::import::Importer;
use jagua_rs::probs::spp::entities::{SPInstance, SPProblem, SPSolution};
use jagua_rs::probs::spp::io::ext_repr::ExtSPInstance;
use rand::rngs::Xoshiro256PlusPlus;
use rand::{Rng, SeedableRng};

use crate::config::*;
use crate::consts::*;
use crate::optimizer::compress::compression_phase;
use crate::optimizer::lbf::LBFBuilder;
use crate::optimizer::separator::Separator;
use crate::util::io::{self, ExtSPOutput};
use crate::util::listener::SolutionListener;
use crate::util::listener::ReportType;
use crate::util::svg_exporter::SvgExporter;
use crate::util::terminator::{BasicTerminator, Terminator};

use jagua_rs::Instant;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

// ── Phase ───────────────────────────────────────────────────────────

#[derive(PartialEq)]
enum Phase {
    Init,
    Exploration,
    Compression,
    Done,
}

// ── Opaque handle ───────────────────────────────────────────────────

pub struct SparrowStepHandle {
    instance:     SPInstance,
    ext_instance: ExtSPInstance,
    prob:         SPProblem,
    rng:          Xoshiro256PlusPlus,
    expl_config:  ExplorationConfig,
    cmpr_config:  CompressionConfig,
    phase:        Phase,
    solutions:    Vec<SPSolution>,
    solution:     Option<SPSolution>,
    stop_flag:    Arc<AtomicBool>,
    solution_callback:     Option<SolutionCallback>,
    callback_user_data:    *mut std::ffi::c_void,
    emit_intermediate_json: bool,
    svg_exporter: SvgExporter,
    output_dir:   String,
    name:         String,
}

// ── Helpers ─────────────────────────────────────────────────────────

/// Callback type: matches ffi::SolutionCallback.
/// report_type: 0=ExplFeas,1=ExplInfeas,2=ExplImproving,3=CmprFeas,4=Final
/// solution_json: valid only during the callback (NULL = scalar-only report)
type SolutionCallback = extern "C" fn(i32, f32, f32, *const c_char, *mut std::ffi::c_void);

unsafe fn cstr_to_str<'a>(ptr: *const c_char) -> &'a str {
    if ptr.is_null() { "" } else { unsafe { CStr::from_ptr(ptr) }.to_str().unwrap_or("") }
}

fn next_rng(rng: &mut Xoshiro256PlusPlus) -> Xoshiro256PlusPlus {
    Xoshiro256PlusPlus::seed_from_u64(rng.next_u64())
}

fn build_svg_exporter(output_dir: &str, instance_name: &str) -> SvgExporter {
    let final_svg = Some(format!("{output_dir}/final_{instance_name}.svg"));
    let intermediate_dir = match cfg!(feature = "only_final_svg") {
        true => None,
        false => Some(format!("{output_dir}/sols_{instance_name}")),
    };
    let live_svg = match cfg!(feature = "live_svg") {
        true => Some("data/live/.live_solution.svg".to_string()),
        false => None,
    };
    SvgExporter::new(final_svg, intermediate_dir, live_svg)
}

// ── CompositeListener (ponytail: chain svg + callback, avoids boxing) ──

struct CompositeListener<'a> {
    svg: &'a mut SvgExporter,
    cb: &'a mut CallbackListener,
}

impl SolutionListener for CompositeListener<'_> {
    fn report(&mut self, report: ReportType, solution: &SPSolution, instance: &SPInstance) {
        self.svg.report(report.clone(), solution, instance);
        self.cb.report(report, solution, instance);
    }
}

// ── CallbackListener (ponytail: duplicated from ffi.rs, stable 50 lines) ──

struct CallbackListener {
    callback: SolutionCallback,
    user_data: *mut std::ffi::c_void,
    emit_json: bool,
    last_emit: Option<Instant>,
    min_interval: Duration,
}

impl SolutionListener for CallbackListener {
    fn report(&mut self, report: ReportType, solution: &SPSolution, instance: &SPInstance) {
        let report_type = match report {
            ReportType::ExplFeas => 0,
            ReportType::ExplInfeas => 1,
            ReportType::ExplImproving => 2,
            ReportType::CmprFeas => 3,
            ReportType::Final => 4,
        };
        let strip_width = solution.strip_width();
        let density = solution.density(instance);

        let improving = matches!(report, ReportType::ExplImproving | ReportType::CmprFeas | ReportType::Final);
        let is_final = matches!(report, ReportType::Final);
        let throttled = match self.last_emit {
            Some(t) => t.elapsed() < self.min_interval,
            None => false,
        };

        if self.emit_json && improving && (is_final || !throttled) {
            let ext_sol = jagua_rs::probs::spp::io::export(instance, solution, *crate::EPOCH);
            if let Ok(json) = serde_json::to_string(&ext_sol) {
                if let Ok(cs) = CString::new(json) {
                    (self.callback)(report_type, strip_width, density, cs.as_ptr(), self.user_data);
                    self.last_emit = Some(Instant::now());
                    return;
                }
            }
        }
        (self.callback)(report_type, strip_width, density, std::ptr::null(), self.user_data);
    }
}

// ── sparrow_set_callback ────────────────────────────────────────────

/// Register a solution callback for intermediate/final progress.
/// Call BEFORE sparrow_explore_phase.  Pass null callback to disable.
/// emit_json: 1 = include solution JSON on improving/final reports, 0 = scalar only.
#[unsafe(no_mangle)]
pub extern "C" fn sparrow_set_callback(
    handle: *mut SparrowStepHandle,
    callback: Option<SolutionCallback>,
    user_data: *mut std::ffi::c_void,
    emit_json: i32,
) {
    if handle.is_null() { return; }
    let h = unsafe { &mut *handle };
    h.solution_callback = callback;
    h.callback_user_data = user_data;
    h.emit_intermediate_json = emit_json != 0;
}

// ── sparrow_init ────────────────────────────────────────────────────

/// Load input, build initial solution, create SVG exporter.
/// Returns null on failure.
#[unsafe(no_mangle)]
pub extern "C" fn sparrow_init(
    input_path: *const c_char,
    output_dir: *const c_char,
) -> *mut SparrowStepHandle {
    let run = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| -> Option<*mut SparrowStepHandle> {
        let input_str = unsafe { cstr_to_str(input_path) };
        let out_str   = unsafe { cstr_to_str(output_dir) };
        let out_str   = if out_str.is_empty() { "output" } else { out_str };

        if input_str.is_empty() { return None; }
        std::fs::create_dir_all(out_str).ok()?;

        let input_p = std::path::Path::new(input_str);
        let (ext_inst, ext_sol) = io::read_spp_input(input_p).ok()?;
        let inst_name = ext_inst.name.clone();

        let config = DEFAULT_SPARROW_CONFIG;

        let mut rng: Xoshiro256PlusPlus = match config.rng_seed {
            Some(s) => Xoshiro256PlusPlus::seed_from_u64(s as u64),
            None    => Xoshiro256PlusPlus::seed_from_u64(rand::random()),
        };

        let importer = Importer::new(
            config.cde_config,
            config.poly_simpl_tolerance,
            config.min_item_separation,
            config.narrow_concavity_cutoff_ratio,
        );
        let instance = jagua_rs::probs::spp::io::import_instance(&importer, &ext_inst).ok()?;

        let prob = match ext_sol {
            None => {
                let builder = LBFBuilder::new(instance.clone(), next_rng(&mut rng), LBF_SAMPLE_CONFIG).construct();
                builder.prob
            }
            Some(ref sol) => {
                let init = jagua_rs::probs::spp::io::import_solution(&instance, sol);
                let mut p = SPProblem::new(instance.clone());
                p.restore(&init);
                p
            }
        };

        let svg_exporter = build_svg_exporter(out_str, &inst_name);

        let handle = Box::new(SparrowStepHandle {
            instance,
            ext_instance: ext_inst,
            prob,
            rng,
            expl_config: config.expl_cfg.clone(),
            cmpr_config: config.cmpr_cfg.clone(),
            phase: Phase::Init,
            solutions: vec![],
            solution: None,
            stop_flag: Arc::new(AtomicBool::new(false)),
            solution_callback: None,
            callback_user_data: std::ptr::null_mut(),
            emit_intermediate_json: false,
            svg_exporter,
            output_dir: out_str.to_string(),
            name: inst_name,
        });
        Some(Box::into_raw(handle))
    }));

    match run {
        Ok(Some(ptr)) => ptr,
        _ => std::ptr::null_mut(),
    }
}

// ── sparrow_explore_phase ───────────────────────────────────────────

/// Run the full exploration phase.  Returns 0 on success, -1 on error.
#[unsafe(no_mangle)]
pub extern "C" fn sparrow_explore_phase(handle: *mut SparrowStepHandle) -> i32 {
    if handle.is_null() { return -1; }
    let h = unsafe { &mut *handle };
    if h.phase != Phase::Init { return -1; }

    let run = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut term = CancellableTerminator {
            inner: BasicTerminator::new(),
            stop: h.stop_flag.clone(),
        };
        term.new_timeout(h.expl_config.time_limit);
        let mut sep = Separator::new(
            h.instance.clone(), h.prob.clone(), next_rng(&mut h.rng), h.expl_config.separator_config,
        );

        // ponytail: branch listener dispatch (generic fn, can't dyn)
        let solutions = if let Some(cb) = h.solution_callback {
            let mut cb_listener = CallbackListener {
                callback: cb,
                user_data: h.callback_user_data,
                emit_json: h.emit_intermediate_json,
                last_emit: None,
                min_interval: Duration::from_millis(500),
            };
            let mut composite = CompositeListener { svg: &mut h.svg_exporter, cb: &mut cb_listener };

            #[cfg(not(feature = "mymods"))]
            let solutions = crate::optimizer::explore::exploration_phase(
                &h.instance, &mut sep, &mut composite, &term, &h.expl_config,
            );
            #[cfg(feature = "mymods")]
            let solutions = crate::optimizer::explore_mods::exploration_phase_adaptive(
                &h.instance, &mut sep, &mut composite, &term, &h.expl_config,
            );
            solutions
        } else {
            #[cfg(not(feature = "mymods"))]
            let solutions = crate::optimizer::explore::exploration_phase(
                &h.instance, &mut sep, &mut h.svg_exporter, &term, &h.expl_config,
            );
            #[cfg(feature = "mymods")]
            let solutions = crate::optimizer::explore_mods::exploration_phase_adaptive(
                &h.instance, &mut sep, &mut h.svg_exporter, &term, &h.expl_config,
            );
            solutions
        };

        h.solutions = solutions;
        h.prob = sep.prob;    // carry forward the mutated problem
        h.phase = Phase::Exploration;
    }));

    match run {
        Ok(()) => 0,
        Err(_) => -1,
    }
}

// ── sparrow_compress_phase ──────────────────────────────────────────

/// Run the full compression phase.  Returns 0 on success, -1 on error.
#[unsafe(no_mangle)]
pub extern "C" fn sparrow_compress_phase(handle: *mut SparrowStepHandle) -> i32 {
    if handle.is_null() { return -1; }
    let h = unsafe { &mut *handle };
    if h.phase != Phase::Exploration { return -1; }

    let final_explore = match h.solutions.last() {
        Some(s) => s.clone(),
        None => return -1,
    };

    let run = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut term = CancellableTerminator {
            inner: BasicTerminator::new(),
            stop: h.stop_flag.clone(),
        };
        term.new_timeout(h.cmpr_config.time_limit);
        let mut sep = Separator::new(
            h.instance.clone(), h.prob.clone(), next_rng(&mut h.rng), h.cmpr_config.separator_config,
        );

        // ponytail: branch listener dispatch (same pattern as explore)
        let cmpr_sol = if let Some(cb) = h.solution_callback {
            let mut cb_listener = CallbackListener {
                callback: cb,
                user_data: h.callback_user_data,
                emit_json: h.emit_intermediate_json,
                last_emit: None,
                min_interval: Duration::from_millis(500),
            };
            let mut composite = CompositeListener { svg: &mut h.svg_exporter, cb: &mut cb_listener };
            let sol = compression_phase(
                &h.instance, &mut sep, &final_explore, &mut composite, &term, &h.cmpr_config,
            );
            composite.report(ReportType::Final, &sol, &h.instance);
            sol
        } else {
            let sol = compression_phase(
                &h.instance, &mut sep, &final_explore, &mut h.svg_exporter, &term, &h.cmpr_config,
            );
            h.svg_exporter.report(ReportType::Final, &sol, &h.instance);
            sol
        };
        h.solution = Some(cmpr_sol);
        h.phase = Phase::Compression;
    }));

    match run {
        Ok(()) => 0,
        Err(_) => -1,
    }
}

// ── sparrow_get_solution_json ───────────────────────────────────────

/// Return the final solution as a JSON string.
/// The caller must free it with `sparrow_free_string`.
/// Returns null if no solution is available or on error.
#[unsafe(no_mangle)]
pub extern "C" fn sparrow_get_solution_json(handle: *mut SparrowStepHandle) -> *mut c_char {
    if handle.is_null() { return std::ptr::null_mut(); }
    let h = unsafe { &mut *handle };

    let sol = match &h.solution {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };

    let json_output = ExtSPOutput {
        instance: h.ext_instance.clone(),
        solution: jagua_rs::probs::spp::io::export(&h.instance, sol, *crate::EPOCH),
    };

    let json_str = match serde_json::to_string(&json_output) {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

    match CString::new(json_str) {
        Ok(cs) => cs.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

// ── sparrow_get_solution_svg ────────────────────────────────────────

/// Return the final-solution SVG as a string (same content written to disk).
/// The caller must free it with `sparrow_free_string`.
/// Returns null if no solution is available or on error.
#[unsafe(no_mangle)]
pub extern "C" fn sparrow_get_solution_svg(handle: *mut SparrowStepHandle) -> *mut c_char {
    if handle.is_null() { return std::ptr::null_mut(); }
    let h = unsafe { &mut *handle };

    let sol = match &h.solution {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };

    let svg = jagua_rs::io::svg::s_layout_to_svg(
        &sol.layout_snapshot,
        &h.instance,
        crate::consts::DRAW_OPTIONS,
        &h.name,
    );

    match CString::new(svg.to_string()) {
        Ok(cs) => cs.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

// ── sparrow_free ────────────────────────────────────────────────────

// ── CancellableTerminator ───────────────────────────────────────────

struct CancellableTerminator {
    inner: BasicTerminator,
    stop: Arc<AtomicBool>,
}

impl Terminator for CancellableTerminator {
    fn kill(&self) -> bool {
        self.stop.load(Ordering::Relaxed) || self.inner.kill()
    }
    fn new_timeout(&mut self, timeout: Duration) {
        self.inner.new_timeout(timeout)
    }
    fn timeout_at(&self) -> Option<Instant> {
        self.inner.timeout_at()
    }
}

// ── sparrow_request_stop ────────────────────────────────────────────

/// Signal the running phase to stop as soon as possible.
/// Safe to call from any thread while explore/compress is running.
#[unsafe(no_mangle)]
pub extern "C" fn sparrow_request_stop(handle: *mut SparrowStepHandle) {
    if !handle.is_null() {
        let h = unsafe { &*handle };
        h.stop_flag.store(true, Ordering::Relaxed);
    }
}

// ── sparrow_free ────────────────────────────────────────────────────

/// Free the handle and all associated resources.
#[unsafe(no_mangle)]
pub extern "C" fn sparrow_free(handle: *mut SparrowStepHandle) {
    if !handle.is_null() {
        let _ = unsafe { Box::from_raw(handle) };
    }
}
