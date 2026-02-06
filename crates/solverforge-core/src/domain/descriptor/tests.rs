//! Tests for descriptor types.

use super::*;
use crate::domain::variable::ValueRangeType;
use crate::domain::{TypedEntityExtractor, VariableType};
use std::any::{Any, TypeId};

#[derive(Clone, Debug)]
struct TestEntity {
    id: i64,
    row: Option<i32>,
}

// Entity extraction tests

#[derive(Clone, Debug)]
struct TestSolution {
    entities: Vec<TestEntity>,
}

fn get_entities(s: &TestSolution) -> &Vec<TestEntity> {
    &s.entities
}

fn get_entities_mut(s: &mut TestSolution) -> &mut Vec<TestEntity> {
    &mut s.entities
}

#[test]
fn test_entity_descriptor_with_extractor() {
    let extractor = Box::new(TypedEntityExtractor::new(
        "TestEntity",
        "entities",
        get_entities,
        get_entities_mut,
    ));

    let descriptor = EntityDescriptor::new("TestEntity", TypeId::of::<TestEntity>(), "entities")
        .with_extractor(extractor);

    assert!(descriptor.has_extractor());
}

#[test]
fn test_entity_descriptor_entity_count() {
    let extractor = Box::new(TypedEntityExtractor::new(
        "TestEntity",
        "entities",
        get_entities,
        get_entities_mut,
    ));

    let descriptor = EntityDescriptor::new("TestEntity", TypeId::of::<TestEntity>(), "entities")
        .with_extractor(extractor);

    let solution = TestSolution {
        entities: vec![
            TestEntity {
                id: 1,
                row: Some(1),
            },
            TestEntity {
                id: 2,
                row: Some(2),
            },
            TestEntity { id: 3, row: None },
        ],
    };

    assert_eq!(descriptor.entity_count(&solution as &dyn Any), Some(3));
}

#[test]
fn test_entity_descriptor_get_entity() {
    let extractor = Box::new(TypedEntityExtractor::new(
        "TestEntity",
        "entities",
        get_entities,
        get_entities_mut,
    ));

    let descriptor = EntityDescriptor::new("TestEntity", TypeId::of::<TestEntity>(), "entities")
        .with_extractor(extractor);

    let solution = TestSolution {
        entities: vec![
            TestEntity {
                id: 1,
                row: Some(10),
            },
            TestEntity {
                id: 2,
                row: Some(20),
            },
        ],
    };

    let entity = descriptor.get_entity(&solution as &dyn Any, 0);
    assert!(entity.is_some());
    let entity = entity.unwrap().downcast_ref::<TestEntity>().unwrap();
    assert_eq!(entity.id, 1);
    assert_eq!(entity.row, Some(10));

    let entity = descriptor.get_entity(&solution as &dyn Any, 1);
    assert!(entity.is_some());
    let entity = entity.unwrap().downcast_ref::<TestEntity>().unwrap();
    assert_eq!(entity.id, 2);
}

#[test]
fn test_entity_descriptor_get_entity_mut() {
    let extractor = Box::new(TypedEntityExtractor::new(
        "TestEntity",
        "entities",
        get_entities,
        get_entities_mut,
    ));

    let descriptor = EntityDescriptor::new("TestEntity", TypeId::of::<TestEntity>(), "entities")
        .with_extractor(extractor);

    let mut solution = TestSolution {
        entities: vec![TestEntity {
            id: 1,
            row: Some(10),
        }],
    };

    let entity = descriptor.get_entity_mut(&mut solution as &mut dyn Any, 0);
    assert!(entity.is_some());
    let entity = entity.unwrap().downcast_mut::<TestEntity>().unwrap();
    entity.row = Some(100);

    assert_eq!(solution.entities[0].row, Some(100));
}

#[test]
fn test_entity_descriptor_entity_refs() {
    let extractor = Box::new(TypedEntityExtractor::new(
        "TestEntity",
        "entities",
        get_entities,
        get_entities_mut,
    ));

    let descriptor = EntityDescriptor::new("TestEntity", TypeId::of::<TestEntity>(), "entities")
        .with_extractor(extractor);

    let solution = TestSolution {
        entities: vec![
            TestEntity {
                id: 1,
                row: Some(10),
            },
            TestEntity {
                id: 2,
                row: Some(20),
            },
        ],
    };

    let refs = descriptor.entity_refs(&solution as &dyn Any);
    assert_eq!(refs.len(), 2);
    assert_eq!(refs[0].index, 0);
    assert_eq!(refs[1].index, 1);
}

#[test]
fn test_entity_descriptor_for_each_entity() {
    let extractor = Box::new(TypedEntityExtractor::new(
        "TestEntity",
        "entities",
        get_entities,
        get_entities_mut,
    ));

    let descriptor = EntityDescriptor::new("TestEntity", TypeId::of::<TestEntity>(), "entities")
        .with_extractor(extractor);

    let solution = TestSolution {
        entities: vec![
            TestEntity {
                id: 1,
                row: Some(10),
            },
            TestEntity {
                id: 2,
                row: Some(20),
            },
        ],
    };

    let mut count = 0;
    let mut sum = 0i32;
    descriptor.for_each_entity(&solution as &dyn Any, |_, entity| {
        let e = entity.downcast_ref::<TestEntity>().unwrap();
        count += 1;
        sum += e.row.unwrap_or(0);
    });

    assert_eq!(count, 2);
    assert_eq!(sum, 30);
}

#[test]
fn test_entity_descriptor_no_extractor() {
    let descriptor = EntityDescriptor::new("TestEntity", TypeId::of::<TestEntity>(), "entities");

    assert!(!descriptor.has_extractor());

    let solution = TestSolution {
        entities: vec![TestEntity {
            id: 1,
            row: Some(10),
        }],
    };

    assert!(descriptor.entity_count(&solution as &dyn Any).is_none());
    assert!(descriptor.get_entity(&solution as &dyn Any, 0).is_none());
    assert!(descriptor.entity_refs(&solution as &dyn Any).is_empty());
}

// SolutionDescriptor tests

fn create_test_entity_descriptor() -> EntityDescriptor {
    let extractor = Box::new(TypedEntityExtractor::new(
        "TestEntity",
        "entities",
        get_entities,
        get_entities_mut,
    ));

    EntityDescriptor::new("TestEntity", TypeId::of::<TestEntity>(), "entities")
        .with_extractor(extractor)
}

#[test]
fn test_solution_descriptor_total_entity_count() {
    let entity_desc = create_test_entity_descriptor();

    let solution_desc = SolutionDescriptor::new("TestSolution", TypeId::of::<TestSolution>())
        .with_entity(entity_desc);

    let solution = TestSolution {
        entities: vec![
            TestEntity {
                id: 1,
                row: Some(1),
            },
            TestEntity {
                id: 2,
                row: Some(2),
            },
            TestEntity {
                id: 3,
                row: Some(3),
            },
        ],
    };

    assert_eq!(
        solution_desc.total_entity_count(&solution as &dyn Any),
        Some(3)
    );
}

#[test]
fn test_solution_descriptor_all_entity_refs() {
    let entity_desc = create_test_entity_descriptor();

    let solution_desc = SolutionDescriptor::new("TestSolution", TypeId::of::<TestSolution>())
        .with_entity(entity_desc);

    let solution = TestSolution {
        entities: vec![
            TestEntity {
                id: 1,
                row: Some(1),
            },
            TestEntity {
                id: 2,
                row: Some(2),
            },
        ],
    };

    let refs = solution_desc.all_entity_refs(&solution as &dyn Any);
    assert_eq!(refs.len(), 2);
    assert_eq!(refs[0].0, 0); // descriptor index
    assert_eq!(refs[0].1.index, 0); // entity index
    assert_eq!(refs[1].0, 0);
    assert_eq!(refs[1].1.index, 1);
}

#[test]
fn test_solution_descriptor_for_each_entity() {
    let entity_desc = create_test_entity_descriptor();

    let solution_desc = SolutionDescriptor::new("TestSolution", TypeId::of::<TestSolution>())
        .with_entity(entity_desc);

    let solution = TestSolution {
        entities: vec![
            TestEntity {
                id: 1,
                row: Some(10),
            },
            TestEntity {
                id: 2,
                row: Some(20),
            },
        ],
    };

    let mut collected = Vec::new();
    solution_desc.for_each_entity(&solution as &dyn Any, |desc_idx, entity_idx, entity| {
        let e = entity.downcast_ref::<TestEntity>().unwrap();
        collected.push((desc_idx, entity_idx, e.id));
    });

    assert_eq!(collected, vec![(0, 0, 1), (0, 1, 2)]);
}

#[test]
fn test_solution_descriptor_get_entity() {
    let entity_desc = create_test_entity_descriptor();

    let solution_desc = SolutionDescriptor::new("TestSolution", TypeId::of::<TestSolution>())
        .with_entity(entity_desc);

    let solution = TestSolution {
        entities: vec![
            TestEntity {
                id: 1,
                row: Some(10),
            },
            TestEntity {
                id: 2,
                row: Some(20),
            },
        ],
    };

    let entity = solution_desc.get_entity(&solution as &dyn Any, 0, 1);
    assert!(entity.is_some());
    let entity = entity.unwrap().downcast_ref::<TestEntity>().unwrap();
    assert_eq!(entity.id, 2);
    assert_eq!(entity.row, Some(20));

    // Invalid descriptor index
    assert!(solution_desc
        .get_entity(&solution as &dyn Any, 99, 0)
        .is_none());
    // Invalid entity index
    assert!(solution_desc
        .get_entity(&solution as &dyn Any, 0, 99)
        .is_none());
}

#[test]
fn test_solution_descriptor_all_extractors_configured() {
    // With extractor
    let entity_desc = create_test_entity_descriptor();
    let solution_desc = SolutionDescriptor::new("TestSolution", TypeId::of::<TestSolution>())
        .with_entity(entity_desc);
    assert!(solution_desc.all_extractors_configured());

    // Without extractor
    let entity_desc_no_extractor =
        EntityDescriptor::new("TestEntity", TypeId::of::<TestEntity>(), "entities");
    let solution_desc = SolutionDescriptor::new("TestSolution", TypeId::of::<TestSolution>())
        .with_entity(entity_desc_no_extractor);
    assert!(!solution_desc.all_extractors_configured());
}

#[test]
fn test_solution_descriptor_counts() {
    let entity_desc = create_test_entity_descriptor();
    let fact_desc = ProblemFactDescriptor::new("SomeFact", TypeId::of::<i32>(), "facts");

    let solution_desc = SolutionDescriptor::new("TestSolution", TypeId::of::<TestSolution>())
        .with_entity(entity_desc)
        .with_problem_fact(fact_desc);

    assert_eq!(solution_desc.entity_descriptor_count(), 1);
    assert_eq!(solution_desc.problem_fact_descriptor_count(), 1);
}

// ============ VariableDescriptor Tests ============

#[test]
fn test_variable_descriptor_genuine() {
    let desc = VariableDescriptor::genuine("my_var");
    assert_eq!(desc.name, "my_var");
    assert_eq!(desc.variable_type, VariableType::Genuine);
    assert!(!desc.allows_unassigned);
    assert!(desc.variable_type.is_genuine());
    assert!(desc.variable_type.is_basic());
    assert!(!desc.variable_type.is_chained());
}

#[test]
fn test_variable_descriptor_chained() {
    let desc = VariableDescriptor::chained("previous");
    assert_eq!(desc.name, "previous");
    assert_eq!(desc.variable_type, VariableType::Chained);
    assert!(!desc.allows_unassigned); // Chained vars must point to something
    assert!(desc.variable_type.is_genuine()); // Chained is a genuine variable type
    assert!(!desc.variable_type.is_basic()); // But not basic
    assert!(desc.variable_type.is_chained());
}

#[test]
fn test_variable_descriptor_list() {
    let desc = VariableDescriptor::list("tasks");
    assert_eq!(desc.name, "tasks");
    assert_eq!(desc.variable_type, VariableType::List);
    assert!(desc.variable_type.is_list());
    assert!(desc.variable_type.is_genuine());
    assert!(!desc.variable_type.is_chained());
}

#[test]
fn test_variable_descriptor_with_value_range() {
    let desc = VariableDescriptor::genuine("var").with_value_range("range_provider");
    assert_eq!(desc.value_range_provider, Some("range_provider"));
}

#[test]
fn test_variable_descriptor_with_allows_unassigned() {
    let desc = VariableDescriptor::genuine("var").with_allows_unassigned(true);
    assert!(desc.allows_unassigned);
}

#[test]
fn test_variable_descriptor_piggyback() {
    use crate::domain::ShadowVariableKind;
    let desc = VariableDescriptor::piggyback("arrival_time", "departure_time");
    assert_eq!(
        desc.variable_type,
        VariableType::Shadow(ShadowVariableKind::Piggyback)
    );
    assert!(desc.allows_unassigned);
    assert_eq!(desc.source_variable, Some("departure_time"));
    assert!(desc.variable_type.is_shadow());
    assert!(!desc.variable_type.is_genuine());
}

#[test]
fn test_variable_descriptor_with_value_range_type() {
    let desc = VariableDescriptor::genuine("var")
        .with_value_range_type(ValueRangeType::CountableRange { from: 0, to: 100 });
    assert_eq!(
        desc.value_range_type,
        ValueRangeType::CountableRange { from: 0, to: 100 }
    );
}
