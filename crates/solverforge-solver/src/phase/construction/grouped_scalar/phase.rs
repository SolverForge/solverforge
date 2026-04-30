use std::time::Instant;

use solverforge_config::{ConstructionHeuristicConfig, ConstructionHeuristicType};
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use solverforge_scoring::Director;
use tracing::info;

use crate::builder::context::{ScalarGroupContext, ScalarGroupLimits};
use crate::descriptor_scalar::ResolvedVariableBinding;
use crate::scope::{PhaseScope, ProgressCallback, SolverScope, StepScope};
use crate::stats::{format_duration, whole_units_per_second};

use super::candidate::normalize_grouped_candidates;
use super::selection::{select_candidate_for_next_group_slot, GroupedSelection};

pub(crate) fn solve_grouped_scalar_construction<S, D, ProgressCb>(
    config: Option<&ConstructionHeuristicConfig>,
    group_index: usize,
    group: &ScalarGroupContext<S>,
    scalar_bindings: &[ResolvedVariableBinding<S>],
    solver_scope: &mut SolverScope<'_, S, D, ProgressCb>,
) -> bool
where
    S: PlanningSolution + 'static,
    S::Score: Score + Copy,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    let construction_type = config
        .map(|cfg| cfg.construction_heuristic_type)
        .unwrap_or(ConstructionHeuristicType::FirstFit);
    let construction_obligation = config
        .map(|cfg| cfg.construction_obligation)
        .unwrap_or_default();
    let limits = ScalarGroupLimits {
        value_candidate_limit: config.and_then(|cfg| cfg.value_candidate_limit),
        group_candidate_limit: None,
        max_moves_per_step: None,
    };
    let group_candidate_limit = config.and_then(|cfg| cfg.group_candidate_limit);

    let phase_name = "Grouped Scalar Construction";
    let mut phase_scope = PhaseScope::with_phase_type(solver_scope, 0, phase_name);
    let phase_index = phase_scope.phase_index();
    let start_score = phase_scope
        .solver_scope()
        .current_score()
        .map(|score| score.to_string())
        .unwrap_or_else(|| "none".to_string());
    let mut ran_step = false;
    let mut last_progress_time = Instant::now();

    info!(
        event = "phase_start",
        phase = phase_name,
        group = group.group_name,
        phase_index = phase_index,
        score = start_score,
    );
    phase_scope.solver_scope().report_progress();

    loop {
        if phase_scope
            .solver_scope_mut()
            .should_terminate_construction()
        {
            break;
        }

        let candidates = normalize_grouped_candidates(
            &phase_scope,
            group_index,
            group,
            scalar_bindings,
            limits,
            group_candidate_limit,
        );
        let Some(selection) = select_candidate_for_next_group_slot(
            &mut phase_scope,
            candidates,
            construction_type,
            construction_obligation,
        ) else {
            break;
        };

        ran_step = true;
        match selection {
            GroupedSelection::Commit {
                group_slot,
                scalar_slots,
                mov,
                score,
            } => {
                let mut step_scope = StepScope::new(&mut phase_scope);
                step_scope.phase_scope_mut().record_move_accepted();
                step_scope.apply_committed_move(&mov);
                step_scope.phase_scope_mut().record_move_applied();
                step_scope
                    .phase_scope_mut()
                    .solver_scope_mut()
                    .mark_group_slot_completed(group_slot);
                for slot in scalar_slots {
                    step_scope
                        .phase_scope_mut()
                        .solver_scope_mut()
                        .mark_scalar_slot_completed(slot);
                }
                step_scope
                    .phase_scope_mut()
                    .record_construction_slot_assigned();
                step_scope.set_step_score(score);
                step_scope.complete();
                if last_progress_time.elapsed().as_secs() >= 1 {
                    step_scope.phase_scope().solver_scope().report_progress();
                    last_progress_time = Instant::now();
                }
            }
            GroupedSelection::CompleteOnly {
                group_slot,
                scalar_slots,
                kept,
            } => {
                let mut step_scope = StepScope::new(&mut phase_scope);
                step_scope
                    .phase_scope_mut()
                    .solver_scope_mut()
                    .mark_group_slot_completed(group_slot);
                for slot in scalar_slots {
                    step_scope
                        .phase_scope_mut()
                        .solver_scope_mut()
                        .mark_scalar_slot_completed(slot);
                }
                if kept {
                    step_scope.phase_scope_mut().record_construction_slot_kept();
                } else {
                    step_scope
                        .phase_scope_mut()
                        .record_construction_slot_no_doable();
                }
                let score = step_scope.calculate_score();
                step_scope.set_step_score(score);
                step_scope.complete();
                if last_progress_time.elapsed().as_secs() >= 1 {
                    step_scope.phase_scope().solver_scope().report_progress();
                    last_progress_time = Instant::now();
                }
            }
        }
    }

    if ran_step {
        phase_scope.update_best_solution();
    }

    phase_scope.solver_scope().report_progress();
    let best_score = phase_scope
        .solver_scope()
        .best_score()
        .map(|score| score.to_string())
        .unwrap_or_else(|| "none".to_string());
    let duration = phase_scope.elapsed();
    let steps = phase_scope.step_count();
    let speed = whole_units_per_second(steps, duration);
    let stats = phase_scope.stats();

    info!(
        event = "phase_end",
        phase = phase_name,
        group = group.group_name,
        phase_index = phase_index,
        duration = %format_duration(duration),
        steps = steps,
        moves_generated = stats.moves_generated,
        moves_evaluated = stats.moves_evaluated,
        moves_accepted = stats.moves_accepted,
        score_calculations = stats.score_calculations,
        generation_time = %format_duration(stats.generation_time()),
        evaluation_time = %format_duration(stats.evaluation_time()),
        speed = speed,
        score = best_score,
    );

    ran_step
}
