//! Variable Neighborhood Descent (VND) phase.
//!
//! VND systematically explores multiple neighborhood structures, restarting
//! from the first neighborhood whenever an improvement is found. This provides
//! a structured way to combine multiple move types for better optimization.
//!
//! # Algorithm
//!
//! 1. Start with neighborhood k = 0
//! 2. Find the best improving move in neighborhood k
//! 3. If improvement found: apply move, restart from k = 0
//! 4. If no improvement: move to k = k + 1
//! 5. Terminate when k exceeds the number of neighborhoods
//!
//! # Zero-Erasure Design
//!
//! Uses macro-generated tuple implementations for neighborhoods. Each neighborhood
//! is a concrete `MoveSelector` type, enabling full monomorphization.

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::{RecordingScoreDirector, ScoreDirector};

use crate::heuristic::r#move::{Move, MoveArena};
use crate::heuristic::selector::MoveSelector;
use crate::phase::Phase;
use crate::scope::{PhaseScope, SolverScope, StepScope};

/// Variable Neighborhood Descent phase.
///
/// Wraps a tuple of move selectors (neighborhoods) and explores them in sequence,
/// restarting from the first whenever an improvement is found.
///
/// Uses macro-generated tuple implementations for zero type erasure.
///
/// # Type Parameters
/// * `T` - Tuple of move selectors
/// * `M` - The move type produced by all selectors
///
/// # Examples
///
/// ```ignore
/// // Two neighborhoods
/// let vnd: VndPhase<_, ChangeMove<Sol, i32>> = VndPhase::new((change_selector, swap_selector));
///
/// // Three neighborhoods
/// let vnd: VndPhase<_, MyMove> = VndPhase::new((sel1, sel2, sel3));
/// ```
pub struct VndPhase<T, M>(pub T, PhantomData<M>);

impl<T: Debug, M> Debug for VndPhase<T, M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("VndPhase").field(&self.0).finish()
    }
}

impl<T, M> VndPhase<T, M> {
    /// Creates a new VND phase from a tuple of move selectors.
    pub fn new(neighborhoods: T) -> Self {
        Self(neighborhoods, PhantomData)
    }
}

/// Generates `Phase` implementations for VndPhase with tuple neighborhoods.
macro_rules! impl_vnd_phase {
    // Single neighborhood
    ($idx:tt: $MS:ident) => {
        impl<S, M, $MS> Phase<S> for VndPhase<($MS,), M>
        where
            S: PlanningSolution,
            M: Move<S>,
            $MS: MoveSelector<S, M>,
        {
            fn solve(&mut self, solver_scope: &mut SolverScope<S>) {
                let mut arena = MoveArena::<M>::new();
                let mut phase_scope = PhaseScope::new(solver_scope, 0);
                let mut current_score = phase_scope.calculate_score();

                loop {
                    let mut step_scope = StepScope::new(&mut phase_scope);
                    arena.reset();
                    arena.extend((self.0).$idx.iter_moves(step_scope.score_director()));

                    let best_move = find_best_improving_move(&arena, &mut step_scope, &current_score);

                    if let Some((selected_move, selected_score)) = best_move {
                        selected_move.do_move(step_scope.score_director_mut());
                        step_scope.set_step_score(selected_score.clone());
                        current_score = selected_score;
                        step_scope.phase_scope_mut().update_best_solution();
                        step_scope.complete();
                    } else {
                        step_scope.complete();
                        break;
                    }
                }
            }

            fn phase_type_name(&self) -> &'static str {
                "VariableNeighborhoodDescent"
            }
        }
    };

    // Multiple neighborhoods
    ($($idx:tt: $MS:ident),+) => {
        impl<S, M, $($MS),+> Phase<S> for VndPhase<($($MS,)+), M>
        where
            S: PlanningSolution,
            M: Move<S>,
            $($MS: MoveSelector<S, M>,)+
        {
            fn solve(&mut self, solver_scope: &mut SolverScope<S>) {
                const COUNT: usize = impl_vnd_phase!(@count $($idx),+);
                let mut arena = MoveArena::<M>::new();
                let mut phase_scope = PhaseScope::new(solver_scope, 0);
                let mut current_score = phase_scope.calculate_score();
                let mut k = 0usize;

                while k < COUNT {
                    let mut step_scope = StepScope::new(&mut phase_scope);
                    arena.reset();

                    // Populate arena from neighborhood k
                    match k {
                        $($idx => arena.extend((self.0).$idx.iter_moves(step_scope.score_director())),)+
                        _ => {}
                    }

                    let best_move = find_best_improving_move(&arena, &mut step_scope, &current_score);

                    if let Some((selected_move, selected_score)) = best_move {
                        selected_move.do_move(step_scope.score_director_mut());
                        step_scope.set_step_score(selected_score.clone());
                        current_score = selected_score;
                        step_scope.phase_scope_mut().update_best_solution();
                        step_scope.complete();
                        k = 0; // Restart from first neighborhood
                    } else {
                        step_scope.complete();
                        k += 1; // Try next neighborhood
                    }
                }
            }

            fn phase_type_name(&self) -> &'static str {
                "VariableNeighborhoodDescent"
            }
        }
    };

    // Helper: count tuple elements
    (@count $($idx:tt),+) => {
        0 $(+ { let _ = $idx; 1 })+
    };
}

/// Finds the best improving move in the arena.
fn find_best_improving_move<S, M>(
    arena: &MoveArena<M>,
    step_scope: &mut StepScope<'_, '_, S>,
    current_score: &S::Score,
) -> Option<(M, S::Score)>
where
    S: PlanningSolution,
    M: Move<S>,
{
    let mut best_move: Option<(M, S::Score)> = None;

    for i in 0..arena.len() {
        let m = arena.get(i).unwrap();

        if !m.is_doable(step_scope.score_director()) {
            continue;
        }

        let mut recording = RecordingScoreDirector::new(step_scope.score_director_mut());
        m.do_move(&mut recording);
        let move_score = recording.calculate_score();
        recording.undo_changes();

        if move_score > *current_score {
            match &best_move {
                Some((_, best_score)) if move_score > *best_score => {
                    best_move = Some((m.clone(), move_score));
                }
                None => {
                    best_move = Some((m.clone(), move_score));
                }
                _ => {}
            }
        }
    }

    best_move
}

impl_vnd_phase!(0: MS0);
impl_vnd_phase!(0: MS0, 1: MS1);
impl_vnd_phase!(0: MS0, 1: MS1, 2: MS2);
impl_vnd_phase!(0: MS0, 1: MS1, 2: MS2, 3: MS3);
impl_vnd_phase!(0: MS0, 1: MS1, 2: MS2, 3: MS3, 4: MS4);
impl_vnd_phase!(0: MS0, 1: MS1, 2: MS2, 3: MS3, 4: MS4, 5: MS5);
impl_vnd_phase!(0: MS0, 1: MS1, 2: MS2, 3: MS3, 4: MS4, 5: MS5, 6: MS6);
impl_vnd_phase!(0: MS0, 1: MS1, 2: MS2, 3: MS3, 4: MS4, 5: MS5, 6: MS6, 7: MS7);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::heuristic::r#move::ChangeMove;
    use crate::heuristic::selector::ChangeMoveSelector;
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
                        if row1 == row2 {
                            conflicts += 1;
                        }
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

    fn create_director(
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
    type NQueensMoveSelector = ChangeMoveSelector<NQueensSolution, i32>;

    fn create_move_selector(values: Vec<i32>) -> NQueensMoveSelector {
        ChangeMoveSelector::simple(get_queen_row, set_queen_row, 0, "row", values)
    }

    #[test]
    fn test_vnd_improves_solution() {
        let director = create_director(&[0, 0, 0, 0]);
        let mut solver_scope = SolverScope::new(Box::new(director));

        let initial_score = solver_scope.calculate_score();
        assert!(initial_score < SimpleScore::of(0));

        let values: Vec<i32> = (0..4).collect();
        let mut phase: VndPhase<_, NQueensMove> = VndPhase::new((
            create_move_selector(values.clone()),
            create_move_selector(values),
        ));

        phase.solve(&mut solver_scope);

        let final_score = solver_scope.best_score().cloned().unwrap_or(initial_score);
        assert!(final_score >= initial_score);
    }

    #[test]
    fn test_vnd_single_neighborhood() {
        let director = create_director(&[0, 0, 0, 0]);
        let mut solver_scope = SolverScope::new(Box::new(director));

        let initial_score = solver_scope.calculate_score();

        let values: Vec<i32> = (0..4).collect();
        let mut phase: VndPhase<_, NQueensMove> = VndPhase::new((create_move_selector(values),));

        phase.solve(&mut solver_scope);

        let final_score = solver_scope.best_score().cloned().unwrap_or(initial_score);
        assert!(final_score >= initial_score);
    }
}
