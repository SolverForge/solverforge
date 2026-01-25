//! Variable Neighborhood Descent phase implementation.

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use solverforge_scoring::api::constraint_set::ConstraintSet;

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
/// # Example
///
/// ```
/// use solverforge_solver::phase::vnd::VndPhase;
/// use solverforge_solver::heuristic::r#move::ChangeMove;
/// use solverforge_solver::heuristic::selector::MoveSelector;
/// use solverforge_core::domain::PlanningSolution;
/// use solverforge_core::score::SimpleScore;
/// use solverforge_scoring::api::constraint_set::ConstraintSet;
/// use solverforge_scoring::ScoreDirector;
///
/// #[derive(Clone, Debug)]
/// struct MySolution {
///     values: Vec<Option<i32>>,
///     score: Option<SimpleScore>,
/// }
///
/// impl PlanningSolution for MySolution {
///     type Score = SimpleScore;
///     fn score(&self) -> Option<Self::Score> { self.score }
///     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
/// }
///
/// type MyMove = ChangeMove<MySolution, i32>;
///
/// // Simple mock selector for demonstration
/// #[derive(Debug)]
/// struct MockSelector;
/// impl MoveSelector<MySolution, MyMove> for MockSelector {
///     fn iter_moves<'a, C>(&'a self, _: &'a ScoreDirector<MySolution, C>)
///         -> Box<dyn Iterator<Item = MyMove> + 'a>
///         where C: ConstraintSet<MySolution, SimpleScore> { Box::new(std::iter::empty()) }
///     fn size<C>(&self, _: &ScoreDirector<MySolution, C>) -> usize
///         where C: ConstraintSet<MySolution, SimpleScore> { 3 }
///     fn is_never_ending(&self) -> bool { false }
/// }
///
/// // Single neighborhood VND
/// let vnd: VndPhase<_, MyMove> = VndPhase::new((MockSelector,));
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
        impl<S, C, M, $MS> Phase<S, C> for VndPhase<($MS,), M>
        where
            S: PlanningSolution + solverforge_scoring::ShadowVariableSupport,
            S::Score: Score,
            C: ConstraintSet<S, S::Score>,
            M: Move<S>,
            $MS: MoveSelector<S, M>,
        {
            fn solve(&mut self, solver_scope: &mut SolverScope<S, C>) {
                let mut arena = MoveArena::<M>::new();
                let mut phase_scope = PhaseScope::new(solver_scope, 0);
                let mut current_score = phase_scope.calculate_score();

                loop {
                    let mut step_scope = StepScope::new(&mut phase_scope);
                    arena.reset();
                    arena.extend((self.0).$idx.iter_moves(step_scope.score_director()));

                    let best_index = find_best_improving_move_index(&arena, &mut step_scope, &current_score);

                    if let Some((selected_index, selected_score)) = best_index {
                        let selected_move = arena.take(selected_index);
                        selected_move.do_move(step_scope.score_director_mut());
                        step_scope.score_director_mut().clear_undo_stack();
                        step_scope.set_step_score(selected_score);
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
        impl<S, C, M, $($MS),+> Phase<S, C> for VndPhase<($($MS,)+), M>
        where
            S: PlanningSolution + solverforge_scoring::ShadowVariableSupport,
            S::Score: Score,
            C: ConstraintSet<S, S::Score>,
            M: Move<S>,
            $($MS: MoveSelector<S, M>,)+
        {
            fn solve(&mut self, solver_scope: &mut SolverScope<S, C>) {
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

                    let best_index = find_best_improving_move_index(&arena, &mut step_scope, &current_score);

                    if let Some((selected_index, selected_score)) = best_index {
                        let selected_move = arena.take(selected_index);
                        selected_move.do_move(step_scope.score_director_mut());
                        step_scope.score_director_mut().clear_undo_stack();
                        step_scope.set_step_score(selected_score);
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

/// Finds the index of the best improving move in the arena.
///
/// Returns `Some((index, score))` if an improving move is found, `None` otherwise.
fn find_best_improving_move_index<S, C, M>(
    arena: &MoveArena<M>,
    step_scope: &mut StepScope<'_, '_, '_, S, C>,
    current_score: &S::Score,
) -> Option<(usize, S::Score)>
where
    S: PlanningSolution + solverforge_scoring::ShadowVariableSupport,
    S::Score: Score,
    C: ConstraintSet<S, S::Score>,
    M: Move<S>,
{
    let mut best: Option<(usize, S::Score)> = None;

    for i in 0..arena.len() {
        let m = arena.get(i).unwrap();

        if !m.is_doable(step_scope.score_director()) {
            continue;
        }

        // Evaluate move: save score, execute, calculate, undo
        let sd = step_scope.score_director_mut();
        sd.save_score_snapshot();
        m.do_move(sd);
        let move_score = sd.calculate_score();
        sd.undo_changes();

        if move_score > *current_score {
            match &best {
                Some((_, best_score)) if move_score > *best_score => {
                    best = Some((i, move_score));
                }
                None => {
                    best = Some((i, move_score));
                }
                _ => {}
            }
        }
    }

    best
}

impl_vnd_phase!(0: MS0);
impl_vnd_phase!(0: MS0, 1: MS1);
impl_vnd_phase!(0: MS0, 1: MS1, 2: MS2);
impl_vnd_phase!(0: MS0, 1: MS1, 2: MS2, 3: MS3);
impl_vnd_phase!(0: MS0, 1: MS1, 2: MS2, 3: MS3, 4: MS4);
impl_vnd_phase!(0: MS0, 1: MS1, 2: MS2, 3: MS3, 4: MS4, 5: MS5);
impl_vnd_phase!(0: MS0, 1: MS1, 2: MS2, 3: MS3, 4: MS4, 5: MS5, 6: MS6);
impl_vnd_phase!(0: MS0, 1: MS1, 2: MS2, 3: MS3, 4: MS4, 5: MS5, 6: MS6, 7: MS7);
