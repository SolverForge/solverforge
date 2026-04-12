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
use crate::heuristic::selector::move_selector::MoveSelector;

/// Probabilistically filters moves from an inner selector.
///
/// Each move has a probability of being included based on a weight function.
/// Uses interior mutability for the RNG since `iter_moves` takes `&self`.
///
/// # Example
///
/// ```
/// use solverforge_solver::heuristic::selector::decorator::ProbabilityMoveSelector;
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
/// fn weight_by_value(m: &ChangeMove<Solution, i32>) -> f64 {
///     m.to_value().map_or(0.0, |&v| v as f64)
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
pub struct ProbabilityMoveSelector<S, M, Inner> {
    inner: Inner,
    weight_fn: fn(&M) -> f64,
    rng: RefCell<StdRng>,
    _phantom: PhantomData<(fn() -> S, fn() -> M)>,
}

// SAFETY: RefCell<StdRng> is only accessed from a single thread at a time
unsafe impl<S, M, Inner: Send> Send for ProbabilityMoveSelector<S, M, Inner> {}

impl<S, M, Inner> ProbabilityMoveSelector<S, M, Inner> {
    pub fn new(inner: Inner, weight_fn: fn(&M) -> f64) -> Self {
        Self {
            inner,
            weight_fn,
            rng: RefCell::new(StdRng::from_rng(&mut rand::rng())),
            _phantom: PhantomData,
        }
    }

    pub fn with_seed(inner: Inner, weight_fn: fn(&M) -> f64, seed: u64) -> Self {
        Self {
            inner,
            weight_fn,
            rng: RefCell::new(StdRng::seed_from_u64(seed)),
            _phantom: PhantomData,
        }
    }
}

impl<S, M, Inner: Debug> Debug for ProbabilityMoveSelector<S, M, Inner> {
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
    fn iter_moves<'a, D: Director<S>>(
        &'a self,
        score_director: &'a D,
    ) -> impl Iterator<Item = M> + 'a {
        let weight_fn = self.weight_fn;

        let moves_with_weights: Vec<(M, f64)> = self
            .inner
            .iter_moves(score_director)
            .map(|m| {
                let w = weight_fn(&m);
                (m, w)
            })
            .collect();

        let total_weight: f64 = moves_with_weights.iter().map(|(_, w)| w).sum();

        let mut selected = Vec::new();

        if total_weight > 0.0 {
            let mut rng = self.rng.borrow_mut();
            selected.reserve(moves_with_weights.len());

            for (m, weight) in moves_with_weights {
                let probability = weight / total_weight;
                if rng.random::<f64>() < probability {
                    selected.push(m);
                }
            }
        }

        selected.into_iter()
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        self.inner.size(score_director)
    }

    fn is_never_ending(&self) -> bool {
        self.inner.is_never_ending()
    }
}

#[cfg(test)]
#[path = "probability_tests.rs"]
mod tests;
