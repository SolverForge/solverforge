use super::*;
use std::any::TypeId;

#[test]
fn test_total_entity_count() {
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
fn test_all_entity_refs() {
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
fn test_for_each_entity() {
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
fn test_get_entity() {
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
fn test_all_extractors_configured() {
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
fn test_counts() {
    let entity_desc = create_test_entity_descriptor();
    let fact_desc = ProblemFactDescriptor::new("SomeFact", TypeId::of::<i32>(), "facts");

    let solution_desc = SolutionDescriptor::new("TestSolution", TypeId::of::<TestSolution>())
        .with_entity(entity_desc)
        .with_problem_fact(fact_desc);

    assert_eq!(solution_desc.entity_descriptor_count(), 1);
    assert_eq!(solution_desc.problem_fact_descriptor_count(), 1);
}
