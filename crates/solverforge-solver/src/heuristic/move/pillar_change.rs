//! PillarChangeMove - assigns a value to all entities in a pillar.
//!
//! A pillar is a group of entities that share the same variable value.
//! This move changes all of them to a new value atomically.
//!
//! # Zero-Erasure Design
//!
//! PillarChangeMove uses typed function pointers instead of `dyn Any` for complete
//! compile-time type safety. No runtime type checks or downcasting.

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use super::Move;

/// A move that assigns a value to all entities in a pillar.
///
/// Stores entity indices and typed function pointers for zero-erasure access.
/// Undo is handled by `RecordingScoreDirector`, not by this move.
///
/// # Type Parameters
/// * `S` - The planning solution type
/// * `V` - The variable value type
#[derive(Clone)]
pub struct PillarChangeMove<S, V> {
    entity_indices: Vec<usize>,
    descriptor_index: usize,
    variable_name: &'static str,
    to_value: Option<V>,
    /// Typed getter function pointer - zero erasure.
    getter: fn(&S, usize) -> Option<V>,
    /// Typed setter function pointer - zero erasure.
    setter: fn(&mut S, usize, Option<V>),
    _phantom: PhantomData<S>,
}

impl<S, V: Debug> Debug for PillarChangeMove<S, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PillarChangeMove")
            .field("entity_indices", &self.entity_indices)
            .field("descriptor_index", &self.descriptor_index)
            .field("variable_name", &self.variable_name)
            .field("to_value", &self.to_value)
            .finish()
    }
}

impl<S, V> PillarChangeMove<S, V> {
    /// Creates a new pillar change move with typed function pointers.
    ///
    /// # Arguments
    /// * `entity_indices` - Indices of entities in the pillar
    /// * `to_value` - The new value to assign to all entities
    /// * `getter` - Typed getter function pointer
    /// * `setter` - Typed setter function pointer
    /// * `variable_name` - Name of the variable being changed
    /// * `descriptor_index` - Index in the entity descriptor
    pub fn new(
        entity_indices: Vec<usize>,
        to_value: Option<V>,
        getter: fn(&S, usize) -> Option<V>,
        setter: fn(&mut S, usize, Option<V>),
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        Self {
            entity_indices,
            descriptor_index,
            variable_name,
            to_value,
            getter,
            setter,
            _phantom: PhantomData,
        }
    }

    /// Returns the pillar size.
    pub fn pillar_size(&self) -> usize {
        self.entity_indices.len()
    }

    /// Returns the target value.
    pub fn to_value(&self) -> Option<&V> {
        self.to_value.as_ref()
    }
}

impl<S, V> Move<S> for PillarChangeMove<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    fn is_doable(&self, score_director: &dyn ScoreDirector<S>) -> bool {
        if self.entity_indices.is_empty() {
            return false;
        }

        // Check first entity exists
        let count = score_director.entity_count(self.descriptor_index);
        if let Some(&first_idx) = self.entity_indices.first() {
            if count.is_none_or(|c| first_idx >= c) {
                return false;
            }

            // Get current value using typed getter - zero erasure
            let current = (self.getter)(score_director.working_solution(), first_idx);

            match (&current, &self.to_value) {
                (None, None) => false,
                (Some(cur), Some(target)) => cur != target,
                _ => true,
            }
        } else {
            false
        }
    }

    fn do_move(&self, score_director: &mut dyn ScoreDirector<S>) {
        // Capture old values using typed getter - zero erasure
        let old_values: Vec<(usize, Option<V>)> = self
            .entity_indices
            .iter()
            .map(|&idx| (idx, (self.getter)(score_director.working_solution(), idx)))
            .collect();

        // Notify before changes for all entities
        for &idx in &self.entity_indices {
            score_director.before_variable_changed(self.descriptor_index, idx, self.variable_name);
        }

        // Apply new value to all entities using typed setter - zero erasure
        for &idx in &self.entity_indices {
            (self.setter)(
                score_director.working_solution_mut(),
                idx,
                self.to_value.clone(),
            );
        }

        // Notify after changes
        for &idx in &self.entity_indices {
            score_director.after_variable_changed(self.descriptor_index, idx, self.variable_name);
        }

        // Register typed undo closure
        let setter = self.setter;
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
    struct Employee {
        id: usize,
        shift: Option<i32>,
    }

    #[derive(Clone, Debug)]
    struct ScheduleSolution {
        employees: Vec<Employee>,
        score: Option<SimpleScore>,
    }

    impl PlanningSolution for ScheduleSolution {
        type Score = SimpleScore;
        fn score(&self) -> Option<Self::Score> {
            self.score
        }
        fn set_score(&mut self, score: Option<Self::Score>) {
            self.score = score;
        }
    }

    // Typed getter - zero erasure
    fn get_shift(s: &ScheduleSolution, idx: usize) -> Option<i32> {
        s.employees.get(idx).and_then(|e| e.shift)
    }

    // Typed setter - zero erasure
    fn set_shift(s: &mut ScheduleSolution, idx: usize, v: Option<i32>) {
        if let Some(e) = s.employees.get_mut(idx) {
            e.shift = v;
        }
    }

    fn create_director(
        employees: Vec<Employee>,
    ) -> SimpleScoreDirector<ScheduleSolution, impl Fn(&ScheduleSolution) -> SimpleScore> {
        let solution = ScheduleSolution {
            employees,
            score: None,
        };

        let extractor = Box::new(TypedEntityExtractor::new(
            "Employee",
            "employees",
            |s: &ScheduleSolution| &s.employees,
            |s: &mut ScheduleSolution| &mut s.employees,
        ));
        let entity_desc = EntityDescriptor::new("Employee", TypeId::of::<Employee>(), "employees")
            .with_extractor(extractor);

        let descriptor =
            SolutionDescriptor::new("ScheduleSolution", TypeId::of::<ScheduleSolution>())
                .with_entity(entity_desc);

        SimpleScoreDirector::with_calculator(solution, descriptor, |_| SimpleScore::of(0))
    }

    #[test]
    fn test_pillar_change_all_entities() {
        let mut director = create_director(vec![
            Employee {
                id: 0,
                shift: Some(1),
            },
            Employee {
                id: 1,
                shift: Some(1),
            },
            Employee {
                id: 2,
                shift: Some(2),
            },
        ]);

        // Change pillar [0, 1] from shift 1 to shift 5
        let m = PillarChangeMove::<ScheduleSolution, i32>::new(
            vec![0, 1],
            Some(5),
            get_shift,
            set_shift,
            "shift",
            0,
        );

        assert!(m.is_doable(&director));
        assert_eq!(m.pillar_size(), 2);

        {
            let mut recording = RecordingScoreDirector::new(&mut director);
            m.do_move(&mut recording);

            // Verify ALL entities changed using typed getter
            assert_eq!(get_shift(recording.working_solution(), 0), Some(5));
            assert_eq!(get_shift(recording.working_solution(), 1), Some(5));
            assert_eq!(get_shift(recording.working_solution(), 2), Some(2)); // Unchanged

            // Undo
            recording.undo_changes();
        }

        assert_eq!(get_shift(director.working_solution(), 0), Some(1));
        assert_eq!(get_shift(director.working_solution(), 1), Some(1));
        assert_eq!(get_shift(director.working_solution(), 2), Some(2));

        // Verify entity identity preserved
        let solution = director.working_solution();
        assert_eq!(solution.employees[0].id, 0);
        assert_eq!(solution.employees[1].id, 1);
        assert_eq!(solution.employees[2].id, 2);
    }

    #[test]
    fn test_pillar_change_same_value_not_doable() {
        let director = create_director(vec![
            Employee {
                id: 0,
                shift: Some(5),
            },
            Employee {
                id: 1,
                shift: Some(5),
            },
        ]);

        let m = PillarChangeMove::<ScheduleSolution, i32>::new(
            vec![0, 1],
            Some(5),
            get_shift,
            set_shift,
            "shift",
            0,
        );

        assert!(!m.is_doable(&director));
    }

    #[test]
    fn test_pillar_change_empty_pillar_not_doable() {
        let director = create_director(vec![Employee {
            id: 0,
            shift: Some(1),
        }]);

        let m = PillarChangeMove::<ScheduleSolution, i32>::new(
            vec![],
            Some(5),
            get_shift,
            set_shift,
            "shift",
            0,
        );

        assert!(!m.is_doable(&director));
    }

    #[test]
    fn test_pillar_change_entity_indices() {
        let m = PillarChangeMove::<ScheduleSolution, i32>::new(
            vec![1, 3, 5],
            Some(5),
            get_shift,
            set_shift,
            "shift",
            0,
        );
        assert_eq!(m.entity_indices(), &[1, 3, 5]);
    }
}
