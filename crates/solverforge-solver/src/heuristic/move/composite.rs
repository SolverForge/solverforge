//! CompositeMove - applies two moves in sequence.
//!
//! Combines two typed moves into a single atomic move, enabling cartesian
//! product move selection and multi-variable moves.
//!
//! # Zero-Erasure Design
//!
//! Both inner moves are stored as concrete types `M1` and `M2`. No trait objects,
//! no boxing. The combined entity indices are stored inline in a SmallVec.
//!
//! # Example
//!
//! ```
//! use solverforge_solver::heuristic::r#move::{CompositeMove, ChangeMove, Move};
//! use solverforge_core::domain::PlanningSolution;
//! use solverforge_core::score::SimpleScore;
//! use solverforge_scoring::{ScoreDirector, SimpleScoreDirector};
//! use solverforge_core::domain::SolutionDescriptor;
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
//! fn get_x(s: &Sol, i: usize) -> Option<i32> { s.tasks.get(i).and_then(|t| t.x) }
//! fn set_x(s: &mut Sol, i: usize, v: Option<i32>) {
//!     if let Some(t) = s.tasks.get_mut(i) { t.x = v; }
//! }
//! fn get_y(s: &Sol, i: usize) -> Option<i32> { s.tasks.get(i).and_then(|t| t.y) }
//! fn set_y(s: &mut Sol, i: usize, v: Option<i32>) {
//!     if let Some(t) = s.tasks.get_mut(i) { t.y = v; }
//! }
//!
//! // Create two change moves for different variables
//! let move_x = ChangeMove::new(0, Some(5), get_x, set_x, "x", 0);
//! let move_y = ChangeMove::new(0, Some(10), get_y, set_y, "y", 0);
//!
//! // Combine into a composite move
//! let composite = CompositeMove::new(move_x, move_y);
//!
//! // Create a score director for testing
//! let solution = Sol { tasks: vec![Task { x: Some(1), y: Some(2) }], score: None };
//! let descriptor = SolutionDescriptor::new("Sol", TypeId::of::<Sol>());
//! let mut director = SimpleScoreDirector::with_calculator(
//!     solution, descriptor, |_| SimpleScore::of(0)
//! );
//!
//! // Composite is doable if both moves are doable
//! assert!(composite.is_doable(&director));
//!
//! // Execute applies both moves in sequence
//! composite.do_move(&mut director);
//!
//! // Both changes applied
//! assert_eq!(get_x(director.working_solution(), 0), Some(5));
//! assert_eq!(get_y(director.working_solution(), 0), Some(10));
//! ```

use std::fmt::Debug;

use smallvec::SmallVec;
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use super::Move;

/// A move that applies two moves in sequence.
///
/// Combines typed moves `M1` and `M2` into a single atomic move.
/// Execution order is: first, then second.
///
/// # Type Parameters
/// * `S` - The planning solution type
/// * `M1` - The first move type
/// * `M2` - The second move type
///
/// # Zero-Erasure
///
/// Both moves are stored inline as concrete types. No `Box<dyn Move>`,
/// no trait objects in the hot path.
#[derive(Clone)]
pub struct CompositeMove<S, M1, M2>
where
    S: PlanningSolution,
    M1: Move<S>,
    M2: Move<S>,
{
    first: M1,
    second: M2,
    /// Combined entity indices from both moves
    combined_indices: SmallVec<[usize; 8]>,
    _phantom: std::marker::PhantomData<S>,
}

impl<S, M1, M2> Debug for CompositeMove<S, M1, M2>
where
    S: PlanningSolution,
    M1: Move<S>,
    M2: Move<S>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompositeMove")
            .field("first", &self.first)
            .field("second", &self.second)
            .field("combined_indices", &self.combined_indices.as_slice())
            .finish()
    }
}

impl<S, M1, M2> CompositeMove<S, M1, M2>
where
    S: PlanningSolution,
    M1: Move<S>,
    M2: Move<S>,
{
    /// Creates a new composite move combining two moves.
    ///
    /// # Arguments
    /// * `first` - The first move to apply
    /// * `second` - The second move to apply
    ///
    /// The moves are executed in order: first, then second.
    pub fn new(first: M1, second: M2) -> Self {
        // Combine entity indices from both moves
        let mut combined_indices = SmallVec::new();
        combined_indices.extend_from_slice(first.entity_indices());
        for idx in second.entity_indices() {
            if !combined_indices.contains(idx) {
                combined_indices.push(*idx);
            }
        }

        Self {
            first,
            second,
            combined_indices,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Returns a reference to the first move.
    pub fn first(&self) -> &M1 {
        &self.first
    }

    /// Returns a reference to the second move.
    pub fn second(&self) -> &M2 {
        &self.second
    }
}

impl<S, M1, M2> Move<S> for CompositeMove<S, M1, M2>
where
    S: PlanningSolution,
    M1: Move<S>,
    M2: Move<S>,
{
    fn is_doable(&self, score_director: &dyn ScoreDirector<S>) -> bool {
        // Both moves must be doable
        self.first.is_doable(score_director) && self.second.is_doable(score_director)
    }

    fn do_move(&self, score_director: &mut dyn ScoreDirector<S>) {
        // Execute in sequence: first, then second
        self.first.do_move(score_director);
        self.second.do_move(score_director);
    }

    fn descriptor_index(&self) -> usize {
        // Use first move's descriptor
        self.first.descriptor_index()
    }

    fn entity_indices(&self) -> &[usize] {
        &self.combined_indices
    }

    fn variable_name(&self) -> &str {
        // Use first move's variable name
        self.first.variable_name()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::heuristic::r#move::ChangeMove;
    use solverforge_core::domain::SolutionDescriptor;
    use solverforge_core::score::SimpleScore;
    use solverforge_scoring::{RecordingScoreDirector, SimpleScoreDirector};
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

    fn create_director(
        tasks: Vec<Task>,
    ) -> SimpleScoreDirector<Sol, impl Fn(&Sol) -> SimpleScore> {
        let solution = Sol { tasks, score: None };
        let descriptor = SolutionDescriptor::new("Sol", TypeId::of::<Sol>());
        SimpleScoreDirector::with_calculator(solution, descriptor, |_| SimpleScore::of(0))
    }

    #[test]
    fn composite_applies_both_moves() {
        let tasks = vec![Task {
            x: Some(1),
            y: Some(2),
        }];
        let mut director = create_director(tasks);

        let move_x = ChangeMove::new(0, Some(5), get_x, set_x, "x", 0);
        let move_y = ChangeMove::new(0, Some(10), get_y, set_y, "y", 0);
        let composite = CompositeMove::new(move_x, move_y);

        assert!(composite.is_doable(&director));
        composite.do_move(&mut director);

        assert_eq!(get_x(director.working_solution(), 0), Some(5));
        assert_eq!(get_y(director.working_solution(), 0), Some(10));
    }

    #[test]
    fn composite_undo_restores_both() {
        let tasks = vec![Task {
            x: Some(1),
            y: Some(2),
        }];
        let mut director = create_director(tasks);

        let move_x = ChangeMove::new(0, Some(5), get_x, set_x, "x", 0);
        let move_y = ChangeMove::new(0, Some(10), get_y, set_y, "y", 0);
        let composite = CompositeMove::new(move_x, move_y);

        {
            let mut recording = RecordingScoreDirector::new(&mut director);
            composite.do_move(&mut recording);

            assert_eq!(get_x(recording.working_solution(), 0), Some(5));
            assert_eq!(get_y(recording.working_solution(), 0), Some(10));

            recording.undo_changes();
        }

        // Both restored
        assert_eq!(get_x(director.working_solution(), 0), Some(1));
        assert_eq!(get_y(director.working_solution(), 0), Some(2));
    }

    #[test]
    fn composite_not_doable_if_first_not_doable() {
        let tasks = vec![Task {
            x: Some(5),
            y: Some(2),
        }];
        let director = create_director(tasks);

        // First move: x is already 5, so not doable
        let move_x = ChangeMove::new(0, Some(5), get_x, set_x, "x", 0);
        let move_y = ChangeMove::new(0, Some(10), get_y, set_y, "y", 0);
        let composite = CompositeMove::new(move_x, move_y);

        assert!(!composite.is_doable(&director));
    }

    #[test]
    fn composite_not_doable_if_second_not_doable() {
        let tasks = vec![Task {
            x: Some(1),
            y: Some(10),
        }];
        let director = create_director(tasks);

        let move_x = ChangeMove::new(0, Some(5), get_x, set_x, "x", 0);
        // Second move: y is already 10, so not doable
        let move_y = ChangeMove::new(0, Some(10), get_y, set_y, "y", 0);
        let composite = CompositeMove::new(move_x, move_y);

        assert!(!composite.is_doable(&director));
    }

    #[test]
    fn composite_combines_entity_indices() {
        // Different entities
        let move_x = ChangeMove::new(0, Some(5), get_x, set_x, "x", 0);
        let move_y = ChangeMove::new(1, Some(10), get_y, set_y, "y", 0);
        let composite = CompositeMove::new(move_x, move_y);

        assert_eq!(composite.entity_indices(), &[0, 1]);
    }

    #[test]
    fn composite_deduplicates_entity_indices() {
        // Same entity
        let move_x = ChangeMove::new(0, Some(5), get_x, set_x, "x", 0);
        let move_y = ChangeMove::new(0, Some(10), get_y, set_y, "y", 0);
        let composite = CompositeMove::new(move_x, move_y);

        // Should only have entity 0 once
        assert_eq!(composite.entity_indices(), &[0]);
    }

    #[test]
    fn composite_uses_first_move_descriptor() {
        let move_x = ChangeMove::new(0, Some(5), get_x, set_x, "x", 1);
        let move_y = ChangeMove::new(0, Some(10), get_y, set_y, "y", 2);
        let composite = CompositeMove::new(move_x, move_y);

        assert_eq!(composite.descriptor_index(), 1);
        assert_eq!(composite.variable_name(), "x");
    }
}
