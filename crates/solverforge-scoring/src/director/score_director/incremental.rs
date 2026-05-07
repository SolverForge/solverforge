use std::marker::PhantomData;

use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};
use solverforge_core::score::Score;

use crate::api::constraint_set::{ConstraintMetadata, ConstraintSet};

/* A zero-erasure score director for incremental scoring.

Unlike `IncrementalDirector` which uses BAVET session with trait objects,
this director uses a monomorphized `ConstraintSet` where all constraint types
are known at compile time, enabling complete monomorphization.

# Type Parameters

- `S`: The solution type (must implement `PlanningSolution`)
- `C`: The constraint set type (tuple of monomorphized constraints)

# Example

```
use solverforge_scoring::director::score_director::ScoreDirector;
use solverforge_scoring::api::constraint_set::{ConstraintSet, IncrementalConstraint};
use solverforge_scoring::constraint::incremental::IncrementalUniConstraint;
use solverforge_core::domain::PlanningSolution;
use solverforge_core::{ConstraintRef, ImpactType};
use solverforge_core::score::SoftScore;

#[derive(Clone)]
struct Solution {
values: Vec<Option<i32>>,
score: Option<SoftScore>,
}

impl PlanningSolution for Solution {
type Score = SoftScore;
fn score(&self) -> Option<Self::Score> { self.score }
fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
}

// Create zero-erasure constraint (all closures as generics)
let c1 = IncrementalUniConstraint::new(
ConstraintRef::new("", "Unassigned"),
ImpactType::Penalty,
|s: &Solution| s.values.as_slice(),
|_s: &Solution, v: &Option<i32>| v.is_none(),
|_: &Option<i32>| SoftScore::of(1),
false,
);

// Create a zero-erasure director with a tuple-based constraint set
let solution = Solution { values: vec![Some(1), None, Some(2)], score: None };
let mut director = ScoreDirector::new(solution, (c1,));

// First calculation evaluates all constraints
let score = director.calculate_score();
assert_eq!(score, SoftScore::of(-1)); // One unassigned

// Subsequent calculations are O(1) - return cached score
let score2 = director.calculate_score();
assert_eq!(score, score2);
```
*/
pub struct ScoreDirector<S, C>
where
    S: PlanningSolution,
    C: ConstraintSet<S, S::Score>,
{
    pub(super) working_solution: S,
    constraints: C,
    cached_score: S::Score,
    initialized: bool,
    pub(super) solution_descriptor: SolutionDescriptor,
    /* Entity counter function.

    Returns the number of entities for the given descriptor index.
    This concrete function pointer preserves full type information
    throughout the solver pipeline.
    */
    pub(super) entity_counter: fn(&S, usize) -> usize,
    _phantom: PhantomData<fn() -> S>,
}

impl<S, C> ScoreDirector<S, C>
where
    S: PlanningSolution,
    S::Score: Score,
    C: ConstraintSet<S, S::Score>,
{
    /* Creates a new zero-erasure score director with an empty descriptor.

    Use this for manual solver loops that don't need the `Director` trait.
    For full solver infrastructure integration, use `with_descriptor()`.

    The constraints should be a tuple of monomorphized constraints (e.g., `(C1, C2, C3)`).
    */
    pub fn new(solution: S, constraints: C) -> Self {
        use std::any::TypeId;
        Self::with_descriptor(
            solution,
            constraints,
            SolutionDescriptor::new("", TypeId::of::<()>()),
            |_, _| 0,
        )
    }

    /* Creates a new zero-erasure score director with a solution descriptor.

    This constructor enables the `Director` trait implementation for
    integration with the full solver infrastructure (phases, move selectors, etc.).

    # Arguments

    * `solution` - The initial solution
    * `constraints` - Tuple of monomorphized constraints (e.g., `(C1, C2, C3)`)
    * `solution_descriptor` - Metadata for solver infrastructure
    * `entity_counter` - Function returning entity count for descriptor index
    */
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

    /* =========================================================================
    Private implementation methods (shared between inherent and trait impl)
    =========================================================================
    */

    pub(crate) fn calculate_score_impl(&mut self) -> S::Score {
        if !self.initialized {
            self.working_solution.update_all_shadows();
            self.cached_score = self.constraints.initialize_all(&self.working_solution);
            self.initialized = true;
        }
        self.working_solution.set_score(Some(self.cached_score));
        self.cached_score
    }

    pub(crate) fn before_variable_changed_impl(
        &mut self,
        descriptor_index: usize,
        entity_index: usize,
    ) {
        if !self.initialized {
            return;
        }
        let delta =
            self.constraints
                .on_retract_all(&self.working_solution, entity_index, descriptor_index);
        self.cached_score = self.cached_score + delta;
    }

    pub(crate) fn after_variable_changed_impl(
        &mut self,
        descriptor_index: usize,
        entity_index: usize,
    ) {
        if !self.initialized {
            return;
        }
        self.working_solution
            .update_entity_shadows(descriptor_index, entity_index);
        let delta =
            self.constraints
                .on_insert_all(&self.working_solution, entity_index, descriptor_index);
        self.cached_score = self.cached_score + delta;
    }

    pub(crate) fn reset_impl(&mut self) {
        self.constraints.reset_all();
        self.initialized = false;
        self.cached_score = S::Score::zero();
    }

    pub(crate) fn snapshot_score_state_impl(
        &self,
    ) -> crate::director::DirectorScoreState<S::Score> {
        crate::director::DirectorScoreState {
            solution_score: self.working_solution.score(),
            committed_score: self.initialized.then_some(self.cached_score),
            initialized: self.initialized,
        }
    }

    pub(crate) fn restore_score_state_impl(
        &mut self,
        state: crate::director::DirectorScoreState<S::Score>,
    ) {
        self.working_solution.set_score(state.solution_score);
        if state.initialized {
            self.cached_score = state
                .committed_score
                .expect("initialized score state must include committed_score");
            self.initialized = true;
        } else {
            self.constraints.reset_all();
            self.cached_score = S::Score::zero();
            self.initialized = false;
        }
    }

    pub(crate) fn clone_working_solution_impl(&self) -> S {
        let mut cloned = self.working_solution.clone();
        cloned.set_score(Some(self.cached_score));
        cloned
    }

    // Returns a reference to the working solution.
    pub fn working_solution(&self) -> &S {
        &self.working_solution
    }

    /* Returns a mutable reference to the working solution.

    Note: After modifying the solution directly, you must call
    `reset()` to recalculate the score from scratch.
    */
    pub fn working_solution_mut(&mut self) -> &mut S {
        &mut self.working_solution
    }

    // Consumes the director and returns the working solution with final score set.
    pub fn into_working_solution(mut self) -> S {
        self.working_solution.set_score(Some(self.cached_score));
        self.working_solution
    }

    /* Calculates and returns the current score.

    On first call, updates all configured shadows and initializes all constraints
    (O(n) for uni, O(n²) for bi). Subsequent calls return the cached score (O(1)).

    Also sets the score on the working solution to keep it in sync.
    */
    pub fn calculate_score(&mut self) -> S::Score {
        self.calculate_score_impl()
    }

    /* Called before changing an entity's variable.

    This retracts the entity from all constraints, computing the delta
    that will be applied when the change completes.

    # Arguments

    * `descriptor_index` - Index of the entity descriptor (entity class)
    * `entity_index` - Index of the entity being changed
    */
    #[inline]
    pub fn before_variable_changed(&mut self, descriptor_index: usize, entity_index: usize) {
        self.before_variable_changed_impl(descriptor_index, entity_index);
    }

    /* Called after changing an entity's variable.

    This updates any configured shadows for the descriptor/entity pair, then
    inserts the entity (with new state) into all constraints, computing the
    delta and updating the cached score.

    # Arguments

    * `descriptor_index` - Index of the entity descriptor (entity class)
    * `entity_index` - Index of the entity that was changed
    */
    #[inline]
    pub fn after_variable_changed(&mut self, descriptor_index: usize, entity_index: usize) {
        self.after_variable_changed_impl(descriptor_index, entity_index);
    }

    /* Convenience method for a complete variable change cycle. */
    #[inline]
    pub fn do_change<F>(
        &mut self,
        descriptor_index: usize,
        entity_index: usize,
        change_fn: F,
    ) -> S::Score
    where
        F: FnOnce(&mut S),
    {
        self.before_variable_changed(descriptor_index, entity_index);
        change_fn(&mut self.working_solution);
        self.after_variable_changed(descriptor_index, entity_index);
        self.cached_score
    }

    /* Returns the cached score without recalculation. */
    #[inline]
    pub fn get_score(&self) -> S::Score {
        self.cached_score
    }

    /* Resets the director state. */
    pub fn reset(&mut self) {
        self.reset_impl();
    }

    /* Clones the working solution. */
    pub fn clone_working_solution(&self) -> S {
        self.clone_working_solution_impl()
    }

    // Returns a reference to the constraint set.
    pub fn constraints(&self) -> &C {
        &self.constraints
    }

    // Returns a mutable reference to the constraint set.
    pub fn constraints_mut(&mut self) -> &mut C {
        &mut self.constraints
    }

    // Returns immutable scoring-constraint metadata.
    pub fn constraint_metadata(&self) -> Vec<ConstraintMetadata<'_>> {
        self.constraints.constraint_metadata()
    }

    // Returns the number of constraints.
    pub fn constraint_count(&self) -> usize {
        self.constraints.constraint_count()
    }

    // Returns whether the director is initialized.
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /* Returns constraint match totals for score analysis. */
    pub fn constraint_match_totals(&self) -> Vec<(String, S::Score, S::Score, usize)> {
        self.constraints
            .evaluate_each(&self.working_solution)
            .into_iter()
            .map(|r| {
                let weight = if r.match_count > 0 {
                    r.score
                } else {
                    S::Score::zero()
                };
                (r.name.to_string(), weight, r.score, r.match_count)
            })
            .collect()
    }

    /* Consumes the director and returns the working solution.

    Use this to extract the final solution after solving.

    # Examples

    ```
    use solverforge_scoring::director::score_director::ScoreDirector;
    use solverforge_core::domain::PlanningSolution;
    use solverforge_core::score::SoftScore;

    #[derive(Clone)]
    struct Solution {
    values: Vec<i32>,
    score: Option<SoftScore>,
    }

    impl PlanningSolution for Solution {
    type Score = SoftScore;
    fn score(&self) -> Option<Self::Score> { self.score }
    fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
    }

    let solution = Solution { values: vec![1, 2, 3], score: None };
    let director = ScoreDirector::new(solution, ());
    let result = director.take_solution();
    assert_eq!(result.values, vec![1, 2, 3]);
    ```
    */
    pub fn take_solution(self) -> S {
        self.working_solution
    }
}

impl<S> ScoreDirector<S, ()>
where
    S: PlanningSolution,
    S::Score: Score,
{
    /* Creates a non-incremental director for use in tests and simple scenarios.

    Uses an empty constraint set — `calculate_score()` always returns zero.
    For tests that set score directly on the solution, this is sufficient.
    */
    pub fn simple(
        solution: S,
        descriptor: SolutionDescriptor,
        entity_counter: fn(&S, usize) -> usize,
    ) -> Self {
        Self::with_descriptor(solution, (), descriptor, entity_counter)
    }

    /* Creates a non-incremental director with empty descriptor and zero entity counter. */
    pub fn simple_zero(solution: S) -> Self {
        use std::any::TypeId;
        Self::with_descriptor(
            solution,
            (),
            SolutionDescriptor::new("", TypeId::of::<()>()),
            |_, _| 0,
        )
    }
}
