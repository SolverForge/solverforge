//! Tests for pillar selectors.

use super::*;
use crate::heuristic::selector::entity::FromSolutionEntitySelector;
use solverforge_core::domain::{
    EntityDescriptor, PlanningSolution, SolutionDescriptor, TypedEntityExtractor,
};
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

    let descriptor = SolutionDescriptor::new("ScheduleSolution", TypeId::of::<ScheduleSolution>())
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

    let entity_selector = FromSolutionEntitySelector::new(0);
    let selector = DefaultPillarSelector::<ScheduleSolution, i32, _, _>::new(
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

    let entity_selector = FromSolutionEntitySelector::new(0);
    let selector = DefaultPillarSelector::<ScheduleSolution, i32, _, _>::new(
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

    let entity_selector = FromSolutionEntitySelector::new(0);
    let selector = DefaultPillarSelector::<ScheduleSolution, i32, _, _>::new(
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

    let entity_selector = FromSolutionEntitySelector::new(0);
    let selector = DefaultPillarSelector::<ScheduleSolution, i32, _, _>::new(
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
