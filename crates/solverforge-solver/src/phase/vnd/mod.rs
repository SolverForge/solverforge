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

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::{RecordingScoreDirector, ScoreDirector};

use crate::heuristic::r#move::{Move, MoveArena};
use crate::heuristic::selector::MoveSelector;
use crate::phase::Phase;
use crate::scope::{PhaseScope, SolverScope, StepScope};

/// Trait for a tuple of neighborhoods that can be indexed at runtime.
///
/// This enables VND's dynamic neighborhood switching while preserving
/// zero-erasure through macro-generated match arms.
pub trait NeighborhoodTuple<S: PlanningSolution, M: Move<S>>: Send + Debug {
    /// Returns the number of neighborhoods.
    fn count(&self) -> usize;

    /// Populates the arena with moves from neighborhood at index `k`.
    fn populate_arena(
        &self,
        k: usize,
        arena: &mut MoveArena<M>,
        score_director: &dyn ScoreDirector<S>,
    );
}

/// Generates `NeighborhoodTuple` implementations for tuples of move selectors.
macro_rules! impl_neighborhood_tuple {
    // Single neighborhood
    ($idx:tt: $MS:ident) => {
        impl<S, M, $MS> NeighborhoodTuple<S, M> for ($MS,)
        where
            S: PlanningSolution,
            M: Move<S>,
            $MS: MoveSelector<S, M>,
        {
            fn count(&self) -> usize {
                1
            }

            fn populate_arena(
                &self,
                k: usize,
                arena: &mut MoveArena<M>,
                score_director: &dyn ScoreDirector<S>,
            ) {
                match k {
                    $idx => arena.extend(self.$idx.iter_moves(score_director)),
                    _ => {}
                }
            }
        }
    };

    // Multiple neighborhoods
    ($($idx:tt: $MS:ident),+) => {
        impl<S, M, $($MS),+> NeighborhoodTuple<S, M> for ($($MS,)+)
        where
            S: PlanningSolution,
            M: Move<S>,
            $($MS: MoveSelector<S, M>,)+
        {
            fn count(&self) -> usize {
                impl_neighborhood_tuple!(@count $($idx),+)
            }

            fn populate_arena(
                &self,
                k: usize,
                arena: &mut MoveArena<M>,
                score_director: &dyn ScoreDirector<S>,
            ) {
                match k {
                    $($idx => arena.extend(self.$idx.iter_moves(score_director)),)+
                    _ => {}
                }
            }
        }
    };

    // Helper: count tuple elements
    (@count $($idx:tt),+) => {
        0 $(+ { let _ = $idx; 1 })+
    };
}

impl_neighborhood_tuple!(0: MS0);
impl_neighborhood_tuple!(0: MS0, 1: MS1);
impl_neighborhood_tuple!(0: MS0, 1: MS1, 2: MS2);
impl_neighborhood_tuple!(0: MS0, 1: MS1, 2: MS2, 3: MS3);
impl_neighborhood_tuple!(0: MS0, 1: MS1, 2: MS2, 3: MS3, 4: MS4);
impl_neighborhood_tuple!(0: MS0, 1: MS1, 2: MS2, 3: MS3, 4: MS4, 5: MS5);
impl_neighborhood_tuple!(0: MS0, 1: MS1, 2: MS2, 3: MS3, 4: MS4, 5: MS5, 6: MS6);
impl_neighborhood_tuple!(0: MS0, 1: MS1, 2: MS2, 3: MS3, 4: MS4, 5: MS5, 6: MS6, 7: MS7);

/// Variable Neighborhood Descent phase.
///
/// Explores multiple neighborhoods in sequence, restarting from the first
/// whenever an improvement is found.
///
/// # Type Parameters
/// * `S` - The planning solution type
/// * `M` - The move type (all neighborhoods must produce this type)
/// * `N` - The neighborhoods tuple type
pub struct VndPhase<S, M, N>
where
    S: PlanningSolution,
    M: Move<S>,
    N: NeighborhoodTuple<S, M>,
{
    /// Tuple of move selectors (neighborhoods) - zero erasure
    neighborhoods: N,
    /// Arena for moves - reused each step
    arena: MoveArena<M>,
    /// Maximum steps per neighborhood before giving up
    step_limit_per_neighborhood: Option<u64>,
    _phantom: std::marker::PhantomData<(S, M)>,
}

impl<S, M, N> Debug for VndPhase<S, M, N>
where
    S: PlanningSolution,
    M: Move<S>,
    N: NeighborhoodTuple<S, M>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VndPhase")
            .field("neighborhoods", &self.neighborhoods)
            .field("step_limit_per_neighborhood", &self.step_limit_per_neighborhood)
            .finish()
    }
}

impl<S, M, N> VndPhase<S, M, N>
where
    S: PlanningSolution,
    M: Move<S> + 'static,
    N: NeighborhoodTuple<S, M>,
{
    /// Creates a new VND phase with the given neighborhoods tuple.
    ///
    /// Neighborhoods are explored in order. When an improvement is found,
    /// exploration restarts from the first neighborhood.
    pub fn new(neighborhoods: N) -> Self {
        Self {
            neighborhoods,
            arena: MoveArena::new(),
            step_limit_per_neighborhood: None,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Sets the maximum steps to try per neighborhood before moving on.
    pub fn with_step_limit(mut self, limit: u64) -> Self {
        self.step_limit_per_neighborhood = Some(limit);
        self
    }

    /// Returns the number of neighborhoods.
    pub fn neighborhood_count(&self) -> usize {
        self.neighborhoods.count()
    }
}

impl<S, M, N, D> Phase<S, D> for VndPhase<S, M, N>
where
    S: PlanningSolution,
    M: Move<S>,
    N: NeighborhoodTuple<S, M>,
    D: ScoreDirector<S>,
{
    fn solve(&mut self, solver_scope: &mut SolverScope<S, D>) {
        let neighborhood_count = self.neighborhoods.count();
        if neighborhood_count == 0 {
            return;
        }

        let mut phase_scope = PhaseScope::new(solver_scope, 0);
        let mut current_score = phase_scope.calculate_score();

        let mut neighborhood_idx = 0;
        let mut steps_in_neighborhood = 0u64;

        while neighborhood_idx < neighborhood_count {
            // Check step limit for current neighborhood
            if let Some(limit) = self.step_limit_per_neighborhood {
                if steps_in_neighborhood >= limit {
                    // Move to next neighborhood
                    neighborhood_idx += 1;
                    steps_in_neighborhood = 0;
                    continue;
                }
            }

            let mut step_scope = StepScope::new(&mut phase_scope);

            // Reset arena and populate with moves from current neighborhood
            self.arena.reset();
            self.neighborhoods.populate_arena(
                neighborhood_idx,
                &mut self.arena,
                step_scope.score_director(),
            );

            // Find best improving move
            let mut best_move: Option<(M, S::Score)> = None;

            for i in 0..self.arena.len() {
                let m = self.arena.get(i).unwrap();

                if !m.is_doable(step_scope.score_director()) {
                    continue;
                }

                let mut recording = RecordingScoreDirector::new(step_scope.score_director_mut());
                m.do_move(&mut recording);
                let move_score = recording.calculate_score();
                recording.undo_changes();

                // Only consider improving moves
                if move_score > current_score {
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

            if let Some((selected_move, selected_score)) = best_move {
                // Apply the improving move
                selected_move.do_move(step_scope.score_director_mut());
                step_scope.set_step_score(selected_score.clone());
                current_score = selected_score;

                // Update best solution
                step_scope.phase_scope_mut().update_best_solution();

                // Restart from first neighborhood
                neighborhood_idx = 0;
                steps_in_neighborhood = 0;
            } else {
                // No improvement in this neighborhood, try next
                neighborhood_idx += 1;
                steps_in_neighborhood = 0;
            }

            steps_in_neighborhood += 1;
            step_scope.complete();
        }
    }

    fn phase_type_name(&self) -> &'static str {
        "VariableNeighborhoodDescent"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::heuristic::r#move::ChangeMove;
    use crate::heuristic::selector::ChangeMoveSelector;
    use crate::heuristic::selector::entity::FromSolutionEntitySelector;
    use crate::heuristic::selector::typed_value::StaticTypedValueSelector;
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
    type NQueensMoveSelector = ChangeMoveSelector<
        NQueensSolution,
        i32,
        FromSolutionEntitySelector,
        StaticTypedValueSelector<NQueensSolution, i32>,
    >;

    fn create_move_selector(values: Vec<i32>) -> NQueensMoveSelector {
        ChangeMoveSelector::simple(get_queen_row, set_queen_row, 0, "row", values)
    }

    #[test]
    fn test_vnd_improves_solution() {
        let director = create_director(&[0, 0, 0, 0]);
        let mut solver_scope = SolverScope::new(director);

        let initial_score = solver_scope.calculate_score();
        assert!(initial_score < SimpleScore::of(0));

        let values: Vec<i32> = (0..4).collect();
        // Two neighborhoods as a tuple
        let neighborhoods = (
            create_move_selector(values.clone()),
            create_move_selector(values.clone()),
        );
        let mut phase = VndPhase::new(neighborhoods);

        phase.solve(&mut solver_scope);

        let final_score = solver_scope.best_score().cloned().unwrap_or(initial_score);
        assert!(final_score >= initial_score);
    }

    #[test]
    fn test_vnd_single_neighborhood() {
        let director = create_director(&[0, 0, 0, 0]);
        let mut solver_scope = SolverScope::new(director);

        let initial_score = solver_scope.calculate_score();

        let values: Vec<i32> = (0..4).collect();
        // Single neighborhood as a 1-tuple
        let neighborhoods = (create_move_selector(values),);
        let mut phase = VndPhase::new(neighborhoods);

        phase.solve(&mut solver_scope);

        let final_score = solver_scope.best_score().cloned().unwrap_or(initial_score);
        assert!(final_score >= initial_score);
    }

    #[test]
    fn test_neighborhood_tuple_count() {
        let values: Vec<i32> = (0..4).collect();
        let n1 = (create_move_selector(values.clone()),);
        let n2 = (
            create_move_selector(values.clone()),
            create_move_selector(values.clone()),
        );
        let n3 = (
            create_move_selector(values.clone()),
            create_move_selector(values.clone()),
            create_move_selector(values),
        );

        assert_eq!(n1.count(), 1);
        assert_eq!(n2.count(), 2);
        assert_eq!(n3.count(), 3);
    }
}
