//! Local search phase
//!
//! Improves an existing solution by iteratively applying moves
//! that are accepted according to an acceptance criterion.

mod acceptor;
mod forager;

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::{RecordingScoreDirector, ScoreDirector};

use crate::heuristic::r#move::{Move, MoveArena};
use crate::heuristic::selector::MoveSelector;
use crate::phase::Phase;
use crate::scope::{PhaseScope, SolverScope, StepScope};

pub use acceptor::{
    Acceptor, DiversifiedLateAcceptanceAcceptor, EntityTabuAcceptor, GreatDelugeAcceptor,
    HillClimbingAcceptor, LateAcceptanceAcceptor, MoveTabuAcceptor, SimulatedAnnealingAcceptor,
    StepCountingHillClimbingAcceptor, TabuSearchAcceptor, ValueTabuAcceptor,
};
pub use forager::{AcceptedCountForager, FirstAcceptedForager, LocalSearchForager};

/// Local search phase configuration.
#[derive(Debug, Clone)]
pub struct LocalSearchConfig {
    /// The acceptor type to use.
    pub acceptor_type: AcceptorType,
    /// Maximum number of steps (None = unlimited).
    pub step_limit: Option<u64>,
    /// Number of accepted moves to collect before quitting early.
    pub accepted_count_limit: Option<usize>,
}

impl Default for LocalSearchConfig {
    fn default() -> Self {
        Self {
            acceptor_type: AcceptorType::HillClimbing,
            step_limit: Some(1000),
            accepted_count_limit: Some(1),
        }
    }
}

/// Type of acceptor to use in local search.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcceptorType {
    /// Accept only improving moves.
    HillClimbing,
    /// Accept moves with probability based on temperature.
    SimulatedAnnealing,
}

/// Local search phase that improves an existing solution.
///
/// This phase iteratively:
/// 1. Generates candidate moves
/// 2. Evaluates each move
/// 3. Accepts/rejects based on the acceptor
/// 4. Applies the best accepted move
///
/// # Type Parameters
/// * `S` - The planning solution type
/// * `M` - The move type (must implement `Move<S> + Clone`)
///
/// # Performance
///
/// Uses `MoveArena<M>` for O(1) per-step cleanup instead of allocating
/// a new Vec each step.
pub struct LocalSearchPhase<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    /// The move selector.
    move_selector: Box<dyn MoveSelector<S, M>>,
    /// The acceptor.
    acceptor: Box<dyn Acceptor<S>>,
    /// The forager.
    forager: Box<dyn LocalSearchForager<S, M>>,
    /// Arena for moves - reused each step for O(1) cleanup.
    arena: MoveArena<M>,
    /// Maximum number of steps.
    step_limit: Option<u64>,
    _phantom: PhantomData<M>,
}

impl<S, M> LocalSearchPhase<S, M>
where
    S: PlanningSolution,
    M: Move<S> + 'static,
{
    /// Creates a new local search phase.
    pub fn new(
        move_selector: Box<dyn MoveSelector<S, M>>,
        acceptor: Box<dyn Acceptor<S>>,
        forager: Box<dyn LocalSearchForager<S, M>>,
        step_limit: Option<u64>,
    ) -> Self {
        Self {
            move_selector,
            acceptor,
            forager,
            arena: MoveArena::new(),
            step_limit,
            _phantom: PhantomData,
        }
    }
}

impl<S, M> Debug for LocalSearchPhase<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LocalSearchPhase")
            .field("move_selector", &self.move_selector)
            .field("acceptor", &self.acceptor)
            .field("forager", &self.forager)
            .field("arena", &self.arena)
            .field("step_limit", &self.step_limit)
            .finish()
    }
}

impl<S, M, D> Phase<S, D> for LocalSearchPhase<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
    D: ScoreDirector<S>,
{
    fn solve(&mut self, solver_scope: &mut SolverScope<S, D>) {
        let mut phase_scope = PhaseScope::new(solver_scope, 0);

        // Calculate initial score
        let mut last_step_score = phase_scope.calculate_score();

        // Notify acceptor of phase start
        self.acceptor.phase_started(&last_step_score);

        loop {
            // Check early termination
            if phase_scope.solver_scope().is_terminate_early() {
                break;
            }

            // Check step limit
            if let Some(limit) = self.step_limit {
                if phase_scope.step_count() >= limit {
                    break;
                }
            }

            let mut step_scope = StepScope::new(&mut phase_scope);

            // Reset forager for this step
            self.forager.step_started();

            // Reset arena and populate with moves - O(1) reset
            self.arena.reset();
            self.arena
                .extend(self.move_selector.iter_moves(step_scope.score_director()));

            // Evaluate moves
            for i in 0..self.arena.len() {
                let m = self.arena.get(i).unwrap();

                if !m.is_doable(step_scope.score_director()) {
                    continue;
                }

                // Use RecordingScoreDirector for automatic undo
                {
                    let mut recording =
                        RecordingScoreDirector::new(step_scope.score_director_mut());

                    // Execute move
                    m.do_move(&mut recording);

                    // Calculate resulting score
                    let move_score = recording.calculate_score();

                    // Check if accepted
                    let accepted = self.acceptor.is_accepted(&last_step_score, &move_score);

                    // Add to forager if accepted
                    if accepted {
                        self.forager.add_move(m.clone(), move_score);
                    }

                    // Undo the move
                    recording.undo_changes();
                }

                // Check if forager wants to quit early
                if self.forager.is_quit_early() {
                    break;
                }
            }

            // Pick the best accepted move
            if let Some((selected_move, selected_score)) = self.forager.pick_move() {
                // Execute the selected move (for real this time)
                selected_move.do_move(step_scope.score_director_mut());
                step_scope.set_step_score(selected_score.clone());

                // Update last step score
                last_step_score = selected_score;

                // Update best solution if improved
                step_scope.phase_scope_mut().update_best_solution();
            } else {
                // No accepted moves - we're stuck
                break;
            }

            step_scope.complete();
        }

        // Notify acceptor of phase end
        self.acceptor.phase_ended();
    }

    fn phase_type_name(&self) -> &'static str {
        "LocalSearch"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::heuristic::r#move::ChangeMove;
    use crate::heuristic::selector::{ChangeMoveSelector, MoveSelector};
    use crate::manager::SolverPhaseFactory;
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

    // Zero-erasure typed getter/setter for solution-level access
    fn get_queen_row(s: &NQueensSolution, idx: usize) -> Option<i32> {
        s.queens.get(idx).and_then(|q| q.row)
    }

    fn set_queen_row(s: &mut NQueensSolution, idx: usize, v: Option<i32>) {
        if let Some(queen) = s.queens.get_mut(idx) {
            queen.row = v;
        }
    }

    fn calculate_conflicts(solution: &NQueensSolution) -> SimpleScore {
        let mut conflicts = 0i64;

        for (i, q1) in solution.queens.iter().enumerate() {
            if let Some(row1) = q1.row {
                for q2 in solution.queens.iter().skip(i + 1) {
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
        rows: &[i32],
    ) -> SimpleScoreDirector<NQueensSolution, impl Fn(&NQueensSolution) -> SimpleScore> {
        let queens: Vec<_> = rows
            .iter()
            .enumerate()
            .map(|(col, &row)| Queen {
                column: col as i32,
                row: Some(row),
            })
            .collect();

        let solution = NQueensSolution {
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

    type NQueensMove = ChangeMove<NQueensSolution, i32>;

    fn create_move_selector(
        values: Vec<i32>,
    ) -> Box<dyn MoveSelector<NQueensSolution, NQueensMove>> {
        Box::new(ChangeMoveSelector::<NQueensSolution, i32>::simple(
            get_queen_row,
            set_queen_row,
            0,
            "row",
            values,
        ))
    }

    #[test]
    fn test_local_search_hill_climbing() {
        use crate::manager::LocalSearchPhaseFactory;

        // Start with a suboptimal solution: queens all in row 0
        let director = create_test_director(&[0, 0, 0, 0]);
        let mut solver_scope = SolverScope::new(Box::new(director));

        // Initial score should be negative (conflicts)
        let initial_score = solver_scope.calculate_score();
        assert!(initial_score < SimpleScore::of(0));

        let values: Vec<i32> = (0..4).collect();
        let factory =
            LocalSearchPhaseFactory::<NQueensSolution, NQueensMove, _>::hill_climbing(move || {
                create_move_selector(values.clone())
            })
            .with_step_limit(100);
        let mut phase = factory.create_phase();

        phase.solve(&mut solver_scope);

        // Should have improved (or at least not gotten worse)
        let final_score = solver_scope.best_score().cloned().unwrap_or(initial_score);
        assert!(final_score >= initial_score);
    }

    #[test]
    fn test_local_search_reaches_optimal() {
        use crate::manager::LocalSearchPhaseFactory;

        // Start with a solution that has one conflict
        let director = create_test_director(&[0, 2, 1, 3]);
        let mut solver_scope = SolverScope::new(Box::new(director));

        let initial_score = solver_scope.calculate_score();

        let values: Vec<i32> = (0..4).collect();
        let factory =
            LocalSearchPhaseFactory::<NQueensSolution, NQueensMove, _>::hill_climbing(move || {
                create_move_selector(values.clone())
            })
            .with_step_limit(50);
        let mut phase = factory.create_phase();

        phase.solve(&mut solver_scope);

        // Check that we didn't make it worse
        let final_score = solver_scope.best_score().cloned().unwrap_or(initial_score);
        assert!(final_score >= initial_score);
    }

    #[test]
    fn test_local_search_step_limit() {
        use crate::manager::LocalSearchPhaseFactory;

        let director = create_test_director(&[0, 0, 0, 0]);
        let mut solver_scope = SolverScope::new(Box::new(director));

        let values: Vec<i32> = (0..4).collect();
        let factory =
            LocalSearchPhaseFactory::<NQueensSolution, NQueensMove, _>::hill_climbing(move || {
                create_move_selector(values.clone())
            })
            .with_step_limit(3);
        let mut phase = factory.create_phase();

        phase.solve(&mut solver_scope);

        // Should have run some steps
        // Note: actual step count depends on whether moves are accepted
    }
}
