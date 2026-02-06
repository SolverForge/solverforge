//! Cartesian product move selector.
//!
//! Combines moves from two selectors by storing them in separate arenas
//! and yielding CompositeMove references for each pair.
//!
//! # Zero-Erasure Design
//!
//! Moves are stored in typed arenas. The cartesian product iterator
//! yields indices into both arenas. The caller creates CompositeMove
//! references on-the-fly for each evaluation - no cloning.

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use crate::heuristic::r#move::{Move, MoveArena};
use crate::heuristic::selector::MoveSelector;

/// Holds two arenas of moves and provides iteration over all pairs.
///
/// This is NOT a MoveSelector - it's a specialized structure for
/// cartesian product iteration that preserves zero-erasure.
///
/// # Type Parameters
/// * `S` - The planning solution type
/// * `M1` - First move type
/// * `M2` - Second move type
pub struct CartesianProductArena<S, M1, M2>
where
    S: PlanningSolution,
    M1: Move<S>,
    M2: Move<S>,
{
    arena_1: MoveArena<M1>,
    arena_2: MoveArena<M2>,
    _phantom: PhantomData<fn() -> S>,
}

impl<S, M1, M2> CartesianProductArena<S, M1, M2>
where
    S: PlanningSolution,
    M1: Move<S>,
    M2: Move<S>,
{
    /// Creates a new empty cartesian product arena.
    pub fn new() -> Self {
        Self {
            arena_1: MoveArena::new(),
            arena_2: MoveArena::new(),
            _phantom: PhantomData,
        }
    }

    /// Resets both arenas for the next step.
    pub fn reset(&mut self) {
        self.arena_1.reset();
        self.arena_2.reset();
    }

    /// Populates arena 1 from a move selector.
    pub fn populate_first<D, MS>(&mut self, selector: &MS, score_director: &D)
    where
        D: ScoreDirector<S>,
        MS: MoveSelector<S, M1>,
    {
        self.arena_1.extend(selector.iter_moves(score_director));
    }

    /// Populates arena 2 from a move selector.
    pub fn populate_second<D, MS>(&mut self, selector: &MS, score_director: &D)
    where
        D: ScoreDirector<S>,
        MS: MoveSelector<S, M2>,
    {
        self.arena_2.extend(selector.iter_moves(score_director));
    }

    /// Returns the number of pairs (size of cartesian product).
    pub fn len(&self) -> usize {
        self.arena_1.len() * self.arena_2.len()
    }

    /// Returns true if either arena is empty.
    pub fn is_empty(&self) -> bool {
        self.arena_1.is_empty() || self.arena_2.is_empty()
    }

    /// Returns the first move at the given index.
    pub fn get_first(&self, index: usize) -> Option<&M1> {
        self.arena_1.get(index)
    }

    /// Returns the second move at the given index.
    pub fn get_second(&self, index: usize) -> Option<&M2> {
        self.arena_2.get(index)
    }

    /// Returns an iterator over all (i, j) index pairs.
    pub fn iter_indices(&self) -> impl Iterator<Item = (usize, usize)> + '_ {
        let len_1 = self.arena_1.len();
        let len_2 = self.arena_2.len();
        (0..len_1).flat_map(move |i| (0..len_2).map(move |j| (i, j)))
    }

    /// Returns an iterator over all (i, j) pairs with references to both moves.
    pub fn iter_pairs(&self) -> impl Iterator<Item = (usize, usize, &M1, &M2)> + '_ {
        self.iter_indices().filter_map(|(i, j)| {
            let m1 = self.arena_1.get(i)?;
            let m2 = self.arena_2.get(j)?;
            Some((i, j, m1, m2))
        })
    }
}

impl<S, M1, M2> Default for CartesianProductArena<S, M1, M2>
where
    S: PlanningSolution,
    M1: Move<S>,
    M2: Move<S>,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<S, M1, M2> Debug for CartesianProductArena<S, M1, M2>
where
    S: PlanningSolution,
    M1: Move<S>,
    M2: Move<S>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CartesianProductArena")
            .field("arena_1_len", &self.arena_1.len())
            .field("arena_2_len", &self.arena_2.len())
            .finish()
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
        x: Option<i32>,
        y: Option<i32>,
    }

    #[derive(Clone, Debug)]
    struct Sol {
        tasks: Vec<Task>,
        score: Option<SimpleScore>,
    }

    impl PlanningSolution for Sol {
        type Score = SimpleScore;
        fn score(&self) -> Option<Self::Score> {
            self.score
        }
        fn set_score(&mut self, score: Option<Self::Score>) {
            self.score = score;
        }
    }

    fn get_tasks(s: &Sol) -> &Vec<Task> {
        &s.tasks
    }
    fn get_tasks_mut(s: &mut Sol) -> &mut Vec<Task> {
        &mut s.tasks
    }
    fn get_x(s: &Sol, i: usize) -> Option<i32> {
        s.tasks.get(i).and_then(|t| t.x)
    }
    fn set_x(s: &mut Sol, i: usize, v: Option<i32>) {
        if let Some(t) = s.tasks.get_mut(i) {
            t.x = v;
        }
    }
    fn get_y(s: &Sol, i: usize) -> Option<i32> {
        s.tasks.get(i).and_then(|t| t.y)
    }
    fn set_y(s: &mut Sol, i: usize, v: Option<i32>) {
        if let Some(t) = s.tasks.get_mut(i) {
            t.y = v;
        }
    }

    fn create_director(tasks: Vec<Task>) -> SimpleScoreDirector<Sol, impl Fn(&Sol) -> SimpleScore> {
        let solution = Sol { tasks, score: None };
        let extractor = Box::new(TypedEntityExtractor::new(
            "Task",
            "tasks",
            get_tasks,
            get_tasks_mut,
        ));
        let entity_desc =
            EntityDescriptor::new("Task", TypeId::of::<Task>(), "tasks").with_extractor(extractor);
        let descriptor =
            SolutionDescriptor::new("Sol", TypeId::of::<Sol>()).with_entity(entity_desc);
        SimpleScoreDirector::with_calculator(solution, descriptor, |_| SimpleScore::of(0))
    }

    #[test]
    fn cartesian_product_arena_yields_all_pairs() {
        let director = create_director(vec![Task {
            x: Some(0),
            y: Some(0),
        }]);

        let x_selector = ChangeMoveSelector::simple(get_x, set_x, 0, "x", vec![1, 2]);
        let y_selector = ChangeMoveSelector::simple(get_y, set_y, 0, "y", vec![10, 20, 30]);

        let mut arena: CartesianProductArena<Sol, ChangeMove<Sol, i32>, ChangeMove<Sol, i32>> =
            CartesianProductArena::new();

        arena.populate_first(&x_selector, &director);
        arena.populate_second(&y_selector, &director);

        // 2 x-moves * 3 y-moves = 6 pairs
        assert_eq!(arena.len(), 6);

        let pairs: Vec<_> = arena.iter_pairs().collect();
        assert_eq!(pairs.len(), 6);
    }

    #[test]
    fn reset_clears_both_arenas() {
        let director = create_director(vec![Task {
            x: Some(0),
            y: Some(0),
        }]);

        let x_selector = ChangeMoveSelector::simple(get_x, set_x, 0, "x", vec![1, 2]);
        let y_selector = ChangeMoveSelector::simple(get_y, set_y, 0, "y", vec![10, 20]);

        let mut arena: CartesianProductArena<Sol, ChangeMove<Sol, i32>, ChangeMove<Sol, i32>> =
            CartesianProductArena::new();

        arena.populate_first(&x_selector, &director);
        arena.populate_second(&y_selector, &director);
        assert_eq!(arena.len(), 4);

        arena.reset();
        assert!(arena.is_empty());
        assert_eq!(arena.len(), 0);
    }
}
