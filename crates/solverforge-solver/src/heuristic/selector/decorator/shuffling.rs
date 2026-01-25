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
/// use solverforge_solver::heuristic::selector::MoveSelector;
/// use solverforge_solver::heuristic::r#move::ChangeMove;
/// use solverforge_core::domain::PlanningSolution;
/// use solverforge_core::score::SimpleScore;
/// use solverforge_scoring::api::constraint_set::ConstraintSet;
/// use solverforge_scoring::ScoreDirector;
///
/// #[derive(Clone, Debug)]
/// struct Solution { value: i32, score: Option<SimpleScore> }
///
/// impl PlanningSolution for Solution {
///     type Score = SimpleScore;
///     fn score(&self) -> Option<Self::Score> { self.score }
///     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
/// }
///
/// // Simple mock selector
/// #[derive(Debug)]
/// struct MockSelector;
/// impl MoveSelector<Solution, ChangeMove<Solution, i32>> for MockSelector {
///     fn iter_moves<'a, C>(&'a self, _: &'a ScoreDirector<Solution, C>)
///         -> Box<dyn Iterator<Item = ChangeMove<Solution, i32>> + 'a>
///         where C: ConstraintSet<Solution, SimpleScore> { Box::new(std::iter::empty()) }
///     fn size<C>(&self, _: &ScoreDirector<Solution, C>) -> usize
///         where C: ConstraintSet<Solution, SimpleScore> { 5 }
///     fn is_never_ending(&self) -> bool { false }
/// }
///
/// let inner = MockSelector;
/// // Shuffle with a fixed seed for reproducibility
/// let shuffled: ShufflingMoveSelector<Solution, ChangeMove<Solution, i32>, _> =
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
