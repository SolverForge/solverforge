//! Pillar selector for selecting groups of entities with the same variable value.
//!
//! A pillar is a group of entities that share the same planning variable value.
//! Pillar moves operate on entire pillars, changing or swapping all entities
//! in the pillar atomically.

use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use super::entity::{EntityReference, EntitySelector};

/// A pillar is a group of entity references that share the same variable value.
///
/// All entities in a pillar have the same value for a specific planning variable,
/// which allows them to be moved together atomically.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pillar {
    /// The entity references in this pillar.
    pub entities: Vec<EntityReference>,
}

impl Pillar {
    /// Creates a new pillar with the given entities.
    pub fn new(entities: Vec<EntityReference>) -> Self {
        Self { entities }
    }

    /// Returns the number of entities in this pillar.
    pub fn size(&self) -> usize {
        self.entities.len()
    }

    /// Returns true if this pillar is empty.
    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }

    /// Returns the first entity reference in this pillar.
    pub fn first(&self) -> Option<&EntityReference> {
        self.entities.first()
    }

    /// Returns an iterator over the entity references.
    pub fn iter(&self) -> impl Iterator<Item = &EntityReference> {
        self.entities.iter()
    }
}

/// Trait for selecting pillars of entities.
///
/// A pillar selector groups entities by their variable values and returns
/// groups (pillars) that can be moved together.
pub trait PillarSelector<S: PlanningSolution>: Send + Debug {
    /// Returns an iterator over pillars.
    ///
    /// Each pillar contains entity references for entities that share
    /// the same variable value.
    fn iter<'a>(
        &'a self,
        score_director: &'a dyn ScoreDirector<S>,
    ) -> Box<dyn Iterator<Item = Pillar> + 'a>;

    /// Returns the approximate number of pillars.
    fn size(&self, score_director: &dyn ScoreDirector<S>) -> usize;

    /// Returns true if this selector may return the same pillar multiple times.
    fn is_never_ending(&self) -> bool {
        false
    }

    /// Returns the descriptor index this selector operates on.
    fn descriptor_index(&self) -> usize;
}

/// Configuration for sub-pillar selection.
#[derive(Debug, Clone)]
pub struct SubPillarConfig {
    /// Whether sub-pillar selection is enabled.
    pub enabled: bool,
    /// Minimum size of a sub-pillar (default: 1).
    pub minimum_size: usize,
    /// Maximum size of a sub-pillar (default: usize::MAX).
    pub maximum_size: usize,
}

impl Default for SubPillarConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            minimum_size: 1,
            maximum_size: usize::MAX,
        }
    }
}

impl SubPillarConfig {
    /// Creates a new sub-pillar config with sub-pillars disabled.
    pub fn none() -> Self {
        Self::default()
    }

    /// Creates a new sub-pillar config with sub-pillars enabled.
    pub fn all() -> Self {
        Self {
            enabled: true,
            minimum_size: 1,
            maximum_size: usize::MAX,
        }
    }

    /// Sets the minimum sub-pillar size.
    pub fn with_minimum_size(mut self, size: usize) -> Self {
        self.minimum_size = size.max(1);
        self
    }

    /// Sets the maximum sub-pillar size.
    pub fn with_maximum_size(mut self, size: usize) -> Self {
        self.maximum_size = size;
        self
    }
}

/// A pillar selector that groups entities by their variable value.
///
/// This selector uses an entity selector to get entities, then groups them
/// by the value of a specified variable using a value extractor function.
///
/// # Zero-Erasure Design
///
/// The extractor function `E` is stored as a concrete generic type parameter,
/// eliminating virtual dispatch overhead when grouping entities by value.
pub struct DefaultPillarSelector<S, V, E>
where
    S: PlanningSolution,
    V: Clone + Eq + Hash + Send + Sync + 'static,
    E: Fn(&dyn ScoreDirector<S>, usize, usize) -> Option<V> + Send + Sync,
{
    /// The underlying entity selector.
    entity_selector: Box<dyn EntitySelector<S>>,
    /// The descriptor index.
    descriptor_index: usize,
    /// The variable name for grouping.
    variable_name: &'static str,
    /// Function to extract the value from an entity for grouping (zero-erasure).
    value_extractor: E,
    /// Sub-pillar configuration.
    sub_pillar_config: SubPillarConfig,
}

impl<S, V, E> Debug for DefaultPillarSelector<S, V, E>
where
    S: PlanningSolution,
    V: Clone + Eq + Hash + Send + Sync + 'static,
    E: Fn(&dyn ScoreDirector<S>, usize, usize) -> Option<V> + Send + Sync,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DefaultPillarSelector")
            .field("entity_selector", &self.entity_selector)
            .field("descriptor_index", &self.descriptor_index)
            .field("variable_name", &self.variable_name)
            .field("sub_pillar_config", &self.sub_pillar_config)
            .finish()
    }
}

impl<S, V, E> DefaultPillarSelector<S, V, E>
where
    S: PlanningSolution,
    V: Clone + Eq + Hash + Send + Sync + 'static,
    E: Fn(&dyn ScoreDirector<S>, usize, usize) -> Option<V> + Send + Sync,
{
    /// Creates a new default pillar selector.
    ///
    /// # Arguments
    ///
    /// * `entity_selector` - The entity selector to get entities from
    /// * `descriptor_index` - The entity descriptor index
    /// * `variable_name` - The variable name for grouping
    /// * `value_extractor` - Function to extract the grouping value from an entity
    pub fn new(
        entity_selector: Box<dyn EntitySelector<S>>,
        descriptor_index: usize,
        variable_name: &'static str,
        value_extractor: E,
    ) -> Self {
        Self {
            entity_selector,
            descriptor_index,
            variable_name,
            value_extractor,
            sub_pillar_config: SubPillarConfig::default(),
        }
    }

    /// Sets the sub-pillar configuration.
    pub fn with_sub_pillar_config(mut self, config: SubPillarConfig) -> Self {
        self.sub_pillar_config = config;
        self
    }

    /// Returns the variable name.
    pub fn variable_name(&self) -> &str {
        self.variable_name
    }

    /// Builds the pillar list from the current solution state.
    fn build_pillars(&self, score_director: &dyn ScoreDirector<S>) -> Vec<Pillar> {
        // Group entities by their value
        let mut value_to_entities: HashMap<Option<V>, Vec<EntityReference>> = HashMap::new();

        for entity_ref in self.entity_selector.iter(score_director) {
            let value = (self.value_extractor)(
                score_director,
                entity_ref.descriptor_index,
                entity_ref.entity_index,
            );
            value_to_entities.entry(value).or_default().push(entity_ref);
        }

        // Filter by minimum size and create pillars
        let min_size = self.sub_pillar_config.minimum_size;
        value_to_entities
            .into_values()
            .filter(|entities| entities.len() >= min_size)
            .map(Pillar::new)
            .collect()
    }
}

impl<S, V, E> PillarSelector<S> for DefaultPillarSelector<S, V, E>
where
    S: PlanningSolution,
    V: Clone + Eq + Hash + Send + Sync + 'static,
    E: Fn(&dyn ScoreDirector<S>, usize, usize) -> Option<V> + Send + Sync,
{
    fn iter<'a>(
        &'a self,
        score_director: &'a dyn ScoreDirector<S>,
    ) -> Box<dyn Iterator<Item = Pillar> + 'a> {
        let pillars = self.build_pillars(score_director);
        Box::new(pillars.into_iter())
    }

    fn size(&self, score_director: &dyn ScoreDirector<S>) -> usize {
        self.build_pillars(score_director).len()
    }

    fn descriptor_index(&self) -> usize {
        self.descriptor_index
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::heuristic::selector::entity::FromSolutionEntitySelector;
    use solverforge_core::domain::{EntityDescriptor, SolutionDescriptor, TypedEntityExtractor};
    use solverforge_core::score::SimpleScore;
    use solverforge_scoring::SimpleScoreDirector;
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

    fn get_employees(s: &ScheduleSolution) -> &Vec<Employee> {
        &s.employees
    }

    fn get_employees_mut(s: &mut ScheduleSolution) -> &mut Vec<Employee> {
        &mut s.employees
    }

    fn create_test_director(
        employees: Vec<Employee>,
    ) -> SimpleScoreDirector<ScheduleSolution, impl Fn(&ScheduleSolution) -> SimpleScore> {
        let solution = ScheduleSolution {
            employees,
            score: None,
        };

        let extractor = Box::new(TypedEntityExtractor::new(
            "Employee",
            "employees",
            get_employees,
            get_employees_mut,
        ));
        let entity_desc = EntityDescriptor::new("Employee", TypeId::of::<Employee>(), "employees")
            .with_extractor(extractor);

        let descriptor =
            SolutionDescriptor::new("ScheduleSolution", TypeId::of::<ScheduleSolution>())
                .with_entity(entity_desc);

        SimpleScoreDirector::with_calculator(solution, descriptor, |_| SimpleScore::of(0))
    }

    #[test]
    fn test_pillar_new() {
        let pillar = Pillar::new(vec![EntityReference::new(0, 0), EntityReference::new(0, 1)]);

        assert_eq!(pillar.size(), 2);
        assert!(!pillar.is_empty());
        assert_eq!(pillar.first(), Some(&EntityReference::new(0, 0)));
    }

    #[test]
    fn test_pillar_empty() {
        let pillar = Pillar::new(vec![]);
        assert!(pillar.is_empty());
        assert_eq!(pillar.first(), None);
    }

    #[test]
    fn test_default_pillar_selector_groups_by_value() {
        // Create employees with shifts: [1, 1, 2, 2, 2, 3]
        let employees = vec![
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
            Employee {
                id: 3,
                shift: Some(2),
            },
            Employee {
                id: 4,
                shift: Some(2),
            },
            Employee {
                id: 5,
                shift: Some(3),
            },
        ];
        let director = create_test_director(employees);

        // Verify entity IDs
        let solution = director.working_solution();
        for (i, emp) in solution.employees.iter().enumerate() {
            assert_eq!(emp.id, i);
        }

        let entity_selector = Box::new(FromSolutionEntitySelector::new(0));
        let selector = DefaultPillarSelector::<ScheduleSolution, i32, _>::new(
            entity_selector,
            0,
            "shift",
            |sd: &dyn ScoreDirector<ScheduleSolution>, _desc_idx, entity_idx| {
                let solution = sd.working_solution();
                solution.employees.get(entity_idx).and_then(|e| e.shift)
            },
        );

        let pillars: Vec<_> = selector.iter(&director).collect();

        // Should have 3 pillars (for shift values 1, 2, 3)
        assert_eq!(pillars.len(), 3);

        // Find pillar sizes
        let mut sizes: Vec<_> = pillars.iter().map(|p| p.size()).collect();
        sizes.sort();

        // Should have pillars of size 1, 2, and 3
        assert_eq!(sizes, vec![1, 2, 3]);
    }

    #[test]
    fn test_pillar_selector_with_minimum_size() {
        // Create employees with shifts: [1, 1, 2, 2, 2, 3]
        let employees = vec![
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
            Employee {
                id: 3,
                shift: Some(2),
            },
            Employee {
                id: 4,
                shift: Some(2),
            },
            Employee {
                id: 5,
                shift: Some(3),
            },
        ];
        let director = create_test_director(employees);

        // Verify entity IDs
        let solution = director.working_solution();
        for (i, emp) in solution.employees.iter().enumerate() {
            assert_eq!(emp.id, i);
        }

        let entity_selector = Box::new(FromSolutionEntitySelector::new(0));
        let selector = DefaultPillarSelector::<ScheduleSolution, i32, _>::new(
            entity_selector,
            0,
            "shift",
            |sd: &dyn ScoreDirector<ScheduleSolution>, _desc_idx, entity_idx| {
                let solution = sd.working_solution();
                solution.employees.get(entity_idx).and_then(|e| e.shift)
            },
        )
        .with_sub_pillar_config(SubPillarConfig::none().with_minimum_size(2));

        let pillars: Vec<_> = selector.iter(&director).collect();

        // Should only have 2 pillars (shift 1 has 2 entities, shift 2 has 3 entities)
        // Shift 3 only has 1 entity, so it's filtered out
        assert_eq!(pillars.len(), 2);
    }

    #[test]
    fn test_pillar_selector_with_none_values() {
        // Create employees with some unassigned
        let employees = vec![
            Employee {
                id: 0,
                shift: Some(1),
            },
            Employee { id: 1, shift: None },
            Employee { id: 2, shift: None },
            Employee {
                id: 3,
                shift: Some(1),
            },
        ];
        let director = create_test_director(employees);

        // Verify entity IDs
        let solution = director.working_solution();
        for (i, emp) in solution.employees.iter().enumerate() {
            assert_eq!(emp.id, i);
        }

        let entity_selector = Box::new(FromSolutionEntitySelector::new(0));
        let selector = DefaultPillarSelector::<ScheduleSolution, i32, _>::new(
            entity_selector,
            0,
            "shift",
            |sd: &dyn ScoreDirector<ScheduleSolution>, _desc_idx, entity_idx| {
                let solution = sd.working_solution();
                solution.employees.get(entity_idx).and_then(|e| e.shift)
            },
        );

        let pillars: Vec<_> = selector.iter(&director).collect();

        // Should have 2 pillars: one for shift 1 (2 entities), one for None (2 entities)
        assert_eq!(pillars.len(), 2);
    }

    #[test]
    fn test_pillar_selector_empty_solution() {
        let director = create_test_director(vec![]);

        let entity_selector = Box::new(FromSolutionEntitySelector::new(0));
        let selector = DefaultPillarSelector::<ScheduleSolution, i32, _>::new(
            entity_selector,
            0,
            "shift",
            |sd: &dyn ScoreDirector<ScheduleSolution>, _desc_idx, entity_idx| {
                let solution = sd.working_solution();
                solution.employees.get(entity_idx).and_then(|e| e.shift)
            },
        );

        let pillars: Vec<_> = selector.iter(&director).collect();
        assert!(pillars.is_empty());
        assert_eq!(selector.size(&director), 0);
    }

    #[test]
    fn test_sub_pillar_config() {
        let config = SubPillarConfig::none();
        assert!(!config.enabled);
        assert_eq!(config.minimum_size, 1);

        let config = SubPillarConfig::all();
        assert!(config.enabled);

        let config = SubPillarConfig::none()
            .with_minimum_size(2)
            .with_maximum_size(5);
        assert_eq!(config.minimum_size, 2);
        assert_eq!(config.maximum_size, 5);
    }
}
