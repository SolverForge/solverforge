//! ChangeMove - assigns a value to a planning variable.
//!
//! This is the most fundamental move type. It takes a value and assigns
//! it to a planning variable on an entity.
//!
//! # Zero-Erasure Design
//!
//! This move stores typed function pointers that operate directly on
//! the solution. No `Arc<dyn>`, no `Box<dyn Any>`, no `downcast_ref`.

use std::fmt::Debug;

use solverforge_scoring::ScoreDirector;
use solverforge_core::domain::PlanningSolution;

use super::Move;

/// A move that assigns a value to an entity's variable.
///
/// Stores typed function pointers for zero-erasure execution.
/// No trait objects, no boxing - all operations are fully typed at compile time.
///
/// # Type Parameters
/// * `S` - The planning solution type
/// * `V` - The variable value type
#[derive(Clone, Copy)]
pub struct ChangeMove<S, V> {
    entity_index: usize,
    to_value: Option<V>,
    getter: fn(&S, usize) -> Option<V>,
    setter: fn(&mut S, usize, Option<V>),
    variable_name: &'static str,
    descriptor_index: usize,
}

impl<S, V: Debug> Debug for ChangeMove<S, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChangeMove")
            .field("entity_index", &self.entity_index)
            .field("descriptor_index", &self.descriptor_index)
            .field("variable_name", &self.variable_name)
            .field("to_value", &self.to_value)
            .finish()
    }
}

impl<S, V> ChangeMove<S, V> {
    /// Creates a new change move with typed function pointers.
    ///
    /// # Arguments
    /// * `entity_index` - Index of the entity in its collection
    /// * `to_value` - The value to assign (None to unassign)
    /// * `getter` - Function pointer to get current value from solution
    /// * `setter` - Function pointer to set value on solution
    /// * `variable_name` - Name of the variable (for debugging)
    /// * `descriptor_index` - Index of the entity descriptor
    pub fn new(
        entity_index: usize,
        to_value: Option<V>,
        getter: fn(&S, usize) -> Option<V>,
        setter: fn(&mut S, usize, Option<V>),
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        Self {
            entity_index,
            to_value,
            getter,
            setter,
            variable_name,
            descriptor_index,
        }
    }

    /// Returns the entity index.
    pub fn entity_index(&self) -> usize {
        self.entity_index
    }

    /// Returns the target value.
    pub fn to_value(&self) -> Option<&V> {
        self.to_value.as_ref()
    }

    /// Returns the getter function pointer.
    pub fn getter(&self) -> fn(&S, usize) -> Option<V> {
        self.getter
    }

    /// Returns the setter function pointer.
    pub fn setter(&self) -> fn(&mut S, usize, Option<V>) {
        self.setter
    }
}

impl<S, V> Move<S> for ChangeMove<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    fn is_doable(&self, score_director: &dyn ScoreDirector<S>) -> bool {
        // Get current value using typed getter - no boxing, no downcast
        let current = (self.getter)(score_director.working_solution(), self.entity_index);

        // Compare directly - fully typed comparison
        match (&current, &self.to_value) {
            (None, None) => false, // Both unassigned
            (Some(cur), Some(target)) => cur != target, // Different values
            _ => true, // One assigned, one not
        }
    }

    fn do_move(&self, score_director: &mut dyn ScoreDirector<S>) {
        // Capture old value using typed getter - zero erasure
        let old_value = (self.getter)(score_director.working_solution(), self.entity_index);

        // Notify before change
        score_director.before_variable_changed(
            self.descriptor_index,
            self.entity_index,
            self.variable_name,
        );

        // Set value using typed setter - no boxing
        (self.setter)(
            score_director.working_solution_mut(),
            self.entity_index,
            self.to_value.clone(),
        );

        // Notify after change
        score_director.after_variable_changed(
            self.descriptor_index,
            self.entity_index,
            self.variable_name,
        );

        // Register typed undo closure - zero erasure
        let setter = self.setter;
        let idx = self.entity_index;
        score_director.register_undo(Box::new(move |s: &mut S| {
            setter(s, idx, old_value);
        }));
    }

    fn descriptor_index(&self) -> usize {
        self.descriptor_index
    }

    fn entity_indices(&self) -> &[usize] {
        std::slice::from_ref(&self.entity_index)
    }

    fn variable_name(&self) -> &str {
        self.variable_name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solverforge_scoring::SimpleScoreDirector;
    use solverforge_core::domain::SolutionDescriptor;
    use solverforge_core::score::SimpleScore;
    use std::any::TypeId;

    #[derive(Clone, Debug, PartialEq)]
    struct Task {
        id: usize,
        priority: Option<i32>,
    }

    #[derive(Clone, Debug)]
    struct TaskSolution {
        tasks: Vec<Task>,
        score: Option<SimpleScore>,
    }

    impl PlanningSolution for TaskSolution {
        type Score = SimpleScore;
        fn score(&self) -> Option<Self::Score> { self.score }
        fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
    }

    // Typed getter: extracts priority from task at index
    fn get_priority(s: &TaskSolution, i: usize) -> Option<i32> {
        s.tasks.get(i).and_then(|t| t.priority)
    }

    // Typed setter: sets priority on task at index
    fn set_priority(s: &mut TaskSolution, i: usize, v: Option<i32>) {
        if let Some(task) = s.tasks.get_mut(i) {
            task.priority = v;
        }
    }

    fn create_director(tasks: Vec<Task>) -> SimpleScoreDirector<TaskSolution, impl Fn(&TaskSolution) -> SimpleScore> {
        let solution = TaskSolution { tasks, score: None };
        let descriptor = SolutionDescriptor::new("TaskSolution", TypeId::of::<TaskSolution>());
        SimpleScoreDirector::with_calculator(solution, descriptor, |_| SimpleScore::of(0))
    }

    #[test]
    fn test_change_move_is_doable() {
        let tasks = vec![
            Task { id: 0, priority: Some(1) },
            Task { id: 1, priority: Some(2) },
        ];
        let director = create_director(tasks);

        // Different value - doable
        let m = ChangeMove::new(0, Some(5), get_priority, set_priority, "priority", 0);
        assert!(m.is_doable(&director));

        // Same value - not doable
        let m = ChangeMove::new(0, Some(1), get_priority, set_priority, "priority", 0);
        assert!(!m.is_doable(&director));
    }

    #[test]
    fn test_change_move_do_move() {
        let tasks = vec![Task { id: 0, priority: Some(1) }];
        let mut director = create_director(tasks);

        let m = ChangeMove::new(0, Some(5), get_priority, set_priority, "priority", 0);
        m.do_move(&mut director);

        // Verify change using typed getter directly
        let val = get_priority(director.working_solution(), 0);
        assert_eq!(val, Some(5));
    }

    #[test]
    fn test_change_move_to_none() {
        let tasks = vec![Task { id: 0, priority: Some(5) }];
        let mut director = create_director(tasks);

        let m = ChangeMove::new(0, None, get_priority, set_priority, "priority", 0);
        assert!(m.is_doable(&director));

        m.do_move(&mut director);

        let val = get_priority(director.working_solution(), 0);
        assert_eq!(val, None);
    }

    #[test]
    fn test_change_move_entity_indices() {
        let m = ChangeMove::new(3, Some(5), get_priority, set_priority, "priority", 0);
        assert_eq!(m.entity_indices(), &[3]);
    }

    #[test]
    fn test_change_move_clone() {
        let m1 = ChangeMove::new(0, Some(5), get_priority, set_priority, "priority", 0);
        let m2 = m1.clone();
        assert_eq!(m1.entity_index, m2.entity_index);
        assert_eq!(m1.to_value, m2.to_value);
    }
}
