//! End-to-end integration tests for SolverBuilder with derive macros.
//!
//! These tests verify the complete workflow from domain definition using
//! derive macros through to solver construction and WASM generation.

use serde::{Deserialize, Serialize};
use solverforge_core::constraints::{Constraint, ConstraintSet, StreamComponent, WasmFunction};
use solverforge_core::domain::PlanningAnnotation;
use solverforge_core::{
    EnvironmentMode, FunctionHandle, HardSoftScore, LanguageBridge, ObjectHandle, PlanningEntity,
    PlanningSolution, SolverBuilder, SolverForgeResult, TerminationConfig, Value,
};
use solverforge_derive::{PlanningEntity, PlanningSolution};

/// A minimal mock bridge for testing SolverBuilder without a running service.
struct TestBridge;

impl LanguageBridge for TestBridge {
    fn call_function(&self, _func: FunctionHandle, _args: &[Value]) -> SolverForgeResult<Value> {
        Ok(Value::Null)
    }

    fn get_field(&self, _obj: ObjectHandle, _field: &str) -> SolverForgeResult<Value> {
        Ok(Value::Null)
    }

    fn set_field(&self, _obj: ObjectHandle, _field: &str, _value: Value) -> SolverForgeResult<()> {
        Ok(())
    }

    fn serialize_object(&self, _obj: ObjectHandle) -> SolverForgeResult<String> {
        Ok("{}".to_string())
    }

    fn deserialize_object(
        &self,
        _json: &str,
        _class_name: &str,
    ) -> SolverForgeResult<ObjectHandle> {
        Ok(ObjectHandle::new(0))
    }

    fn get_class_info(&self, _obj: ObjectHandle) -> SolverForgeResult<solverforge_core::ClassInfo> {
        Ok(solverforge_core::ClassInfo::new("TestClass"))
    }

    fn register_function(&self, _func: ObjectHandle) -> SolverForgeResult<FunctionHandle> {
        Ok(FunctionHandle::new(0))
    }

    fn clone_object(&self, _obj: ObjectHandle) -> SolverForgeResult<ObjectHandle> {
        Ok(ObjectHandle::new(0))
    }

    fn get_list_size(&self, _obj: ObjectHandle) -> SolverForgeResult<usize> {
        Ok(0)
    }

    fn get_list_item(&self, _obj: ObjectHandle, _index: usize) -> SolverForgeResult<Value> {
        Ok(Value::Null)
    }
}

// ============================================================================
// TIMETABLING DOMAIN
// ============================================================================

/// A timeslot for scheduling lessons.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct Timeslot {
    id: String,
    day_of_week: String,
    start_time: String,
    end_time: String,
}

/// A room where lessons can be held.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct Room {
    id: String,
    name: String,
    capacity: i32,
}

/// A lesson that needs to be scheduled.
#[derive(PlanningEntity, Clone, Debug, PartialEq)]
struct Lesson {
    #[planning_id]
    id: String,
    subject: String,
    teacher: String,
    student_group: String,
    #[planning_variable(value_range_provider = "timeslots")]
    timeslot: Option<String>,
    #[planning_variable(value_range_provider = "rooms")]
    room: Option<String>,
}

/// The timetable solution containing all scheduling data.
#[derive(PlanningSolution, Clone, Debug)]
#[constraint_provider = "define_timetable_constraints"]
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

fn define_timetable_constraints() -> ConstraintSet {
    ConstraintSet::new()
        .with_constraint(
            Constraint::new("Room conflict")
                .with_component(StreamComponent::for_each_unique_pair("Lesson"))
                .with_component(StreamComponent::filter(WasmFunction::new("roomConflict")))
                .with_component(StreamComponent::penalize("1hard")),
        )
        .with_constraint(
            Constraint::new("Teacher conflict")
                .with_component(StreamComponent::for_each_unique_pair("Lesson"))
                .with_component(StreamComponent::filter(WasmFunction::new(
                    "teacherConflict",
                )))
                .with_component(StreamComponent::penalize("1hard")),
        )
        .with_constraint(
            Constraint::new("Student group conflict")
                .with_component(StreamComponent::for_each_unique_pair("Lesson"))
                .with_component(StreamComponent::filter(WasmFunction::new(
                    "studentGroupConflict",
                )))
                .with_component(StreamComponent::penalize("1hard")),
        )
}

// ============================================================================
// EMPLOYEE SCHEDULING DOMAIN
// ============================================================================

/// An employee who can be assigned to shifts.
#[derive(PlanningEntity, Clone, Debug, PartialEq)]
struct Shift {
    #[planning_id]
    id: String,
    start_time: String,
    end_time: String,
    location: String,
    required_skill: String,
    #[planning_variable(value_range_provider = "employees", allows_unassigned = true)]
    employee: Option<String>,
}

/// Employee availability constraint.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct Availability {
    employee_id: String,
    date: String,
    availability_type: String, // "AVAILABLE", "UNAVAILABLE", "PREFERRED"
}

/// The employee schedule solution.
#[derive(PlanningSolution, Clone, Debug)]
#[constraint_provider = "define_employee_constraints"]
struct EmployeeSchedule {
    #[problem_fact_collection]
    #[value_range_provider(id = "employees")]
    employees: Vec<String>,
    #[problem_fact_collection]
    availabilities: Vec<Availability>,
    #[planning_entity_collection]
    shifts: Vec<Shift>,
    #[planning_score]
    score: Option<HardSoftScore>,
}

fn define_employee_constraints() -> ConstraintSet {
    ConstraintSet::new()
        .with_constraint(
            Constraint::new("Required skill")
                .with_component(StreamComponent::for_each("Shift"))
                .with_component(StreamComponent::filter(WasmFunction::new(
                    "hasRequiredSkill",
                )))
                .with_component(StreamComponent::penalize("1hard")),
        )
        .with_constraint(
            Constraint::new("Overlapping shifts")
                .with_component(StreamComponent::for_each_unique_pair("Shift"))
                .with_component(StreamComponent::filter(WasmFunction::new("overlaps")))
                .with_component(StreamComponent::penalize("1hard")),
        )
        .with_constraint(
            Constraint::new("Unavailable employee")
                .with_component(StreamComponent::for_each("Shift"))
                .with_component(StreamComponent::filter(WasmFunction::new("isUnavailable")))
                .with_component(StreamComponent::penalize("1hard")),
        )
        .with_constraint(
            Constraint::new("Preferred availability")
                .with_component(StreamComponent::for_each("Shift"))
                .with_component(StreamComponent::filter(WasmFunction::new("isPreferred")))
                .with_component(StreamComponent::reward("1soft")),
        )
}

// ============================================================================
// TIMETABLING TESTS
// ============================================================================

#[test]
fn test_timetable_solver_builder_creation() {
    let solver = SolverBuilder::<Timetable>::new()
        .with_termination(TerminationConfig::new().with_spent_limit("PT5M"))
        .build::<TestBridge>()
        .unwrap();

    assert_eq!(
        solver.config().solution_class,
        Some("Timetable".to_string())
    );
}

#[test]
fn test_timetable_solver_builder_domain_model() {
    let model = SolverBuilder::<Timetable>::domain_model();

    // Verify solution class
    let solution = model.get_solution_class().unwrap();
    assert_eq!(solution.name, "Timetable");
    assert!(solution.is_planning_solution());

    // Verify entity class
    let lesson = model.get_class("Lesson").unwrap();
    assert!(lesson.is_planning_entity());
    assert_eq!(lesson.get_planning_variables().count(), 2); // timeslot, room
}

#[test]
fn test_timetable_solver_builder_constraints() {
    let constraints = SolverBuilder::<Timetable>::constraints();

    assert_eq!(constraints.len(), 3);
    let names: Vec<&str> = constraints.iter().map(|c| c.name.as_str()).collect();
    assert!(names.contains(&"Room conflict"));
    assert!(names.contains(&"Teacher conflict"));
    assert!(names.contains(&"Student group conflict"));
}

#[test]
fn test_timetable_solver_builder_wasm_generation() {
    let solver = SolverBuilder::<Timetable>::new()
        .build::<TestBridge>()
        .unwrap();

    let wasm = solver.wasm_module();
    assert!(wasm.starts_with("AGFzbQ")); // Base64 of "\0asm"
    assert!(wasm.len() > 100); // Should have substantial content
}

#[test]
fn test_timetable_solver_builder_full_configuration() {
    let solver = SolverBuilder::<Timetable>::new()
        .with_service_url("http://solver.example.com:8080")
        .with_termination(
            TerminationConfig::new()
                .with_spent_limit("PT10M")
                .with_best_score_feasible(true),
        )
        .with_environment_mode(EnvironmentMode::Reproducible)
        .with_random_seed(42)
        .build::<TestBridge>()
        .unwrap();

    let config = solver.config();
    assert_eq!(config.environment_mode, Some(EnvironmentMode::Reproducible));
    assert_eq!(config.random_seed, Some(42));
    assert!(config.termination.is_some());
    assert_eq!(solver.service_url(), "http://solver.example.com:8080");
}

#[test]
fn test_timetable_problem_creation_and_serialization() {
    let timetable = Timetable {
        timeslots: vec![
            Timeslot {
                id: "ts1".to_string(),
                day_of_week: "MONDAY".to_string(),
                start_time: "08:30".to_string(),
                end_time: "09:30".to_string(),
            },
            Timeslot {
                id: "ts2".to_string(),
                day_of_week: "MONDAY".to_string(),
                start_time: "09:30".to_string(),
                end_time: "10:30".to_string(),
            },
        ],
        rooms: vec![
            Room {
                id: "r1".to_string(),
                name: "Room A".to_string(),
                capacity: 30,
            },
            Room {
                id: "r2".to_string(),
                name: "Room B".to_string(),
                capacity: 20,
            },
        ],
        lessons: vec![
            Lesson {
                id: "l1".to_string(),
                subject: "Math".to_string(),
                teacher: "A. Turing".to_string(),
                student_group: "9th grade".to_string(),
                timeslot: None,
                room: None,
            },
            Lesson {
                id: "l2".to_string(),
                subject: "Physics".to_string(),
                teacher: "M. Curie".to_string(),
                student_group: "9th grade".to_string(),
                timeslot: None,
                room: None,
            },
        ],
        score: None,
    };

    // Test JSON serialization roundtrip
    let json = timetable.to_json().unwrap();
    let restored = Timetable::from_json(&json).unwrap();

    assert_eq!(restored.timeslots.len(), 2);
    assert_eq!(restored.rooms.len(), 2);
    assert_eq!(restored.lessons.len(), 2);
    assert_eq!(restored.lessons[0].id, "l1");
    assert_eq!(restored.lessons[0].subject, "Math");
}

#[test]
fn test_timetable_with_assigned_variables() {
    let timetable = Timetable {
        timeslots: vec![Timeslot {
            id: "ts1".to_string(),
            day_of_week: "MONDAY".to_string(),
            start_time: "08:30".to_string(),
            end_time: "09:30".to_string(),
        }],
        rooms: vec![Room {
            id: "r1".to_string(),
            name: "Room A".to_string(),
            capacity: 30,
        }],
        lessons: vec![Lesson {
            id: "l1".to_string(),
            subject: "Math".to_string(),
            teacher: "A. Turing".to_string(),
            student_group: "9th grade".to_string(),
            timeslot: Some("ts1".to_string()),
            room: Some("r1".to_string()),
        }],
        score: Some(HardSoftScore::of(0, -5)),
    };

    // Verify score access
    assert_eq!(timetable.score(), Some(HardSoftScore::of(0, -5)));

    // Test serialization with assigned values
    let json = timetable.to_json().unwrap();
    assert!(json.contains("\"timeslot\":\"ts1\""));
    assert!(json.contains("\"room\":\"r1\""));

    // Verify roundtrip preserves assignments
    let restored = Timetable::from_json(&json).unwrap();
    assert_eq!(restored.lessons[0].timeslot, Some("ts1".to_string()));
    assert_eq!(restored.lessons[0].room, Some("r1".to_string()));
}

#[test]
fn test_timetable_entity_value_conversion() {
    let lesson = Lesson {
        id: "l1".to_string(),
        subject: "Math".to_string(),
        teacher: "A. Turing".to_string(),
        student_group: "9th grade".to_string(),
        timeslot: Some("ts1".to_string()),
        room: None,
    };

    // Test to_value
    let value = lesson.to_value();
    match &value {
        Value::Object(map) => {
            assert_eq!(map.get("id"), Some(&Value::String("l1".to_string())));
            assert_eq!(map.get("subject"), Some(&Value::String("Math".to_string())));
            assert_eq!(map.get("timeslot"), Some(&Value::String("ts1".to_string())));
            assert_eq!(map.get("room"), Some(&Value::Null));
        }
        _ => panic!("Expected Object"),
    }

    // Test roundtrip
    let restored = Lesson::from_value(&value).unwrap();
    assert_eq!(lesson, restored);
}

#[test]
fn test_timetable_planning_id() {
    let lesson = Lesson {
        id: "lesson-42".to_string(),
        subject: "Chemistry".to_string(),
        teacher: "Dr. White".to_string(),
        student_group: "10th grade".to_string(),
        timeslot: None,
        room: None,
    };

    assert_eq!(lesson.planning_id(), Value::String("lesson-42".to_string()));
}

// ============================================================================
// EMPLOYEE SCHEDULING TESTS
// ============================================================================

#[test]
fn test_employee_schedule_solver_builder_creation() {
    let solver = SolverBuilder::<EmployeeSchedule>::new()
        .with_termination(TerminationConfig::new().with_spent_limit("PT2M"))
        .build::<TestBridge>()
        .unwrap();

    assert_eq!(
        solver.config().solution_class,
        Some("EmployeeSchedule".to_string())
    );
}

#[test]
fn test_employee_schedule_domain_model() {
    let model = SolverBuilder::<EmployeeSchedule>::domain_model();

    // Verify solution class
    let solution = model.get_solution_class().unwrap();
    assert_eq!(solution.name, "EmployeeSchedule");

    // Verify entity class
    let shift = model.get_class("Shift").unwrap();
    assert!(shift.is_planning_entity());

    // Verify planning variable allows unassigned
    let employee_var = shift
        .get_planning_variables()
        .find(|f| f.name == "employee")
        .unwrap();
    let allows_unassigned = employee_var.annotations.iter().any(|ann| {
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

#[test]
fn test_employee_schedule_constraints() {
    let constraints = SolverBuilder::<EmployeeSchedule>::constraints();

    assert_eq!(constraints.len(), 4);
    let names: Vec<&str> = constraints.iter().map(|c| c.name.as_str()).collect();
    assert!(names.contains(&"Required skill"));
    assert!(names.contains(&"Overlapping shifts"));
    assert!(names.contains(&"Unavailable employee"));
    assert!(names.contains(&"Preferred availability"));
}

#[test]
fn test_employee_schedule_wasm_generation() {
    let solver = SolverBuilder::<EmployeeSchedule>::new()
        .build::<TestBridge>()
        .unwrap();

    let wasm = solver.wasm_module();
    assert!(wasm.starts_with("AGFzbQ"));
}

#[test]
fn test_employee_schedule_problem_creation() {
    let schedule = EmployeeSchedule {
        employees: vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Charlie".to_string(),
        ],
        availabilities: vec![
            Availability {
                employee_id: "Alice".to_string(),
                date: "2024-01-15".to_string(),
                availability_type: "AVAILABLE".to_string(),
            },
            Availability {
                employee_id: "Bob".to_string(),
                date: "2024-01-15".to_string(),
                availability_type: "UNAVAILABLE".to_string(),
            },
        ],
        shifts: vec![
            Shift {
                id: "s1".to_string(),
                start_time: "09:00".to_string(),
                end_time: "17:00".to_string(),
                location: "Office A".to_string(),
                required_skill: "Customer Service".to_string(),
                employee: None,
            },
            Shift {
                id: "s2".to_string(),
                start_time: "13:00".to_string(),
                end_time: "21:00".to_string(),
                location: "Office B".to_string(),
                required_skill: "Technical Support".to_string(),
                employee: None,
            },
        ],
        score: None,
    };

    // Test JSON serialization roundtrip
    let json = schedule.to_json().unwrap();
    let restored = EmployeeSchedule::from_json(&json).unwrap();

    assert_eq!(restored.employees.len(), 3);
    assert_eq!(restored.availabilities.len(), 2);
    assert_eq!(restored.shifts.len(), 2);
}

#[test]
fn test_employee_schedule_with_assignments() {
    let schedule = EmployeeSchedule {
        employees: vec!["Alice".to_string(), "Bob".to_string()],
        availabilities: vec![],
        shifts: vec![
            Shift {
                id: "s1".to_string(),
                start_time: "09:00".to_string(),
                end_time: "17:00".to_string(),
                location: "Office A".to_string(),
                required_skill: "Support".to_string(),
                employee: Some("Alice".to_string()),
            },
            Shift {
                id: "s2".to_string(),
                start_time: "13:00".to_string(),
                end_time: "21:00".to_string(),
                location: "Office B".to_string(),
                required_skill: "Support".to_string(),
                employee: None, // Unassigned
            },
        ],
        score: Some(HardSoftScore::of(-1, -10)),
    };

    let json = schedule.to_json().unwrap();
    let restored = EmployeeSchedule::from_json(&json).unwrap();

    assert_eq!(restored.shifts[0].employee, Some("Alice".to_string()));
    assert_eq!(restored.shifts[1].employee, None);
}

#[test]
fn test_shift_planning_id() {
    let shift = Shift {
        id: "shift-123".to_string(),
        start_time: "09:00".to_string(),
        end_time: "17:00".to_string(),
        location: "HQ".to_string(),
        required_skill: "Admin".to_string(),
        employee: None,
    };

    assert_eq!(shift.planning_id(), Value::String("shift-123".to_string()));
}

// ============================================================================
// CROSS-CUTTING TESTS
// ============================================================================

#[test]
fn test_solver_builder_service_availability_check() {
    // Test with an unreachable service URL
    let solver = SolverBuilder::<Timetable>::new()
        .with_service_url("http://localhost:19999")
        .build::<TestBridge>()
        .unwrap();

    assert!(!solver.is_service_available());
}

#[test]
fn test_domain_model_entity_class_list() {
    let solver = SolverBuilder::<Timetable>::new()
        .build::<TestBridge>()
        .unwrap();

    let config = solver.config();
    assert!(config.entity_class_list.contains(&"Lesson".to_string()));
}

#[test]
fn test_multiple_planning_variables_in_entity() {
    let class = Lesson::domain_class();
    let vars: Vec<_> = class.get_planning_variables().collect();

    assert_eq!(vars.len(), 2);
    let var_names: Vec<&str> = vars.iter().map(|f| f.name.as_str()).collect();
    assert!(var_names.contains(&"timeslot"));
    assert!(var_names.contains(&"room"));
}

#[test]
fn test_value_range_providers() {
    let model = SolverBuilder::<Timetable>::domain_model();
    let solution = model.get_solution_class().unwrap();

    // Check timeslots field has value range provider
    let timeslots_field = solution
        .fields
        .iter()
        .find(|f| f.name == "timeslots")
        .unwrap();
    let has_vrp = timeslots_field.annotations.iter().any(|ann| {
        matches!(
            ann,
            PlanningAnnotation::ValueRangeProvider { id: Some(ref vrp_id), .. } if vrp_id == "timeslots"
        )
    });
    assert!(has_vrp);

    // Check rooms field has value range provider
    let rooms_field = solution.fields.iter().find(|f| f.name == "rooms").unwrap();
    let has_vrp = rooms_field.annotations.iter().any(|ann| {
        matches!(
            ann,
            PlanningAnnotation::ValueRangeProvider { id: Some(ref vrp_id), .. } if vrp_id == "rooms"
        )
    });
    assert!(has_vrp);
}

#[test]
fn test_problem_fact_collections() {
    let model = SolverBuilder::<Timetable>::domain_model();
    let solution = model.get_solution_class().unwrap();

    let problem_facts: Vec<_> = solution
        .fields
        .iter()
        .filter(|f| {
            f.annotations
                .iter()
                .any(|ann| matches!(ann, PlanningAnnotation::ProblemFactCollectionProperty))
        })
        .collect();

    assert_eq!(problem_facts.len(), 2); // timeslots, rooms
}

#[test]
fn test_planning_entity_collections() {
    let model = SolverBuilder::<Timetable>::domain_model();
    let solution = model.get_solution_class().unwrap();

    let entity_collections: Vec<_> = solution
        .fields
        .iter()
        .filter(|f| {
            f.annotations
                .iter()
                .any(|ann| matches!(ann, PlanningAnnotation::PlanningEntityCollectionProperty))
        })
        .collect();

    assert_eq!(entity_collections.len(), 1); // lessons
    assert_eq!(entity_collections[0].name, "lessons");
}

#[test]
fn test_score_field_configuration() {
    let model = SolverBuilder::<Timetable>::domain_model();
    let solution = model.get_solution_class().unwrap();

    let score_field = solution.fields.iter().find(|f| f.name == "score").unwrap();
    let has_planning_score = score_field
        .annotations
        .iter()
        .any(|ann| matches!(ann, PlanningAnnotation::PlanningScore { .. }));
    assert!(has_planning_score);
}

#[test]
fn test_solver_builder_with_all_options() {
    use solverforge_core::MoveThreadCount;

    let solver = SolverBuilder::<EmployeeSchedule>::new()
        .with_service_url("http://custom-solver:9090")
        .with_termination(
            TerminationConfig::new()
                .with_spent_limit("PT30M")
                .with_unimproved_spent_limit("PT5M")
                .with_best_score_feasible(true),
        )
        .with_environment_mode(EnvironmentMode::FullAssert)
        .with_random_seed(12345)
        .with_move_thread_count(MoveThreadCount::Count(4))
        .build::<TestBridge>()
        .unwrap();

    let config = solver.config();
    assert_eq!(config.environment_mode, Some(EnvironmentMode::FullAssert));
    assert_eq!(config.random_seed, Some(12345));
    assert_eq!(config.move_thread_count, Some(MoveThreadCount::Count(4)));
    assert!(config.termination.is_some());
}
