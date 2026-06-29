//! 커스텀 알고리즘 변형 모음 (upstream 원본에는 없는 fork 전용 파일).
//!
//! 설계 원칙 — **원본 파일(`explore.rs`)을 직접 뜯어고치지 않는다.**
//!   1. 원본과 "동일한 시그니처"의 새 함수를 여기(새 파일)에 만든다.
//!   2. `optimizer/mod.rs::optimize()` 에서 `#[cfg(feature = "mymods")]`로
//!      원본 함수와 이 변형 함수를 컴파일 타임에 갈아끼운다.
//!
//! 이렇게 하면:
//!   - upstream(`git merge upstream/main`) 시 충돌이 거의 없다.
//!   - `--features mymods` 를 켜고 끄며 원본 vs 내 알고리즘을 A/B 비교할 수 있다.
//!
//! 사용:  cargo +nightly run --bin sparrow-cli --features mymods -- -i data/... -t 30

use crate::config::ExplorationConfig;
use crate::optimizer::explore::disrupt_solution; // explore.rs에서 pub(crate)로 연 헬퍼 재사용
use crate::optimizer::separator::Separator;
use crate::util::listener::{ReportType, SolutionListener};
use crate::util::terminator::Terminator;
use crate::FMT;
use jagua_rs::probs::spp::entities::{SPInstance, SPSolution};
use log::info;
use rand::prelude::Distribution;
use rand_distr::Normal;

/// 원본 `exploration_phase`(Algorithm 12)의 커스텀 변형.
///
/// **차이점은 단 하나** — strip 폭을 줄이는 `shrink_step` 을 고정값이 아니라
/// 현재 밀도(density)에 따라 동적으로 조절한다("밀도 기반 적응형 shrink").
///
/// 직관:
///   - 밀도가 낮다 = 아직 빈 공간이 많다 → **과감히 크게** 줄여 빠르게 수렴
///   - 밀도가 높다 = 거의 꽉 찼다(목표 근접) → **신중히 작게** 줄여 infeasible 폭주 방지
///
/// 공식:  `adaptive_step = base_step * clamp(1 + GAIN*(1 - density), 1, MAX_MULT)`
///   - density 0.50 → mult≈3.0 (3배 빠르게)
///   - density 0.90 → mult≈1.4
///   - density 0.97 → mult≈1.12 (원본과 거의 동일)
///
/// 나머지 로직(separate / infeasible 풀 관리 / rollback / disrupt)은 원본과 100% 동일.
pub fn exploration_phase_adaptive(
    instance: &SPInstance,
    sep: &mut Separator,
    sol_listener: &mut impl SolutionListener,
    term: &impl Terminator,
    config: &ExplorationConfig,
) -> Vec<SPSolution> {
    // ── 적응형 shrink 튜닝 파라미터 ──
    const GAIN: f32 = 4.0; // 적응 강도 (클수록 저밀도에서 더 공격적)
    const MAX_MULT: f32 = 5.0; // base_step 대비 최대 배율 상한

    info!("[mymods] exploration_phase_adaptive ACTIVE (GAIN={GAIN}, MAX_MULT={MAX_MULT})");

    let mut current_width = sep.prob.strip_width();
    let mut best_width = current_width;

    let mut feasible_sols = vec![sep.prob.save()];

    sol_listener.report(ReportType::ExplFeas, &feasible_sols[0], instance);
    info!(
        "[EXPL] starting optimization with initial width: {:.3} ({:.3}%)",
        current_width,
        sep.prob.density() * 100.0
    );

    let mut infeas_sol_pool: Vec<(SPSolution, f32)> = vec![];

    while !term.kill() {
        // Attempt to separate the current layout
        let local_best = sep.separate(term, sol_listener);
        let total_loss = local_best.1.get_total_loss();

        if total_loss == 0.0 {
            // If successfully separated
            if current_width < best_width {
                info!(
                    "[EXPL] feasible solution found! (width: {:.3}, dens: {:.3}%)",
                    current_width,
                    sep.prob.density() * 100.0
                );
                best_width = current_width;
                feasible_sols.push(local_best.0.clone());
                sol_listener.report(ReportType::ExplFeas, &local_best.0, instance);
            }

            // ★★★ 여기가 원본과 유일하게 다른 부분: 밀도 기반 적응형 shrink_step ★★★
            let density = sep.prob.density(); // 0.0 ~ 1.0
            let mult = (1.0 + GAIN * (1.0 - density)).clamp(1.0, MAX_MULT);
            let adaptive_step = config.shrink_step * mult;
            let next_width = current_width * (1.0 - adaptive_step);
            info!(
                "[mymods] adaptive shrink: dens={:.3}% mult={:.2} step={:.4}% : {:.3} -> {:.3}",
                density * 100.0,
                mult,
                adaptive_step * 100.0,
                current_width,
                next_width
            );
            sep.change_strip_width(next_width, None);
            current_width = next_width;
            infeas_sol_pool.clear();
        } else {
            info!(
                "[EXPL] unable to reach feasibility (width: {:.3}, dens: {:.3}%, min loss: {:.3})",
                current_width,
                sep.prob.density() * 100.0,
                FMT().fmt2(total_loss)
            );
            sol_listener.report(ReportType::ExplInfeas, &local_best.0, instance);

            // Separation was not successful add it to the pool of infeasible solutions
            match infeas_sol_pool.binary_search_by(|(_, o)| o.partial_cmp(&total_loss).unwrap()) {
                Ok(idx) | Err(idx) => infeas_sol_pool.insert(idx, (local_best.0.clone(), total_loss)),
            }

            if infeas_sol_pool.len() >= config.max_conseq_failed_attempts.unwrap_or(usize::MAX) {
                info!(
                    "[EXPL] max consecutive failed attempts ({}), terminating",
                    infeas_sol_pool.len()
                );
                break;
            }

            // Restore to a random solution from the pool (better solutions favoured)
            let selected_sol = {
                let distribution = Normal::new(0.0, config.solution_pool_distribution_stddev).unwrap();
                let sample = distribution.sample(&mut sep.rng).abs().min(0.999);
                let selected_idx = (sample * infeas_sol_pool.len() as f32) as usize;

                let (selected_sol, loss) = &infeas_sol_pool[selected_idx];
                info!(
                    "[EXPL] starting solution {}/{} selected from solution pool (l: {}) to disrupt",
                    selected_idx,
                    infeas_sol_pool.len(),
                    FMT().fmt2(*loss)
                );
                selected_sol
            };

            // Rollback to this solution and disrupt it (reuse upstream helper).
            sep.rollback(selected_sol, None);
            disrupt_solution(sep, config);
        }
    }

    info!(
        "[EXPL] finished, best feasible solution: width: {:.3} ({:.3}%)",
        best_width,
        feasible_sols.last().unwrap().density(instance) * 100.0
    );

    feasible_sols
}
