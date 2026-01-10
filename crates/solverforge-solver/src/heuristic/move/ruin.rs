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
use std::marker::PhantomData;

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
/// * `D` - The score director type
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
/// let m = RuinMove::<Schedule, _, i32>::new(
///     &[0, 2, 4],
///     get_task, set_task,
///     "assigned_to", 0,
/// );
/// ```
#[derive(Clone)]
pub struct RuinMove<S, D, V> {
    /// Indices of entities to unassign
    entity_indices: SmallVec<[usize; 8]>,
    /// Get current value for an entity
    getter: fn(&S, usize) -> Option<V>,
    /// Set value for an entity
    setter: fn(&mut S, usize, Option<V>),
    variable_name: &'static str,
    descriptor_index: usize,
    _phantom: PhantomData<(D, V)>,
}

impl<S, D, V: Debug> Debug for RuinMove<S, D, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RuinMove")
            .field("entities", &self.entity_indices.as_slice())
            .field("variable_name", &self.variable_name)
            .finish()
    }
}

impl<S, D, V> RuinMove<S, D, V> {
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
            _phantom: PhantomData,
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

impl<S, D, V> Move<S, D> for RuinMove<S, D, V>
where
    S: PlanningSolution,
    D: ScoreDirector<S>,
    V: Clone + Send + Sync + Debug + 'static,
{
    fn is_doable(&self, score_director: &D) -> bool {
        // At least one entity must be currently assigned
        let solution = score_director.working_solution();
        self.entity_indices
            .iter()
            .any(|&idx| (self.getter)(solution, idx).is_some())
    }

    fn do_move(&self, score_director: &mut D) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use solverforge_core::domain::{EntityDescriptor, SolutionDescriptor, TypedEntityExtractor};
    use solverforge_core::score::SimpleScore;
    use solverforge_scoring::{RecordingScoreDirector, SimpleScoreDirector};
    use std::any::TypeId;

    #[derive(Clone, Debug)]
    struct Task {
        assigned_to: Option<i32>,
    }

    #[derive(Clone, Debug)]
    struct Schedule {
        tasks: Vec<Task>,
        score: Option<SimpleScore>,
    }

    impl PlanningSolution for Schedule {
        type Score = SimpleScore;
        fn score(&self) -> Option<Self::Score> {
            self.score
        }
        fn set_score(&mut self, score: Option<Self::Score>) {
            self.score = score;
        }
    }

    fn get_tasks(s: &Schedule) -> &Vec<Task> {
        &s.tasks
    }
    fn get_tasks_mut(s: &mut Schedule) -> &mut Vec<Task> {
        &mut s.tasks
    }

    fn get_assigned(s: &Schedule, idx: usize) -> Option<i32> {
        s.tasks.get(idx).and_then(|t| t.assigned_to)
    }
    fn set_assigned(s: &mut Schedule, idx: usize, v: Option<i32>) {
        if let Some(t) = s.tasks.get_mut(idx) {
            t.assigned_to = v;
        }
    }

    fn create_director(
        assignments: &[Option<i32>],
    ) -> SimpleScoreDirector<Schedule, impl Fn(&Schedule) -> SimpleScore> {
        let tasks: Vec<Task> = assignments
            .iter()
            .map(|&a| Task { assigned_to: a })
            .collect();
        let solution = Schedule { tasks, score: None };
        let extractor = Box::new(TypedEntityExtractor::new(
            "Task",
            "tasks",
            get_tasks,
            get_tasks_mut,
        ));
        let entity_desc =
            EntityDescriptor::new("Task", TypeId::of::<Task>(), "tasks").with_extractor(extractor);
        let descriptor =
            SolutionDescriptor::new("Schedule", TypeId::of::<Schedule>()).with_entity(entity_desc);
        SimpleScoreDirector::with_calculator(solution, descriptor, |_| SimpleScore::of(0))
    }

    #[test]
    fn ruin_single_entity() {
        let mut director = create_director(&[Some(1), Some(2), Some(3)]);

        let m = RuinMove::<Schedule, _, i32>::new(&[1], get_assigned, set_assigned, "assigned_to", 0);

        assert!(m.is_doable(&director));

        {
            let mut recording = RecordingScoreDirector::new(&mut director);
            m.do_move(&mut recording);

            assert_eq!(get_assigned(recording.working_solution(), 0), Some(1));
            assert_eq!(get_assigned(recording.working_solution(), 1), None);
            assert_eq!(get_assigned(recording.working_solution(), 2), Some(3));

            recording.undo_changes();
        }

        // Restored
        assert_eq!(get_assigned(director.working_solution(), 1), Some(2));
    }

    #[test]
    fn ruin_multiple_entities() {
        let mut director = create_director(&[Some(1), Some(2), Some(3), Some(4)]);

        let m = RuinMove::<Schedule, _, i32>::new(
            &[0, 2, 3],
            get_assigned,
            set_assigned,
            "assigned_to",
            0,
        );

        assert!(m.is_doable(&director));
        assert_eq!(m.ruin_count(), 3);

        {
            let mut recording = RecordingScoreDirector::new(&mut director);
            m.do_move(&mut recording);

            assert_eq!(get_assigned(recording.working_solution(), 0), None);
            assert_eq!(get_assigned(recording.working_solution(), 1), Some(2));
            assert_eq!(get_assigned(recording.working_solution(), 2), None);
            assert_eq!(get_assigned(recording.working_solution(), 3), None);

            recording.undo_changes();
        }

        // All restored
        assert_eq!(get_assigned(director.working_solution(), 0), Some(1));
        assert_eq!(get_assigned(director.working_solution(), 2), Some(3));
        assert_eq!(get_assigned(director.working_solution(), 3), Some(4));
    }

    #[test]
    fn ruin_already_unassigned_is_doable() {
        // One assigned, one unassigned
        let director = create_director(&[Some(1), None]);

        // Ruin both - still doable because entity 0 is assigned
        let m =
            RuinMove::<Schedule, _, i32>::new(&[0, 1], get_assigned, set_assigned, "assigned_to", 0);

        assert!(m.is_doable(&director));
    }

    #[test]
    fn ruin_all_unassigned_not_doable() {
        let director = create_director(&[None, None]);

        let m =
            RuinMove::<Schedule, _, i32>::new(&[0, 1], get_assigned, set_assigned, "assigned_to", 0);

        assert!(!m.is_doable(&director));
    }
}
