//! Construction heuristic phase implementation.

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

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

        // Get all placements (entities that need values assigned)
        let placements = self.placer.get_placements(phase_scope.score_director());

        for mut placement in placements {
            // Check early termination
            if phase_scope.solver_scope().should_terminate() {
                break;
            }

            let mut step_scope = StepScope::new(&mut phase_scope);

            // Use forager to pick the best move index for this placement
            let selected_idx = self
                .forager
                .pick_move_index(&placement, step_scope.score_director_mut());

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
    }

    #[test]
    fn test_construction_best_fit() {
        let director = create_simple_nqueens_director(4);
        let mut solver_scope = SolverScope::new(director);

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
    }

    #[test]
    fn test_construction_empty_solution() {
        let director = create_simple_nqueens_director(0);
        let mut solver_scope = SolverScope::new(director);

        let values: Vec<i64> = vec![];
        let placer = create_placer(values);
        let forager = FirstFitForager::new();
        let mut phase = ConstructionHeuristicPhase::new(placer, forager);

        phase.solve(&mut solver_scope);
    }
}
