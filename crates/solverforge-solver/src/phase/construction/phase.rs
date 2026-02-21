//! Construction heuristic phase implementation.

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;
use tracing::info;

use crate::heuristic::r#move::Move;
use crate::phase::construction::{ConstructionForager, EntityPlacer};
use crate::phase::Phase;
use crate::scope::{PhaseScope, SolverScope, StepScope};

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
    /// Creates a new construction heuristic phase.
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

impl<S, D, M, P, Fo> Phase<S, D> for ConstructionHeuristicPhase<S, M, P, Fo>
where
    S: PlanningSolution,
    D: ScoreDirector<S>,
    M: Move<S>,
    P: EntityPlacer<S, M>,
    Fo: ConstructionForager<S, M>,
{
    fn solve(&mut self, solver_scope: &mut SolverScope<S, D>) {
        let mut phase_scope = PhaseScope::new(solver_scope, 0);
        let phase_index = phase_scope.phase_index();

        info!(
            event = "phase_start",
            phase = "Construction Heuristic",
            phase_index = phase_index,
        );

        // Get all placements (entities that need values assigned)
        let placements = self.placer.get_placements(phase_scope.score_director());

        for mut placement in placements {
            // Construction must complete — only stop for external flag or time limit,
            // never for step/move count limits (those are for local search).
            if phase_scope.solver_scope().should_terminate_construction() {
                break;
            }

            // Record move evaluations at call-site (Option C: maintains trait purity)
            // BestFitForager evaluates ALL moves in the placement
            let moves_in_placement = placement.moves.len() as u64;

            let mut step_scope = StepScope::new(&mut phase_scope);

            // Use forager to pick the best move index for this placement
            let selected_idx = self
                .forager
                .pick_move_index(&placement, step_scope.score_director_mut());

            // Record all moves as evaluated, with one accepted if selection succeeded
            for i in 0..moves_in_placement {
                let accepted = selected_idx == Some(i as usize);
                step_scope
                    .phase_scope_mut()
                    .solver_scope_mut()
                    .stats_mut()
                    .record_move(accepted);
            }

            if let Some(idx) = selected_idx {
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
        let speed = if duration.as_secs_f64() > 0.0 {
            (steps as f64 / duration.as_secs_f64()) as u64
        } else {
            0
        };

        info!(
            event = "phase_end",
            phase = "Construction Heuristic",
            phase_index = phase_index,
            duration_ms = duration.as_millis() as u64,
            steps = steps,
            speed = speed,
            score = best_score,
        );
    }

    fn phase_type_name(&self) -> &'static str {
        "ConstructionHeuristic"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::heuristic::selector::{FromSolutionEntitySelector, StaticTypedValueSelector};
    use crate::phase::construction::{BestFitForager, FirstFitForager, QueuedEntityPlacer};
    use crate::test_utils::{
        create_simple_nqueens_director, get_queen_row, set_queen_row, NQueensSolution,
    };

    fn create_placer(
        values: Vec<i64>,
    ) -> QueuedEntityPlacer<
        NQueensSolution,
        i64,
        FromSolutionEntitySelector,
        StaticTypedValueSelector<NQueensSolution, i64>,
    > {
        let es = FromSolutionEntitySelector::new(0);
        let vs = StaticTypedValueSelector::new(values);
        QueuedEntityPlacer::new(es, vs, get_queen_row, set_queen_row, 0, "row")
    }

    #[test]
    fn test_construction_first_fit() {
        let director = create_simple_nqueens_director(4);
        let mut solver_scope = SolverScope::new(director);
        solver_scope.start_solving();

        let values: Vec<i64> = (0..4).collect();
        let placer = create_placer(values);
        let forager = FirstFitForager::new();
        let mut phase = ConstructionHeuristicPhase::new(placer, forager);

        phase.solve(&mut solver_scope);

        let solution = solver_scope.working_solution();
        assert_eq!(solution.queens.len(), 4);
        for queen in &solution.queens {
            assert!(queen.row.is_some(), "Queen should have a row assigned");
        }

        assert!(solver_scope.best_solution().is_some());
        // Verify stats were recorded
        assert!(solver_scope.stats().moves_evaluated > 0);
    }

    #[test]
    fn test_construction_best_fit() {
        let director = create_simple_nqueens_director(4);
        let mut solver_scope = SolverScope::new(director);
        solver_scope.start_solving();

        let values: Vec<i64> = (0..4).collect();
        let placer = create_placer(values);
        let forager = BestFitForager::new();
        let mut phase = ConstructionHeuristicPhase::new(placer, forager);

        phase.solve(&mut solver_scope);

        let solution = solver_scope.working_solution();
        for queen in &solution.queens {
            assert!(queen.row.is_some(), "Queen should have a row assigned");
        }

        assert!(solver_scope.best_solution().is_some());
        assert!(solver_scope.best_score().is_some());

        // BestFitForager evaluates all moves: 4 entities * 4 values = 16 moves
        assert_eq!(solver_scope.stats().moves_evaluated, 16);
    }

    #[test]
    fn test_construction_empty_solution() {
        let director = create_simple_nqueens_director(0);
        let mut solver_scope = SolverScope::new(director);
        solver_scope.start_solving();

        let values: Vec<i64> = vec![];
        let placer = create_placer(values);
        let forager = FirstFitForager::new();
        let mut phase = ConstructionHeuristicPhase::new(placer, forager);

        phase.solve(&mut solver_scope);

        // No moves should be evaluated for empty solution
        assert_eq!(solver_scope.stats().moves_evaluated, 0);
    }
}
