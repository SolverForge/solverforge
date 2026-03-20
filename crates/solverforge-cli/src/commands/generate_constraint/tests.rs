use super::{
    domain::{
        find_annotated_struct, find_score_type, parse_vec_field, DomainModel, EntityInfo, FactInfo,
    },
    mod_rewriter::{extend_tuple, extract_types, insert_mod_decl_assemble},
    skeleton::{generate_skeleton, Pattern},
    utils::{snake_to_title, validate_name},
};

#[test]
fn test_validate_name() {
    assert!(validate_name("max_hours").is_ok());
    assert!(validate_name("required_skill").is_ok());
    assert!(validate_name("a").is_ok());
    assert!(validate_name("MaxHours").is_err());
    assert!(validate_name("1bad").is_err());
    assert!(validate_name("bad-name").is_err());
    assert!(validate_name("").is_err());
}

#[test]
fn test_snake_to_title() {
    assert_eq!(snake_to_title("max_hours"), "Max Hours");
    assert_eq!(snake_to_title("required_skill"), "Required Skill");
    assert_eq!(snake_to_title("all_assigned"), "All Assigned");
    assert_eq!(snake_to_title("capacity"), "Capacity");
}

#[test]
fn test_extract_types_assemble() {
    let src = r#"mod assemble {
    pub fn create_constraints() -> impl ConstraintSet<Plan, HardSoftScore> {
        (all_assigned::constraint(),)
    }
}"#;
    let (s, t) = extract_types(src);
    assert_eq!(s, "Plan");
    assert_eq!(t, "HardSoftScore");
}

#[test]
fn test_extract_types_flat() {
    let src = r#"pub fn create_constraints() -> impl ConstraintSet<VrpPlan, HardSoftScore> {
    (capacity, distance)
}"#;
    let (s, t) = extract_types(src);
    assert_eq!(s, "VrpPlan");
    assert_eq!(t, "HardSoftScore");
}

#[test]
fn test_extend_tuple_single_trailing_comma() {
    let src = r#"mod assemble {
    pub fn create_constraints() -> impl ConstraintSet<Plan, HardSoftScore> {
        (all_assigned::constraint(),)
    }
}"#;
    let result = extend_tuple(src, "max_hours::constraint()");
    assert!(result.contains("all_assigned::constraint(), max_hours::constraint()"));
}

#[test]
fn test_extend_tuple_flat_no_trailing_comma() {
    let src = r#"pub fn create_constraints() -> impl ConstraintSet<VrpPlan, HardSoftScore> {
    (capacity, distance)
}"#;
    let result = extend_tuple(src, "max_hours::constraint()");
    assert!(result.contains("distance, max_hours::constraint()"));
}

#[test]
fn test_insert_mod_decl_assemble() {
    let src = "mod all_assigned;\n\npub use self::assemble::create_constraints;\n";
    let result = insert_mod_decl_assemble(src, "mod max_hours;");
    assert!(
        result.contains("mod all_assigned;\n\nmod max_hours;"),
        "got: {:?}",
        result
    );
    assert!(result.contains("pub use self::assemble::create_constraints;"));
}

#[test]
fn test_parse_vec_field() {
    assert_eq!(
        parse_vec_field("    pub shifts: Vec<Shift>,"),
        Some(("shifts".to_string(), "Shift".to_string()))
    );
    assert_eq!(
        parse_vec_field("    employees: Vec<Employee>"),
        Some(("employees".to_string(), "Employee".to_string()))
    );
}

#[test]
fn test_find_annotated_struct() {
    let src = "#[planning_solution]\npub struct EmployeeSchedule {\n    pub score: Option<HardSoftDecimalScore>,\n}\n";
    assert_eq!(
        find_annotated_struct(src, "planning_solution"),
        Some("EmployeeSchedule".to_string())
    );
}

#[test]
fn test_find_score_type() {
    let src = "#[planning_solution]\npub struct Plan {\n    pub score: Option<HardSoftScore>,\n}\n";
    assert_eq!(find_score_type(src, "Plan"), Some("HardSoftScore".to_string()));
}

#[test]
fn test_generate_skeleton_unary_hard() {
    let domain = DomainModel {
        solution_type: "EmployeeSchedule".to_string(),
        score_type: "HardSoftDecimalScore".to_string(),
        entities: vec![EntityInfo {
            field_name: "shifts".to_string(),
            item_type: "Shift".to_string(),
            planning_vars: vec!["employee_idx".to_string()],
        }],
        facts: vec![],
    };
    let result = generate_skeleton(
        "no_overlap",
        Pattern::Unary,
        false,
        "EmployeeSchedule",
        "HardSoftDecimalScore",
        "No Overlap",
        Some(&domain),
    );
    assert!(result.contains("for_each(|s: &EmployeeSchedule| s.shifts.as_slice())"));
    assert!(result.contains("HardSoftDecimalScore::ONE_HARD"));
    assert!(result.contains("HARD:"));
}

#[test]
fn test_generate_skeleton_pair_hard() {
    let domain = DomainModel {
        solution_type: "EmployeeSchedule".to_string(),
        score_type: "HardSoftDecimalScore".to_string(),
        entities: vec![EntityInfo {
            field_name: "shifts".to_string(),
            item_type: "Shift".to_string(),
            planning_vars: vec!["employee_idx".to_string()],
        }],
        facts: vec![],
    };
    let result = generate_skeleton(
        "no_overlap",
        Pattern::Pair,
        false,
        "EmployeeSchedule",
        "HardSoftDecimalScore",
        "No Overlap",
        Some(&domain),
    );
    assert!(result.contains("for_each_unique_pair"));
    assert!(result.contains("joiner::equal(|e: &Shift| e.employee_idx)"));
}

#[test]
fn test_generate_skeleton_join_hard() {
    let domain = DomainModel {
        solution_type: "EmployeeSchedule".to_string(),
        score_type: "HardSoftDecimalScore".to_string(),
        entities: vec![EntityInfo {
            field_name: "shifts".to_string(),
            item_type: "Shift".to_string(),
            planning_vars: vec!["employee_idx".to_string()],
        }],
        facts: vec![FactInfo {
            field_name: "employees".to_string(),
            item_type: "Employee".to_string(),
        }],
    };
    let result = generate_skeleton(
        "required_skill",
        Pattern::Join,
        false,
        "EmployeeSchedule",
        "HardSoftDecimalScore",
        "Required Skill",
        Some(&domain),
    );
    assert!(result.contains("equal_bi"));
    assert!(result.contains("employees.as_slice()"));
    assert!(result.contains("Employee"));
}

#[test]
fn test_generate_skeleton_balance_soft() {
    let domain = DomainModel {
        solution_type: "EmployeeSchedule".to_string(),
        score_type: "HardSoftDecimalScore".to_string(),
        entities: vec![EntityInfo {
            field_name: "shifts".to_string(),
            item_type: "Shift".to_string(),
            planning_vars: vec!["employee_idx".to_string()],
        }],
        facts: vec![],
    };
    let result = generate_skeleton(
        "balance",
        Pattern::Balance,
        true,
        "EmployeeSchedule",
        "HardSoftDecimalScore",
        "Balance",
        Some(&domain),
    );
    assert!(result.contains(".balance(|e: &Shift| e.employee_idx)"));
    assert!(result.contains("SOFT:"));
}
