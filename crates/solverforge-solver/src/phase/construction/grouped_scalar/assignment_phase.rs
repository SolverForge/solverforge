use std::time::Instant;

use solverforge_config::{ConstructionHeuristicConfig, ConstructionObligation};
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use solverforge_scoring::Director;
use tracing::info;

use crate::builder::{ScalarAssignmentBinding, ScalarGroupLimits};
use crate::heuristic::r#move::{CompoundScalarMove, Move};
use crate::phase::construction::evaluation::evaluate_trial_move;
use crate::phase::hard_delta::{hard_score_delta, HardScoreDelta};
use crate::scope::{PhaseScope, ProgressCallback, SolverScope, StepScope};
use crate::stats::{format_duration, whole_units_per_second};

use super::assignment_candidate::{
    optional_assignment_moves, remaining_required_count, required_assignment_moves,
    ScalarAssignmentMoveOptions,
};

pub(super) fn solve_scalar_assignment_construction<S, D, ProgressCb>(
    config: Option<&ConstructionHeuristicConfig>,
    group_name: &'static str,
    group: ScalarAssignmentBinding<S>,
    limits: ScalarGroupLimits,
    solver_scope: &mut SolverScope<'_, S, D, ProgressCb>,
) -> bool
where
    S: PlanningSolution + 'static,
    S::Score: Score + Copy,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    let options = ScalarAssignmentMoveOptions::for_construction(
        limits,
        config.and_then(|cfg| cfg.value_candidate_limit),
        config.and_then(|cfg| cfg.group_candidate_limit),
    );
    let construction_obligation = config
        .map(|cfg| cfg.construction_obligation)
        .unwrap_or_default();
    let phase_name = "Scalar Assignment Construction";
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
        group = group_name,
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
        let remaining =
            remaining_required_count(&group, phase_scope.score_director().working_solution());
        if remaining == 0 {
            break;
        }
        let moves = required_assignment_moves(
            &group,
            phase_scope.score_director().working_solution(),
            options,
        );
        let Some((mov, score)) =
            select_required_move(&mut phase_scope, moves, construction_obligation)
        else {
            phase_scope.record_construction_slot_no_doable();
            phase_scope.record_scalar_assignment_required_remaining(group_name, remaining);
            break;
        };
        commit_assignment_move(&mut phase_scope, &mov, score, &mut ran_step);
        if last_progress_time.elapsed().as_secs() >= 1 {
            phase_scope.solver_scope().report_progress();
            last_progress_time = Instant::now();
        }
    }

    if remaining_required_count(&group, phase_scope.score_director().working_solution()) == 0 {
        loop {
            if phase_scope
                .solver_scope_mut()
                .should_terminate_construction()
            {
                break;
            }
            let moves = optional_assignment_moves(
                &group,
                phase_scope.score_director().working_solution(),
                options,
            );
            let Some((mov, score)) = select_optional_move(&mut phase_scope, moves) else {
                break;
            };
            commit_assignment_move(&mut phase_scope, &mov, score, &mut ran_step);
            if last_progress_time.elapsed().as_secs() >= 1 {
                phase_scope.solver_scope().report_progress();
                last_progress_time = Instant::now();
            }
        }
    }

    let remaining =
        remaining_required_count(&group, phase_scope.score_director().working_solution());
    phase_scope.record_scalar_assignment_required_remaining(group_name, remaining);
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
        group = group_name,
        phase_index = phase_index,
        required_remaining = remaining,
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

fn select_required_move<S, D, ProgressCb>(
    phase_scope: &mut PhaseScope<'_, '_, S, D, ProgressCb>,
    moves: Vec<CompoundScalarMove<S>>,
    construction_obligation: ConstructionObligation,
) -> Option<(CompoundScalarMove<S>, S::Score)>
where
    S: PlanningSolution,
    S::Score: Score + Copy,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    let current = phase_scope.calculate_score();
    let mode = match construction_obligation {
        ConstructionObligation::PreserveUnassigned => AssignmentSelectionMode::RequiredPreserve,
        ConstructionObligation::AssignWhenCandidateExists => {
            AssignmentSelectionMode::RequiredAssignWhenCandidateExists
        }
    };
    select_move(phase_scope, moves, current, mode)
}

fn select_optional_move<S, D, ProgressCb>(
    phase_scope: &mut PhaseScope<'_, '_, S, D, ProgressCb>,
    moves: Vec<CompoundScalarMove<S>>,
) -> Option<(CompoundScalarMove<S>, S::Score)>
where
    S: PlanningSolution,
    S::Score: Score + Copy,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    let current = phase_scope.calculate_score();
    select_move(
        phase_scope,
        moves,
        current,
        AssignmentSelectionMode::Optional,
    )
}

#[derive(Clone, Copy)]
enum AssignmentSelectionMode {
    RequiredPreserve,
    RequiredAssignWhenCandidateExists,
    Optional,
}

fn select_move<S, D, ProgressCb>(
    phase_scope: &mut PhaseScope<'_, '_, S, D, ProgressCb>,
    moves: Vec<CompoundScalarMove<S>>,
    current: S::Score,
    mode: AssignmentSelectionMode,
) -> Option<(CompoundScalarMove<S>, S::Score)>
where
    S: PlanningSolution,
    S::Score: Score + Copy,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    for mov in moves {
        let generation_started = Instant::now();
        phase_scope.record_generated_move(generation_started.elapsed());
        let evaluation_started = Instant::now();
        if !mov.is_doable(phase_scope.score_director()) {
            phase_scope.record_evaluated_move(evaluation_started.elapsed());
            phase_scope.record_move_not_doable();
            continue;
        }
        let score = evaluate_trial_move(phase_scope.score_director_mut(), &mov);
        phase_scope.record_score_calculation();
        phase_scope.record_evaluated_move(evaluation_started.elapsed());
        if matches!(
            mode,
            AssignmentSelectionMode::RequiredAssignWhenCandidateExists
        ) {
            return Some((mov, score));
        }
        let hard_delta = hard_score_delta(current, score);
        if hard_delta == Some(HardScoreDelta::Worse) {
            continue;
        }
        match mode {
            AssignmentSelectionMode::RequiredPreserve => {
                if hard_delta == Some(HardScoreDelta::Improving) || score >= current {
                    return Some((mov, score));
                }
            }
            AssignmentSelectionMode::RequiredAssignWhenCandidateExists => unreachable!(),
            AssignmentSelectionMode::Optional => {
                if score > current {
                    return Some((mov, score));
                }
            }
        }
    }
    None
}

fn commit_assignment_move<S, D, ProgressCb>(
    phase_scope: &mut PhaseScope<'_, '_, S, D, ProgressCb>,
    mov: &CompoundScalarMove<S>,
    score: S::Score,
    ran_step: &mut bool,
) where
    S: PlanningSolution,
    S::Score: Score + Copy,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    *ran_step = true;
    let mut step_scope = StepScope::new(phase_scope);
    step_scope.phase_scope_mut().record_move_accepted();
    step_scope.apply_committed_move(mov);
    step_scope.phase_scope_mut().record_move_applied();
    step_scope
        .phase_scope_mut()
        .record_construction_slot_assigned();
    step_scope.set_step_score(score);
    step_scope.complete();
}
