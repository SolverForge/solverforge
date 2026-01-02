//! Integration tests for the PlanningEntity derive macro.

use solverforge_core::domain::{FieldType, PlanningAnnotation, PrimitiveType};
use solverforge_core::{PlanningEntity, Value};
use solverforge_derive::PlanningEntity;

/// Simple entity with just an ID and one planning variable.
#[derive(PlanningEntity, Clone, Debug, PartialEq)]
struct SimpleLesson {
    #[planning_id]
    id: String,
    subject: String,
    #[planning_variable(value_range_provider = "rooms")]
    room: Option<String>,
}

/// Entity with multiple planning variables and different types.
#[derive(PlanningEntity, Clone, Debug, PartialEq)]
struct ComplexLesson {
    #[planning_id]
    id: String,
    subject: String,
    teacher: String,
    #[planning_variable(value_range_provider = "timeslots")]
    timeslot: Option<String>,
    #[planning_variable(value_range_provider = "rooms", allows_unassigned = true)]
    room: Option<String>,
}

/// Entity with numeric ID.
#[derive(PlanningEntity, Clone, Debug, PartialEq)]
struct NumericIdEntity {
    #[planning_id]
    id: i64,
    name: String,
    #[planning_variable(value_range_provider = "values")]
    value: Option<i64>,
}

// ============== domain_class() tests ==============

#[test]
fn test_simple_lesson_domain_class_name() {
    let class = SimpleLesson::domain_class();
    assert_eq!(class.name, "SimpleLesson");
}

#[test]
fn test_simple_lesson_is_planning_entity() {
    let class = SimpleLesson::domain_class();
    assert!(class.is_planning_entity());
}

#[test]
fn test_simple_lesson_has_planning_id_field() {
    let class = SimpleLesson::domain_class();
    let id_field = class.get_planning_id_field();
    assert!(id_field.is_some());
    assert_eq!(id_field.unwrap().name, "id");
}

#[test]
fn test_simple_lesson_has_planning_variable() {
    let class = SimpleLesson::domain_class();
    let vars: Vec<_> = class.get_planning_variables().collect();
    assert_eq!(vars.len(), 1);
    assert_eq!(vars[0].name, "room");
}

#[test]
fn test_simple_lesson_field_count() {
    let class = SimpleLesson::domain_class();
    assert_eq!(class.fields.len(), 3); // id, subject, room
}

#[test]
fn test_complex_lesson_multiple_planning_variables() {
    let class = ComplexLesson::domain_class();
    let vars: Vec<_> = class.get_planning_variables().collect();
    assert_eq!(vars.len(), 2); // timeslot, room
}

#[test]
fn test_complex_lesson_field_types() {
    let class = ComplexLesson::domain_class();

    // Check id field type (String)
    let id_field = class.fields.iter().find(|f| f.name == "id").unwrap();
    assert!(matches!(
        id_field.field_type,
        FieldType::Primitive(PrimitiveType::String)
    ));

    // Check timeslot field type (String from Option<String>)
    let timeslot_field = class.fields.iter().find(|f| f.name == "timeslot").unwrap();
    assert!(matches!(
        timeslot_field.field_type,
        FieldType::Primitive(PrimitiveType::String)
    ));
}

#[test]
fn test_numeric_id_entity_field_types() {
    let class = NumericIdEntity::domain_class();

    // Check id field type (Long from i64)
    let id_field = class.fields.iter().find(|f| f.name == "id").unwrap();
    assert!(matches!(
        id_field.field_type,
        FieldType::Primitive(PrimitiveType::Long)
    ));
}

// ============== planning_id() tests ==============

#[test]
fn test_simple_lesson_planning_id() {
    let lesson = SimpleLesson {
        id: "L1".to_string(),
        subject: "Math".to_string(),
        room: Some("R1".to_string()),
    };
    assert_eq!(lesson.planning_id(), Value::String("L1".to_string()));
}

#[test]
fn test_numeric_id_planning_id() {
    let entity = NumericIdEntity {
        id: 42,
        name: "Test".to_string(),
        value: Some(100),
    };
    assert_eq!(entity.planning_id(), Value::Int(42));
}

// ============== to_value() tests ==============

#[test]
fn test_simple_lesson_to_value() {
    let lesson = SimpleLesson {
        id: "L1".to_string(),
        subject: "Math".to_string(),
        room: Some("R1".to_string()),
    };

    let value = lesson.to_value();
    match value {
        Value::Object(map) => {
            assert_eq!(map.get("id"), Some(&Value::String("L1".to_string())));
            assert_eq!(map.get("subject"), Some(&Value::String("Math".to_string())));
            assert_eq!(map.get("room"), Some(&Value::String("R1".to_string())));
        }
        _ => panic!("Expected Object value"),
    }
}

#[test]
fn test_simple_lesson_to_value_with_null() {
    let lesson = SimpleLesson {
        id: "L2".to_string(),
        subject: "Science".to_string(),
        room: None,
    };

    let value = lesson.to_value();
    match value {
        Value::Object(map) => {
            assert_eq!(map.get("room"), Some(&Value::Null));
        }
        _ => panic!("Expected Object value"),
    }
}

#[test]
fn test_complex_lesson_to_value() {
    let lesson = ComplexLesson {
        id: "CL1".to_string(),
        subject: "Physics".to_string(),
        teacher: "Dr. Smith".to_string(),
        timeslot: Some("Morning".to_string()),
        room: None,
    };

    let value = lesson.to_value();
    match value {
        Value::Object(map) => {
            assert_eq!(map.get("id"), Some(&Value::String("CL1".to_string())));
            assert_eq!(
                map.get("subject"),
                Some(&Value::String("Physics".to_string()))
            );
            assert_eq!(
                map.get("teacher"),
                Some(&Value::String("Dr. Smith".to_string()))
            );
            assert_eq!(
                map.get("timeslot"),
                Some(&Value::String("Morning".to_string()))
            );
            assert_eq!(map.get("room"), Some(&Value::Null));
        }
        _ => panic!("Expected Object value"),
    }
}

// ============== from_value() tests ==============

#[test]
fn test_simple_lesson_from_value() {
    let mut map = std::collections::HashMap::new();
    map.insert("id".to_string(), Value::String("L1".to_string()));
    map.insert("subject".to_string(), Value::String("Math".to_string()));
    map.insert("room".to_string(), Value::String("R1".to_string()));

    let lesson = SimpleLesson::from_value(&Value::Object(map)).unwrap();
    assert_eq!(lesson.id, "L1");
    assert_eq!(lesson.subject, "Math");
    assert_eq!(lesson.room, Some("R1".to_string()));
}

#[test]
fn test_simple_lesson_from_value_with_null() {
    let mut map = std::collections::HashMap::new();
    map.insert("id".to_string(), Value::String("L2".to_string()));
    map.insert("subject".to_string(), Value::String("Science".to_string()));
    map.insert("room".to_string(), Value::Null);

    let lesson = SimpleLesson::from_value(&Value::Object(map)).unwrap();
    assert_eq!(lesson.id, "L2");
    assert_eq!(lesson.subject, "Science");
    assert_eq!(lesson.room, None);
}

#[test]
fn test_simple_lesson_from_value_missing_optional() {
    let mut map = std::collections::HashMap::new();
    map.insert("id".to_string(), Value::String("L3".to_string()));
    map.insert("subject".to_string(), Value::String("History".to_string()));
    // room is missing - should be None

    let lesson = SimpleLesson::from_value(&Value::Object(map)).unwrap();
    assert_eq!(lesson.room, None);
}

#[test]
fn test_simple_lesson_from_value_missing_required_field() {
    let mut map = std::collections::HashMap::new();
    map.insert("id".to_string(), Value::String("L4".to_string()));
    // subject is missing - should error

    let result = SimpleLesson::from_value(&Value::Object(map));
    assert!(result.is_err());
}

#[test]
fn test_simple_lesson_from_value_wrong_type() {
    let result = SimpleLesson::from_value(&Value::String("not an object".to_string()));
    assert!(result.is_err());
}

// ============== roundtrip tests ==============

#[test]
fn test_simple_lesson_roundtrip() {
    let original = SimpleLesson {
        id: "L1".to_string(),
        subject: "Math".to_string(),
        room: Some("R1".to_string()),
    };

    let value = original.to_value();
    let restored = SimpleLesson::from_value(&value).unwrap();
    assert_eq!(original, restored);
}

#[test]
fn test_simple_lesson_roundtrip_with_null() {
    let original = SimpleLesson {
        id: "L2".to_string(),
        subject: "Science".to_string(),
        room: None,
    };

    let value = original.to_value();
    let restored = SimpleLesson::from_value(&value).unwrap();
    assert_eq!(original, restored);
}

#[test]
fn test_complex_lesson_roundtrip() {
    let original = ComplexLesson {
        id: "CL1".to_string(),
        subject: "Physics".to_string(),
        teacher: "Dr. Smith".to_string(),
        timeslot: Some("Morning".to_string()),
        room: Some("Lab1".to_string()),
    };

    let value = original.to_value();
    let restored = ComplexLesson::from_value(&value).unwrap();
    assert_eq!(original, restored);
}

// ============== planning variable attribute tests ==============

#[test]
fn test_planning_variable_value_range_provider() {
    let class = SimpleLesson::domain_class();
    let room_field = class.fields.iter().find(|f| f.name == "room").unwrap();

    let has_variable_annotation = room_field.annotations.iter().any(|ann| {
        matches!(
            ann,
            PlanningAnnotation::PlanningVariable {
                value_range_provider_refs,
                ..
            } if value_range_provider_refs.contains(&"rooms".to_string())
        )
    });
    assert!(has_variable_annotation);
}

#[test]
fn test_planning_variable_allows_unassigned_false() {
    let class = ComplexLesson::domain_class();
    let timeslot_field = class.fields.iter().find(|f| f.name == "timeslot").unwrap();

    let allows_unassigned = timeslot_field.annotations.iter().any(|ann| {
        matches!(
            ann,
            PlanningAnnotation::PlanningVariable {
                allows_unassigned: false,
                ..
            }
        )
    });
    assert!(allows_unassigned);
}

#[test]
fn test_planning_variable_allows_unassigned_true() {
    let class = ComplexLesson::domain_class();
    let room_field = class.fields.iter().find(|f| f.name == "room").unwrap();

    let allows_unassigned = room_field.annotations.iter().any(|ann| {
        matches!(
            ann,
            PlanningAnnotation::PlanningVariable {
                allows_unassigned: true,
                ..
            }
        )
    });
    assert!(allows_unassigned);
}
