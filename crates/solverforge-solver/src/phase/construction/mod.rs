//! Construction heuristic phase
//!
//! Builds an initial solution by assigning values to uninitialized
//! planning variables one at a time.

mod forager;
mod placer;

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use crate::heuristic::r#move::Move;
use crate::phase::Phase;
use crate::scope::{PhaseScope, SolverScope, StepScope};

pub use forager::{
    BestFitForager, ConstructionForager, FirstFeasibleForager, FirstFitForager,
    StrongestFitForager, WeakestFitForager,
};
pub use placer::{EntityPlacer, Placement, QueuedEntityPlacer, SortedEntityPlacer};

/// Construction heuristic phase configuration.
#[derive(Debug, Clone)]
pub struct ConstructionHeuristicConfig {
    /// The forager type to use.
    pub forager_type: ForagerType,
}

impl Default for ConstructionHeuristicConfig {
    fn default() -> Self {
        Self {
            forager_type: ForagerType::FirstFit,
        }
    }
}

/// Type of forager to use in construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForagerType {
    /// Accept the first feasible move.
    FirstFit,
    /// Evaluate all moves and pick the best.
    BestFit,
}

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
    /// The entity placer.
    placer: P,
    /// The forager for selecting moves.
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
            if phase_scope.solver_scope().is_terminate_early() {
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
    use solverforge_core::domain::{EntityDescriptor, SolutionDescriptor, TypedEntityExtractor};
    use solverforge_core::score::SimpleScore;
    use solverforge_scoring::SimpleScoreDirector;
    use std::any::TypeId;

    #[derive(Clone, Debug)]
    struct Queen {
        column: i32,
        row: Option<i32>,
    }

    #[derive(Clone, Debug)]
    struct NQueensSolution {
        n: i32,
        queens: Vec<Queen>,
        score: Option<SimpleScore>,
    }

    impl PlanningSolution for NQueensSolution {
        type Score = SimpleScore;

        fn score(&self) -> Option<Self::Score> {
            self.score
        }

        fn set_score(&mut self, score: Option<Self::Score>) {
            self.score = score;
        }
    }

    fn get_queens(s: &NQueensSolution) -> &Vec<Queen> {
        &s.queens
    }

    fn get_queens_mut(s: &mut NQueensSolution) -> &mut Vec<Queen> {
        &mut s.queens
    }

    // Typed getter - zero erasure
    fn get_queen_row(s: &NQueensSolution, idx: usize) -> Option<i32> {
        s.queens.get(idx).and_then(|q| q.row)
    }

    // Typed setter - zero erasure
    fn set_queen_row(s: &mut NQueensSolution, idx: usize, v: Option<i32>) {
        if let Some(queen) = s.queens.get_mut(idx) {
            queen.row = v;
        }
    }

    fn calculate_conflicts(solution: &NQueensSolution) -> SimpleScore {
        let mut conflicts = 0i64;

        for (i, q1) in solution.queens.iter().enumerate() {
            if let Some(row1) = q1.row {
                for (_, q2) in solution.queens.iter().enumerate().skip(i + 1) {
                    if let Some(row2) = q2.row {
                        // Same row conflict
                        if row1 == row2 {
                            conflicts += 1;
                        }
                        // Diagonal conflict
                        let col_diff = (q2.column - q1.column).abs();
                        let row_diff = (row2 - row1).abs();
                        if col_diff == row_diff {
                            conflicts += 1;
                        }
                    }
                }
            }
        }

        SimpleScore::of(-conflicts)
    }

    fn create_test_director(
        n: i32,
    ) -> SimpleScoreDirector<NQueensSolution, impl Fn(&NQueensSolution) -> SimpleScore> {
        let queens: Vec<_> = (0..n)
            .map(|col| Queen {
                column: col,
                row: None,
            })
            .collect();

        let solution = NQueensSolution {
            n,
            queens,
            score: None,
        };

        let extractor = Box::new(TypedEntityExtractor::new(
            "Queen",
            "queens",
            get_queens,
            get_queens_mut,
        ));
        let entity_desc = EntityDescriptor::new("Queen", TypeId::of::<Queen>(), "queens")
            .with_extractor(extractor);

        let descriptor =
            SolutionDescriptor::new("NQueensSolution", TypeId::of::<NQueensSolution>())
                .with_entity(entity_desc);

        SimpleScoreDirector::with_calculator(solution, descriptor, calculate_conflicts)
    }

    fn create_placer(
        values: Vec<i32>,
    ) -> QueuedEntityPlacer<
        NQueensSolution,
        i32,
        FromSolutionEntitySelector,
        StaticTypedValueSelector<NQueensSolution, i32>,
    > {
        let es = FromSolutionEntitySelector::new(0);
        let vs = StaticTypedValueSelector::new(values);
        QueuedEntityPlacer::new(es, vs, get_queen_row, set_queen_row, 0, "row")
    }

    #[test]
    fn test_construction_first_fit() {
        let director = create_test_director(4);
        let mut solver_scope = SolverScope::new(director);

        let values: Vec<i32> = (0..4).collect();
        let placer = create_placer(values);
        let forager = FirstFitForager::new();
        let mut phase = ConstructionHeuristicPhase::new(placer, forager);

        phase.solve(&mut solver_scope);

        // Check that all queens have rows assigned
        let solution = solver_scope.working_solution();
        assert_eq!(solution.n, 4);
        for queen in &solution.queens {
            assert!(queen.row.is_some(), "Queen should have a row assigned");
        }

        // Best solution should be set
        assert!(solver_scope.best_solution().is_some());
    }

    #[test]
    fn test_construction_best_fit() {
        let director = create_test_director(4);
        let mut solver_scope = SolverScope::new(director);

        let values: Vec<i32> = (0..4).collect();
        let placer = create_placer(values);
        let forager = BestFitForager::new();
        let mut phase = ConstructionHeuristicPhase::new(placer, forager);

        phase.solve(&mut solver_scope);

        // Check that all queens have rows assigned
        let solution = solver_scope.working_solution();
        for queen in &solution.queens {
            assert!(queen.row.is_some(), "Queen should have a row assigned");
        }

        // Best solution should be set
        assert!(solver_scope.best_solution().is_some());
        assert!(solver_scope.best_score().is_some());
    }

    #[test]
    fn test_construction_empty_solution() {
        let director = create_test_director(0);
        let mut solver_scope = SolverScope::new(director);

        let values: Vec<i32> = vec![];
        let placer = create_placer(values);
        let forager = FirstFitForager::new();
        let mut phase = ConstructionHeuristicPhase::new(placer, forager);

        // Should not panic
        phase.solve(&mut solver_scope);
    }
}
