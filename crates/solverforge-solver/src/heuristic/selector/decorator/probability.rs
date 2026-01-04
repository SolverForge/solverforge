//! Probability move selector decorator.
//!
//! Probabilistically filters moves from an inner selector.

use std::cell::RefCell;
use std::fmt::Debug;
use std::marker::PhantomData;

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::typed_move_selector::MoveSelector;

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
/// // Weight function: higher values have higher probability
/// fn weight_by_value(m: &ChangeMove<Solution, i32>) -> f64 {
///     m.to_value().map_or(0.0, |&v| v as f64)
/// }
///
/// let inner = ChangeMoveSelector::<Solution, i32>::simple(
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
    _phantom: PhantomData<(S, M)>,
}

// SAFETY: RefCell<StdRng> is only accessed from a single thread at a time
// via the `iter_moves` method. The Send bound on MoveSelector ensures
// the selector itself is only used from one thread.
unsafe impl<S, M, Inner: Send> Send for ProbabilityMoveSelector<S, M, Inner> {}

impl<S, M, Inner> ProbabilityMoveSelector<S, M, Inner> {
    /// Creates a new probability selector with a random seed.
    ///
    /// # Arguments
    /// * `inner` - The inner selector to filter
    /// * `weight_fn` - Function that returns the selection weight for a move
    pub fn new(inner: Inner, weight_fn: fn(&M) -> f64) -> Self {
        Self {
            inner,
            weight_fn,
            rng: RefCell::new(StdRng::from_os_rng()),
            _phantom: PhantomData,
        }
    }

    /// Creates a new probability selector with a specific seed.
    ///
    /// Use this for reproducible selection in tests.
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
    fn iter_moves<'a>(
        &'a self,
        score_director: &'a dyn ScoreDirector<S>,
    ) -> Box<dyn Iterator<Item = M> + 'a> {
        let weight_fn = self.weight_fn;

        // Collect moves with their weights
        let moves_with_weights: Vec<(M, f64)> = self
            .inner
            .iter_moves(score_director)
            .map(|m| {
                let w = weight_fn(&m);
                (m, w)
            })
            .collect();

        let total_weight: f64 = moves_with_weights.iter().map(|(_, w)| w).sum();

        if total_weight <= 0.0 {
            return Box::new(std::iter::empty());
        }

        // Use weighted random selection (roulette wheel)
        // Each move's probability = weight / total_weight
        let mut rng = self.rng.borrow_mut();
        let mut selected = Vec::with_capacity(moves_with_weights.len());

        for (m, weight) in moves_with_weights {
            let probability = weight / total_weight;
            if rng.random::<f64>() < probability {
                selected.push(m);
            }
        }

        Box::new(selected.into_iter())
    }

    fn size(&self, score_director: &dyn ScoreDirector<S>) -> usize {
        // Size is approximate - we don't know how many will be selected
        self.inner.size(score_director)
    }

    fn is_never_ending(&self) -> bool {
        self.inner.is_never_ending()
    }
}

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
    struct Task {
        priority: Option<i32>,
    }

    #[derive(Clone, Debug)]
    struct TaskSolution {
        tasks: Vec<Task>,
        score: Option<SimpleScore>,
    }

    impl PlanningSolution for TaskSolution {
        type Score = SimpleScore;
        fn score(&self) -> Option<Self::Score> {
            self.score
        }
        fn set_score(&mut self, score: Option<Self::Score>) {
            self.score = score;
        }
    }

    fn get_tasks(s: &TaskSolution) -> &Vec<Task> {
        &s.tasks
    }
    fn get_tasks_mut(s: &mut TaskSolution) -> &mut Vec<Task> {
        &mut s.tasks
    }
    fn get_priority(s: &TaskSolution, i: usize) -> Option<i32> {
        s.tasks.get(i).and_then(|t| t.priority)
    }
    fn set_priority(s: &mut TaskSolution, i: usize, v: Option<i32>) {
        if let Some(t) = s.tasks.get_mut(i) {
            t.priority = v;
        }
    }

    fn create_director(
        tasks: Vec<Task>,
    ) -> SimpleScoreDirector<TaskSolution, impl Fn(&TaskSolution) -> SimpleScore> {
        let solution = TaskSolution { tasks, score: None };
        let extractor = Box::new(TypedEntityExtractor::new(
            "Task",
            "tasks",
            get_tasks,
            get_tasks_mut,
        ));
        let entity_desc =
            EntityDescriptor::new("Task", TypeId::of::<Task>(), "tasks").with_extractor(extractor);
        let descriptor = SolutionDescriptor::new("TaskSolution", TypeId::of::<TaskSolution>())
            .with_entity(entity_desc);
        SimpleScoreDirector::with_calculator(solution, descriptor, |_| SimpleScore::of(0))
    }

    fn uniform_weight(_: &ChangeMove<TaskSolution, i32>) -> f64 {
        1.0
    }

    fn zero_weight(_: &ChangeMove<TaskSolution, i32>) -> f64 {
        0.0
    }

    #[test]
    fn selects_some_moves_with_uniform_weight() {
        let director = create_director(vec![Task { priority: Some(1) }]);

        // Run multiple times to ensure some moves are selected
        let mut total_selected = 0;
        for seed in 0..10 {
            let inner = ChangeMoveSelector::<TaskSolution, i32>::simple(
                get_priority,
                set_priority,
                0,
                "priority",
                vec![10, 20, 30, 40, 50],
            );
            let prob = ProbabilityMoveSelector::with_seed(inner, uniform_weight, seed);
            total_selected += prob.iter_moves(&director).count();
        }

        // With uniform weight, each move has 20% chance of selection
        // Over 10 runs of 5 moves each, we expect some selection
        assert!(total_selected > 0);
    }

    #[test]
    fn zero_weight_selects_nothing() {
        let director = create_director(vec![Task { priority: Some(1) }]);

        let inner = ChangeMoveSelector::<TaskSolution, i32>::simple(
            get_priority,
            set_priority,
            0,
            "priority",
            vec![10, 20, 30],
        );
        let prob = ProbabilityMoveSelector::with_seed(inner, zero_weight, 42);

        let moves: Vec<_> = prob.iter_moves(&director).collect();
        assert!(moves.is_empty());
    }

    #[test]
    fn same_seed_produces_same_selection() {
        let director = create_director(vec![Task { priority: Some(1) }]);

        let inner1 = ChangeMoveSelector::<TaskSolution, i32>::simple(
            get_priority,
            set_priority,
            0,
            "priority",
            vec![10, 20, 30, 40, 50],
        );
        let prob1 = ProbabilityMoveSelector::with_seed(inner1, uniform_weight, 42);

        let inner2 = ChangeMoveSelector::<TaskSolution, i32>::simple(
            get_priority,
            set_priority,
            0,
            "priority",
            vec![10, 20, 30, 40, 50],
        );
        let prob2 = ProbabilityMoveSelector::with_seed(inner2, uniform_weight, 42);

        let moves1: Vec<_> = prob1
            .iter_moves(&director)
            .filter_map(|m| m.to_value().copied())
            .collect();
        let moves2: Vec<_> = prob2
            .iter_moves(&director)
            .filter_map(|m| m.to_value().copied())
            .collect();

        assert_eq!(moves1, moves2);
    }
}
