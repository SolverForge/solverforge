/* Probability move selector decorator.

Probabilistically filters moves from an inner selector.
*/

use std::cell::RefCell;
use std::fmt::Debug;
use std::marker::PhantomData;

use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::move_selector::{MoveCandidateRef, MoveCursor, MoveSelector};

use super::indexed_cursor::IndexedMoveCursor;

/// Probabilistically filters moves from an inner selector.
///
/// Each move has a probability of being included based on a weight function.
/// Uses interior mutability for the RNG since `iter_moves` takes `&self`.
///
/// # Example
///
/// ```
/// use solverforge_solver::heuristic::selector::decorator::ProbabilityMoveSelector;
/// use solverforge_solver::heuristic::selector::move_selector::MoveCandidateRef;
/// use solverforge_solver::heuristic::selector::{ChangeMoveSelector, MoveSelector};
/// use solverforge_solver::heuristic::r#move::ChangeMove;
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
/// // Weight function: higher values have higher probability
/// fn weight_by_value(
///     m: MoveCandidateRef<'_, Solution, ChangeMove<Solution, i32>>,
/// ) -> f64 {
///     match m {
///         MoveCandidateRef::Borrowed(mov) => mov.to_value().map_or(0.0, |&v| v as f64),
///         _ => 0.0,
///     }
/// }
///
/// let inner = ChangeMoveSelector::simple(
///     get_priority, set_priority, 0, "priority", vec![10, 50, 100],
/// );
/// // Moves are selected proportionally to their weights
/// let probabilistic: ProbabilityMoveSelector<Solution, _, _> =
///     ProbabilityMoveSelector::with_seed(inner, weight_by_value, 42);
/// assert!(!probabilistic.is_never_ending());
/// ```
pub struct ProbabilityMoveSelector<S, M, Inner>
where
    S: PlanningSolution,
    M: Move<S>,
{
    inner: Inner,
    weight_fn: for<'a> fn(MoveCandidateRef<'a, S, M>) -> f64,
    rng: RefCell<StdRng>,
    _phantom: PhantomData<(fn() -> S, fn() -> M)>,
}

// SAFETY: RefCell<StdRng> is only accessed from a single thread at a time
unsafe impl<S, M, Inner: Send> Send for ProbabilityMoveSelector<S, M, Inner>
where
    S: PlanningSolution,
    M: Move<S>,
{
}

impl<S, M, Inner> ProbabilityMoveSelector<S, M, Inner>
where
    S: PlanningSolution,
    M: Move<S>,
{
    pub fn new(inner: Inner, weight_fn: for<'a> fn(MoveCandidateRef<'a, S, M>) -> f64) -> Self {
        Self {
            inner,
            weight_fn,
            rng: RefCell::new(StdRng::from_rng(&mut rand::rng())),
            _phantom: PhantomData,
        }
    }

    pub fn with_seed(
        inner: Inner,
        weight_fn: for<'a> fn(MoveCandidateRef<'a, S, M>) -> f64,
        seed: u64,
    ) -> Self {
        Self {
            inner,
            weight_fn,
            rng: RefCell::new(StdRng::seed_from_u64(seed)),
            _phantom: PhantomData,
        }
    }
}

impl<S, M, Inner: Debug> Debug for ProbabilityMoveSelector<S, M, Inner>
where
    S: PlanningSolution,
    M: Move<S>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProbabilityMoveSelector")
            .field("inner", &self.inner)
            .finish()
    }
}

impl<S, M, Inner> MoveSelector<S, M> for ProbabilityMoveSelector<S, M, Inner>
where
    S: PlanningSolution,
    M: Move<S>,
    Inner: MoveSelector<S, M>,
{
    type Cursor<'a>
        = IndexedMoveCursor<S, M, Inner::Cursor<'a>>
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<S>>(&'a self, score_director: &D) -> Self::Cursor<'a> {
        let mut inner = self.inner.open_cursor(score_director);
        let weight_fn = self.weight_fn;
        let mut weighted_indices = Vec::new();
        while let Some((child_index, candidate)) = inner.next_candidate() {
            weighted_indices.push((child_index, weight_fn(candidate)));
        }

        let total_weight: f64 = weighted_indices.iter().map(|(_, weight)| weight).sum();
        let mut selected = Vec::new();

        if total_weight > 0.0 {
            let mut rng = self.rng.borrow_mut();
            selected.reserve(weighted_indices.len());

            for (child_index, weight) in weighted_indices {
                let probability = weight / total_weight;
                if rng.random::<f64>() < probability {
                    selected.push(child_index);
                }
            }
        }

        IndexedMoveCursor::new(inner, selected)
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        self.inner.size(score_director)
    }

    fn is_never_ending(&self) -> bool {
        self.inner.is_never_ending()
    }
}

#[cfg(test)]
mod tests;
