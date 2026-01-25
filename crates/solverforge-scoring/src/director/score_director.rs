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
    /// Undo stack for move reversal.
    ///
    /// Moves register undo closures via `register_undo()`. These closures
    /// are executed in reverse order by `undo_changes()` to restore the
    /// solution to its pre-move state.
    undo_stack: Vec<Box<dyn FnOnce(&mut S) + Send>>,
    /// Entities modified during the current move evaluation.
    ///
    /// Tracks (descriptor_index, entity_index) pairs. During undo, we need to
    /// properly retract/insert these entities to keep constraint tracking in sync.
    modified_entities: Vec<(usize, usize)>,
    /// Pre-move score for debug assertions.
    ///
    /// Used to verify that undo_changes() correctly restores the score.
    #[cfg(debug_assertions)]
    pre_move_score: Option<S::Score>,
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
            undo_stack: Vec::with_capacity(16),
            modified_entities: Vec::with_capacity(8),
            #[cfg(debug_assertions)]
            pre_move_score: None,
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

        // Track element for proper undo
        self.modified_entities.push((0, element_idx));
        // Track entity if different from element
        if entity_index != element_idx {
            self.modified_entities.push((0, entity_index));
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
        // Track this entity for proper undo (retract/insert during undo_changes)
        self.modified_entities
            .push((descriptor_index, entity_index));

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
    ///
    /// Moves call this method to register closures that restore the solution
    /// to its pre-move state. The closures are stored on a stack and executed
    /// in reverse order by `undo_changes()`.
    ///
    /// # Arguments
    /// * `undo` - A closure that restores the solution state
    #[inline]
    pub fn register_undo(&mut self, undo: Box<dyn FnOnce(&mut S) + Send>) {
        self.undo_stack.push(undo);
    }

    /// Prepares for move evaluation by saving the current score for debug verification.
    ///
    /// Call this BEFORE evaluating a move. The actual undo mechanism tracks
    /// modified entities automatically via `before_variable_changed()`.
    ///
    /// In debug builds, this saves the current score so that `undo_changes()`
    /// can verify the score was correctly restored.
    #[inline]
    pub fn save_score_snapshot(&mut self) {
        #[cfg(debug_assertions)]
        {
            self.pre_move_score = Some(self.cached_score);
        }
    }

    /// Undoes all registered changes with proper constraint tracking restoration.
    ///
    /// This method properly restores both the solution state AND the constraint
    /// tracking by:
    /// 1. Retracting current (post-move) values from constraints (reverse order)
    /// 2. Running undo closures to restore old values (reverse order)
    /// 3. Inserting restored (pre-move) values into constraints (reverse order)
    ///
    /// The net effect is that both the solution and cached_score return to
    /// their pre-move state.
    ///
    /// # Why Reverse Order for Insert?
    ///
    /// For bi-constraints that track entity pairs (e.g., same-employee conflicts),
    /// the order of inserts matters. When A and B have the same join key, the
    /// match (A,B) should be found exactly once during insert. Using reverse
    /// order mirrors the forward move's sequence, ensuring consistent behavior.
    #[inline]
    pub fn undo_changes(&mut self) {
        if !self.initialized {
            // Just clear the stacks if not initialized
            self.undo_stack.clear();
            self.modified_entities.clear();
            #[cfg(debug_assertions)]
            {
                self.pre_move_score = None;
            }
            return;
        }

        // Step 1: Retract current (post-move) values from constraints
        // This undoes the after_variable_changed inserts
        for &(descriptor_index, entity_index) in self.modified_entities.iter().rev() {
            let delta = self.constraints.on_retract_all(
                &self.working_solution,
                descriptor_index,
                entity_index,
            );
            self.cached_score = self.cached_score + delta;
        }

        // Step 2: Run undo closures in reverse order to restore old values
        while let Some(undo) = self.undo_stack.pop() {
            undo(&mut self.working_solution);
        }

        // Step 3: Insert restored (pre-move) values into constraints
        // This undoes the before_variable_changed retracts
        // Must collect first and iterate in REVERSE order to mirror forward sequence
        let entities: Vec<_> = self.modified_entities.drain(..).collect();
        for (descriptor_index, entity_index) in entities.into_iter().rev() {
            let delta = self.constraints.on_insert_all(
                &self.working_solution,
                descriptor_index,
                entity_index,
            );
            self.cached_score = self.cached_score + delta;
        }

        // Debug assertion: verify score was correctly restored
        #[cfg(debug_assertions)]
        if let Some(expected) = self.pre_move_score.take() {
            debug_assert_eq!(
                self.cached_score, expected,
                "Undo score mismatch: expected {:?}, got {:?}",
                expected, self.cached_score
            );
        }
    }

    /// Clears the undo stack without executing the closures.
    ///
    /// Call this after a move has been permanently applied (i.e., accepted
    /// and executed "for real"). This discards the undo closures since the
    /// move is now part of the committed solution state.
    #[inline]
    pub fn clear_undo_stack(&mut self) {
        self.undo_stack.clear();
        self.modified_entities.clear();
        #[cfg(debug_assertions)]
        {
            self.pre_move_score = None;
        }
    }

    /// Returns the number of undo closures on the stack.
    #[inline]
    pub fn undo_stack_len(&self) -> usize {
        self.undo_stack.len()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constraint::incremental::IncrementalUniConstraint;
    use crate::constraint::IncrementalBiConstraint;
    use solverforge_core::score::SimpleScore;
    use solverforge_core::{ConstraintRef, ImpactType};

    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    struct Shift {
        id: usize,
        employee: Option<usize>,
    }

    #[derive(Clone)]
    struct TestSolution {
        shifts: Vec<Shift>,
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

    fn create_unassigned_constraint() -> IncrementalUniConstraint<
        TestSolution,
        Shift,
        impl Fn(&TestSolution) -> &[Shift],
        impl Fn(&TestSolution, &Shift) -> bool,
        impl Fn(&Shift) -> SimpleScore,
        SimpleScore,
    > {
        IncrementalUniConstraint::new(
            ConstraintRef::new("", "Unassigned"),
            ImpactType::Penalty,
            |s: &TestSolution| s.shifts.as_slice(),
            |_s: &TestSolution, shift: &Shift| shift.employee.is_none(),
            |_: &Shift| SimpleScore::of(1),
            true,
            0,
        )
    }

    fn create_conflict_constraint() -> IncrementalBiConstraint<
        TestSolution,
        Shift,
        Option<usize>,
        impl Fn(&TestSolution) -> &[Shift],
        impl Fn(&Shift) -> Option<usize>,
        impl Fn(&TestSolution, &Shift, &Shift) -> bool,
        impl Fn(&Shift, &Shift) -> SimpleScore,
        SimpleScore,
    > {
        IncrementalBiConstraint::new(
            ConstraintRef::new("", "Employee conflict"),
            ImpactType::Penalty,
            |s: &TestSolution| s.shifts.as_slice(),
            |shift: &Shift| shift.employee,
            |_s: &TestSolution, a: &Shift, b: &Shift| {
                // Only count pairs where both are assigned to same employee
                a.employee.is_some() && a.id < b.id
            },
            |_: &Shift, _: &Shift| SimpleScore::of(1),
            true,
            0,
        )
    }

    /// Test 1: Single entity undo (simulates ChangeMove)
    #[test]
    fn test_change_move_undo_restores_score() {
        let solution = TestSolution {
            shifts: vec![
                Shift {
                    id: 0,
                    employee: None,
                },
                Shift {
                    id: 1,
                    employee: Some(1),
                },
            ],
            score: None,
        };

        let constraint = create_unassigned_constraint();
        let mut director = ScoreDirector::new(solution, (constraint,));

        // Initialize and get initial score
        let initial_score = director.calculate_score();
        assert_eq!(initial_score, SimpleScore::of(-1)); // One unassigned

        // Simulate ChangeMove: assign employee to shift 0
        director.save_score_snapshot();
        director.before_variable_changed(0, 0);
        director.working_solution_mut().shifts[0].employee = Some(1);
        // Register undo closure
        director.register_undo(Box::new(|s: &mut TestSolution| {
            s.shifts[0].employee = None;
        }));
        director.after_variable_changed(0, 0);

        // Score should be 0 (no unassigned shifts)
        let post_move_score = director.get_score();
        assert_eq!(post_move_score, SimpleScore::of(0));

        // Undo the move
        director.undo_changes();

        // Score should be restored to initial
        let restored_score = director.get_score();
        assert_eq!(restored_score, initial_score);

        // Solution should be restored
        assert_eq!(director.working_solution().shifts[0].employee, None);
    }

    /// Test 2: Multi-entity undo (simulates SwapMove)
    #[test]
    fn test_swap_move_undo_restores_score() {
        let solution = TestSolution {
            shifts: vec![
                Shift {
                    id: 0,
                    employee: Some(1),
                },
                Shift {
                    id: 1,
                    employee: None,
                },
            ],
            score: None,
        };

        let constraint = create_unassigned_constraint();
        let mut director = ScoreDirector::new(solution, (constraint,));

        let initial_score = director.calculate_score();
        assert_eq!(initial_score, SimpleScore::of(-1)); // One unassigned (shift 1)

        // Simulate SwapMove: swap employees between shift 0 and shift 1
        director.save_score_snapshot();

        // SwapMove modifies both entities
        director.before_variable_changed(0, 0);
        director.before_variable_changed(0, 1);

        let old_emp_0 = director.working_solution().shifts[0].employee;
        let old_emp_1 = director.working_solution().shifts[1].employee;

        director.working_solution_mut().shifts[0].employee = old_emp_1; // None
        director.working_solution_mut().shifts[1].employee = old_emp_0; // Some(1)

        // Register undo closures
        director.register_undo(Box::new(move |s: &mut TestSolution| {
            s.shifts[0].employee = old_emp_0;
        }));
        director.register_undo(Box::new(move |s: &mut TestSolution| {
            s.shifts[1].employee = old_emp_1;
        }));

        director.after_variable_changed(0, 0);
        director.after_variable_changed(0, 1);

        // Score should still be -1 (still one unassigned, but different one)
        let post_move_score = director.get_score();
        assert_eq!(post_move_score, SimpleScore::of(-1));

        // Undo the move
        director.undo_changes();

        // Score should be restored
        assert_eq!(director.get_score(), initial_score);

        // Solution should be restored
        assert_eq!(director.working_solution().shifts[0].employee, Some(1));
        assert_eq!(director.working_solution().shifts[1].employee, None);
    }

    /// Test 3: Bi-constraint tracking restoration
    #[test]
    fn test_bi_constraint_tracking_restored_after_undo() {
        let solution = TestSolution {
            shifts: vec![
                Shift {
                    id: 0,
                    employee: Some(1),
                }, // Same employee
                Shift {
                    id: 1,
                    employee: Some(1),
                }, // Same employee - conflict!
                Shift {
                    id: 2,
                    employee: Some(2),
                }, // Different employee
            ],
            score: None,
        };

        let constraint = create_conflict_constraint();
        let mut director = ScoreDirector::new(solution, (constraint,));

        // Initialize: should have 1 conflict (shifts 0 and 1 have same employee)
        let initial_score = director.calculate_score();
        assert_eq!(initial_score, SimpleScore::of(-1)); // One conflict

        // Change shift 1 to different employee (removes conflict)
        director.save_score_snapshot();
        director.before_variable_changed(0, 1);
        director.working_solution_mut().shifts[1].employee = Some(3);
        director.register_undo(Box::new(|s: &mut TestSolution| {
            s.shifts[1].employee = Some(1);
        }));
        director.after_variable_changed(0, 1);

        // Should have no conflicts now
        let post_move_score = director.get_score();
        assert_eq!(post_move_score, SimpleScore::of(0));

        // Undo the move
        director.undo_changes();

        // Score and tracking should be restored
        let restored_score = director.get_score();
        assert_eq!(restored_score, initial_score);
    }

    /// Test 4: Multiple sequential move evaluations
    #[test]
    fn test_multiple_move_evaluations() {
        let solution = TestSolution {
            shifts: vec![
                Shift {
                    id: 0,
                    employee: None,
                },
                Shift {
                    id: 1,
                    employee: None,
                },
                Shift {
                    id: 2,
                    employee: None,
                },
            ],
            score: None,
        };

        let constraint = create_unassigned_constraint();
        let mut director = ScoreDirector::new(solution, (constraint,));

        let initial_score = director.calculate_score();
        assert_eq!(initial_score, SimpleScore::of(-3)); // Three unassigned

        // Evaluate multiple moves (like local search does)
        // Each iteration: try assigning one shift, then undo
        for i in 0..3 {
            director.save_score_snapshot();
            director.before_variable_changed(0, i);
            director.working_solution_mut().shifts[i].employee = Some(1);
            director.register_undo(Box::new(move |s: &mut TestSolution| {
                s.shifts[i].employee = None;
            }));
            director.after_variable_changed(0, i);

            // Each move assigns one shift (from 3 unassigned to 2 unassigned)
            assert_eq!(
                director.get_score(),
                SimpleScore::of(-2),
                "Move {} should result in score -2 (2 unassigned)",
                i
            );

            // Undo
            director.undo_changes();

            // Score should be back to initial
            assert_eq!(
                director.get_score(),
                initial_score,
                "After undo of move {}, score should be restored to initial",
                i
            );
        }

        // Final score should still be initial
        assert_eq!(director.get_score(), initial_score);

        // Solution should be unchanged
        for shift in director.working_solution().shifts.iter() {
            assert_eq!(shift.employee, None);
        }
    }

    /// Test 5: Undo with uninitialized director (edge case)
    #[test]
    fn test_undo_uninitialized() {
        let solution = TestSolution {
            shifts: vec![Shift {
                id: 0,
                employee: None,
            }],
            score: None,
        };

        let constraint = create_unassigned_constraint();
        let mut director = ScoreDirector::new(solution, (constraint,));

        // Don't initialize, just register some undo closures
        director.register_undo(Box::new(|_: &mut TestSolution| {}));

        // Undo should just clear stacks without panic
        director.undo_changes();

        assert_eq!(director.undo_stack_len(), 0);
    }

    /// Test 6: Clear undo stack
    #[test]
    fn test_clear_undo_stack() {
        let solution = TestSolution {
            shifts: vec![Shift {
                id: 0,
                employee: None,
            }],
            score: None,
        };

        let constraint = create_unassigned_constraint();
        let mut director = ScoreDirector::new(solution, (constraint,));

        director.calculate_score();

        // Simulate move
        director.save_score_snapshot();
        director.before_variable_changed(0, 0);
        director.working_solution_mut().shifts[0].employee = Some(1);
        director.register_undo(Box::new(|s: &mut TestSolution| {
            s.shifts[0].employee = None;
        }));
        director.after_variable_changed(0, 0);

        // Clear undo stack (move is accepted)
        director.clear_undo_stack();

        assert_eq!(director.undo_stack_len(), 0);
        // Score should remain at post-move value
        assert_eq!(director.get_score(), SimpleScore::of(0));
    }
}
