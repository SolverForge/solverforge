// Core domain traits

use std::any::Any;
use std::hash::Hash;

use crate::constraint_weights::ConstraintWeights;
use crate::score::Score;

/// Marker trait for planning solutions.
///
/// A planning solution represents both the problem definition and the
/// (potentially partial) solution. It contains:
/// - Problem facts: Immutable input data
/// - Planning entities: Things to be optimized
/// - Score: The quality of the current solution
///
/// # Example
///
/// ```
/// use solverforge_core::{PlanningSolution, score::SoftScore};
///
/// #[derive(Clone)]
/// struct NQueens {
///     n: usize,
///     rows: Vec<Option<usize>>,
///     score: Option<SoftScore>,
/// }
///
/// impl PlanningSolution for NQueens {
///     type Score = SoftScore;
///
///     fn score(&self) -> Option<Self::Score> {
///         self.score
///     }
///
///     fn set_score(&mut self, score: Option<Self::Score>) {
///         self.score = score;
///     }
/// }
/// ```
///
/// For complex solutions, use the `#[derive(PlanningSolution)]` macro from `solverforge-macros`.
///
/// # Thread Safety
///
/// Planning solutions must be `Send + Sync` to support multi-threaded solving.
pub trait PlanningSolution: Clone + Send + Sync + 'static {
    // The score type used to evaluate this solution.
    type Score: Score;

    /* Returns the current score of this solution, if calculated.

    Returns `None` if the solution has not been scored yet.
    */
    fn score(&self) -> Option<Self::Score>;

    fn set_score(&mut self, score: Option<Self::Score>);

    /* Updates shadow variables for the descriptor/entity pair.

    Default implementation is a no-op. Solutions with derived state can override
    this to keep cached values synchronized during solving.
    */
    fn update_entity_shadows(&mut self, _descriptor_index: usize, _entity_index: usize) {}

    /* Updates all shadow variables for the full solution.

    Default implementation is a no-op. Solutions with derived state can override
    this to refresh all cached values before initialization.
    */
    fn update_all_shadows(&mut self) {}

    /* Whether this solution type carries runtime constraint weights.

    Weightedness is a property of the type, not of a value: score directors
    branch on this constant, so a solution type that leaves it `false` compiles
    to the unweighted scoring path with no weight binding and no per-call
    check. `#[planning_solution]` sets it to `true` exactly when a field is
    marked `#[constraint_weights]`. A manual implementation that overrides
    `constraint_weights` must set it to `true` as well, or the weights are
    never applied.
    */
    const HAS_CONSTRAINT_WEIGHTS: bool = false;

    /* Returns the runtime constraint weight multipliers carried by this solution.

    Default implementation returns `None`: every constraint keeps its coded
    weight. `#[planning_solution]` generates this accessor for the field marked
    `#[constraint_weights]`. When `HAS_CONSTRAINT_WEIGHTS` is `true`, score
    directors bind these multipliers on construction and on every
    (re)initialization, so a running solve must reset its director after
    changing them.
    */
    fn constraint_weights(&self) -> Option<&ConstraintWeights> {
        None
    }

    /* Returns the elements whose list shadows
    `update_entity_shadows(descriptor_index, entity_index)` writes, with the
    change source their stream rows are reported under.

    A score director retracts these rows before the shadow write and inserts
    them after it, so streams that read element shadows stay incremental.
    Default implementation reports no list shadows.
    */
    fn list_shadow_elements(
        &self,
        _descriptor_index: usize,
        _entity_index: usize,
    ) -> Option<(usize, &[usize])> {
        None
    }

    /* Clears the list shadows of an element that no list holds any more.

    Default implementation is a no-op.
    */
    fn clear_list_shadows(&mut self, _element: usize) {}

    /* Returns the change source under which rows of `collection` are reported
    when they carry list shadows, or `None` when the collection holds none.
    */
    fn list_shadow_source(_collection: &str) -> Option<usize> {
        None
    }

    /* Returns true if this solution is fully initialized.

    A solution is initialized when all planning variables have been assigned.
    */
    fn is_initialized(&self) -> bool {
        // Default implementation - can be overridden by derived code
        true
    }
}

/// Marker trait for planning entities.
///
/// A planning entity is something that gets planned/optimized.
/// It contains one or more planning variables that the solver will change.
///
/// # Example
///
/// ```
/// use std::any::Any;
/// use solverforge_core::PlanningEntity;
///
/// #[derive(Clone)]
/// struct Queen {
///     column: i32,
///     row: Option<i32>,
/// }
///
/// impl PlanningEntity for Queen {
///     fn as_any(&self) -> &dyn Any { self }
///     fn as_any_mut(&mut self) -> &mut dyn Any { self }
/// }
/// ```
///
/// For complex entities, use the `#[derive(PlanningEntity)]` macro from `solverforge-macros`.
///
/// # Pinning
///
/// Entities can be "pinned" to preserve their input genuine planning variables
/// during solving. Override `is_pinned()` to return true for pinned entities;
/// manually assembled entity descriptors must also supply a pin predicate.
pub trait PlanningEntity: Clone + Send + Sync + Any + 'static {
    /* Returns true if this entity is pinned (should not be changed).

    Default implementation returns false (entity can be changed).
    */
    fn is_pinned(&self) -> bool {
        false
    }

    // Cast to Any for dynamic typing support.
    fn as_any(&self) -> &dyn Any;

    // Cast to mutable Any for dynamic typing support.
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

/// Marker trait for problem facts.
///
/// Problem facts are immutable input data that define the problem.
/// They are used by constraints but not changed during solving.
///
/// # Example
///
/// ```
/// use std::any::Any;
/// use solverforge_core::ProblemFact;
///
/// #[derive(Clone)]
/// struct Room {
///     id: i64,
///     capacity: usize,
/// }
///
/// impl ProblemFact for Room {
///     fn as_any(&self) -> &dyn Any { self }
/// }
/// ```
pub trait ProblemFact: Clone + Send + Sync + Any + 'static {
    // Cast to Any for dynamic typing support.
    fn as_any(&self) -> &dyn Any;
}

/// Trait for unique identification of entities and facts.
///
/// Used for looking up working copies during solving and rebasing moves.
///
/// # Example
///
/// ```
/// use solverforge_core::PlanningId;
///
/// #[derive(Clone)]
/// struct Task {
///     id: i64,
///     name: String,
/// }
///
/// impl PlanningId for Task {
///     type Id = i64;
///     fn planning_id(&self) -> i64 { self.id }
/// }
/// ```
///
/// The ID type must be `Eq + Hash + Clone`.
pub trait PlanningId {
    // The type of the unique identifier.
    type Id: Eq + Hash + Clone + Send + Sync + 'static;

    /* Returns the unique identifier for this object.

    This must never return a value that changes during solving.
    */
    fn planning_id(&self) -> Self::Id;
}

/// Trait for solutions with list-based planning variables.
///
/// Used for problems like VRP where entities (vehicles) have ordered lists
/// of elements (visits) that can be inserted, removed, or reordered.
///
/// # Examples
///
/// ```
/// use solverforge_core::domain::ListVariableSolution;
/// use solverforge_core::PlanningSolution;
/// use solverforge_core::score::SoftScore;
///
/// #[derive(Clone)]
/// struct Vehicle {
///     visits: Vec<usize>,
/// }
///
/// #[derive(Clone)]
/// struct VrpSolution {
///     vehicles: Vec<Vehicle>,
///     visit_count: usize,
///     score: Option<SoftScore>,
/// }
///
/// impl PlanningSolution for VrpSolution {
///     type Score = SoftScore;
///     fn score(&self) -> Option<Self::Score> { self.score }
///     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
/// }
///
/// impl ListVariableSolution for VrpSolution {
///     type Element = usize;
///
///     fn entity_count(&self) -> usize {
///         self.vehicles.len()
///     }
///
///     fn list_len(&self, entity_idx: usize) -> usize {
///         self.vehicles[entity_idx].visits.len()
///     }
///
///     fn list_get(&self, entity_idx: usize, position: usize) -> Self::Element {
///         self.vehicles[entity_idx].visits[position]
///     }
///
///     fn list_push(&mut self, entity_idx: usize, elem: Self::Element) {
///         self.vehicles[entity_idx].visits.push(elem);
///     }
///
///     fn list_insert(&mut self, entity_idx: usize, position: usize, elem: Self::Element) {
///         self.vehicles[entity_idx].visits.insert(position, elem);
///     }
///
///     fn list_remove(&mut self, entity_idx: usize, position: usize) -> Self::Element {
///         self.vehicles[entity_idx].visits.remove(position)
///     }
///
///     fn list_reverse(&mut self, entity_idx: usize, start: usize, end: usize) {
///         self.vehicles[entity_idx].visits[start..end].reverse();
///     }
///
///     fn unassigned_elements(&self) -> Vec<Self::Element> {
///         use std::collections::HashSet;
///         let assigned: HashSet<usize> = self.vehicles
///             .iter()
///             .flat_map(|v| v.visits.iter().copied())
///             .collect();
///         (0..self.visit_count)
///             .filter(|i| !assigned.contains(i))
///             .collect()
///     }
/// }
/// ```
pub trait ListVariableSolution: PlanningSolution {
    // The type of elements in the list (typically an index or ID).
    type Element: Copy + Send + Sync;

    fn entity_count(&self) -> usize;

    fn list_len(&self, entity_idx: usize) -> usize;

    fn list_get(&self, entity_idx: usize, position: usize) -> Self::Element;

    // Appends an element to the end of the entity's list.
    fn list_push(&mut self, entity_idx: usize, elem: Self::Element);

    // Inserts an element at the given position in the entity's list.
    fn list_insert(&mut self, entity_idx: usize, position: usize, elem: Self::Element);

    // Removes and returns the element at the given position.
    fn list_remove(&mut self, entity_idx: usize, position: usize) -> Self::Element;

    // Reverses the elements in the range [start, end) for the entity's list.
    fn list_reverse(&mut self, entity_idx: usize, start: usize, end: usize);

    // Returns all elements not currently assigned to any entity.
    fn unassigned_elements(&self) -> Vec<Self::Element>;
}
