//! Employee Scheduling integration test
//!
//! Tests employee scheduling with skill and time-based constraints:
//! - Employees with skills (NURSE, DOCTOR, ADMIN)
//! - Shifts with requiredSkill and start/end times (3 shifts per day)
//! - Constraints: requiredSkill (skill matching), noOverlappingShifts (time overlap)
//! - Configurable scale via EMPLOYEE_COUNT, SHIFT_COUNT env vars
//!
//! The Java HostFunctionProvider dynamically parses domain models from DTOs.

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use indexmap::IndexMap;
use solverforge_core::{
    DomainAccessor, DomainObjectDto, DomainObjectMapper, FieldDescriptor, ListAccessorDto,
    SolveRequest, SolveResponse, SolverPlanningAnnotation as PA, StreamComponent,
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

/// Employee Scheduling WASM module with constraint predicates.
///
/// Memory layout:
/// - Employee: [id: i32 @ 0, skill: i32 @ 4] (8 bytes, skill is string ptr)
/// - Shift: [employee: i32 @ 0, start: i32 @ 4, end: i32 @ 8, requiredSkill: i32 @ 12] (16 bytes)
/// - Schedule: [employees: i32 @ 0, shifts: i32 @ 32, score @ 64]
const EMPLOYEE_SCHEDULING_WAT: &str = r#"
(module
    ;; Type definitions
    (type (;0;) (func (param i32) (result i32)))
    (type (;1;) (func (result i32)))
    (type (;2;) (func (param i32 i32) (result i32)))
    (type (;3;) (func (param i32 i32 i32)))
    (type (;4;) (func (param i32 i32)))
    (type (;5;) (func (param i32) (result i32)))
    (type (;6;) (func (param f32) (result i32)))
    (type (;7;) (func))

    ;; Host function imports
    (import "host" "hparseSchedule" (func $hparseSchedule (type 2)))
    (import "host" "hscheduleString" (func $hscheduleString (type 5)))
    (import "host" "hnewList" (func $hnewList (type 1)))
    (import "host" "hgetItem" (func $hgetItem (type 2)))
    (import "host" "hsetItem" (func $hsetItem (type 3)))
    (import "host" "hsize" (func $hsize (type 0)))
    (import "host" "happend" (func $happend (type 4)))
    (import "host" "hinsert" (func $hinsert (type 3)))
    (import "host" "hremove" (func $hremove (type 4)))
    (import "host" "hround" (func $hround (type 6)))

    (memory 1024)  ;; 1024 pages = 64MB, supports large problem sizes

    ;; ============== Core Infrastructure ==============

    ;; Memory allocator (bump allocator)
    (func (export "alloc") (param $size i32) (result i32)
        (local $out i32)
        (i32.const 0) (i32.load) (local.set $out)
        (i32.const 0) (i32.add (local.get $out) (local.get $size)) (i32.store)
        (local.get $out)
    )

    (func (export "dealloc") (param $pointer i32)
        return
    )

    (func (export "_start")
        (i32.const 0) (i32.const 32) (i32.store)  ;; Start heap at 32
    )

    ;; ============== Solution Mapper ==============

    (func (export "parseSchedule") (param $length i32) (param $schedule i32) (result i32)
        (local.get $length) (local.get $schedule) (call $hparseSchedule)
    )

    (func (export "scheduleString") (param $schedule i32) (result i32)
        (local.get $schedule) (call $hscheduleString)
    )

    ;; ============== List Operations ==============

    (func (export "newList") (result i32) (call $hnewList))
    (func (export "getItem") (param $list i32) (param $index i32) (result i32)
        (local.get $list) (local.get $index) (call $hgetItem)
    )
    (func (export "setItem") (param $list i32) (param $index i32) (param $item i32)
        (local.get $list) (local.get $index) (local.get $item) (call $hsetItem)
    )
    (func (export "size") (param $list i32) (result i32)
        (local.get $list) (call $hsize)
    )
    (func (export "append") (param $list i32) (param $item i32)
        (local.get $list) (local.get $item) (call $happend)
    )
    (func (export "insert") (param $list i32) (param $index i32) (param $item i32)
        (local.get $list) (local.get $index) (local.get $item) (call $hinsert)
    )
    (func (export "remove") (param $list i32) (param $index i32)
        (local.get $list) (local.get $index) (call $hremove)
    )
    (func (export "round") (param $value f32) (result i32)
        (local.get $value) (call $hround)
    )

    ;; ============== Employee Accessors ==============
    ;; Memory layout: [id: i32 @ 0, skill: i32 @ 4] (8 bytes total)

    (func (export "getEmployeeId") (param $employee i32) (result i32)
        (local.get $employee) (i32.load)
    )

    (func (export "getEmployeeSkill") (param $employee i32) (result i32)
        (i32.load (i32.add (local.get $employee) (i32.const 4)))
    )

    ;; ============== Shift Accessors ==============
    ;; Memory layout: [employee: i32 @ 0, start: i32 @ 4, end: i32 @ 8, requiredSkill: i32 @ 12]

    (func (export "getEmployee") (param $shift i32) (result i32)
        (local.get $shift) (i32.load)
    )

    (func (export "setEmployee") (param $shift i32) (param $employee i32)
        (local.get $shift) (local.get $employee) (i32.store)
    )

    (func (export "getShiftStart") (param $shift i32) (result i32)
        (i32.load (i32.add (local.get $shift) (i32.const 4)))
    )

    (func (export "getShiftEnd") (param $shift i32) (result i32)
        (i32.load (i32.add (local.get $shift) (i32.const 8)))
    )

    (func (export "getShiftRequiredSkill") (param $shift i32) (result i32)
        (i32.load (i32.add (local.get $shift) (i32.const 12)))
    )

    ;; Helper to get employee ID from shift (for constraint predicates)
    (func (export "getShiftEmployeeId") (param $shift i32) (result i32)
        (local.get $shift) (i32.load) (i32.load)
    )

    ;; ============== Schedule Accessors ==============
    ;; Memory layout: [employees: i32 @ 0, shifts: i32 @ 32]

    (func (export "getEmployees") (param $schedule i32) (result i32)
        (local.get $schedule) (i32.load)
    )

    (func (export "setEmployees") (param $schedule i32) (param $employees i32)
        (local.get $schedule) (local.get $employees) (i32.store)
    )

    (func (export "getShifts") (param $schedule i32) (result i32)
        (i32.load (i32.add (local.get $schedule) (i32.const 32)))
    )

    (func (export "setShifts") (param $schedule i32) (param $shifts i32)
        (i32.store (i32.add (local.get $schedule) (i32.const 32)) (local.get $shifts))
    )

    ;; ============== Constraint Predicates ==============

    ;; Check if shift's employee has id == 0 (penalize this assignment)
    ;; Returns 1 (true) if employee id is 0
    (func (export "isEmployeeId0") (param $shift i32) (param $employee i32) (result i32)
        (i32.eq (local.get $shift) (i32.load) (i32.load) (i32.const 0))
    )

    ;; Check if two shifts have the same employee assigned
    ;; Used for unique pair constraint to prevent double booking
    (func (export "sameEmployee") (param $shift1 i32) (param $shift2 i32) (result i32)
        (local $emp1 i32)
        (local $emp2 i32)

        ;; Get employee pointers from each shift
        (local.set $emp1 (i32.load (local.get $shift1)))
        (local.set $emp2 (i32.load (local.get $shift2)))

        ;; If either is null (0), return 0 (no match)
        (if (i32.or (i32.eqz (local.get $emp1)) (i32.eqz (local.get $emp2)))
            (then (return (i32.const 0)))
        )

        ;; Return 1 if same employee pointer
        (i32.eq (local.get $emp1) (local.get $emp2))
    )

    ;; Check if two shifts overlap in time AND have the same employee
    ;; Returns 1 if: same employee AND time ranges overlap
    ;; Only counts when shift1 < shift2 to avoid double counting
    (func (export "shiftsOverlap") (param $shift1 i32) (param $shift2 i32) (result i32)
        (local $emp1 i32) (local $emp2 i32)
        (local $start1 i32) (local $end1 i32)
        (local $start2 i32) (local $end2 i32)

        ;; Only count when shift1 address < shift2 address (avoid double counting)
        (if (i32.ge_u (local.get $shift1) (local.get $shift2))
            (then (return (i32.const 0)))
        )

        ;; Get employee pointers from each shift
        (local.set $emp1 (i32.load (local.get $shift1)))
        (local.set $emp2 (i32.load (local.get $shift2)))

        ;; If different employees or either is null, no overlap conflict
        (if (i32.or (i32.eqz (local.get $emp1)) (i32.ne (local.get $emp1) (local.get $emp2)))
            (then (return (i32.const 0)))
        )

        ;; Get times: start @ offset 4, end @ offset 8
        (local.set $start1 (i32.load (i32.add (local.get $shift1) (i32.const 4))))
        (local.set $end1 (i32.load (i32.add (local.get $shift1) (i32.const 8))))
        (local.set $start2 (i32.load (i32.add (local.get $shift2) (i32.const 4))))
        (local.set $end2 (i32.load (i32.add (local.get $shift2) (i32.const 8))))

        ;; Overlap if: start1 < end2 AND start2 < end1
        (i32.and
            (i32.lt_s (local.get $start1) (local.get $end2))
            (i32.lt_s (local.get $start2) (local.get $end1))
        )
    )

    ;; Check if shift's assigned employee skill mismatches shift's requiredSkill
    ;; Returns 1 if: employee is assigned AND skill != requiredSkill
    ;; Used for skill matching constraint
    (func (export "skillMismatch") (param $shift i32) (result i32)
        (local $employee i32)
        (local $empSkill i32)
        (local $reqSkill i32)

        ;; Get assigned employee pointer from shift (offset 0)
        (local.set $employee (i32.load (local.get $shift)))

        ;; If no employee assigned, no mismatch (skip)
        (if (i32.eqz (local.get $employee))
            (then (return (i32.const 0)))
        )

        ;; Get employee's skill (offset 4 in Employee)
        (local.set $empSkill (i32.load (i32.add (local.get $employee) (i32.const 4))))

        ;; Get shift's requiredSkill (offset 12 in Shift)
        (local.set $reqSkill (i32.load (i32.add (local.get $shift) (i32.const 12))))

        ;; Return 1 if skills don't match (string pointers are equal if same string)
        (i32.ne (local.get $empSkill) (local.get $reqSkill))
    )

    ;; Utility predicates
    (func (export "scaleByCount") (param $count i32) (result i32)
        (local.get $count)
    )
)
"#;

/// Build the employee scheduling domain model
/// Uses the same simple layout as the original test (since HostFunctionProvider is hardcoded)
/// Uses IndexMap to preserve field insertion order, which is critical for WASM memory layout.
fn build_employee_scheduling_domain() -> IndexMap<String, DomainObjectDto> {
    let mut domain = IndexMap::new();

    // Employee with PlanningId and skill
    domain.insert(
        "Employee".to_string(),
        DomainObjectDto::new()
            .with_field(
                "id",
                FieldDescriptor::new("int")
                    .with_accessor(DomainAccessor::new("getEmployeeId"))
                    .with_annotation(PA::planning_id()),
            )
            .with_field(
                "skill",
                FieldDescriptor::new("String")
                    .with_accessor(DomainAccessor::new("getEmployeeSkill")),
            ),
    );

    // Shift with PlanningVariable, time fields, and requiredSkill
    domain.insert(
        "Shift".to_string(),
        DomainObjectDto::new()
            .with_field(
                "employee",
                FieldDescriptor::new("Employee")
                    .with_accessor(DomainAccessor::getter_setter("getEmployee", "setEmployee"))
                    .with_annotation(PA::planning_variable()),
            )
            .with_field(
                "start",
                FieldDescriptor::new("int").with_accessor(DomainAccessor::new("getShiftStart")),
            )
            .with_field(
                "end",
                FieldDescriptor::new("int").with_accessor(DomainAccessor::new("getShiftEnd")),
            )
            .with_field(
                "requiredSkill",
                FieldDescriptor::new("String")
                    .with_accessor(DomainAccessor::new("getShiftRequiredSkill")),
            ),
    );

    // Schedule (solution) with collections and score
    domain.insert(
        "Schedule".to_string(),
        DomainObjectDto::new()
            .with_field(
                "employees",
                FieldDescriptor::new("Employee[]")
                    .with_accessor(DomainAccessor::getter_setter(
                        "getEmployees",
                        "setEmployees",
                    ))
                    .with_annotation(PA::problem_fact_collection_property())
                    .with_annotation(PA::value_range_provider()),
            )
            .with_field(
                "shifts",
                FieldDescriptor::new("Shift[]")
                    .with_accessor(DomainAccessor::getter_setter("getShifts", "setShifts"))
                    .with_annotation(PA::planning_entity_collection_property()),
            )
            .with_field(
                "score",
                FieldDescriptor::new("SimpleScore").with_annotation(PA::planning_score()),
            )
            .with_mapper(DomainObjectMapper::new("parseSchedule", "scheduleString")),
    );

    domain
}

/// Build constraints for employee scheduling
fn build_employee_scheduling_constraints() -> IndexMap<String, Vec<StreamComponent>> {
    let mut constraints = IndexMap::new();

    // Constraint 1: Employee must have the skill required by the shift
    // forEach(Shift).filter(skillMismatch).penalize(1)
    constraints.insert(
        "requiredSkill".to_string(),
        vec![
            StreamComponent::for_each("Shift"),
            StreamComponent::filter(WasmFunction::new("skillMismatch")),
            StreamComponent::penalize("1"),
        ],
    );

    // Constraint 2: No overlapping shifts for same employee
    // forEach(Shift).join(Shift).filter(shiftsOverlap).penalize(1)
    // shiftsOverlap checks: same employee AND time ranges overlap AND shift1 < shift2
    constraints.insert(
        "noOverlappingShifts".to_string(),
        vec![
            StreamComponent::for_each("Shift"),
            StreamComponent::join("Shift"),
            StreamComponent::filter(WasmFunction::new("shiftsOverlap")),
            StreamComponent::penalize("1"),
        ],
    );

    constraints
}

/// Compile WAT to WASM and base64 encode
fn compile_employee_scheduling_wasm() -> String {
    let wasm_bytes =
        wat::parse_str(EMPLOYEE_SCHEDULING_WAT).expect("Failed to parse Employee Scheduling WAT");
    BASE64.encode(&wasm_bytes)
}

#[test]
fn test_employee_scheduling_solve() {
    env_logger::try_init().ok();

    // Start the service
    let config = ServiceConfig::new()
        .with_startup_timeout(Duration::from_secs(120))
        .with_java_home(PathBuf::from(JAVA_24_HOME))
        .with_submodule_dir(PathBuf::from(SUBMODULE_DIR));

    let service = EmbeddedService::start(config).expect("Failed to start service");
    println!("Service started on {}", service.url());

    let domain = build_employee_scheduling_domain();
    let constraints = build_employee_scheduling_constraints();
    let wasm_base64 = compile_employee_scheduling_wasm();

    let list_accessor = ListAccessorDto::new(
        "newList", "getItem", "setItem", "size", "append", "insert", "remove", "dealloc",
    );

    // Configurable problem scale via environment variables
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

    // The score reflects constraint violations:
    // - requiredSkill: penalizes skill mismatches
    // - noOverlappingShifts: penalizes time overlaps for same employee
    let score: i64 = score_str.parse().unwrap_or(-999);

    println!("\n=== Summary ===");
    println!(
        "Scale: {} employees, {} shifts",
        employee_count, shift_count
    );
    println!("Score: {} (constraint violations)", score);
    println!("Skill mismatches: {}", skill_mismatches);
    println!("Test completed successfully - solver found a solution!");
}

#[test]
fn test_employee_scheduling_wat_compiles() {
    // Quick sanity check that the WAT is valid
    let wasm_bytes = wat::parse_str(EMPLOYEE_SCHEDULING_WAT);
    assert!(
        wasm_bytes.is_ok(),
        "WAT should compile: {:?}",
        wasm_bytes.err()
    );

    let bytes = wasm_bytes.unwrap();
    assert!(!bytes.is_empty(), "WASM should not be empty");
    assert_eq!(&bytes[0..4], b"\0asm", "Should have WASM magic number");
}
