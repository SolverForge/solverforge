use super::*;
use std::any::Any;

#[test]
fn test_entity_creation() {
    let e1 = TestEntity::new(1, Some(10));
    assert_eq!(e1.id, 1);
    assert_eq!(e1.value, Some(10));

    let e2 = TestEntity::assigned(2, 20);
    assert_eq!(e2.id, 2);
    assert_eq!(e2.value, Some(20));

    let e3 = TestEntity::unassigned(3);
    assert_eq!(e3.id, 3);
    assert_eq!(e3.value, None);
}

#[test]
fn test_solution_creation() {
    let empty = TestSolution::empty();
    assert!(empty.entities.is_empty());

    let with_entities = TestSolution::with_entities(vec![
        TestEntity::assigned(1, 10),
        TestEntity::assigned(2, 20),
    ]);
    assert_eq!(with_entities.entities.len(), 2);
}

#[test]
fn test_extractor_creation() {
    let extractor = create_test_entity_extractor();
    let solution = TestSolution::with_entities(vec![
        TestEntity::assigned(1, 10),
        TestEntity::assigned(2, 20),
    ]);

    assert_eq!(extractor.count(&solution as &dyn Any), Some(2));
}

#[test]
fn test_get_set_value() {
    let mut solution =
        TestSolution::with_entities(vec![TestEntity::unassigned(1), TestEntity::unassigned(2)]);

    assert_eq!(get_entity_value(&solution, 0), None);
    set_entity_value(&mut solution, 0, Some(42));
    assert_eq!(get_entity_value(&solution, 0), Some(42));
}
