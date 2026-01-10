//! Cartesian product move selector.
//!
//! Combines moves from two selectors into composite moves.
//! For selectors A (producing M1) and B (producing M2), yields
//! CompositeMove for each pair (a, b) where a ∈ A and b ∈ B.
//!
//! # Zero-Erasure Design
//!
//! Both selectors are stored as concrete types. The resulting composite
//! moves are fully typed CompositeMove<S, M1, M2>.
//!
//! # Example
//!
//! ```
//! use solverforge_solver::heuristic::selector::decorator::CartesianProductMoveSelector;
//! use solverforge_solver::heuristic::selector::{ChangeMoveSelector, MoveSelector};
//! use solverforge_solver::heuristic::r#move::CompositeMove;
//! use solverforge_core::domain::{PlanningSolution, EntityDescriptor, SolutionDescriptor, TypedEntityExtractor};
//! use solverforge_core::score::SimpleScore;
//! use solverforge_scoring::{ScoreDirector, SimpleScoreDirector};
//! use std::any::TypeId;
//!
//! #[derive(Clone, Debug)]
//! struct Task { x: Option<i32>, y: Option<i32> }
//!
//! #[derive(Clone, Debug)]
//! struct Sol { tasks: Vec<Task>, score: Option<SimpleScore> }
//!
//! impl PlanningSolution for Sol {
//!     type Score = SimpleScore;
//!     fn score(&self) -> Option<Self::Score> { self.score }
//!     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
//! }
//!
//! fn get_tasks(s: &Sol) -> &Vec<Task> { &s.tasks }
//! fn get_tasks_mut(s: &mut Sol) -> &mut Vec<Task> { &mut s.tasks }
//! fn get_x(s: &Sol, i: usize) -> Option<i32> { s.tasks.get(i).and_then(|t| t.x) }
//! fn set_x(s: &mut Sol, i: usize, v: Option<i32>) {
//!     if let Some(t) = s.tasks.get_mut(i) { t.x = v; }
//! }
//! fn get_y(s: &Sol, i: usize) -> Option<i32> { s.tasks.get(i).and_then(|t| t.y) }
//! fn set_y(s: &mut Sol, i: usize, v: Option<i32>) {
//!     if let Some(t) = s.tasks.get_mut(i) { t.y = v; }
//! }
//!
//! // Selector for x values: 1, 2
//! let x_selector = ChangeMoveSelector::simple(
//!     get_x, set_x, 0, "x", vec![1, 2],
//! );
//! // Selector for y values: 10, 20, 30
//! let y_selector = ChangeMoveSelector::simple(
//!     get_y, set_y, 0, "y", vec![10, 20, 30],
//! );
//!
//! // Cartesian product: 2 * 3 = 6 composite moves
//! let product = CartesianProductMoveSelector::new(x_selector, y_selector);
//!
//! let solution = Sol { tasks: vec![Task { x: Some(0), y: Some(0) }], score: None };
//! let extractor = Box::new(TypedEntityExtractor::new("Task", "tasks", get_tasks, get_tasks_mut));
//! let entity_desc = EntityDescriptor::new("Task", TypeId::of::<Task>(), "tasks")
//!     .with_extractor(extractor);
//! let descriptor = SolutionDescriptor::new("Sol", TypeId::of::<Sol>()).with_entity(entity_desc);
//! let director = SimpleScoreDirector::with_calculator(
//!     solution, descriptor, |_| SimpleScore::of(0)
//! );
//!
//! // Produces 6 moves: (x=1,y=10), (x=1,y=20), (x=1,y=30), (x=2,y=10), (x=2,y=20), (x=2,y=30)
//! assert_eq!(product.size(&director), 6);
//! let moves: Vec<_> = product.iter_moves(&director).collect();
//! assert_eq!(moves.len(), 6);
//! ```

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use crate::heuristic::r#move::{CompositeMove, Move};
use crate::heuristic::selector::MoveSelector;

/// Combines moves from two selectors into composite moves (cartesian product).
///
/// For each move `m1` from the first selector and each move `m2` from the second,
/// yields a `CompositeMove<S, M1, M2>` that applies both moves in sequence.
///
/// # Type Parameters
/// * `S` - The planning solution type
/// * `M1` - Move type from first selector
/// * `M2` - Move type from second selector
/// * `A` - First selector type
/// * `B` - Second selector type
///
/// # Zero-Erasure
///
/// Both selectors are stored inline as concrete types. The resulting
/// CompositeMove is fully typed - no trait objects in the hot path.
pub struct CartesianProductMoveSelector<S, M1, M2, A, B> {
    first: A,
    second: B,
    _phantom: PhantomData<(S, M1, M2)>,
}

impl<S, M1, M2, A, B> CartesianProductMoveSelector<S, M1, M2, A, B> {
    /// Creates a new cartesian product selector.
    ///
    /// # Arguments
    /// * `first` - Selector for the first move type
    /// * `second` - Selector for the second move type
    ///
    /// The resulting moves apply `first` then `second`.
    pub fn new(first: A, second: B) -> Self {
        Self {
            first,
            second,
            _phantom: PhantomData,
        }
    }

    /// Returns a reference to the first selector.
    pub fn first(&self) -> &A {
        &self.first
    }

    /// Returns a reference to the second selector.
    pub fn second(&self) -> &B {
        &self.second
    }
}

impl<S, M1, M2, A: Debug, B: Debug> Debug for CartesianProductMoveSelector<S, M1, M2, A, B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CartesianProductMoveSelector")
            .field("first", &self.first)
            .field("second", &self.second)
            .finish()
    }
}

impl<S, M1, M2, A, B> MoveSelector<S, CompositeMove<S, M1, M2>>
    for CartesianProductMoveSelector<S, M1, M2, A, B>
where
    S: PlanningSolution,
    M1: Move<S>,
    M2: Move<S>,
    A: MoveSelector<S, M1>,
    B: MoveSelector<S, M2>,
{
    fn iter_moves<'a, D: ScoreDirector<S>>(
        &'a self,
        score_director: &'a D,
    ) -> Box<dyn Iterator<Item = CompositeMove<S, M1, M2>> + 'a> {
        // Materialize first selector's moves (required for cartesian product)
        let first_moves: Vec<M1> = self.first.iter_moves(score_director).collect();

        Box::new(first_moves.into_iter().flat_map(move |m1| {
            self.second
                .iter_moves(score_director)
                .map(move |m2| CompositeMove::new(m1.clone(), m2))
        }))
    }

    fn size<D: ScoreDirector<S>>(&self, score_director: &D) -> usize {
        self.first.size(score_director) * self.second.size(score_director)
    }

    fn is_never_ending(&self) -> bool {
        // If either is never-ending and the other is non-empty, product is never-ending
        self.first.is_never_ending() || self.second.is_never_ending()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn cartesian_product_yields_all_pairs() {
        let director = create_director(vec![Task {
            x: Some(0),
            y: Some(0),
        }]);

        let x_sel = ChangeMoveSelector::simple(get_x, set_x, 0, "x", vec![1, 2]);
        let y_sel = ChangeMoveSelector::simple(get_y, set_y, 0, "y", vec![10, 20, 30]);

        let product = CartesianProductMoveSelector::new(x_sel, y_sel);

        let moves: Vec<_> = product.iter_moves(&director).collect();

        // 2 * 3 = 6 moves
        assert_eq!(moves.len(), 6);
        assert_eq!(product.size(&director), 6);
    }

    #[test]
    fn cartesian_product_applies_both_moves() {
        let mut director = create_director(vec![Task {
            x: Some(0),
            y: Some(0),
        }]);

        let x_sel = ChangeMoveSelector::simple(get_x, set_x, 0, "x", vec![5]);
        let y_sel = ChangeMoveSelector::simple(get_y, set_y, 0, "y", vec![50]);

        let product = CartesianProductMoveSelector::new(x_sel, y_sel);
        let moves: Vec<_> = product.iter_moves(&director).collect();

        assert_eq!(moves.len(), 1);

        // Apply the composite move
        moves[0].do_move(&mut director);

        assert_eq!(get_x(director.working_solution(), 0), Some(5));
        assert_eq!(get_y(director.working_solution(), 0), Some(50));
    }

    #[test]
    fn empty_first_yields_empty() {
        let director = create_director(vec![Task {
            x: Some(0),
            y: Some(0),
        }]);

        let x_sel = ChangeMoveSelector::simple(get_x, set_x, 0, "x", vec![]);
        let y_sel = ChangeMoveSelector::simple(get_y, set_y, 0, "y", vec![10, 20]);

        let product = CartesianProductMoveSelector::new(x_sel, y_sel);
        let moves: Vec<_> = product.iter_moves(&director).collect();

        assert!(moves.is_empty());
        assert_eq!(product.size(&director), 0);
    }

    #[test]
    fn empty_second_yields_empty() {
        let director = create_director(vec![Task {
            x: Some(0),
            y: Some(0),
        }]);

        let x_sel = ChangeMoveSelector::simple(get_x, set_x, 0, "x", vec![1, 2]);
        let y_sel = ChangeMoveSelector::simple(get_y, set_y, 0, "y", vec![]);

        let product = CartesianProductMoveSelector::new(x_sel, y_sel);
        let moves: Vec<_> = product.iter_moves(&director).collect();

        assert!(moves.is_empty());
        assert_eq!(product.size(&director), 0);
    }

    #[test]
    fn single_from_each_yields_single_composite() {
        let director = create_director(vec![Task {
            x: Some(0),
            y: Some(0),
        }]);

        let x_sel = ChangeMoveSelector::simple(get_x, set_x, 0, "x", vec![1]);
        let y_sel = ChangeMoveSelector::simple(get_y, set_y, 0, "y", vec![10]);

        let product = CartesianProductMoveSelector::new(x_sel, y_sel);
        let moves: Vec<_> = product.iter_moves(&director).collect();

        assert_eq!(moves.len(), 1);
        assert_eq!(product.size(&director), 1);
    }

    #[test]
    fn maintains_order() {
        let director = create_director(vec![Task {
            x: Some(0),
            y: Some(0),
        }]);

        let x_sel = ChangeMoveSelector::simple(get_x, set_x, 0, "x", vec![1, 2]);
        let y_sel = ChangeMoveSelector::simple(get_y, set_y, 0, "y", vec![10, 20]);

        let product = CartesianProductMoveSelector::new(x_sel, y_sel);
        let moves: Vec<_> = product.iter_moves(&director).collect();

        // Order should be: (1,10), (1,20), (2,10), (2,20)
        assert_eq!(moves.len(), 4);

        // Check first move's x-move and second move's y-move values
        assert_eq!(moves[0].first().to_value(), Some(&1));
        assert_eq!(moves[0].second().to_value(), Some(&10));

        assert_eq!(moves[1].first().to_value(), Some(&1));
        assert_eq!(moves[1].second().to_value(), Some(&20));

        assert_eq!(moves[2].first().to_value(), Some(&2));
        assert_eq!(moves[2].second().to_value(), Some(&10));

        assert_eq!(moves[3].first().to_value(), Some(&2));
        assert_eq!(moves[3].second().to_value(), Some(&20));
    }
}
