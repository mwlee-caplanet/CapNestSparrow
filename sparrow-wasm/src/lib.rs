use jagua_rs::probs::spp::io::ext_repr::ExtSPInstance;
use rand::Rng;
use rand::SeedableRng;
use sparrow_core::config::DEFAULT_SPARROW_CONFIG;
use sparrow_core::optimizer;
use sparrow_core::util::listener::DummySolListener;
use sparrow_core::util::terminator::BasicTerminator;
use std::time::Duration;
use wasm_bindgen::prelude::*;

// ── 초기화: panic 메시지를 브라우저 콘솔로 출력 ──
#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
    let _ = wasm_logger::init(wasm_logger::Config::default());
}

/// Sparrow nesting 최적화를 실행하고 결과 JSON을 반환합니다.
///
/// `instance_json` — SPP 인스턴스 JSON (ExtSPInstance 형식)
/// `time_limit_secs` — 전체 시간 제한 (초). exploration 90% + compression 10%로 분배
///
/// 반환: `ExtSPSolution` JSON 문자열
#[wasm_bindgen]
pub fn optimize(instance_json: &str, time_limit_secs: f64) -> Result<String, JsValue> {
    // ── 1. 입력 파싱 ──
    let ext_instance: ExtSPInstance = serde_json::from_str(instance_json)
        .map_err(|e| JsValue::from_str(&format!("JSON parse error: {e}")))?;

    // ── 2. config 구성 (기본값 + 시간 제한만 덮어쓰기) ──
    let mut config = DEFAULT_SPARROW_CONFIG;
    let total = Duration::from_secs_f64(time_limit_secs.max(1.0));
    config.expl_cfg.time_limit = total.mul_f64(0.9);
    config.cmpr_cfg.time_limit = total.mul_f64(0.1);
    // WASM은 싱글스레드이므로 worker 수를 1로 제한
    config.expl_cfg.separator_config.n_workers = 1;
    config.cmpr_cfg.separator_config.n_workers = 1;

    // ── 3. 내부 형식으로 import ──
    let importer = jagua_rs::io::import::Importer::new(
        config.cde_config,
        config.poly_simpl_tolerance,
        config.min_item_separation,
        config.narrow_concavity_cutoff_ratio,
    );
    let instance = jagua_rs::probs::spp::io::import_instance(&importer, &ext_instance)
        .map_err(|e| JsValue::from_str(&format!("Import error: {e}")))?;

    // export용 instance 복사본 (optimize가 소유권을 가져가므로 미리 clone)
    let instance_clone = instance.clone();

    // ── 4. 난수 생성기 (WASM에서는 crypto.getRandomValues 사용) ──
    let rng = match config.rng_seed {
        Some(seed) => rand::rngs::Xoshiro256PlusPlus::seed_from_u64(seed as u64),
        None => {
            let seed = rand::rng().next_u64();
            rand::rngs::Xoshiro256PlusPlus::seed_from_u64(seed)
        }
    };

    // ── 5. 최적화 실행 ──
    let mut terminator = BasicTerminator::new();
    let mut listener = DummySolListener;
    let solution = optimizer::optimize(
        instance,
        rng,
        &mut listener,
        &mut terminator,
        &config.expl_cfg,
        &config.cmpr_cfg,
        None,
    );

    // ── 6. 결과 직렬화 ──
    let ext_solution =
        jagua_rs::probs::spp::io::export(&instance_clone, &solution, jagua_rs::Instant::now());

    serde_json::to_string(&ext_solution)
        .map_err(|e| JsValue::from_str(&format!("Serialize error: {e}")))
}
