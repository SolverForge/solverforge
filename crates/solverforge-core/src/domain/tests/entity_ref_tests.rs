use std::any::Any;

use crate::domain::{EntityExtractor, TypedEntityExtractor};

#[derive(Clone, Debug)]
struct TestEntity {
    id: i64,
    value: Option<i32>,
}

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
fn test_typed_entity_extractor_count() {
    let extractor =
        TypedEntityExtractor::new("TestEntity", "entities", get_entities, get_entities_mut);

    let solution = TestSolution {
        entities: vec![
            TestEntity {
                id: 1,
                value: Some(10),
            },
            TestEntity {
                id: 2,
                value: Some(20),
            },
            TestEntity { id: 3, value: None },
        ],
    };

    let count = extractor.count(&solution as &dyn Any);
    assert_eq!(count, Some(3));
}

#[test]
fn test_typed_entity_extractor_get() {
    let extractor =
        TypedEntityExtractor::new("TestEntity", "entities", get_entities, get_entities_mut);

    let solution = TestSolution {
        entities: vec![
            TestEntity {
                id: 1,
                value: Some(10),
            },
            TestEntity {
                id: 2,
                value: Some(20),
            },
        ],
    };

    let entity = extractor.get(&solution as &dyn Any, 0);
    assert!(entity.is_some());
    let entity = entity.unwrap().downcast_ref::<TestEntity>().unwrap();
    assert_eq!(entity.id, 1);
    assert_eq!(entity.value, Some(10));

    // Out of bounds
    let entity = extractor.get(&solution as &dyn Any, 5);
    assert!(entity.is_none());
}

#[test]
fn test_typed_entity_extractor_get_mut() {
    let extractor =
        TypedEntityExtractor::new("TestEntity", "entities", get_entities, get_entities_mut);

    let mut solution = TestSolution {
        entities: vec![TestEntity {
            id: 1,
            value: Some(10),
        }],
    };

    let entity = extractor.get_mut(&mut solution as &mut dyn Any, 0);
    assert!(entity.is_some());
    let entity = entity.unwrap().downcast_mut::<TestEntity>().unwrap();
    entity.value = Some(100);

    assert_eq!(solution.entities[0].value, Some(100));
}

#[test]
fn test_typed_entity_extractor_entity_refs() {
    let extractor =
        TypedEntityExtractor::new("TestEntity", "entities", get_entities, get_entities_mut);

    let solution = TestSolution {
        entities: vec![
            TestEntity {
                id: 1,
                value: Some(10),
            },
            TestEntity {
                id: 2,
                value: Some(20),
            },
        ],
    };

    let refs = extractor.entity_refs(&solution as &dyn Any);
    assert_eq!(refs.len(), 2);
    assert_eq!(refs[0].index, 0);
    assert_eq!(refs[0].type_name, "TestEntity");
    assert_eq!(refs[0].collection_field, "entities");
    assert_eq!(refs[1].index, 1);
}

#[test]
fn test_extractor_wrong_solution_type() {
    let extractor =
        TypedEntityExtractor::new("TestEntity", "entities", get_entities, get_entities_mut);

    let wrong_solution = "not a solution";
    let count = extractor.count(&wrong_solution as &dyn Any);
    assert!(count.is_none());
}

#[test]
fn test_extractor_clone() {
    let extractor: Box<dyn EntityExtractor> = Box::new(TypedEntityExtractor::new(
        "TestEntity",
        "entities",
        get_entities,
        get_entities_mut,
    ));

    let cloned = extractor.clone();

    let solution = TestSolution {
        entities: vec![TestEntity {
            id: 1,
            value: Some(10),
        }],
    };

    assert_eq!(cloned.count(&solution as &dyn Any), Some(1));
}

#[test]
fn test_clone_entity_boxed() {
    let extractor =
        TypedEntityExtractor::new("TestEntity", "entities", get_entities, get_entities_mut);

    let solution = TestSolution {
        entities: vec![
            TestEntity {
                id: 1,
                value: Some(10),
            },
            TestEntity {
                id: 2,
                value: Some(20),
            },
        ],
    };

    // Clone first entity
    let boxed = extractor.clone_entity_boxed(&solution as &dyn Any, 0);
    assert!(boxed.is_some());
    let boxed_entity = boxed.unwrap();
    let entity = boxed_entity.downcast_ref::<TestEntity>().unwrap();
    assert_eq!(entity.id, 1);
    assert_eq!(entity.value, Some(10));

    // Clone second entity
    let boxed = extractor.clone_entity_boxed(&solution as &dyn Any, 1);
    assert!(boxed.is_some());
    let boxed_entity = boxed.unwrap();
    let entity = boxed_entity.downcast_ref::<TestEntity>().unwrap();
    assert_eq!(entity.id, 2);
    assert_eq!(entity.value, Some(20));

    // Out of bounds returns None
    let boxed = extractor.clone_entity_boxed(&solution as &dyn Any, 5);
    assert!(boxed.is_none());
}

#[test]
fn test_entity_type_id() {
    let extractor =
        TypedEntityExtractor::new("TestEntity", "entities", get_entities, get_entities_mut);

    assert_eq!(
        extractor.entity_type_id(),
        std::any::TypeId::of::<TestEntity>()
    );
}
