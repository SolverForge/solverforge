// Variable Neighborhood Descent phase implementation.

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::{Director, RecordingDirector};

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::MoveSelector;
use crate::phase::control::{
    settle_search_interrupt, should_interrupt_evaluation, should_interrupt_generation,
    StepInterrupt,
};
use crate::phase::Phase;
use crate::scope::ProgressCallback;
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
/// use solverforge_solver::heuristic::selector::ChangeMoveSelector;
/// use solverforge_core::domain::PlanningSolution;
/// use solverforge_core::score::SoftScore;
///
/// #[derive(Clone, Debug)]
/// struct MySolution {
///     values: Vec<Option<i32>>,
///     score: Option<SoftScore>,
/// }
///
/// impl PlanningSolution for MySolution {
///     type Score = SoftScore;
///     fn score(&self) -> Option<Self::Score> { self.score }
///     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
/// }
///
/// fn get_value(s: &MySolution, idx: usize) -> Option<i32> {
///     s.values.get(idx).copied().flatten()
/// }
/// fn set_value(s: &mut MySolution, idx: usize, v: Option<i32>) {
///     if let Some(slot) = s.values.get_mut(idx) { *slot = v; }
/// }
///
/// type MyMove = ChangeMove<MySolution, i32>;
///
/// let selector = ChangeMoveSelector::simple(
///     get_value, set_value, 0, "value", vec![1, 2, 3]
/// );
///
/// // Single neighborhood VND
/// let vnd: VndPhase<_, MyMove> = VndPhase::new((selector,));
/// ```
pub struct VndPhase<T, M>(pub T, PhantomData<fn() -> M>);

impl<T: Debug, M> Debug for VndPhase<T, M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("VndPhase").field(&self.0).finish()
    }
}

impl<T, M> VndPhase<T, M> {
    pub fn new(neighborhoods: T) -> Self {
        Self(neighborhoods, PhantomData)
    }
}

// Generates `Phase` implementations for VndPhase with tuple neighborhoods.
macro_rules! impl_vnd_phase {
    // Single neighborhood
    ($idx:tt: $MS:ident) => {
        impl<S, D, BestCb, M, $MS> Phase<S, D, BestCb> for VndPhase<($MS,), M>
        where
            S: PlanningSolution,
            D: Director<S>,
            BestCb: ProgressCallback<S>,
            M: Move<S>,
            $MS: MoveSelector<S, M>,
        {
            fn solve(&mut self, solver_scope: &mut SolverScope<S, D, BestCb>) {
                let mut phase_scope = PhaseScope::new(solver_scope, 0);
                let mut current_score = phase_scope.calculate_score();

                loop {
                    let mut step_scope = StepScope::new(&mut phase_scope);
                    let cursor = (self.0).$idx.open_cursor(step_scope.score_director());

                    match find_best_improving_move(cursor, &mut step_scope, &current_score) {
                        MoveSearchResult::Found(selected_move, selected_score) => {
                            step_scope.apply_committed_move(&selected_move);
                            step_scope.set_step_score(selected_score);
                            current_score = selected_score;
                            step_scope.phase_scope_mut().update_best_solution();
                            step_scope.complete();
                        }
                        MoveSearchResult::NotFound => {
                            step_scope.complete();
                            break;
                        }
                        MoveSearchResult::Interrupted => match settle_search_interrupt(&mut step_scope) {
                            StepInterrupt::Restart => continue,
                            StepInterrupt::TerminatePhase => break,
                        },
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
        impl<S, D, BestCb, M, $($MS),+> Phase<S, D, BestCb> for VndPhase<($($MS,)+), M>
        where
            S: PlanningSolution,
            D: Director<S>,
            BestCb: ProgressCallback<S>,
            M: Move<S>,
            $($MS: MoveSelector<S, M>,)+
        {
            fn solve(&mut self, solver_scope: &mut SolverScope<S, D, BestCb>) {
                const COUNT: usize = impl_vnd_phase!(@count $($idx),+);
                let mut phase_scope = PhaseScope::new(solver_scope, 0);
                let mut current_score = phase_scope.calculate_score();
                let mut k = 0usize;

                while k < COUNT {
                    let mut step_scope = StepScope::new(&mut phase_scope);
                    let search_result = match k {
                        $($idx => {
                            let cursor = (self.0).$idx.open_cursor(step_scope.score_director());
                            find_best_improving_move(cursor, &mut step_scope, &current_score)
                        },)+
                        _ => MoveSearchResult::NotFound,
                    };

                    match search_result {
                        MoveSearchResult::Found(selected_move, selected_score) => {
                            step_scope.apply_committed_move(&selected_move);
                            step_scope.set_step_score(selected_score);
                            current_score = selected_score;
                            step_scope.phase_scope_mut().update_best_solution();
                            step_scope.complete();
                            k = 0; // Restart from first neighborhood
                        }
                        MoveSearchResult::NotFound => {
                            step_scope.complete();
                            k += 1; // Try next neighborhood
                        }
                        MoveSearchResult::Interrupted => match settle_search_interrupt(&mut step_scope) {
                            StepInterrupt::Restart => continue,
                            StepInterrupt::TerminatePhase => break,
                        },
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

/* Finds the index of the best improving move in the arena.

Returns `Some((index, score))` if an improving move is found, `None` otherwise.
*/
enum MoveSearchResult<M, Sc> {
    Found(M, Sc),
    NotFound,
    Interrupted,
}

fn find_best_improving_move<S, D, BestCb, M, I>(
    moves: I,
    step_scope: &mut StepScope<'_, '_, '_, S, D, BestCb>,
    current_score: &S::Score,
) -> MoveSearchResult<M, S::Score>
where
    S: PlanningSolution,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
    M: Move<S>,
    I: IntoIterator<Item = M>,
{
    let mut best: Option<(M, S::Score)> = None;

    for (generated, mov) in moves.into_iter().enumerate() {
        if should_interrupt_generation(step_scope, generated) {
            return MoveSearchResult::Interrupted;
        }

        if should_interrupt_evaluation(step_scope, generated) {
            return MoveSearchResult::Interrupted;
        }

        if !mov.is_doable(step_scope.score_director()) {
            continue;
        }

        let mut recording = RecordingDirector::new(step_scope.score_director_mut());
        mov.do_move(&mut recording);
        let move_score = recording.calculate_score();
        recording.undo_changes();

        if move_score > *current_score {
            match &best {
                Some((_, best_score)) if move_score > *best_score => {
                    best = Some((mov, move_score));
                }
                None => {
                    best = Some((mov, move_score));
                }
                _ => {}
            }
        }
    }

    match best {
        Some((mov, score)) => MoveSearchResult::Found(mov, score),
        None => MoveSearchResult::NotFound,
    }
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
#[path = "phase_tests.rs"]
mod tests;
