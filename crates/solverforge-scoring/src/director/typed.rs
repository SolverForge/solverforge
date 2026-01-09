//! Typed score director for zero-erasure incremental scoring.
//!
//! This module provides `TypedScoreDirector` that uses monomorphized
//! constraint sets instead of trait-object-based scoring.

use std::any::Any;
use std::marker::PhantomData;

use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};
use solverforge_core::score::Score;

use crate::api::constraint_set::ConstraintSet;
use crate::director::ScoreDirector;

/// A typed score director for zero-erasure incremental scoring.
///
/// Unlike `IncrementalScoreDirector` which uses BAVET session with trait objects,
/// this director uses a fully typed `ConstraintSet` where all constraint types
/// are known at compile time, enabling complete monomorphization.
///
/// # Type Parameters
///
/// - `S`: The solution type (must implement `PlanningSolution`)
/// - `C`: The constraint set type (tuple of typed constraints)
///
/// # Example
///
/// ```
/// use solverforge_scoring::director::typed::TypedScoreDirector;
/// use solverforge_scoring::api::constraint_set::{ConstraintSet, IncrementalConstraint};
/// use solverforge_scoring::constraint::incremental::IncrementalUniConstraint;
/// use solverforge_core::domain::PlanningSolution;
/// use solverforge_core::{ConstraintRef, ImpactType};
/// use solverforge_core::score::SimpleScore;
///
/// #[derive(Clone)]
/// struct Solution {
///     values: Vec<Option<i32>>,
///     score: Option<SimpleScore>,
/// }
///
/// impl PlanningSolution for Solution {
///     type Score = SimpleScore;
///     fn score(&self) -> Option<Self::Score> { self.score }
///     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
/// }
///
/// // Create zero-erasure constraint (all closures as generics)
/// let c1 = IncrementalUniConstraint::new(
///     ConstraintRef::new("", "Unassigned"),
///     ImpactType::Penalty,
///     |s: &Solution| s.values.as_slice(),
///     |v: &Option<i32>| v.is_none(),
///     |_: &Option<i32>| SimpleScore::of(1),
///     false,
/// );
///
/// // Create typed director with tuple-based constraint set
/// let solution = Solution { values: vec![Some(1), None, Some(2)], score: None };
/// let mut director = TypedScoreDirector::new(solution, (c1,));
///
/// // First calculation evaluates all constraints
/// let score = director.calculate_score();
/// assert_eq!(score, SimpleScore::of(-1)); // One unassigned
///
/// // Subsequent calculations are O(1) - return cached score
/// let score2 = director.calculate_score();
/// assert_eq!(score, score2);
/// ```
pub struct TypedScoreDirector<S, C>
where
    S: PlanningSolution,
    C: ConstraintSet<S, S::Score>,
{
    /// The working solution.
    working_solution: S,
    /// The typed constraint set.
    constraints: C,
    /// Cached score.
    cached_score: S::Score,
    /// Whether the director has been initialized.
    initialized: bool,
    /// Solution descriptor for trait interface compatibility.
    solution_descriptor: SolutionDescriptor,
    /// Typed entity counter function.
    ///
    /// Returns the number of entities for the given descriptor index.
    /// This is a typed function pointer that preserves full type information
    /// throughout the solver pipeline.
    entity_counter: fn(&S, usize) -> usize,
    /// Phantom for score type.
    _phantom: PhantomData<S::Score>,
}

impl<S, C> TypedScoreDirector<S, C>
where
    S: PlanningSolution,
    S::Score: Score,
    C: ConstraintSet<S, S::Score>,
{
    /// Creates a new typed score director with an empty descriptor.
    ///
    /// Use this for manual solver loops that don't need the `ScoreDirector` trait.
    /// For full solver infrastructure integration, use `with_descriptor()`.
    ///
    /// The constraints should be a tuple of typed constraints (e.g., `(C1, C2, C3)`).
    pub fn new(solution: S, constraints: C) -> Self {
        use std::any::TypeId;
        Self::with_descriptor(
            solution,
            constraints,
            SolutionDescriptor::new("", TypeId::of::<()>()),
            |_, _| 0,
        )
    }

    /// Creates a new typed score director with a solution descriptor.
    ///
    /// This constructor enables the `ScoreDirector` trait implementation for
    /// integration with the full solver infrastructure (phases, move selectors, etc.).
    ///
    /// # Arguments
    ///
    /// * `solution` - The initial solution
    /// * `constraints` - Tuple of typed constraints (e.g., `(C1, C2, C3)`)
    /// * `solution_descriptor` - Metadata for solver infrastructure
    /// * `entity_counter` - Typed function returning entity count for descriptor index
    pub fn with_descriptor(
        solution: S,
        constraints: C,
        solution_descriptor: SolutionDescriptor,
        entity_counter: fn(&S, usize) -> usize,
    ) -> Self {
        Self {
            working_solution: solution,
            constraints,
            cached_score: S::Score::zero(),
            initialized: false,
            solution_descriptor,
            entity_counter,
            _phantom: PhantomData,
        }
    }

    /// Returns a reference to the working solution.
    pub fn working_solution(&self) -> &S {
        &self.working_solution
    }

    /// Returns a mutable reference to the working solution.
    ///
    /// Note: After modifying the solution directly, you must call
    /// `reset()` to recalculate the score from scratch.
    pub fn working_solution_mut(&mut self) -> &mut S {
        &mut self.working_solution
    }

    /// Calculates and returns the current score.
    ///
    /// On first call, initializes all constraints (O(n) for uni, O(n²) for bi).
    /// Subsequent calls return the cached score (O(1)).
    pub fn calculate_score(&mut self) -> S::Score {
        if !self.initialized {
            self.cached_score = self.constraints.initialize_all(&self.working_solution);
            self.initialized = true;
        }
        self.cached_score.clone()
    }

    /// Called before changing an entity's variable.
    ///
    /// This retracts the entity from all constraints, computing the delta
    /// that will be applied when the change completes.
    ///
    /// # Arguments
    ///
    /// * `entity_index` - Index of the entity being changed
    #[inline]
    pub fn before_variable_changed(&mut self, entity_index: usize) {
        if !self.initialized {
            // If not initialized, full calculation will happen on next calculate_score
            return;
        }

        let delta = self
            .constraints
            .on_retract_all(&self.working_solution, entity_index);
        self.cached_score = self.cached_score.clone() + delta;
    }

    /// Called after changing an entity's variable.
    ///
    /// This inserts the entity (with new state) into all constraints,
    /// computing the delta and updating the cached score.
    ///
    /// # Arguments
    ///
    /// * `entity_index` - Index of the entity that was changed
    #[inline]
    pub fn after_variable_changed(&mut self, entity_index: usize) {
        if !self.initialized {
            return;
        }

        let delta = self
            .constraints
            .on_insert_all(&self.working_solution, entity_index);
        self.cached_score = self.cached_score.clone() + delta;
    }

    /// Convenience method for a complete variable change cycle.
    ///
    /// Equivalent to:
    /// 1. `before_variable_changed(entity_index)`
    /// 2. Apply the change via `change_fn`
    /// 3. `after_variable_changed(entity_index)`
    #[inline]
    pub fn do_change<F>(&mut self, entity_index: usize, change_fn: F) -> S::Score
    where
        F: FnOnce(&mut S),
    {
        self.before_variable_changed(entity_index);
        change_fn(&mut self.working_solution);
        self.after_variable_changed(entity_index);
        self.cached_score.clone()
    }

    /// Returns the cached score without recalculation.
    ///
    /// Returns zero score if not yet initialized.
    #[inline]
    pub fn get_score(&self) -> S::Score {
        self.cached_score.clone()
    }

    /// Resets the director state.
    ///
    /// Call this after major solution changes that bypass the
    /// before/after_variable_changed protocol.
    pub fn reset(&mut self) {
        self.constraints.reset_all();
        self.initialized = false;
        self.cached_score = S::Score::zero();
    }

    /// Clones the working solution.
    pub fn clone_working_solution(&self) -> S {
        self.working_solution.clone()
    }

    /// Returns a reference to the constraint set.
    pub fn constraints(&self) -> &C {
        &self.constraints
    }

    /// Returns a mutable reference to the constraint set.
    pub fn constraints_mut(&mut self) -> &mut C {
        &mut self.constraints
    }

    /// Returns the number of constraints.
    pub fn constraint_count(&self) -> usize {
        self.constraints.constraint_count()
    }

    /// Returns whether the director is initialized.
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Consumes the director and returns the working solution.
    ///
    /// Use this to extract the final solution after solving.
    ///
    /// # Examples
    ///
    /// ```
    /// use solverforge_scoring::director::typed::TypedScoreDirector;
    /// use solverforge_core::domain::PlanningSolution;
    /// use solverforge_core::score::SimpleScore;
    ///
    /// #[derive(Clone)]
    /// struct Solution {
    ///     values: Vec<i32>,
    ///     score: Option<SimpleScore>,
    /// }
    ///
    /// impl PlanningSolution for Solution {
    ///     type Score = SimpleScore;
    ///     fn score(&self) -> Option<Self::Score> { self.score }
    ///     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
    /// }
    ///
    /// let solution = Solution { values: vec![1, 2, 3], score: None };
    /// let director = TypedScoreDirector::new(solution, ());
    /// let result = director.take_solution();
    /// assert_eq!(result.values, vec![1, 2, 3]);
    /// ```
    pub fn take_solution(self) -> S {
        self.working_solution
    }
}

impl<S, C> std::fmt::Debug for TypedScoreDirector<S, C>
where
    S: PlanningSolution + std::fmt::Debug,
    S::Score: std::fmt::Debug,
    C: ConstraintSet<S, S::Score>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TypedScoreDirector")
            .field("initialized", &self.initialized)
            .field("cached_score", &self.cached_score)
            .field("constraint_count", &self.constraints.constraint_count())
            .finish()
    }
}

impl<S, C> ScoreDirector<S> for TypedScoreDirector<S, C>
where
    S: PlanningSolution,
    S::Score: Score,
    C: ConstraintSet<S, S::Score> + Send,
{
    fn working_solution(&self) -> &S {
        &self.working_solution
    }

    fn working_solution_mut(&mut self) -> &mut S {
        &mut self.working_solution
    }

    fn calculate_score(&mut self) -> S::Score {
        if !self.initialized {
            self.cached_score = self.constraints.initialize_all(&self.working_solution);
            self.initialized = true;
        }
        self.working_solution
            .set_score(Some(self.cached_score.clone()));
        self.cached_score.clone()
    }

    fn solution_descriptor(&self) -> &SolutionDescriptor {
        &self.solution_descriptor
    }

    fn clone_working_solution(&self) -> S {
        self.working_solution.clone()
    }

    fn before_variable_changed(
        &mut self,
        _descriptor_index: usize,
        entity_index: usize,
        _variable_name: &str,
    ) {
        if !self.initialized {
            return;
        }
        let delta = self
            .constraints
            .on_retract_all(&self.working_solution, entity_index);
        self.cached_score = self.cached_score.clone() + delta;
    }

    fn after_variable_changed(
        &mut self,
        _descriptor_index: usize,
        entity_index: usize,
        _variable_name: &str,
    ) {
        if !self.initialized {
            return;
        }
        let delta = self
            .constraints
            .on_insert_all(&self.working_solution, entity_index);
        self.cached_score = self.cached_score.clone() + delta;
    }

    fn trigger_variable_listeners(&mut self) {
        // No shadow variables in typed director (yet)
    }

    fn entity_count(&self, descriptor_index: usize) -> Option<usize> {
        Some((self.entity_counter)(&self.working_solution, descriptor_index))
    }

    fn total_entity_count(&self) -> Option<usize> {
        // Sum across all descriptor indices
        let count: usize = (0..self.solution_descriptor.entity_descriptors.len())
            .map(|i| (self.entity_counter)(&self.working_solution, i))
            .sum();
        Some(count)
    }

    fn get_entity(&self, _descriptor_index: usize, _entity_index: usize) -> Option<&dyn Any> {
        // Entity access through typed functions, not dyn Any
        None
    }

    fn is_incremental(&self) -> bool {
        true
    }

    fn reset(&mut self) {
        self.constraints.reset_all();
        self.initialized = false;
        self.cached_score = S::Score::zero();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::constraint_set::IncrementalConstraint;
    use crate::constraint::incremental::IncrementalUniConstraint;
    use solverforge_core::score::SimpleScore;
    use solverforge_core::{ConstraintRef, ImpactType};

    #[derive(Clone, Debug)]
    struct TestSolution {
        values: Vec<Option<i32>>,
        score: Option<SimpleScore>,
    }

    impl PlanningSolution for TestSolution {
        type Score = SimpleScore;

        fn score(&self) -> Option<Self::Score> {
            self.score
        }

        fn set_score(&mut self, score: Option<Self::Score>) {
            self.score = score;
        }
    }

    fn make_unassigned_constraint() -> impl IncrementalConstraint<TestSolution, SimpleScore> {
        IncrementalUniConstraint::new(
            ConstraintRef::new("", "Unassigned"),
            ImpactType::Penalty,
            |s: &TestSolution| s.values.as_slice(),
            |v: &Option<i32>| v.is_none(),
            |_v: &Option<i32>| SimpleScore::of(1),
            false,
        )
    }

    #[test]
    fn test_initial_score_calculation() {
        let solution = TestSolution {
            values: vec![Some(1), None, None, Some(2)],
            score: None,
        };

        let c1 = make_unassigned_constraint();
        let mut director = TypedScoreDirector::new(solution, (c1,));

        assert!(!director.is_initialized());
        let score = director.calculate_score();
        assert!(director.is_initialized());
        assert_eq!(score, SimpleScore::of(-2)); // 2 None values
    }

    #[test]
    fn test_cached_score_on_subsequent_calls() {
        let solution = TestSolution {
            values: vec![Some(1), None],
            score: None,
        };

        let c1 = make_unassigned_constraint();
        let mut director = TypedScoreDirector::new(solution, (c1,));

        let score1 = director.calculate_score();
        let score2 = director.calculate_score();
        assert_eq!(score1, score2);
        assert_eq!(score1, SimpleScore::of(-1));
    }

    #[test]
    fn test_incremental_update() {
        let solution = TestSolution {
            values: vec![Some(1), None, Some(2)],
            score: None,
        };

        let c1 = make_unassigned_constraint();
        let mut director = TypedScoreDirector::new(solution, (c1,));

        // Initialize
        let score = director.calculate_score();
        assert_eq!(score, SimpleScore::of(-1)); // One None at index 1

        // Change: None -> Some(3) at index 1
        director.before_variable_changed(1);
        director.working_solution_mut().values[1] = Some(3);
        director.after_variable_changed(1);

        // Score should improve (no more unassigned)
        let new_score = director.get_score();
        assert_eq!(new_score, SimpleScore::of(0));
    }

    #[test]
    fn test_do_change_convenience() {
        let solution = TestSolution {
            values: vec![Some(1), None],
            score: None,
        };

        let c1 = make_unassigned_constraint();
        let mut director = TypedScoreDirector::new(solution, (c1,));

        director.calculate_score();

        let new_score = director.do_change(1, |s| {
            s.values[1] = Some(5);
        });

        assert_eq!(new_score, SimpleScore::of(0));
    }

    #[test]
    fn test_reset() {
        let solution = TestSolution {
            values: vec![Some(1), None],
            score: None,
        };

        let c1 = make_unassigned_constraint();
        let mut director = TypedScoreDirector::new(solution, (c1,));

        director.calculate_score();
        assert!(director.is_initialized());

        director.reset();
        assert!(!director.is_initialized());
        assert_eq!(director.get_score(), SimpleScore::of(0)); // Zero after reset
    }

    #[test]
    fn test_clone_working_solution() {
        let solution = TestSolution {
            values: vec![Some(1), Some(2)],
            score: None,
        };

        let c1 = make_unassigned_constraint();
        let director = TypedScoreDirector::new(solution, (c1,));

        let cloned = director.clone_working_solution();
        assert_eq!(cloned.values.len(), 2);
        assert_eq!(cloned.values[0], Some(1));
    }

    #[test]
    fn test_constraint_count() {
        let solution = TestSolution {
            values: vec![],
            score: None,
        };

        let c1 = make_unassigned_constraint();
        let director = TypedScoreDirector::new(solution, (c1,));

        assert_eq!(director.constraint_count(), 1);
    }

    #[test]
    fn test_multiple_constraints() {
        let solution = TestSolution {
            values: vec![Some(1), None, Some(2)],
            score: None,
        };

        let c1 = make_unassigned_constraint();

        // Second constraint: reward assigned values
        let c2 = IncrementalUniConstraint::new(
            ConstraintRef::new("", "Assigned"),
            ImpactType::Reward,
            |s: &TestSolution| s.values.as_slice(),
            |v: &Option<i32>| v.is_some(),
            |_v: &Option<i32>| SimpleScore::of(1),
            false,
        );

        let mut director = TypedScoreDirector::new(solution, (c1, c2));

        assert_eq!(director.constraint_count(), 2);

        // Score: -1 (one None) + 2 (two Some) = 1
        let score = director.calculate_score();
        assert_eq!(score, SimpleScore::of(1));
    }

    #[test]
    fn test_debug_impl() {
        let solution = TestSolution {
            values: vec![Some(1)],
            score: None,
        };

        let c1 = make_unassigned_constraint();
        let director = TypedScoreDirector::new(solution, (c1,));

        let debug = format!("{:?}", director);
        assert!(debug.contains("TypedScoreDirector"));
        assert!(debug.contains("initialized"));
    }

    #[test]
    fn test_before_change_without_initialization() {
        let solution = TestSolution {
            values: vec![Some(1), None],
            score: None,
        };

        let c1 = make_unassigned_constraint();
        let mut director = TypedScoreDirector::new(solution, (c1,));

        // Call before/after without initialization - should not panic
        director.before_variable_changed(0);
        director.after_variable_changed(0);

        // Score should be calculated correctly on first call
        let score = director.calculate_score();
        assert_eq!(score, SimpleScore::of(-1));
    }

    #[test]
    fn test_add_then_remove_value() {
        let solution = TestSolution {
            values: vec![None, None],
            score: None,
        };

        let c1 = make_unassigned_constraint();
        let mut director = TypedScoreDirector::new(solution, (c1,));

        // Initialize: 2 Nones = -2
        let score = director.calculate_score();
        assert_eq!(score, SimpleScore::of(-2));

        // Assign first value: 1 None = -1
        director.do_change(0, |s| s.values[0] = Some(1));
        assert_eq!(director.get_score(), SimpleScore::of(-1));

        // Unassign first value: back to 2 Nones = -2
        director.do_change(0, |s| s.values[0] = None);
        assert_eq!(director.get_score(), SimpleScore::of(-2));
    }
}
