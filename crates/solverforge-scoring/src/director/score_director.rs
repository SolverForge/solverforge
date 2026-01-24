//! Score director for zero-erasure incremental scoring.
//!
//! This module provides `ScoreDirector` that uses monomorphized
//! constraint sets instead of trait-object-based scoring.

use std::marker::PhantomData;

use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};
use solverforge_core::score::Score;

use crate::api::constraint_set::ConstraintSet;

/// Score director for zero-erasure incremental scoring.
///
/// Uses a fully typed `ConstraintSet` where all constraint types
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
/// use solverforge_scoring::director::ScoreDirector;
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
///     |_s: &Solution, v: &Option<i32>| v.is_none(),
///     |_: &Option<i32>| SimpleScore::of(1),
///     false,
///     0, // descriptor_index
/// );
///
/// // Create score director with tuple-based constraint set
/// let solution = Solution { values: vec![Some(1), None, Some(2)], score: None };
/// let mut director = ScoreDirector::new(solution, (c1,));
///
/// // First calculation evaluates all constraints
/// let score = director.calculate_score();
/// assert_eq!(score, SimpleScore::of(-1)); // One unassigned
///
/// // Subsequent calculations are O(1) - return cached score
/// let score2 = director.calculate_score();
/// assert_eq!(score, score2);
/// ```
pub struct ScoreDirector<S, C>
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

impl<S, C> ScoreDirector<S, C>
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

    /// Consumes the director and returns the working solution with final score set.
    pub fn into_working_solution(mut self) -> S {
        self.working_solution.set_score(Some(self.cached_score));
        self.working_solution
    }

    /// Calculates and returns the current score.
    ///
    /// On first call, initializes all constraints (O(n) for uni, O(n²) for bi).
    /// Subsequent calls return the cached score (O(1)).
    ///
    /// Also sets the score on the working solution to keep it in sync.
    pub fn calculate_score(&mut self) -> S::Score {
        if !self.initialized {
            self.cached_score = self.constraints.initialize_all(&self.working_solution);
            self.initialized = true;
        }
        self.working_solution.set_score(Some(self.cached_score));
        self.cached_score
    }

    /// Called before a list element change. O(1) operation.
    ///
    /// # Arguments
    /// * `entity_index` - Entity owning the list
    /// * `position` - Position in the list
    /// * `element_idx` - Global index of the element
    #[inline]
    pub fn before_list_element_changed(
        &mut self,
        entity_index: usize,
        position: usize,
        element_idx: usize,
    ) {
        // Use all parameters
        let _ = position;
        if !self.initialized {
            return;
        }

        let delta = self
            .constraints
            .on_retract_all(&self.working_solution, 0, element_idx);
        self.cached_score = self.cached_score + delta;

        // Only retract entity if different from element (avoid double-counting)
        if entity_index != element_idx {
            let entity_delta =
                self.constraints
                    .on_retract_all(&self.working_solution, 0, entity_index);
            self.cached_score = self.cached_score + entity_delta;
        }
    }

    /// Called after a list element change. O(1) operation.
    ///
    /// # Arguments
    /// * `entity_index` - Entity owning the list
    /// * `position` - Position in the list
    /// * `element_idx` - Global index of the element
    #[inline]
    pub fn after_list_element_changed(
        &mut self,
        entity_index: usize,
        position: usize,
        element_idx: usize,
    ) {
        // Use all parameters
        let _ = position;
        if !self.initialized {
            return;
        }

        let delta = self
            .constraints
            .on_insert_all(&self.working_solution, 0, element_idx);
        self.cached_score = self.cached_score + delta;

        // Only insert entity if different from element (avoid double-counting)
        if entity_index != element_idx {
            let entity_delta =
                self.constraints
                    .on_insert_all(&self.working_solution, 0, entity_index);
            self.cached_score = self.cached_score + entity_delta;
        }
    }

    /// Called after a list element change with O(1) shadow update.
    ///
    /// # Arguments
    /// * `entity_index` - Entity owning the list
    /// * `position` - Position in the list
    /// * `element_idx` - Global index of the element
    #[inline]
    pub fn after_list_element_changed_with_shadows(
        &mut self,
        entity_index: usize,
        position: usize,
        element_idx: usize,
    ) where
        S: crate::director::ShadowVariableSupport,
    {
        if !self.initialized {
            return;
        }

        // O(1) shadow update for ONE element
        self.working_solution
            .update_element_shadow(entity_index, position, element_idx);

        let delta = self
            .constraints
            .on_insert_all(&self.working_solution, 0, element_idx);
        self.cached_score = self.cached_score + delta;

        let entity_delta = self
            .constraints
            .on_insert_all(&self.working_solution, 0, entity_index);
        self.cached_score = self.cached_score + entity_delta;
    }

    /// Convenience method for a complete list element change cycle.
    #[inline]
    pub fn do_list_change<F>(
        &mut self,
        entity_index: usize,
        position: usize,
        element_idx: usize,
        change_fn: F,
    ) -> S::Score
    where
        F: FnOnce(&mut S),
    {
        self.before_list_element_changed(entity_index, position, element_idx);
        change_fn(&mut self.working_solution);
        self.after_list_element_changed(entity_index, position, element_idx);
        self.cached_score
    }

    /// List element change cycle with O(1) shadow updates.
    #[inline]
    pub fn do_list_change_with_shadows<F>(
        &mut self,
        entity_index: usize,
        position: usize,
        element_idx: usize,
        change_fn: F,
    ) -> S::Score
    where
        S: crate::director::ShadowVariableSupport,
        F: FnOnce(&mut S),
    {
        self.before_list_element_changed(entity_index, position, element_idx);
        change_fn(&mut self.working_solution);
        self.after_list_element_changed_with_shadows(entity_index, position, element_idx);
        self.cached_score
    }

    /// Returns the cached score without recalculation.
    ///
    /// Returns zero score if not yet initialized.
    #[inline]
    pub fn get_score(&self) -> S::Score {
        self.cached_score
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

    /// Returns constraint match totals for score analysis.
    ///
    /// Returns a vector of (name, weight, score, match_count) tuples.
    pub fn constraint_match_totals(&self) -> Vec<(String, S::Score, S::Score, usize)> {
        self.constraints
            .evaluate_each(&self.working_solution)
            .into_iter()
            .map(|r| {
                // Weight is approximated from score / match_count
                let weight = if r.match_count > 0 {
                    r.score
                } else {
                    S::Score::zero()
                };
                (r.name, weight, r.score, r.match_count)
            })
            .collect()
    }

    /// Returns the solution descriptor.
    pub fn solution_descriptor(&self) -> &SolutionDescriptor {
        &self.solution_descriptor
    }

    /// Returns the number of entities for a given descriptor index.
    pub fn entity_count(&self, descriptor_index: usize) -> usize {
        (self.entity_counter)(&self.working_solution, descriptor_index)
    }

    /// Returns the total number of entities across all collections.
    pub fn total_entity_count(&self) -> usize {
        (0..self.solution_descriptor.entity_descriptors.len())
            .map(|i| (self.entity_counter)(&self.working_solution, i))
            .sum()
    }

    /// Returns true - this director supports incremental scoring.
    pub fn is_incremental(&self) -> bool {
        true
    }

    /// Called before a basic (non-list) planning variable is changed.
    pub fn before_variable_changed(&mut self, descriptor_index: usize, entity_index: usize) {
        if !self.initialized {
            return;
        }
        let delta =
            self.constraints
                .on_retract_all(&self.working_solution, descriptor_index, entity_index);
        self.cached_score = self.cached_score + delta;
    }

    /// Called after a basic (non-list) planning variable is changed.
    pub fn after_variable_changed(&mut self, descriptor_index: usize, entity_index: usize) {
        if !self.initialized {
            return;
        }
        let delta =
            self.constraints
                .on_insert_all(&self.working_solution, descriptor_index, entity_index);
        self.cached_score = self.cached_score + delta;
    }

    /// Retracts an element from constraint scoring.
    ///
    /// This is a low-level primitive for constraint operations only.
    /// Does NOT handle shadow variables - that's done by proc-macro-generated code.
    ///
    /// # Arguments
    /// * `descriptor_index` - The descriptor index for the element collection
    /// * `entity_index` - The entity owning the list
    /// * `element_idx` - Global index of the element
    #[inline]
    pub fn retract_element(
        &mut self,
        descriptor_index: usize,
        entity_index: usize,
        element_idx: usize,
    ) {
        if !self.initialized {
            return;
        }

        // Retract the specific element from constraints
        let delta =
            self.constraints
                .on_retract_all(&self.working_solution, descriptor_index, element_idx);
        self.cached_score = self.cached_score + delta;

        // Also retract entity-level constraints
        let entity_delta =
            self.constraints
                .on_retract_all(&self.working_solution, descriptor_index, entity_index);
        self.cached_score = self.cached_score + entity_delta;
    }

    /// Inserts an element into constraint scoring.
    ///
    /// This is a low-level primitive for constraint operations only.
    /// Does NOT handle shadow variables - that's done by proc-macro-generated code.
    ///
    /// # Arguments
    /// * `descriptor_index` - The descriptor index for the element collection
    /// * `entity_index` - The entity owning the list
    /// * `element_idx` - Global index of the element
    #[inline]
    pub fn insert_element(
        &mut self,
        descriptor_index: usize,
        entity_index: usize,
        element_idx: usize,
    ) {
        if !self.initialized {
            return;
        }

        // Insert the specific element into constraints
        let delta =
            self.constraints
                .on_insert_all(&self.working_solution, descriptor_index, element_idx);
        self.cached_score = self.cached_score + delta;

        // Also insert entity-level constraints
        let entity_delta =
            self.constraints
                .on_insert_all(&self.working_solution, descriptor_index, entity_index);
        self.cached_score = self.cached_score + entity_delta;
    }

    /// Registers an undo closure for move reversal.
    #[inline]
    pub fn register_undo(&mut self, _undo: Box<dyn FnOnce(&mut S) + Send>) {
        // Default: no-op - RecordingScoreDirector overrides this
    }

    /// Consumes the director and returns the working solution.
    ///
    /// Use this to extract the final solution after solving.
    ///
    /// # Examples
    ///
    /// ```
    /// use solverforge_scoring::director::ScoreDirector;
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
    /// let director = ScoreDirector::new(solution, ());
    /// let result = director.take_solution();
    /// assert_eq!(result.values, vec![1, 2, 3]);
    /// ```
    pub fn take_solution(self) -> S {
        self.working_solution
    }
}

impl<S, C> std::fmt::Debug for ScoreDirector<S, C>
where
    S: PlanningSolution + std::fmt::Debug,
    S::Score: std::fmt::Debug,
    C: ConstraintSet<S, S::Score>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScoreDirector")
            .field("initialized", &self.initialized)
            .field("cached_score", &self.cached_score)
            .field("constraint_count", &self.constraints.constraint_count())
            .finish()
    }
}
