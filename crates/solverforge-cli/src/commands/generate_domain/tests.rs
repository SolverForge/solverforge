use super::{
    generators::{generate_entity, generate_fact, generate_solution},
    utils::{pluralize, snake_to_pascal, validate_score_type},
    wiring::{add_import, replace_score_type},
};

#[test]
fn test_snake_to_pascal() {
    assert_eq!(snake_to_pascal("shift"), "Shift");
    assert_eq!(snake_to_pascal("employee_schedule"), "EmployeeSchedule");
    assert_eq!(
        snake_to_pascal("vehicle_routing_plan"),
        "VehicleRoutingPlan"
    );
    assert_eq!(snake_to_pascal("plan"), "Plan");
}

#[test]
fn test_pluralize() {
    assert_eq!(pluralize("shift"), "shifts");
    assert_eq!(pluralize("employee"), "employees");
    assert_eq!(pluralize("bus"), "buses");
    assert_eq!(pluralize("category"), "categories");
    assert_eq!(pluralize("day"), "days");
    assert_eq!(pluralize("key"), "keys");
    assert_eq!(pluralize("task"), "tasks");
}

#[test]
fn test_validate_score_type() {
    assert!(validate_score_type("HardSoftScore").is_ok());
    assert!(validate_score_type("HardSoftDecimalScore").is_ok());
    assert!(validate_score_type("HardMediumSoftScore").is_ok());
    assert!(validate_score_type("SimpleScore").is_ok());
    assert!(validate_score_type("BendableScore").is_ok());
    assert!(validate_score_type("FakeScore").is_err());
}

#[test]
fn test_generate_entity_no_var() {
    let src = generate_entity("Shift", None, &[]);
    assert!(src.contains("#[planning_entity]"));
    assert!(src.contains("pub struct Shift"));
    assert!(src.contains("#[planning_id]"));
    assert!(src.contains("pub id: String"));
    assert!(!src.contains("#[planning_variable]"));
}

#[test]
fn test_generate_entity_with_var() {
    let src = generate_entity("Shift", Some("employee_idx"), &[]);
    assert!(src.contains("#[planning_variable(allows_unassigned = true)]"));
    assert!(src.contains("pub employee_idx: Option<usize>"));
    assert!(src.contains("employee_idx: None"));
}

#[test]
fn test_generate_fact() {
    let src = generate_fact("Employee", &[]);
    assert!(src.contains("#[problem_fact]"));
    assert!(src.contains("pub struct Employee"));
    assert!(src.contains("pub index: usize"));
    assert!(src.contains("pub name: String"));
}

#[test]
fn test_generate_solution() {
    let src = generate_solution("Schedule", "HardSoftDecimalScore");
    assert!(src.contains("#[planning_solution]"));
    assert!(src.contains("pub struct Schedule"));
    assert!(src.contains("#[planning_score]"));
    assert!(src.contains("pub score: Option<HardSoftDecimalScore>"));
}

#[test]
fn test_add_import_new() {
    let src = "use solverforge::prelude::*;\n\nstruct Foo;\n";
    let result = add_import(src, "use super::Bar;");
    assert!(result.contains("use super::Bar;"));
    let use_pos = result.find("use solverforge").unwrap();
    let bar_pos = result.find("use super::Bar;").unwrap();
    assert!(bar_pos > use_pos);
}

#[test]
fn test_add_import_idempotent() {
    let src = "use super::Bar;\nstruct Foo;\n";
    let result = add_import(src, "use super::Bar;");
    assert_eq!(result.matches("use super::Bar;").count(), 1);
}

#[test]
fn test_replace_score_type() {
    let src = "pub score: Option<HardSoftScore>,\n";
    let result = replace_score_type(src, "HardSoftScore", "HardSoftDecimalScore").unwrap();
    assert!(result.contains("HardSoftDecimalScore"));
    assert!(!result.contains("HardSoftScore"));
}

#[test]
fn test_replace_score_type_missing() {
    let src = "pub score: Option<HardSoftScore>,\n";
    let result = replace_score_type(src, "SimpleScore", "HardSoftScore");
    assert!(result.is_err());
}

#[test]
fn test_inject_second_planning_variable() {
    use super::wiring::inject_planning_variable;

    let src = generate_entity("Surgery", Some("room_idx"), &[]);
    let result =
        inject_planning_variable(&src, "Surgery", "slot_idx").expect("inject should succeed");

    assert!(
        result.contains("slot_idx: None"),
        "slot_idx: None not found in output"
    );

    let self_start = result.find("Self {").expect("Self { not found");
    let self_block = &result[self_start..];
    let close = self_block.find('}').expect("} not found after Self {");
    let self_literal = &self_block[..=close];
    assert!(
        self_literal.contains("room_idx: None"),
        "room_idx: None not inside Self {{ }}: got: {self_literal}"
    );
    assert!(
        self_literal.contains("slot_idx: None"),
        "slot_idx: None not inside Self {{ }}: got:\n{result}"
    );
}

#[test]
fn test_update_domain_mod_format() {
    let mod_line = format!("mod {};", "shift");
    let use_line = format!("pub use {}::{};", "shift", "Shift");
    assert_eq!(mod_line, "mod shift;");
    assert_eq!(use_line, "pub use shift::Shift;");
}
