/* Shuffling move selector decorator.

Shuffles moves from an inner selector using Fisher-Yates.
*/

use std::cell::RefCell;
use std::fmt::Debug;
use std::marker::PhantomData;

use rand::prelude::SliceRandom;
use rand::rngs::StdRng;
use rand::SeedableRng;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::move_selector::MoveSelector;

/// Shuffles moves from an inner selector randomly.
///
/// Collects all moves from the inner selector and yields them in random order.
/// Uses interior mutability for the RNG since `iter_moves` takes `&self`.
///
/// # Example
///
/// ```
/// use solverforge_solver::heuristic::selector::decorator::ShufflingMoveSelector;
/// use solverforge_solver::heuristic::selector::{ChangeMoveSelector, MoveSelector};
/// use solverforge_core::domain::PlanningSolution;
/// use solverforge_core::score::SoftScore;
///
/// #[derive(Clone, Debug)]
/// struct Task { id: usize, priority: Option<i32> }
///
/// #[derive(Clone, Debug)]
/// struct Solution { tasks: Vec<Task>, score: Option<SoftScore> }
///
/// impl PlanningSolution for Solution {
///     type Score = SoftScore;
///     fn score(&self) -> Option<Self::Score> { self.score }
///     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
/// }
///
/// fn get_priority(s: &Solution, i: usize) -> Option<i32> { s.tasks.get(i).and_then(|t| t.priority) }
/// fn set_priority(s: &mut Solution, i: usize, v: Option<i32>) { if let Some(t) = s.tasks.get_mut(i) { t.priority = v; } }
///
/// let inner = ChangeMoveSelector::simple(
///     get_priority, set_priority, 0, "priority", vec![1, 2, 3, 4, 5],
/// );
/// // Shuffle with a fixed seed for reproducibility
/// let shuffled: ShufflingMoveSelector<Solution, _, _> =
///     ShufflingMoveSelector::with_seed(inner, 42);
/// assert!(!shuffled.is_never_ending());
/// ```
pub struct ShufflingMoveSelector<S, M, Inner> {
    inner: Inner,
    rng: RefCell<StdRng>,
    _phantom: PhantomData<(fn() -> S, fn() -> M)>,
}

/* SAFETY: RefCell<StdRng> is only accessed from a single thread at a time
via the `iter_moves` method. The Send bound on MoveSelector ensures
the selector itself is only used from one thread.
*/
unsafe impl<S, M, Inner: Send> Send for ShufflingMoveSelector<S, M, Inner> {}

impl<S, M, Inner> ShufflingMoveSelector<S, M, Inner> {
    pub fn new(inner: Inner) -> Self {
        Self {
            inner,
            rng: RefCell::new(StdRng::from_rng(&mut rand::rng())),
            _phantom: PhantomData,
        }
    }

    /// Creates a new shuffling selector with a specific seed.
    ///
    /// Use this for reproducible shuffling in tests.
    pub fn with_seed(inner: Inner, seed: u64) -> Self {
        Self {
            inner,
            rng: RefCell::new(StdRng::seed_from_u64(seed)),
            _phantom: PhantomData,
        }
    }
}

impl<S, M, Inner: Debug> Debug for ShufflingMoveSelector<S, M, Inner> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ShufflingMoveSelector")
            .field("inner", &self.inner)
            .finish()
    }
}

impl<S, M, Inner> MoveSelector<S, M> for ShufflingMoveSelector<S, M, Inner>
where
    S: PlanningSolution,
    M: Move<S>,
    Inner: MoveSelector<S, M>,
{
    fn iter_moves<'a, D: Director<S>>(
        &'a self,
        score_director: &'a D,
    ) -> impl Iterator<Item = M> + 'a {
        let mut moves: Vec<M> = self.inner.iter_moves(score_director).collect();
        moves.shuffle(&mut *self.rng.borrow_mut());
        moves.into_iter()
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        self.inner.size(score_director)
    }

    fn is_never_ending(&self) -> bool {
        false
    }
}

#[cfg(test)]
#[path = "shuffling_tests.rs"]
mod tests;
