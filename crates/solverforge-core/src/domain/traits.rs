//! Core domain traits

use std::any::Any;
use std::hash::Hash;

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
/// use solverforge_core::{PlanningSolution, score::SimpleScore};
///
/// #[derive(Clone)]
/// struct NQueens {
///     n: usize,
///     rows: Vec<Option<usize>>,
///     score: Option<SimpleScore>,
/// }
///
/// impl PlanningSolution for NQueens {
///     type Score = SimpleScore;
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
    /// The score type used to evaluate this solution.
    type Score: Score;

    /// Returns the current score of this solution, if calculated.
    ///
    /// Returns `None` if the solution has not been scored yet.
    fn score(&self) -> Option<Self::Score>;

    /// Sets the score of this solution.
    fn set_score(&mut self, score: Option<Self::Score>);

    /// Returns true if this solution is fully initialized.
    ///
    /// A solution is initialized when all planning variables have been assigned.
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
/// Entities can be "pinned" to prevent the solver from changing them.
/// Override `is_pinned()` to return true for pinned entities.
pub trait PlanningEntity: Clone + Send + Sync + Any + 'static {
    /// Returns true if this entity is pinned (should not be changed).
    ///
    /// Default implementation returns false (entity can be changed).
    fn is_pinned(&self) -> bool {
        false
    }

    /// Cast to Any for dynamic typing support.
    fn as_any(&self) -> &dyn Any;

    /// Cast to mutable Any for dynamic typing support.
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
    /// Cast to Any for dynamic typing support.
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
    /// The type of the unique identifier.
    type Id: Eq + Hash + Clone + Send + Sync + 'static;

    /// Returns the unique identifier for this object.
    ///
    /// This must never return a value that changes during solving.
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
/// use solverforge_core::score::SimpleScore;
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
///     score: Option<SimpleScore>,
/// }
///
/// impl PlanningSolution for VrpSolution {
///     type Score = SimpleScore;
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
    /// The type of elements in the list (typically an index or ID).
    type Element: Copy + Send + Sync;

    /// Returns the number of entities (list owners).
    fn entity_count(&self) -> usize;

    /// Returns the length of the list for the given entity.
    fn list_len(&self, entity_idx: usize) -> usize;

    /// Returns the element at the given position in the entity's list.
    fn list_get(&self, entity_idx: usize, position: usize) -> Self::Element;

    /// Appends an element to the end of the entity's list.
    fn list_push(&mut self, entity_idx: usize, elem: Self::Element);

    /// Inserts an element at the given position in the entity's list.
    fn list_insert(&mut self, entity_idx: usize, position: usize, elem: Self::Element);

    /// Removes and returns the element at the given position.
    fn list_remove(&mut self, entity_idx: usize, position: usize) -> Self::Element;

    /// Reverses the elements in the range [start, end) for the entity's list.
    fn list_reverse(&mut self, entity_idx: usize, start: usize, end: usize);

    /// Returns all elements not currently assigned to any entity.
    fn unassigned_elements(&self) -> Vec<Self::Element>;
}
