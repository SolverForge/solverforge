//! Shuffling move selector decorator.
//!
//! Shuffles moves from an inner selector using Fisher-Yates.

use std::cell::RefCell;
use std::fmt::Debug;
use std::marker::PhantomData;

use rand::prelude::SliceRandom;
use rand::rngs::StdRng;
use rand::SeedableRng;

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use solverforge_scoring::api::constraint_set::ConstraintSet;
use solverforge_scoring::ScoreDirector;

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::typed_move_selector::MoveSelector;

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
/// use solverforge_core::score::SimpleScore;
///
/// #[derive(Clone, Debug)]
/// struct Task { id: usize, priority: Option<i32> }
///
/// #[derive(Clone, Debug)]
/// struct Solution { tasks: Vec<Task>, score: Option<SimpleScore> }
///
/// impl PlanningSolution for Solution {
///     type Score = SimpleScore;
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
    _phantom: PhantomData<(S, M)>,
}

// SAFETY: RefCell<StdRng> is only accessed from a single thread at a time
// via the `iter_moves` method. The Send bound on MoveSelector ensures
// the selector itself is only used from one thread.
unsafe impl<S, M, Inner: Send> Send for ShufflingMoveSelector<S, M, Inner> {}

impl<S, M, Inner> ShufflingMoveSelector<S, M, Inner> {
    /// Creates a new shuffling selector with a random seed.
    pub fn new(inner: Inner) -> Self {
        Self {
            inner,
            rng: RefCell::new(StdRng::from_os_rng()),
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
    S::Score: Score,
    M: Move<S>,
    Inner: MoveSelector<S, M>,
{
    fn iter_moves<'a, C>(
        &'a self,
        score_director: &'a ScoreDirector<S, C>,
    ) -> Box<dyn Iterator<Item = M> + 'a>
    where
        C: ConstraintSet<S, S::Score>,
    {
        let mut moves: Vec<M> = self.inner.iter_moves(score_director).collect();
        moves.shuffle(&mut *self.rng.borrow_mut());
        Box::new(moves.into_iter())
    }

    fn size<C>(&self, score_director: &ScoreDirector<S, C>) -> usize
    where
        C: ConstraintSet<S, S::Score>,
    {
        self.inner.size(score_director)
    }

    fn is_never_ending(&self) -> bool {
        false
    }
}
