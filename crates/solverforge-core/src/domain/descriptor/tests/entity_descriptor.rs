use super::*;
use std::any::TypeId;

#[test]
fn test_with_extractor() {
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
fn test_entity_count() {
    let descriptor = create_test_entity_descriptor();

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
fn test_get_entity() {
    let descriptor = create_test_entity_descriptor();

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
fn test_get_entity_mut() {
    let descriptor = create_test_entity_descriptor();

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
fn test_entity_refs() {
    let descriptor = create_test_entity_descriptor();

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
fn test_for_each_entity() {
    let descriptor = create_test_entity_descriptor();

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
fn test_no_extractor() {
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
