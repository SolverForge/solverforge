// Tests for score analysis types.

use crate::api::analysis::*;

#[derive(Clone, Debug, PartialEq)]
struct TestEntity {
    id: i32,
    name: String,
}

#[test]
fn test_entity_ref_creation() {
    let entity = TestEntity {
        id: 1,
        name: "Test".to_string(),
    };
    let entity_ref = EntityRef::new(&entity);

    assert!(entity_ref.type_name.contains("TestEntity"));
    assert!(entity_ref.display.contains("Test"));
    assert!(entity_ref.display.contains("1"));
}

/// Explaining a match needs only `Debug`: the entity type is neither `Clone`
/// nor `'static`, and the reference keeps no copy of it.
#[test]
fn test_entity_ref_describes_a_borrowed_debug_only_entity() {
    #[derive(Debug)]
    struct Borrowed<'a> {
        #[allow(dead_code)]
        name: &'a str,
    }
    let name = String::from("Answer");
    let entity_ref = EntityRef::new(&Borrowed { name: &name });

    assert_eq!(entity_ref.short_type_name(), "Borrowed<'_>");
    assert_eq!(entity_ref.display, r#"Borrowed { name: "Answer" }"#);
}

#[test]
fn test_entity_ref_equality() {
    let entity1 = TestEntity {
        id: 1,
        name: "Test".to_string(),
    };
    let entity2 = TestEntity {
        id: 1,
        name: "Test".to_string(),
    };
    let entity3 = TestEntity {
        id: 2,
        name: "Other".to_string(),
    };

    let ref1 = EntityRef::new(&entity1);
    let ref2 = EntityRef::new(&entity2);
    let ref3 = EntityRef::new(&entity3);

    assert_eq!(ref1, ref2);
    assert_ne!(ref1, ref3);
}

#[test]
fn test_constraint_justification() {
    let entity1 = TestEntity {
        id: 1,
        name: "Alice".to_string(),
    };
    let entity2 = TestEntity {
        id: 2,
        name: "Bob".to_string(),
    };

    let just =
        ConstraintJustification::new(vec![EntityRef::new(&entity1), EntityRef::new(&entity2)]);

    assert_eq!(just.entities.len(), 2);
    assert!(just.description.contains("Alice"));
    assert!(just.description.contains("Bob"));
}
