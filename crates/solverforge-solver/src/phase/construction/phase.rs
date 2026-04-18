// Construction heuristic phase implementation.

use std::any::Any;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::time::Instant;

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use solverforge_scoring::{Director, RecordingDirector};
use tracing::info;

use crate::heuristic::r#move::Move;
use crate::phase::construction::{
    BestFitForager, ConstructionForager, EntityPlacer, FirstFeasibleForager, FirstFitForager,
};
use crate::phase::control::{
    settle_construction_interrupt, should_interrupt_evaluation, StepInterrupt,
};
use crate::phase::Phase;
use crate::scope::ProgressCallback;
use crate::scope::{PhaseScope, SolverScope, StepScope};
use crate::stats::{format_duration, whole_units_per_second};

/// Construction heuristic phase that builds an initial solution.
///
/// This phase iterates over uninitialized entities and assigns values
/// to their planning variables using a greedy approach.
///
/// # Type Parameters
/// * `S` - The planning solution type
/// * `M` - The move type
/// * `P` - The entity placer type
/// * `Fo` - The forager type
pub struct ConstructionHeuristicPhase<S, M, P, Fo>
where
    S: PlanningSolution,
    M: Move<S>,
    P: EntityPlacer<S, M>,
    Fo: ConstructionForager<S, M>,
{
    placer: P,
    forager: Fo,
    _phantom: PhantomData<fn() -> (S, M)>,
}

impl<S, M, P, Fo> ConstructionHeuristicPhase<S, M, P, Fo>
where
    S: PlanningSolution,
    M: Move<S>,
    P: EntityPlacer<S, M>,
    Fo: ConstructionForager<S, M>,
{
    pub fn new(placer: P, forager: Fo) -> Self {
        Self {
            placer,
            forager,
            _phantom: PhantomData,
        }
    }
}

impl<S, M, P, Fo> Debug for ConstructionHeuristicPhase<S, M, P, Fo>
where
    S: PlanningSolution,
    M: Move<S>,
    P: EntityPlacer<S, M> + Debug,
    Fo: ConstructionForager<S, M> + Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConstructionHeuristicPhase")
            .field("placer", &self.placer)
            .field("forager", &self.forager)
            .finish()
    }
}

impl<S, D, BestCb, M, P, Fo> Phase<S, D, BestCb> for ConstructionHeuristicPhase<S, M, P, Fo>
where
    S: PlanningSolution,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
    M: Move<S> + 'static,
    P: EntityPlacer<S, M>,
    Fo: ConstructionForager<S, M> + 'static,
{
    fn solve(&mut self, solver_scope: &mut SolverScope<S, D, BestCb>) {
        let mut phase_scope = PhaseScope::new(solver_scope, 0);
        let phase_index = phase_scope.phase_index();

        info!(
            event = "phase_start",
            phase = "Construction Heuristic",
            phase_index = phase_index,
        );

        // Get all placements (entities that need values assigned)
        let placement_generation_started = Instant::now();
        let placements = self.placer.get_placements(phase_scope.score_director());
        let placement_generation_elapsed = placement_generation_started.elapsed();
        let generated_moves = placements
            .iter()
            .map(|placement| u64::try_from(placement.moves.len()).unwrap_or(u64::MAX))
            .sum();
        phase_scope.record_generated_batch(generated_moves, placement_generation_elapsed);
        let mut placements = placements.into_iter();
        let mut pending_placement = None;

        loop {
            // Construction must complete — only stop for external flag or time limit,
            // never for step/move count limits (those are for local search).
            if phase_scope
                .solver_scope_mut()
                .should_terminate_construction()
            {
                break;
            }

            let mut placement = match pending_placement.take().or_else(|| placements.next()) {
                Some(placement) => placement,
                None => break,
            };

            let mut step_scope = StepScope::new(&mut phase_scope);

            // Use forager to pick the best move index for this placement
            let selected_idx = match select_move_index(&self.forager, &placement, &mut step_scope) {
                ConstructionSelection::Selected(selected_idx) => selected_idx,
                ConstructionSelection::Interrupted => {
                    match settle_construction_interrupt(&mut step_scope) {
                        StepInterrupt::Restart => {
                            pending_placement = Some(placement);
                            continue;
                        }
                        StepInterrupt::TerminatePhase => break,
                    }
                }
            };

            if let Some(idx) = selected_idx {
                step_scope.phase_scope_mut().record_move_accepted();
                // Take ownership of the move
                let m = placement.take_move(idx);

                // Execute the move
                m.do_move(step_scope.score_director_mut());

                // Calculate and record the step score
                let step_score = step_scope.calculate_score();
                step_scope.set_step_score(step_score);
            }

            step_scope.complete();
        }

        // Update best solution at end of phase
        phase_scope.update_best_solution();

        let best_score = phase_scope
            .solver_scope()
            .best_score()
            .map(|s| format!("{}", s))
            .unwrap_or_else(|| "none".to_string());

        let duration = phase_scope.elapsed();
        let steps = phase_scope.step_count();
        let speed = whole_units_per_second(steps, duration);
        let stats = phase_scope.stats();

        info!(
            event = "phase_end",
            phase = "Construction Heuristic",
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
    }

    fn phase_type_name(&self) -> &'static str {
        "ConstructionHeuristic"
    }
}

enum ConstructionSelection {
    Selected(Option<usize>),
    Interrupted,
}

fn select_move_index<S, D, BestCb, M, Fo>(
    forager: &Fo,
    placement: &crate::phase::construction::Placement<S, M>,
    step_scope: &mut StepScope<'_, '_, '_, S, D, BestCb>,
) -> ConstructionSelection
where
    S: PlanningSolution,
    S::Score: Score,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
    M: Move<S> + 'static,
    Fo: ConstructionForager<S, M> + 'static,
{
    let erased = forager as &dyn Any;

    if erased.is::<FirstFitForager<S, M>>() {
        return select_first_fit_index(placement, step_scope);
    }
    if erased.is::<BestFitForager<S, M>>() {
        return select_best_fit_index(placement, step_scope);
    }
    if erased.is::<FirstFeasibleForager<S, M>>() {
        return select_first_feasible_index(placement, step_scope);
    }

    ConstructionSelection::Selected(
        forager.pick_move_index(placement, step_scope.score_director_mut()),
    )
}

fn select_first_fit_index<S, D, BestCb, M>(
    placement: &crate::phase::construction::Placement<S, M>,
    step_scope: &mut StepScope<'_, '_, '_, S, D, BestCb>,
) -> ConstructionSelection
where
    S: PlanningSolution,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
    M: Move<S> + 'static,
{
    for (idx, m) in placement.moves.iter().enumerate() {
        let evaluation_started = Instant::now();
        if should_interrupt_evaluation(step_scope, idx) {
            return ConstructionSelection::Interrupted;
        }
        if m.is_doable(step_scope.score_director()) {
            step_scope
                .phase_scope_mut()
                .record_evaluated_move(evaluation_started.elapsed());
            return ConstructionSelection::Selected(Some(idx));
        }
        step_scope
            .phase_scope_mut()
            .record_evaluated_move(evaluation_started.elapsed());
    }
    ConstructionSelection::Selected(None)
}

fn select_best_fit_index<S, D, BestCb, M>(
    placement: &crate::phase::construction::Placement<S, M>,
    step_scope: &mut StepScope<'_, '_, '_, S, D, BestCb>,
) -> ConstructionSelection
where
    S: PlanningSolution,
    S::Score: Score,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
    M: Move<S> + 'static,
{
    let mut best_idx: Option<usize> = None;
    let mut best_score: Option<S::Score> = None;

    for (idx, m) in placement.moves.iter().enumerate() {
        let evaluation_started = Instant::now();
        if should_interrupt_evaluation(step_scope, idx) {
            return ConstructionSelection::Interrupted;
        }
        if !m.is_doable(step_scope.score_director()) {
            step_scope
                .phase_scope_mut()
                .record_evaluated_move(evaluation_started.elapsed());
            continue;
        }

        let score = {
            let mut recording = RecordingDirector::new(step_scope.score_director_mut());
            m.do_move(&mut recording);
            let score = recording.calculate_score();
            recording.undo_changes();
            score
        };
        step_scope.phase_scope_mut().record_score_calculation();
        step_scope
            .phase_scope_mut()
            .record_evaluated_move(evaluation_started.elapsed());

        let is_better = match &best_score {
            None => true,
            Some(best) => score > *best,
        };
        if is_better {
            best_idx = Some(idx);
            best_score = Some(score);
        }
    }

    ConstructionSelection::Selected(best_idx)
}

fn select_first_feasible_index<S, D, BestCb, M>(
    placement: &crate::phase::construction::Placement<S, M>,
    step_scope: &mut StepScope<'_, '_, '_, S, D, BestCb>,
) -> ConstructionSelection
where
    S: PlanningSolution,
    S::Score: Score,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
    M: Move<S> + 'static,
{
    let mut fallback_idx: Option<usize> = None;
    let mut fallback_score: Option<S::Score> = None;

    for (idx, m) in placement.moves.iter().enumerate() {
        let evaluation_started = Instant::now();
        if should_interrupt_evaluation(step_scope, idx) {
            return ConstructionSelection::Interrupted;
        }
        if !m.is_doable(step_scope.score_director()) {
            step_scope
                .phase_scope_mut()
                .record_evaluated_move(evaluation_started.elapsed());
            continue;
        }

        let score = {
            let mut recording = RecordingDirector::new(step_scope.score_director_mut());
            m.do_move(&mut recording);
            let score = recording.calculate_score();
            recording.undo_changes();
            score
        };
        step_scope.phase_scope_mut().record_score_calculation();
        step_scope
            .phase_scope_mut()
            .record_evaluated_move(evaluation_started.elapsed());

        if score.is_feasible() {
            return ConstructionSelection::Selected(Some(idx));
        }

        let is_better = match &fallback_score {
            None => true,
            Some(best) => score > *best,
        };
        if is_better {
            fallback_idx = Some(idx);
            fallback_score = Some(score);
        }
    }

    ConstructionSelection::Selected(fallback_idx)
}

#[cfg(test)]
#[path = "phase_tests.rs"]
mod tests;
