#ifndef SPARROW_H
#define SPARROW_H

#ifdef __cplusplus
extern "C" {
#endif

#include <stdint.h>

/* ════════════════════════════════════════════════════════════════
 *  Configuration structs  (must match src/ffi.rs #[repr(C)] layout)
 * ════════════════════════════════════════════════════════════════ */

/** Separator configuration (shared by exploration & compression). */
typedef struct {
    int32_t iter_no_imprv_limit;
    int32_t strike_limit;
    int32_t n_workers;
    int32_t n_container_samples;
    int32_t n_focussed_samples;
    int32_t n_coord_descents;
} CSeparatorConfig;

/** Exploration phase configuration. */
typedef struct {
    float            shrink_step;
    float            time_limit_secs;
    int32_t          max_conseq_failed_attempts;   /* -1 = None */
    float            solution_pool_distribution_stddev;
    float            large_item_ch_area_cutoff_percentile;
    CSeparatorConfig separator;
} CExplorationConfig;

/** Compression phase configuration. */
typedef struct {
    float            shrink_range_min;
    float            shrink_range_max;
    float            time_limit_secs;
    int32_t          shrink_decay_strategy;         /* 0 = TimeBased, 1 = FailureBased */
    float            shrink_decay_ratio;            /* only used when FailureBased */
    CSeparatorConfig separator;
} CCompressionConfig;

/** Full sparrow optimizer configuration. */
typedef struct {
    uint64_t           rng_seed;                    /* 0 = random */
    float              poly_simpl_tolerance;        /* negative = None */
    float              min_item_separation;         /* negative = None */
    float              narrow_concavity_cutoff_dist;/* negative = None */
    float              narrow_concavity_cutoff_area;/* negative = None */
    CExplorationConfig exploration;
    CCompressionConfig compression;
} CSparrowConfig;

/**
 * Optimization result.
 * `error_message` and `output_json` are heap-allocated by the library and
 * MUST be released — either individually with sparrow_free_string(), or all
 * at once with sparrow_free_result().
 */
typedef struct {
    float   strip_width;     /* final strip width */
    float   density;         /* final density (0.0 - 1.0) */
    int32_t total_items;     /* number of placed items */
    int32_t error_code;      /* 0 = success, non-zero = error code */
    char   *error_message;   /* NULL unless an error occurred (must be freed) */
    char   *output_json;     /* full result JSON, in-memory (must be freed) */
} COptimizationResult;

/* ════════════════════════════════════════════════════════════════
 *  Callback type
 * ════════════════════════════════════════════════════════════════ */

/**
 * Progress callback, invoked during optimization whenever a solution is reported.
 *
 * @param report_type   0=ExplFeas, 1=ExplInfeas, 2=ExplImproving, 3=CmprFeas, 4=Final
 * @param strip_width   current strip width
 * @param density       current density (0.0 - 1.0)
 * @param solution_json solution-only JSON (UTF-8, null-terminated) for this report,
 *                      or NULL when none is attached (e.g. feasibility-probe reports,
 *                      throttled reports, or when emit_intermediate_json is 0).
 *                      Valid ONLY during the callback — the library frees it right
 *                      after the callback returns, so copy it if you need to keep it.
 * @param user_data     opaque pointer passed through from sparrow_optimize_ex()
 */
typedef void (*SolutionCallback)(
    int32_t     report_type,
    float       strip_width,
    float       density,
    const char *solution_json,
    void       *user_data
);

/* ════════════════════════════════════════════════════════════════
 *  Functions
 * ════════════════════════════════════════════════════════════════ */

/**
 * Run the full sparrow optimization pipeline on a JSON input file.
 * Writes the result JSON to a file in `output_dir`.
 *
 * @param input_path       Path to the input JSON file (instance, or instance+solution).
 * @param output_dir       Directory where output files are written (NULL = "output").
 * @param global_time_secs Total time budget in seconds (0 = default 600s).
 * @param rng_seed         RNG seed (0 = random).
 * @param early_termination Non-zero enables early-termination heuristics.
 * @return 0 on success, non-zero on failure.
 */
int32_t sparrow_optimize(
    const char *input_path,
    const char *output_dir,
    uint64_t    global_time_secs,
    uint64_t    rng_seed,
    int32_t     early_termination
);

/**
 * Run optimization with full configuration control and optional progress callback.
 * The result is returned IN MEMORY via `result` (no file is written) — including
 * the complete output JSON in `result->output_json`.
 *
 * @param input_path Path to the input JSON file.
 * @param output_dir Output directory (used only for directory creation; NULL = "output").
 * @param config     Pointer to a CSparrowConfig (NULL = default config).
 * @param callback   Optional progress callback (NULL = none).
 * @param user_data  Opaque pointer forwarded to the callback.
 * @param result     Pointer to a COptimizationResult that will be filled.
 * @param emit_intermediate_json  When non-zero AND a callback is supplied, the callback
 *                   receives a solution-only JSON string on improving/final reports
 *                   (throttled to ~150 ms; Final always emits). 0 = scalars only.
 * @return 0 on success, non-zero on failure (also stored in result->error_code).
 */
int32_t sparrow_optimize_ex(
    const char                *input_path,
    const char                *output_dir,
    const CSparrowConfig      *config,
    SolutionCallback           callback,
    void                      *user_data,
    COptimizationResult       *result,
    int32_t                    emit_intermediate_json
);

/**
 * Fill a CSparrowConfig struct with the library's default configuration.
 * @param config Pointer to a caller-allocated CSparrowConfig to populate.
 */
void sparrow_get_default_config(CSparrowConfig *config);

/**
 * Free a string previously returned by the library (e.g. output_json, error_message).
 * @param ptr Pointer to the string to free (may be NULL).
 */
void sparrow_free_string(char *ptr);

/**
 * Free all heap-allocated strings inside a COptimizationResult
 * (error_message + output_json) and NULL them out.
 * @param result Pointer to a result filled by sparrow_optimize_ex (may be NULL).
 */
void sparrow_free_result(COptimizationResult *result);

/**
 * Return the library version as a static C string.
 * The returned pointer must NOT be freed.
 * @return Static version string (e.g., "0.1.0").
 */
const char *sparrow_version(void);

#ifdef __cplusplus
}
#endif

#endif /* SPARROW_H */
