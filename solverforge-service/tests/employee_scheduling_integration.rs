//! Employee Scheduling integration test
//!
//! Tests employee scheduling with skill and time-based constraints:
//! - Employees with skills (NURSE, DOCTOR, ADMIN)
//! - Shifts with requiredSkill and start/end times (3 shifts per day)
//! - HardSoftScore: hard constraints must be satisfied for feasibility
//! - Hard Constraints:
//!   - requiredSkill: Employee skill must match shift's requiredSkill
//!   - noOverlappingShifts: Same employee can't work overlapping shifts
//! - Configurable scale via EMPLOYEE_COUNT, SHIFT_COUNT env vars
//!
//! The Java HostFunctionProvider dynamically parses domain models from DTOs.

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use indexmap::IndexMap;
use solverforge_core::{
    DomainObjectDto, Joiner, ListAccessorDto, SolveRequest, SolveResponse, StreamComponent,
    TerminationConfig, WasmFunction,
};
use solverforge_service::{EmbeddedService, ServiceConfig};
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use std::time::Duration;

const JAVA_24_HOME: &str = "/usr/lib64/jvm/java-24-openjdk-24";
const SUBMODULE_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../timefold-wasm-service");

/// Generate problem JSON with configurable scale.
/// Shifts are distributed across days with 3 shifts per day (morning, afternoon, night).
///
/// # Arguments
/// * `employee_count` - Number of employees
/// * `shift_count` - Number of shifts (will be rounded to multiple of 3)
fn generate_problem_json(employee_count: usize, shift_count: usize) -> String {
    // Skills rotate through employees
    let skills = ["NURSE", "DOCTOR", "ADMIN"];

    let employees: Vec<String> = (0..employee_count)
        .map(|id| {
            let skill = skills[id % skills.len()];
            format!(r#"{{"id": {}, "skill": "{}"}}"#, id, skill)
        })
        .collect();

    // Generate shifts with times: 3 shifts per day (8-hour shifts)
    // Morning: 06:00-14:00, Afternoon: 14:00-22:00, Night: 22:00-06:00
    // Each shift requires a skill (cycling through the same skills as employees)
    let shifts_per_day = 3;
    let days = (shift_count + shifts_per_day - 1) / shifts_per_day;

    let mut shifts = Vec::new();

    for day in 0..days {
        let day_offset = day * 24; // Hours since start

        // Morning shift: 06:00-14:00 (requires NURSE)
        if shifts.len() < shift_count {
            let start = day_offset + 6;
            let end = day_offset + 14;
            let skill = skills[shifts.len() % skills.len()];
            shifts.push(format!(
                r#"{{"start": {}, "end": {}, "requiredSkill": "{}"}}"#,
                start, end, skill
            ));
        }

        // Afternoon shift: 14:00-22:00 (requires DOCTOR)
        if shifts.len() < shift_count {
            let start = day_offset + 14;
            let end = day_offset + 22;
            let skill = skills[shifts.len() % skills.len()];
            shifts.push(format!(
                r#"{{"start": {}, "end": {}, "requiredSkill": "{}"}}"#,
                start, end, skill
            ));
        }

        // Night shift: 22:00-06:00 (requires ADMIN)
        if shifts.len() < shift_count {
            let start = day_offset + 22;
            let end = day_offset + 30; // 06:00 next day
            let skill = skills[shifts.len() % skills.len()];
            shifts.push(format!(
                r#"{{"start": {}, "end": {}, "requiredSkill": "{}"}}"#,
                start, end, skill
            ));
        }
    }

    format!(
        r#"{{"employees": [{}], "shifts": [{}]}}"#,
        employees.join(", "),
        shifts.join(", ")
    )
}

/// Build the domain model declaratively using DomainModel API
/// This demonstrates how users will define domain models programmatically
fn build_employee_scheduling_model() -> solverforge_core::domain::DomainModel {
    use solverforge_core::domain::{
        DomainClass, DomainModel, FieldDescriptor, FieldType, PlanningAnnotation, PrimitiveType,
        ScoreType,
    };

    DomainModel::builder()
        .add_class(
            DomainClass::new("Employee")
                .with_field(
                    FieldDescriptor::new("id", FieldType::Primitive(PrimitiveType::Int))
                        .with_planning_annotation(PlanningAnnotation::PlanningId),
                )
                .with_field(FieldDescriptor::new(
                    "skill",
                    FieldType::Primitive(PrimitiveType::String),
                )),
        )
        .add_class(
            DomainClass::new("Shift")
                .with_annotation(PlanningAnnotation::PlanningEntity)
                .with_field(
                    FieldDescriptor::new("employee", FieldType::object("Employee"))
                        .with_planning_annotation(PlanningAnnotation::planning_variable(vec![
                            "employees".to_string(),
                        ])),
                )
                .with_field(FieldDescriptor::new(
                    "start",
                    FieldType::Primitive(PrimitiveType::Int),
                ))
                .with_field(FieldDescriptor::new(
                    "end",
                    FieldType::Primitive(PrimitiveType::Int),
                ))
                .with_field(FieldDescriptor::new(
                    "requiredSkill",
                    FieldType::Primitive(PrimitiveType::String),
                )),
        )
        .add_class(
            DomainClass::new("Schedule")
                .with_annotation(PlanningAnnotation::PlanningSolution)
                .with_field(
                    FieldDescriptor::new(
                        "employees",
                        FieldType::list(FieldType::object("Employee")),
                    )
                    .with_planning_annotation(PlanningAnnotation::ProblemFactCollectionProperty)
                    .with_planning_annotation(
                        PlanningAnnotation::value_range_provider("employees"),
                    ),
                )
                .with_field(
                    FieldDescriptor::new("shifts", FieldType::list(FieldType::object("Shift")))
                        .with_planning_annotation(
                            PlanningAnnotation::PlanningEntityCollectionProperty,
                        ),
                )
                .with_field(
                    FieldDescriptor::new("score", FieldType::Score(ScoreType::HardSoft))
                        .with_planning_annotation(PlanningAnnotation::planning_score()),
                ),
        )
        .build()
}

/// Build skillMismatch predicate: employee.skill != shift.requiredSkill
/// Pattern: single-parameter constraint with null checks and string comparison
fn build_skill_mismatch_predicate() -> solverforge_core::wasm::PredicateDefinition {
    use solverforge_core::wasm::{Expr, FieldAccessExt};

    let shift = Expr::param(0);
    let employee = shift.clone().get("Shift", "employee");

    // employee != null AND employee.skill != shift.requiredSkill
    let predicate = Expr::and(
        Expr::is_not_null(employee.clone()),
        Expr::not(Expr::string_equals(
            employee.get("Employee", "skill"),
            shift.get("Shift", "requiredSkill"),
        )),
    );

    solverforge_core::wasm::PredicateDefinition::from_expression("skillMismatch", 1, predicate)
}

/// Build shiftsOverlap predicate: same employee AND time ranges overlap
/// Pattern: two-parameter constraint with reference equality and range checking
fn build_shifts_overlap_predicate() -> solverforge_core::wasm::PredicateDefinition {
    use solverforge_core::wasm::{Expr, FieldAccessExt};

    let shift1 = Expr::param(0);
    let shift2 = Expr::param(1);

    let emp1 = shift1.clone().get("Shift", "employee");
    let emp2 = shift2.clone().get("Shift", "employee");

    // Same employee: emp1 != null AND emp1 == emp2
    let same_employee = Expr::and(Expr::is_not_null(emp1.clone()), Expr::eq(emp1, emp2));

    // Time ranges overlap: start1 < end2 AND start2 < end1
    let ranges_overlap = Expr::ranges_overlap(
        shift1.clone().get("Shift", "start"),
        shift1.clone().get("Shift", "end"),
        shift2.clone().get("Shift", "start"),
        shift2.get("Shift", "end"),
    );

    let predicate = Expr::and(same_employee, ranges_overlap);

    solverforge_core::wasm::PredicateDefinition::from_expression("shiftsOverlap", 2, predicate)
}

/// Build sameEmployeeSameDay predicate: same employee AND same day
/// Pattern: arithmetic expressions with division for day calculation
fn build_same_employee_same_day_predicate() -> solverforge_core::wasm::PredicateDefinition {
    use solverforge_core::wasm::{Expr, FieldAccessExt};

    let shift1 = Expr::param(0);
    let shift2 = Expr::param(1);

    let emp1 = shift1.clone().get("Shift", "employee");
    let emp2 = shift2.clone().get("Shift", "employee");

    // Same employee check
    let same_employee = Expr::and(Expr::is_not_null(emp1.clone()), Expr::eq(emp1, emp2));

    // Same day: start1 / 24 == start2 / 24 (integer division)
    let day1 = Expr::div(shift1.get("Shift", "start"), Expr::int(24));
    let day2 = Expr::div(shift2.get("Shift", "start"), Expr::int(24));
    let same_day = Expr::eq(day1, day2);

    let predicate = Expr::and(same_employee, same_day);

    solverforge_core::wasm::PredicateDefinition::from_expression(
        "sameEmployeeSameDay",
        2,
        predicate,
    )
}

/// Build lessThan10HoursBetween predicate: gap between shifts < 10 hours
/// Pattern: nested conditional logic with if-then-else for complex calculations
fn build_less_than_10_hours_between_predicate() -> solverforge_core::wasm::PredicateDefinition {
    use solverforge_core::wasm::{Expr, FieldAccessExt};

    let shift1 = Expr::param(0);
    let shift2 = Expr::param(1);

    let emp1 = shift1.clone().get("Shift", "employee");
    let emp2 = shift2.clone().get("Shift", "employee");

    // Same employee check
    let same_employee = Expr::and(Expr::is_not_null(emp1.clone()), Expr::eq(emp1, emp2));

    let start1 = shift1.clone().get("Shift", "start");
    let end1 = shift1.clone().get("Shift", "end");
    let start2 = shift2.clone().get("Shift", "start");
    let end2 = shift2.get("Shift", "end");

    // Gap calculation with nested if-then-else:
    // if end1 <= start2 then start2 - end1
    // else if end2 <= start1 then start1 - end2
    // else 999 (overlapping - handled by shiftsOverlap)
    let gap = Expr::if_then_else(
        Expr::le(end1.clone(), start2.clone()),
        Expr::sub(start2.clone(), end1.clone()),
        Expr::if_then_else(
            Expr::le(end2.clone(), start1.clone()),
            Expr::sub(start1, end2),
            Expr::int(999), // Large number for overlapping case
        ),
    );

    let gap_too_small = Expr::lt(gap, Expr::int(10));
    let predicate = Expr::and(same_employee, gap_too_small);

    solverforge_core::wasm::PredicateDefinition::from_expression(
        "lessThan10HoursBetween",
        2,
        predicate,
    )
}

/// Build the employee scheduling domain DTO from the domain model.
/// Uses model.to_dto() which:
/// - Preserves field insertion order via IndexMap
/// - Generates accessor names matching WasmModuleBuilder: get_{Class}_{field}
/// - Adds setters for PlanningVariable and collection fields
/// - Adds mapper for the solution class
fn build_employee_scheduling_domain() -> IndexMap<String, DomainObjectDto> {
    build_employee_scheduling_model().to_dto()
}

/// Build constraints for employee scheduling
fn build_employee_scheduling_constraints() -> IndexMap<String, Vec<StreamComponent>> {
    let mut constraints = IndexMap::new();

    // Constraint 1: Employee must have the skill required by the shift (HARD)
    // forEach(Shift).filter(skillMismatch).penalize(1hard/0soft)
    constraints.insert(
        "requiredSkill".to_string(),
        vec![
            StreamComponent::for_each("Shift"),
            StreamComponent::filter(WasmFunction::new("skillMismatch")),
            StreamComponent::penalize("1hard/0soft"),
        ],
    );

    // Constraint 2: No overlapping shifts for same employee (HARD)
    // Uses join with equal joiner on employee for indexed lookup instead of O(n²)
    // shiftsOverlap checks: time ranges overlap (same employee is handled by joiner)
    constraints.insert(
        "noOverlappingShifts".to_string(),
        vec![
            StreamComponent::for_each("Shift"),
            StreamComponent::join_with_joiners(
                "Shift",
                vec![Joiner::equal(WasmFunction::new("get_Shift_employee"))],
            ),
            StreamComponent::filter(WasmFunction::new("shiftsOverlap")),
            StreamComponent::penalize("1hard/0soft"),
        ],
    );

    // Constraint 3: One shift per day per employee (HARD)
    // Uses join with equal joiner on employee for indexed lookup instead of O(n²)
    // sameEmployeeSameDay checks: same day (same employee is handled by joiner)
    constraints.insert(
        "oneShiftPerDay".to_string(),
        vec![
            StreamComponent::for_each("Shift"),
            StreamComponent::join_with_joiners(
                "Shift",
                vec![Joiner::equal(WasmFunction::new("get_Shift_employee"))],
            ),
            StreamComponent::filter(WasmFunction::new("sameEmployeeSameDay")),
            StreamComponent::penalize("1hard/0soft"),
        ],
    );

    // Constraint 4: At least 10 hours between shifts for same employee (HARD)
    // Uses join with equal joiner on employee for indexed lookup instead of O(n²)
    // lessThan10HoursBetween checks: gap < 10 hours (same employee is handled by joiner)
    constraints.insert(
        "atLeast10HoursBetweenTwoShifts".to_string(),
        vec![
            StreamComponent::for_each("Shift"),
            StreamComponent::join_with_joiners(
                "Shift",
                vec![Joiner::equal(WasmFunction::new("get_Shift_employee"))],
            ),
            StreamComponent::filter(WasmFunction::new("lessThan10HoursBetween")),
            StreamComponent::penalize("1hard/0soft"),
        ],
    );

    constraints
}

/// Build WASM module using WasmModuleBuilder and expression-based predicates
/// Memory is scaled based on problem size to avoid OOM for large benchmarks.
fn build_employee_scheduling_wasm_with_scale(employee_count: usize, shift_count: usize) -> Vec<u8> {
    use solverforge_core::wasm::{HostFunctionRegistry, WasmModuleBuilder};

    let model = build_employee_scheduling_model();
    let registry = HostFunctionRegistry::with_standard_functions();

    // Estimate memory requirements:
    // - Each Employee: ~32 bytes (id + skill pointer + padding + list overhead)
    // - Each Shift: ~64 bytes (employee ptr + start/end dates + skill ptr + padding)
    // - Working memory during solving: ~10x headroom for temporary allocations
    let estimated_bytes = (employee_count * 32 + shift_count * 64) * 10;
    let pages_needed = ((estimated_bytes / 65536) + 1) as u32;

    let initial_pages = pages_needed.max(16).min(256); // At least 16, at most 256 pages
    let max_pages = (pages_needed * 4).max(256).min(4096); // 4x headroom, max 256MB

    WasmModuleBuilder::new()
        .with_domain_model(model)
        .with_host_functions(registry)
        .with_initial_memory(initial_pages)
        .with_max_memory(Some(max_pages))
        .add_predicate(build_skill_mismatch_predicate())
        .add_predicate(build_shifts_overlap_predicate())
        .add_predicate(build_same_employee_same_day_predicate())
        .add_predicate(build_less_than_10_hours_between_predicate())
        .build()
        .expect("Failed to generate WASM module")
}

/// Build WASM module with default memory configuration (for small problems)
fn build_employee_scheduling_wasm() -> Vec<u8> {
    build_employee_scheduling_wasm_with_scale(5, 10)
}

/// Compile expression-based WASM and base64 encode with memory scaled for problem size
fn compile_employee_scheduling_wasm_with_scale(
    employee_count: usize,
    shift_count: usize,
) -> String {
    let wasm_bytes = build_employee_scheduling_wasm_with_scale(employee_count, shift_count);
    BASE64.encode(&wasm_bytes)
}

#[test]
fn test_employee_scheduling_solve() {
    env_logger::try_init().ok();

    // Configurable problem scale via environment variables
    // Read these first so we can scale WASM memory appropriately
    let employee_count: usize = env::var("EMPLOYEE_COUNT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(5);
    let shift_count: usize = env::var("SHIFT_COUNT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10);

    println!("\n=== Problem Scale ===");
    println!("Employees: {}", employee_count);
    println!("Shifts: {}", shift_count);

    // Start the service
    let config = ServiceConfig::new()
        .with_startup_timeout(Duration::from_secs(120))
        .with_java_home(PathBuf::from(JAVA_24_HOME))
        .with_submodule_dir(PathBuf::from(SUBMODULE_DIR));

    let service = EmbeddedService::start(config).expect("Failed to start service");
    println!("Service started on {}", service.url());

    let domain = build_employee_scheduling_domain();
    let constraints = build_employee_scheduling_constraints();
    // Use scaled WASM module based on problem size
    let wasm_base64 = compile_employee_scheduling_wasm_with_scale(employee_count, shift_count);

    let list_accessor = ListAccessorDto::new(
        "newList", "getItem", "setItem", "size", "append", "insert", "remove", "dealloc",
    );

    // Constraints:
    // - requiredSkill: Employee's skill must match shift's requiredSkill
    // - noOverlappingShifts: Same employee can't work overlapping time slots
    //
    // With 3 non-overlapping shifts per day and multiple days, employees CAN
    // work multiple shifts as long as they don't overlap in time.
    // The solver should try to assign employees with matching skills.
    let problem_json = generate_problem_json(employee_count, shift_count);

    let request = SolveRequest::new(
        domain,
        constraints,
        wasm_base64,
        "alloc".to_string(),
        "dealloc".to_string(),
        list_accessor,
        problem_json.to_string(),
    )
    .with_environment_mode(env::var("SOLVER_MODE").unwrap_or_else(|_| "FULL_ASSERT".to_string()))
    .with_termination(
        TerminationConfig::new().with_move_count_limit(
            env::var("MOVE_LIMIT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(1000),
        ),
    );

    // Send to solver
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .expect("Failed to build HTTP client");

    let request_json = serde_json::to_string_pretty(&request).unwrap();
    println!("Request JSON:\n{}", request_json);

    let response = client
        .post(&format!("{}/solve", service.url()))
        .header("Content-Type", "application/json")
        .body(request_json)
        .send()
        .expect("Failed to send request");

    let status = response.status();
    let response_text = response.text().unwrap_or_default();
    println!("Response status: {}", status);
    println!("Response JSON: {}", response_text);

    // Verify successful response
    assert!(
        status.is_success(),
        "Expected success, got {} with body: {}",
        status,
        response_text
    );

    let result: SolveResponse =
        serde_json::from_str(&response_text).expect("Failed to parse response JSON");

    // Parse the solution JSON
    let solution: serde_json::Value =
        serde_json::from_str(&result.solution).expect("Failed to parse solution JSON");

    println!("\n=== Solver Results ===");
    println!("Score: {}", result.score);

    // Print stats if available
    if let Some(stats) = &result.stats {
        println!("\n=== Performance Stats ===");
        println!("{}", stats.summary());
    }

    println!(
        "\nSolution: {}",
        serde_json::to_string_pretty(&solution).unwrap()
    );

    let score_str = &result.score;

    // Verify solution structure
    let shifts = solution.get("shifts").expect("Solution should have shifts");
    let shifts_array = shifts.as_array().expect("shifts should be an array");
    assert_eq!(
        shifts_array.len(),
        shift_count,
        "Should have {} shifts",
        shift_count
    );

    // Count unassigned shifts (some may be uninitialized in large problems)
    let unassigned_count = shifts_array
        .iter()
        .filter(|s| s.get("employee").map_or(true, |e| e.is_null()))
        .count();
    if unassigned_count > 0 {
        println!(
            "Note: {} shifts have no employee assigned (may need more moves)",
            unassigned_count
        );
    }

    // Count skill mismatches and assignments (only for assigned shifts)
    let mut skill_mismatches = 0;
    let mut assignment_counts: HashMap<i64, i32> = HashMap::new();

    for shift in shifts_array {
        if let Some(employee) = shift.get("employee") {
            if !employee.is_null() {
                if let Some(emp_id) = employee.get("id").and_then(|v| v.as_i64()) {
                    *assignment_counts.entry(emp_id).or_insert(0) += 1;

                    // Check skill mismatch
                    let emp_skill = employee.get("skill").and_then(|v| v.as_str());
                    let req_skill = shift.get("requiredSkill").and_then(|v| v.as_str());
                    if emp_skill != req_skill {
                        skill_mismatches += 1;
                    }
                }
            }
        }
    }

    println!("Assignment counts: {:?}", assignment_counts);
    println!("Skill mismatches: {}", skill_mismatches);

    // Parse HardSoftScore format: "0hard/-5soft" or "-2hard/-3soft"
    // The score reflects constraint violations:
    // - requiredSkill (HARD): penalizes skill mismatches
    // - noOverlappingShifts (HARD): penalizes time overlaps for same employee
    let (hard_score, soft_score) = parse_hard_soft_score(&score_str);

    println!("\n=== Summary ===");
    println!(
        "Scale: {} employees, {} shifts",
        employee_count, shift_count
    );
    println!("Hard Score: {} (hard constraint violations)", hard_score);
    println!("Soft Score: {} (soft constraint violations)", soft_score);
    println!("Skill mismatches: {}", skill_mismatches);

    // Check feasibility - a feasible solution has hard score >= 0
    if hard_score >= 0 {
        println!("Solution is FEASIBLE (no hard constraint violations)");
    } else {
        println!(
            "Solution is INFEASIBLE ({} hard constraint violations)",
            -hard_score
        );
    }

    println!("Test completed successfully - solver found a solution!");
}

/// Parse HardSoftScore format: "0hard/-5soft" or "-2hard/-3soft" or "-2/-3"
fn parse_hard_soft_score(score_str: &str) -> (i64, i64) {
    // Try format with labels: "0hard/-5soft"
    if score_str.contains("hard") {
        let parts: Vec<&str> = score_str.split('/').collect();
        if parts.len() == 2 {
            let hard = parts[0].trim_end_matches("hard").parse().unwrap_or(-999);
            let soft = parts[1].trim_end_matches("soft").parse().unwrap_or(0);
            return (hard, soft);
        }
    }
    // Try simple format: "-2/-3"
    let parts: Vec<&str> = score_str.split('/').collect();
    if parts.len() == 2 {
        let hard = parts[0].parse().unwrap_or(-999);
        let soft = parts[1].parse().unwrap_or(0);
        return (hard, soft);
    }
    // Fallback: single number as hard score
    (score_str.parse().unwrap_or(-999), 0)
}

#[test]
fn test_employee_scheduling_wasm_builds() {
    // Validate WASM module builder generates valid WASM
    let wasm_bytes = build_employee_scheduling_wasm();
    assert!(!wasm_bytes.is_empty(), "WASM should not be empty");
    assert_eq!(&wasm_bytes[0..4], b"\0asm", "Should have WASM magic number");
}
