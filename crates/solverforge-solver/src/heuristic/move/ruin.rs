//! RuinMove - unassigns a subset of entities for Large Neighborhood Search.
//!
//! This move "ruins" (unassigns) selected entities, allowing a construction
//! heuristic to reassign them. This is the fundamental building block for
//! Large Neighborhood Search (LNS) algorithms.
//!
//! # Zero-Erasure Design
//!
//! Uses typed function pointers for variable access. No `dyn Any`, no downcasting.

use std::fmt::Debug;

use smallvec::SmallVec;
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use super::Move;

/// A move that unassigns multiple entities for Large Neighborhood Search.
///
/// This move sets the planning variable to `None` for a set of entities,
/// creating "gaps" that a construction heuristic can fill. Combined with
/// construction, this enables exploring distant regions of the search space.
///
/// # Type Parameters
/// * `S` - The planning solution type
/// * `V` - The variable value type
///
/// # Example
///
/// ```
/// use solverforge_solver::heuristic::r#move::RuinMove;
/// use solverforge_core::domain::PlanningSolution;
/// use solverforge_core::score::SimpleScore;
///
/// #[derive(Clone, Debug)]
/// struct Task { assigned_to: Option<i32>, score: Option<SimpleScore> }
/// #[derive(Clone, Debug)]
/// struct Schedule { tasks: Vec<Task>, score: Option<SimpleScore> }
///
/// impl PlanningSolution for Schedule {
///     type Score = SimpleScore;
///     fn score(&self) -> Option<Self::Score> { self.score }
///     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
/// }
///
/// fn get_task(s: &Schedule, idx: usize) -> Option<i32> {
///     s.tasks.get(idx).and_then(|t| t.assigned_to)
/// }
/// fn set_task(s: &mut Schedule, idx: usize, v: Option<i32>) {
///     if let Some(t) = s.tasks.get_mut(idx) { t.assigned_to = v; }
/// }
///
/// // Ruin entities 0, 2, and 4
/// let m = RuinMove::<Schedule, i32>::new(
///     &[0, 2, 4],
///     get_task, set_task,
///     "assigned_to", 0,
/// );
/// ```
pub struct RuinMove<S, V> {
    /// Indices of entities to unassign
    entity_indices: SmallVec<[usize; 8]>,
    /// Get current value for an entity
    getter: fn(&S, usize) -> Option<V>,
    /// Set value for an entity
    setter: fn(&mut S, usize, Option<V>),
    variable_name: &'static str,
    descriptor_index: usize,
}

impl<S, V> Clone for RuinMove<S, V> {
    fn clone(&self) -> Self {
        Self {
            entity_indices: self.entity_indices.clone(),
            getter: self.getter,
            setter: self.setter,
            variable_name: self.variable_name,
            descriptor_index: self.descriptor_index,
        }
    }
}

impl<S, V: Debug> Debug for RuinMove<S, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RuinMove")
            .field("entities", &self.entity_indices.as_slice())
            .field("variable_name", &self.variable_name)
            .finish()
    }
}

impl<S, V> RuinMove<S, V> {
    /// Creates a new ruin move with typed function pointers.
    ///
    /// # Arguments
    /// * `entity_indices` - Indices of entities to unassign
    /// * `getter` - Function to get current value
    /// * `setter` - Function to set value
    /// * `variable_name` - Name of the planning variable
    /// * `descriptor_index` - Entity descriptor index
    pub fn new(
        entity_indices: &[usize],
        getter: fn(&S, usize) -> Option<V>,
        setter: fn(&mut S, usize, Option<V>),
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        Self {
            entity_indices: SmallVec::from_slice(entity_indices),
            getter,
            setter,
            variable_name,
            descriptor_index,
        }
    }

    /// Returns the entity indices being ruined.
    pub fn entity_indices_slice(&self) -> &[usize] {
        &self.entity_indices
    }

    /// Returns the number of entities being ruined.
    pub fn ruin_count(&self) -> usize {
        self.entity_indices.len()
    }
}

impl<S, V> Move<S> for RuinMove<S, V>
where
    S: PlanningSolution,
    V: Clone + Send + Sync + Debug + 'static,
{
    fn is_doable<D: ScoreDirector<S>>(&self, score_director: &D) -> bool {
        // At least one entity must be currently assigned
        let solution = score_director.working_solution();
        self.entity_indices
            .iter()
            .any(|&idx| (self.getter)(solution, idx).is_some())
    }

    fn do_move<D: ScoreDirector<S>>(&self, score_director: &mut D) {
        let getter = self.getter;
        let setter = self.setter;
        let descriptor = self.descriptor_index;
        let variable_name = self.variable_name;

        // Collect old values for undo
        let old_values: SmallVec<[(usize, Option<V>); 8]> = self
            .entity_indices
            .iter()
            .map(|&idx| {
                let old = getter(score_director.working_solution(), idx);
                (idx, old)
            })
            .collect();

        // Unassign all entities
        for &idx in &self.entity_indices {
            score_director.before_variable_changed(descriptor, idx, variable_name);
            setter(score_director.working_solution_mut(), idx, None);
            score_director.after_variable_changed(descriptor, idx, variable_name);
        }

        // Register undo to restore old values
        score_director.register_undo(Box::new(move |s: &mut S| {
            for (idx, old_value) in old_values {
                setter(s, idx, old_value);
            }
        }));
    }

    fn descriptor_index(&self) -> usize {
        self.descriptor_index
    }

    fn entity_indices(&self) -> &[usize] {
        &self.entity_indices
    }

    fn variable_name(&self) -> &str {
        self.variable_name
    }
}
