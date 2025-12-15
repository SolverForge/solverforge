//! Integration tests for the PlanningSolution derive macro.

use serde::{Deserialize, Serialize};
use solverforge_core::constraints::{Constraint, ConstraintSet};
use solverforge_core::domain::{FieldType, PlanningAnnotation, ScoreType};
use solverforge_core::{HardSoftScore, PlanningSolution, Value};
use solverforge_derive::{PlanningEntity, PlanningSolution};

// ============== Test entity for use in solutions ==============

/// A simple room (problem fact).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct Room {
    id: String,
    name: String,
    capacity: i64,
}

impl From<Room> for Value {
    fn from(room: Room) -> Self {
        let mut map = std::collections::HashMap::new();
        map.insert("id".to_string(), Value::String(room.id));
        map.insert("name".to_string(), Value::String(room.name));
        map.insert("capacity".to_string(), Value::Int(room.capacity));
        Value::Object(map)
    }
}

/// A timeslot (problem fact).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct Timeslot {
    id: String,
    day_of_week: String,
    start_time: String,
    end_time: String,
}

impl From<Timeslot> for Value {
    fn from(ts: Timeslot) -> Self {
        let mut map = std::collections::HashMap::new();
        map.insert("id".to_string(), Value::String(ts.id));
        map.insert("day_of_week".to_string(), Value::String(ts.day_of_week));
        map.insert("start_time".to_string(), Value::String(ts.start_time));
        map.insert("end_time".to_string(), Value::String(ts.end_time));
        Value::Object(map)
    }
}

/// A lesson - planning entity.
#[derive(PlanningEntity, Clone, Debug, PartialEq)]
struct Lesson {
    #[planning_id]
    id: String,
    subject: String,
    teacher: String,
    #[planning_variable(value_range_provider = "timeslots")]
    timeslot: Option<String>,
    #[planning_variable(value_range_provider = "rooms", allows_unassigned = true)]
    room: Option<String>,
}

// ============== Constraint provider function ==============

fn define_constraints() -> ConstraintSet {
    ConstraintSet::new()
        .with_constraint(Constraint::new("Room conflict"))
        .with_constraint(Constraint::new("Teacher conflict"))
}

// ============== Simple solution with just score ==============

#[derive(PlanningSolution, Clone, Debug)]
struct MinimalSolution {
    #[planning_entity_collection]
    lessons: Vec<Lesson>,
    #[planning_score]
    score: Option<HardSoftScore>,
}

// ============== Full timetable solution ==============

#[derive(PlanningSolution, Clone, Debug)]
#[constraint_provider = "define_constraints"]
struct Timetable {
    #[problem_fact_collection]
    #[value_range_provider(id = "timeslots")]
    timeslots: Vec<Timeslot>,
    #[problem_fact_collection]
    #[value_range_provider(id = "rooms")]
    rooms: Vec<Room>,
    #[planning_entity_collection]
    lessons: Vec<Lesson>,
    #[planning_score]
    score: Option<HardSoftScore>,
}

// ============== domain_model() tests ==============

#[test]
fn test_minimal_solution_domain_model() {
    let model = MinimalSolution::domain_model();
    assert!(model.get_solution_class().is_some());
    assert_eq!(model.get_solution_class().unwrap().name, "MinimalSolution");
}

#[test]
fn test_minimal_solution_is_planning_solution() {
    let model = MinimalSolution::domain_model();
    let solution_class = model.get_solution_class().unwrap();
    assert!(solution_class.is_planning_solution());
}

#[test]
fn test_timetable_domain_model_name() {
    let model = Timetable::domain_model();
    assert_eq!(model.get_solution_class().unwrap().name, "Timetable");
}

#[test]
fn test_timetable_has_entity_classes() {
    let model = Timetable::domain_model();
    let entities: Vec<_> = model.get_entity_classes().collect();
    assert_eq!(entities.len(), 1);
    assert_eq!(entities[0].name, "Lesson");
}

#[test]
fn test_timetable_entity_is_planning_entity() {
    let model = Timetable::domain_model();
    let lesson_class = model.get_class("Lesson").unwrap();
    assert!(lesson_class.is_planning_entity());
}

#[test]
fn test_timetable_solution_has_score_field() {
    let model = Timetable::domain_model();
    let solution_class = model.get_solution_class().unwrap();
    let score_field = solution_class.get_score_field();
    assert!(score_field.is_some());
    assert_eq!(score_field.unwrap().name, "score");
}

#[test]
fn test_timetable_score_field_type() {
    let model = Timetable::domain_model();
    let solution_class = model.get_solution_class().unwrap();
    let score_field = solution_class.get_score_field().unwrap();
    assert!(matches!(
        score_field.field_type,
        FieldType::Score(ScoreType::HardSoft)
    ));
}

#[test]
fn test_timetable_has_value_range_providers() {
    let model = Timetable::domain_model();
    let solution_class = model.get_solution_class().unwrap();

    // Check timeslots field has value range provider
    let timeslots_field = solution_class
        .fields
        .iter()
        .find(|f| f.name == "timeslots")
        .unwrap();
    let has_vrp = timeslots_field.planning_annotations.iter().any(|ann| {
        matches!(
            ann,
            PlanningAnnotation::ValueRangeProvider { id } if id == &Some("timeslots".to_string())
        )
    });
    assert!(has_vrp);

    // Check rooms field has value range provider
    let rooms_field = solution_class
        .fields
        .iter()
        .find(|f| f.name == "rooms")
        .unwrap();
    let has_vrp = rooms_field.planning_annotations.iter().any(|ann| {
        matches!(
            ann,
            PlanningAnnotation::ValueRangeProvider { id } if id == &Some("rooms".to_string())
        )
    });
    assert!(has_vrp);
}

#[test]
fn test_timetable_has_problem_fact_collection() {
    let model = Timetable::domain_model();
    let solution_class = model.get_solution_class().unwrap();

    let timeslots_field = solution_class
        .fields
        .iter()
        .find(|f| f.name == "timeslots")
        .unwrap();
    let has_pfc = timeslots_field
        .planning_annotations
        .iter()
        .any(|ann| matches!(ann, PlanningAnnotation::ProblemFactCollectionProperty));
    assert!(has_pfc);
}

#[test]
fn test_timetable_has_planning_entity_collection() {
    let model = Timetable::domain_model();
    let solution_class = model.get_solution_class().unwrap();

    let lessons_field = solution_class
        .fields
        .iter()
        .find(|f| f.name == "lessons")
        .unwrap();
    let has_pec = lessons_field
        .planning_annotations
        .iter()
        .any(|ann| matches!(ann, PlanningAnnotation::PlanningEntityCollectionProperty));
    assert!(has_pec);
}

// ============== constraints() tests ==============

#[test]
fn test_minimal_solution_constraints_empty() {
    let constraints = MinimalSolution::constraints();
    assert!(constraints.is_empty());
}

#[test]
fn test_timetable_constraints_from_provider() {
    let constraints = Timetable::constraints();
    assert_eq!(constraints.len(), 2);

    let names: Vec<_> = constraints.iter().map(|c| c.name.as_str()).collect();
    assert!(names.contains(&"Room conflict"));
    assert!(names.contains(&"Teacher conflict"));
}

// ============== score() and set_score() tests ==============

#[test]
fn test_minimal_solution_score_initially_none() {
    let solution = MinimalSolution {
        lessons: vec![],
        score: None,
    };
    assert!(solution.score().is_none());
}

#[test]
fn test_minimal_solution_set_score() {
    let mut solution = MinimalSolution {
        lessons: vec![],
        score: None,
    };

    solution.set_score(HardSoftScore::of(-1, -5));
    assert_eq!(solution.score(), Some(HardSoftScore::of(-1, -5)));
}

#[test]
fn test_timetable_score() {
    let mut timetable = Timetable {
        timeslots: vec![],
        rooms: vec![],
        lessons: vec![],
        score: None,
    };

    assert!(timetable.score().is_none());

    timetable.set_score(HardSoftScore::of(0, -10));
    assert_eq!(timetable.score(), Some(HardSoftScore::of(0, -10)));
}

#[test]
fn test_score_type_is_hard_soft() {
    let score = HardSoftScore::of(-1, -5);
    let mut solution = MinimalSolution {
        lessons: vec![],
        score: None,
    };
    solution.set_score(score);
    assert_eq!(solution.score().unwrap().hard_score, -1);
    assert_eq!(solution.score().unwrap().soft_score, -5);
}

// ============== to_json() tests ==============

#[test]
fn test_minimal_solution_to_json() {
    let lesson = Lesson {
        id: "L1".to_string(),
        subject: "Math".to_string(),
        teacher: "Dr. Smith".to_string(),
        timeslot: Some("T1".to_string()),
        room: Some("R1".to_string()),
    };

    let solution = MinimalSolution {
        lessons: vec![lesson],
        score: None,
    };

    let json = solution.to_json().unwrap();
    assert!(json.contains("lessons"));
    assert!(json.contains("L1"));
    assert!(json.contains("Math"));
}

#[test]
fn test_timetable_to_json() {
    let timetable = Timetable {
        timeslots: vec![Timeslot {
            id: "T1".to_string(),
            day_of_week: "Monday".to_string(),
            start_time: "09:00".to_string(),
            end_time: "10:00".to_string(),
        }],
        rooms: vec![Room {
            id: "R1".to_string(),
            name: "Room 101".to_string(),
            capacity: 30,
        }],
        lessons: vec![Lesson {
            id: "L1".to_string(),
            subject: "Math".to_string(),
            teacher: "Dr. Smith".to_string(),
            timeslot: Some("T1".to_string()),
            room: Some("R1".to_string()),
        }],
        score: Some(HardSoftScore::of(0, -5)),
    };

    let json = timetable.to_json().unwrap();
    assert!(json.contains("timeslots"));
    assert!(json.contains("Monday"));
    assert!(json.contains("rooms"));
    assert!(json.contains("Room 101"));
    assert!(json.contains("lessons"));
    assert!(json.contains("L1"));
    // Score is serialized as string
    assert!(json.contains("score"));
}

#[test]
fn test_solution_to_json_with_unassigned_variables() {
    let solution = MinimalSolution {
        lessons: vec![Lesson {
            id: "L1".to_string(),
            subject: "Math".to_string(),
            teacher: "Dr. Smith".to_string(),
            timeslot: None,
            room: None,
        }],
        score: None,
    };

    let json = solution.to_json().unwrap();
    // Unassigned variables should be null
    assert!(json.contains("null"));
}

// ============== from_json() tests ==============

#[test]
fn test_minimal_solution_from_json() {
    let json = r#"{
        "lessons": [
            {"id": "L1", "subject": "Math", "teacher": "Dr. Smith", "timeslot": "T1", "room": "R1"}
        ]
    }"#;

    let solution = MinimalSolution::from_json(json).unwrap();
    assert_eq!(solution.lessons.len(), 1);
    assert_eq!(solution.lessons[0].id, "L1");
    assert_eq!(solution.lessons[0].subject, "Math");
    assert!(solution.score.is_none());
}

#[test]
fn test_minimal_solution_from_json_with_null_variables() {
    let json = r#"{
        "lessons": [
            {"id": "L1", "subject": "Math", "teacher": "Dr. Smith", "timeslot": null, "room": null}
        ]
    }"#;

    let solution = MinimalSolution::from_json(json).unwrap();
    assert_eq!(solution.lessons.len(), 1);
    assert!(solution.lessons[0].timeslot.is_none());
    assert!(solution.lessons[0].room.is_none());
}

#[test]
fn test_timetable_from_json() {
    let json = r#"{
        "timeslots": [
            {"id": "T1", "day_of_week": "Monday", "start_time": "09:00", "end_time": "10:00"}
        ],
        "rooms": [
            {"id": "R1", "name": "Room 101", "capacity": 30}
        ],
        "lessons": [
            {"id": "L1", "subject": "Math", "teacher": "Dr. Smith", "timeslot": "T1", "room": "R1"}
        ]
    }"#;

    let timetable = Timetable::from_json(json).unwrap();
    assert_eq!(timetable.timeslots.len(), 1);
    assert_eq!(timetable.timeslots[0].id, "T1");
    assert_eq!(timetable.rooms.len(), 1);
    assert_eq!(timetable.rooms[0].name, "Room 101");
    assert_eq!(timetable.lessons.len(), 1);
    assert!(timetable.score.is_none());
}

#[test]
fn test_from_json_empty_collections() {
    let json = r#"{
        "timeslots": [],
        "rooms": [],
        "lessons": []
    }"#;

    let timetable = Timetable::from_json(json).unwrap();
    assert!(timetable.timeslots.is_empty());
    assert!(timetable.rooms.is_empty());
    assert!(timetable.lessons.is_empty());
}

#[test]
fn test_from_json_invalid_json() {
    let json = "not valid json";
    let result = MinimalSolution::from_json(json);
    assert!(result.is_err());
}

#[test]
fn test_from_json_wrong_type() {
    let json = r#""just a string""#;
    let result = MinimalSolution::from_json(json);
    assert!(result.is_err());
}

// ============== roundtrip tests ==============

#[test]
fn test_minimal_solution_roundtrip() {
    let original = MinimalSolution {
        lessons: vec![Lesson {
            id: "L1".to_string(),
            subject: "Math".to_string(),
            teacher: "Dr. Smith".to_string(),
            timeslot: Some("T1".to_string()),
            room: Some("R1".to_string()),
        }],
        score: None,
    };

    let json = original.to_json().unwrap();
    let restored = MinimalSolution::from_json(&json).unwrap();

    assert_eq!(original.lessons.len(), restored.lessons.len());
    assert_eq!(original.lessons[0], restored.lessons[0]);
}

#[test]
fn test_timetable_roundtrip() {
    let original = Timetable {
        timeslots: vec![Timeslot {
            id: "T1".to_string(),
            day_of_week: "Monday".to_string(),
            start_time: "09:00".to_string(),
            end_time: "10:00".to_string(),
        }],
        rooms: vec![Room {
            id: "R1".to_string(),
            name: "Room 101".to_string(),
            capacity: 30,
        }],
        lessons: vec![Lesson {
            id: "L1".to_string(),
            subject: "Math".to_string(),
            teacher: "Dr. Smith".to_string(),
            timeslot: Some("T1".to_string()),
            room: Some("R1".to_string()),
        }],
        score: None, // Score is reset in from_json
    };

    let json = original.to_json().unwrap();
    let restored = Timetable::from_json(&json).unwrap();

    assert_eq!(original.timeslots.len(), restored.timeslots.len());
    assert_eq!(original.rooms.len(), restored.rooms.len());
    assert_eq!(original.lessons.len(), restored.lessons.len());
    assert_eq!(original.lessons[0], restored.lessons[0]);
}

// ============== domain_model validation tests ==============

#[test]
fn test_domain_model_validates() {
    let model = Timetable::domain_model();
    // The model should have all required components
    assert!(model.get_solution_class().is_some());
    assert!(model.get_class("Lesson").is_some());
    // Validation should pass
    assert!(model.validate().is_ok());
}

#[test]
fn test_entity_has_planning_variables() {
    let model = Timetable::domain_model();
    let lesson_class = model.get_class("Lesson").unwrap();
    let vars: Vec<_> = lesson_class.get_planning_variables().collect();
    assert_eq!(vars.len(), 2); // timeslot and room
}

#[test]
fn test_entity_has_planning_id() {
    let model = Timetable::domain_model();
    let lesson_class = model.get_class("Lesson").unwrap();
    let id_field = lesson_class.get_planning_id_field();
    assert!(id_field.is_some());
    assert_eq!(id_field.unwrap().name, "id");
}

// ============== multiple entities test ==============

/// Another planning entity type.
#[derive(PlanningEntity, Clone, Debug, PartialEq)]
struct Task {
    #[planning_id]
    id: String,
    name: String,
    #[planning_variable(value_range_provider = "employees")]
    assigned_to: Option<String>,
}

#[derive(PlanningSolution, Clone, Debug)]
struct TaskSchedule {
    #[planning_entity_collection]
    tasks: Vec<Task>,
    #[planning_entity_collection]
    lessons: Vec<Lesson>,
    #[planning_score]
    score: Option<HardSoftScore>,
}

#[test]
fn test_multiple_entity_collections() {
    let model = TaskSchedule::domain_model();

    // Should have two entity classes
    let entities: Vec<_> = model.get_entity_classes().collect();
    assert_eq!(entities.len(), 2);

    let names: Vec<_> = entities.iter().map(|e| e.name.as_str()).collect();
    assert!(names.contains(&"Task"));
    assert!(names.contains(&"Lesson"));
}
