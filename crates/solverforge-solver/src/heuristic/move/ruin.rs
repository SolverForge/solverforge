//! RuinMove - unassigns a subset of entities for Large Neighborhood Search.
//!
//! This move "ruins" (unassigns) selected entities, allowing a construction
//! heuristic to reassign them. This is the fundamental building block for
//! Large Neighborhood Search (LNS) algorithms.
//!
//! # Zero-Erasure Design
//!
//! Stores only indices. No value type parameter. Operations use VariableOperations trait.

use std::fmt::Debug;
use std::marker::PhantomData;

use smallvec::SmallVec;
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use crate::operations::VariableOperations;

use super::Move;

/// A move that unassigns multiple entities for Large Neighborhood Search.
///
/// This move removes the value at position 0 for basic variables,
/// creating "gaps" that a construction heuristic can fill. Combined with
/// construction, this enables exploring distant regions of the search space.
///
/// # Type Parameters
/// * `S` - The planning solution type (must implement VariableOperations)
#[derive(Clone)]
pub struct RuinMove<S> {
    /// Indices of entities to unassign
    entity_indices: SmallVec<[usize; 8]>,
    variable_name: &'static str,
    descriptor_index: usize,
    _phantom: PhantomData<fn() -> S>,
}

impl<S> Debug for RuinMove<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RuinMove")
            .field("entities", &self.entity_indices.as_slice())
            .field("variable_name", &self.variable_name)
            .finish()
    }
}

impl<S> RuinMove<S> {
    /// Creates a new ruin move.
    ///
    /// # Arguments
    /// * `entity_indices` - Indices of entities to unassign
    /// * `variable_name` - Name of the planning variable
    /// * `descriptor_index` - Entity descriptor index
    pub fn new(entity_indices: &[usize], variable_name: &'static str, descriptor_index: usize) -> Self {
        Self {
            entity_indices: SmallVec::from_slice(entity_indices),
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

impl<S> Move<S> for RuinMove<S>
where
    S: PlanningSolution + VariableOperations,
{
    fn is_doable<D: ScoreDirector<S>>(&self, score_director: &D) -> bool {
        // At least one entity must be currently assigned
        let solution = score_director.working_solution();
        self.entity_indices
            .iter()
            .any(|&idx| solution.list_len(idx) > 0)
    }

    fn do_move<D: ScoreDirector<S>>(&self, score_director: &mut D) {
        let descriptor = self.descriptor_index;
        let variable_name = self.variable_name;

        // Collect old values for undo
        let old_values: SmallVec<[(usize, Option<<S as VariableOperations>::Element>); 8]> = self
            .entity_indices
            .iter()
            .map(|&idx| {
                let solution = score_director.working_solution();
                let old = if solution.list_len(idx) > 0 {
                    Some(solution.get(idx, 0))
                } else {
                    None
                };
                (idx, old)
            })
            .collect();

        // Unassign all entities
        for &idx in &self.entity_indices {
            score_director.before_variable_changed(descriptor, idx, variable_name);
            let sol = score_director.working_solution_mut();
            if sol.list_len(idx) > 0 {
                sol.remove(idx, 0);
            }
            score_director.after_variable_changed(descriptor, idx, variable_name);
        }

        // Register undo to restore old values
        score_director.register_undo(Box::new(move |s: &mut S| {
            for (idx, old_value) in old_values {
                if let Some(old) = old_value {
                    s.insert(idx, 0, old);
                }
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
    use crate::operations::VariableOperations;
    use solverforge_core::domain::{EntityDescriptor, SolutionDescriptor, TypedEntityExtractor};
    use solverforge_core::score::SimpleScore;
    use solverforge_scoring::{RecordingScoreDirector, SimpleScoreDirector};
    use std::any::TypeId;

    #[derive(Clone, Debug)]
    struct Task {
        assigned_to: Option<usize>,
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

    impl VariableOperations for Schedule {
        type Element = usize;

        fn element_count(&self) -> usize {
            10 // 10 possible assignments
        }

        fn entity_count(&self) -> usize {
            self.tasks.len()
        }

        fn assigned_elements(&self) -> Vec<Self::Element> {
            self.tasks.iter().filter_map(|t| t.assigned_to).collect()
        }

        fn assign(&mut self, entity_idx: usize, elem: Self::Element) {
            self.tasks[entity_idx].assigned_to = Some(elem);
        }

        fn list_len(&self, entity_idx: usize) -> usize {
            if self.tasks[entity_idx].assigned_to.is_some() {
                1
            } else {
                0
            }
        }

        fn remove(&mut self, entity_idx: usize, _pos: usize) -> Self::Element {
            self.tasks[entity_idx].assigned_to.take().unwrap()
        }

        fn insert(&mut self, entity_idx: usize, _pos: usize, elem: Self::Element) {
            self.tasks[entity_idx].assigned_to = Some(elem);
        }

        fn get(&self, entity_idx: usize, _pos: usize) -> Self::Element {
            self.tasks[entity_idx].assigned_to.unwrap()
        }

        fn descriptor_index() -> usize {
            0
        }

        fn variable_name() -> &'static str {
            "assigned_to"
        }

        fn is_list_variable() -> bool {
            false
        }
    }

    fn get_tasks(s: &Schedule) -> &Vec<Task> {
        &s.tasks
    }
    fn get_tasks_mut(s: &mut Schedule) -> &mut Vec<Task> {
        &mut s.tasks
    }

    fn create_director(
        assignments: &[Option<usize>],
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

        let m = RuinMove::<Schedule>::new(&[1], "assigned_to", 0);

        assert!(m.is_doable(&director));

        {
            let mut recording = RecordingScoreDirector::new(&mut director);
            m.do_move(&mut recording);

            assert_eq!(recording.working_solution().tasks[0].assigned_to, Some(1));
            assert_eq!(recording.working_solution().tasks[1].assigned_to, None);
            assert_eq!(recording.working_solution().tasks[2].assigned_to, Some(3));

            recording.undo_changes();
        }

        // Restored
        assert_eq!(director.working_solution().tasks[1].assigned_to, Some(2));
    }

    #[test]
    fn ruin_multiple_entities() {
        let mut director = create_director(&[Some(1), Some(2), Some(3), Some(4)]);

        let m = RuinMove::<Schedule>::new(&[0, 2, 3], "assigned_to", 0);

        assert!(m.is_doable(&director));
        assert_eq!(m.ruin_count(), 3);

        {
            let mut recording = RecordingScoreDirector::new(&mut director);
            m.do_move(&mut recording);

            assert_eq!(recording.working_solution().tasks[0].assigned_to, None);
            assert_eq!(recording.working_solution().tasks[1].assigned_to, Some(2));
            assert_eq!(recording.working_solution().tasks[2].assigned_to, None);
            assert_eq!(recording.working_solution().tasks[3].assigned_to, None);

            recording.undo_changes();
        }

        // All restored
        assert_eq!(director.working_solution().tasks[0].assigned_to, Some(1));
        assert_eq!(director.working_solution().tasks[2].assigned_to, Some(3));
        assert_eq!(director.working_solution().tasks[3].assigned_to, Some(4));
    }

    #[test]
    fn ruin_already_unassigned_is_doable() {
        // One assigned, one unassigned
        let director = create_director(&[Some(1), None]);

        // Ruin both - still doable because entity 0 is assigned
        let m = RuinMove::<Schedule>::new(&[0, 1], "assigned_to", 0);

        assert!(m.is_doable(&director));
    }

    #[test]
    fn ruin_all_unassigned_not_doable() {
        let director = create_director(&[None, None]);

        let m = RuinMove::<Schedule>::new(&[0, 1], "assigned_to", 0);

        assert!(!m.is_doable(&director));
    }
}
