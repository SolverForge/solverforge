//! Employee Scheduling integration test with larger problem
//!
//! Tests a larger employee scheduling scenario with:
//! - Multiple employees (5) and shifts (10)
//! - Multiple constraints (penalizeId0, oneShiftPerEmployee)
//!
//! The Java HostFunctionProvider now dynamically parses domain models,
//! so we can use realistic fields matching the demo data structure.

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use solverforge_core::{
    DomainAccessor, DomainObjectDto, DomainObjectMapper, FieldDescriptor, ListAccessorDto,
    SolveRequest, SolverPlanningAnnotation as PA, StreamComponent, TerminationConfig, WasmFunction,
};
use solverforge_service::{EmbeddedService, ServiceConfig};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

const JAVA_24_HOME: &str = "/usr/lib64/jvm/java-24-openjdk-24";
const SUBMODULE_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../timefold-wasm-service");

/// Employee Scheduling WASM module with constraint predicates.
///
/// Memory layout (using Integer.SIZE = 32 byte offsets for compatibility):
/// - Employee: [id: i32] (32 bytes per field)
/// - Shift: [employee: i32] (32 bytes per field, pointer to Employee)
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

    (memory 1)

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
    ;; Memory layout: [id: i32] (4 bytes total)

    (func (export "getEmployeeId") (param $employee i32) (result i32)
        (local.get $employee) (i32.load)
    )

    ;; ============== Shift Accessors ==============
    ;; Memory layout: [employee: i32] (4 bytes, pointer to Employee)

    (func (export "getEmployee") (param $shift i32) (result i32)
        (local.get $shift) (i32.load)
    )

    (func (export "setEmployee") (param $shift i32) (param $employee i32)
        (local.get $shift) (local.get $employee) (i32.store)
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

    ;; Check if two shifts have same employee AND are different shifts
    ;; (To avoid self-comparison in join without unique pairs)
    ;; Also only count when shift1 < shift2 to avoid double counting
    (func (export "sameEmployeeAndDifferent") (param $shift1 i32) (param $shift2 i32) (result i32)
        (local $emp1 i32)
        (local $emp2 i32)

        ;; Only count when shift1 address < shift2 address (avoid double counting)
        (if (i32.ge_u (local.get $shift1) (local.get $shift2))
            (then (return (i32.const 0)))
        )

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

    ;; Utility predicates
    (func (export "scaleByCount") (param $count i32) (result i32)
        (local.get $count)
    )
)
"#;

/// Build the employee scheduling domain model
/// Uses the same simple layout as the original test (since HostFunctionProvider is hardcoded)
fn build_employee_scheduling_domain() -> HashMap<String, DomainObjectDto> {
    let mut domain = HashMap::new();

    // Employee with PlanningId
    domain.insert(
        "Employee".to_string(),
        DomainObjectDto::new().with_field(
            "id",
            FieldDescriptor::new("int")
                .with_accessor(DomainAccessor::new("getEmployeeId"))
                .with_annotation(PA::planning_id()),
        ),
    );

    // Shift with PlanningVariable
    domain.insert(
        "Shift".to_string(),
        DomainObjectDto::new().with_field(
            "employee",
            FieldDescriptor::new("Employee")
                .with_accessor(DomainAccessor::getter_setter("getEmployee", "setEmployee"))
                .with_annotation(PA::planning_variable()),
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
fn build_employee_scheduling_constraints() -> HashMap<String, Vec<StreamComponent>> {
    let mut constraints = HashMap::new();

    // Constraint 1: Penalize assignments to employee 0
    // forEach(Shift).join(Employee).filter(isEmployeeId0).penalize(1)
    constraints.insert(
        "penalizeId0".to_string(),
        vec![
            StreamComponent::for_each("Shift"),
            StreamComponent::join("Employee"),
            StreamComponent::filter(WasmFunction::new("isEmployeeId0")),
            StreamComponent::penalize("1"),
        ],
    );

    // Constraint 2: One shift per employee (no double booking)
    // forEach(Shift).join(Shift).filter(sameEmployee).penalize(1)
    // Uses join instead of forEachUniquePair since Shift has no PlanningId
    // This will count each conflict twice, so penalty is doubled
    constraints.insert(
        "oneShiftPerEmployee".to_string(),
        vec![
            StreamComponent::for_each("Shift"),
            StreamComponent::join("Shift"),
            StreamComponent::filter(WasmFunction::new("sameEmployeeAndDifferent")),
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

    // Larger problem: 5 employees, 10 shifts
    // Employees: id 0, 1, 2, 3, 4
    // Constraints:
    // - penalizeId0: Avoid assigning employee 0 (id=0)
    // - oneShiftPerEmployee: Each employee should work at most one shift
    //
    // With 5 employees and 10 shifts, some employees MUST work multiple shifts,
    // so the oneShiftPerEmployee constraint will have violations.
    // The solver should minimize violations while avoiding employee 0.
    let problem_json = r#"{
        "employees": [
            {"id": 0},
            {"id": 1},
            {"id": 2},
            {"id": 3},
            {"id": 4}
        ],
        "shifts": [
            {},
            {},
            {},
            {},
            {},
            {},
            {},
            {},
            {},
            {}
        ]
    }"#;

    let request = SolveRequest::new(
        domain,
        constraints,
        wasm_base64,
        "alloc".to_string(),
        "dealloc".to_string(),
        list_accessor,
        problem_json.to_string(),
    )
    .with_environment_mode("FULL_ASSERT")
    .with_termination(TerminationConfig::new().with_move_count_limit(1000));

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
    println!("Response body:\n{}", response_text);

    // Verify successful response
    assert!(
        status.is_success(),
        "Expected success, got {} with body: {}",
        status,
        response_text
    );

    let result: serde_json::Value =
        serde_json::from_str(&response_text).expect("Failed to parse response JSON");

    // Verify response structure
    assert!(
        result.get("solution").is_some(),
        "Response should contain 'solution'"
    );
    assert!(
        result.get("score").is_some(),
        "Response should contain 'score'"
    );

    // Parse the solution JSON
    let solution_str = result.get("solution").unwrap().as_str().unwrap();
    let solution: serde_json::Value =
        serde_json::from_str(solution_str).expect("Failed to parse solution JSON");

    println!(
        "Solution: {}",
        serde_json::to_string_pretty(&solution).unwrap()
    );

    let score_str = result.get("score").unwrap().as_str().unwrap();
    println!("Score: {}", score_str);

    // Verify solution structure
    let shifts = solution.get("shifts").expect("Solution should have shifts");
    let shifts_array = shifts.as_array().expect("shifts should be an array");
    assert_eq!(shifts_array.len(), 10, "Should have 10 shifts");

    // Verify each shift has an employee assigned
    for (i, shift) in shifts_array.iter().enumerate() {
        let employee = shift.get("employee");
        assert!(
            employee.is_some() && !employee.unwrap().is_null(),
            "Shift {} should have an employee assigned",
            i
        );
    }

    // Count how many shifts are assigned to each employee
    let mut assignment_counts: HashMap<i64, i32> = HashMap::new();
    let mut employee0_count = 0;

    for shift in shifts_array {
        let employee = shift.get("employee").unwrap();
        let emp_id = employee.get("id").unwrap().as_i64().unwrap();
        *assignment_counts.entry(emp_id).or_insert(0) += 1;
        if emp_id == 0 {
            employee0_count += 1;
        }
    }

    println!("Assignment counts: {:?}", assignment_counts);
    println!("Employee 0 assigned {} shifts", employee0_count);

    // With the penalizeId0 constraint, the solver should minimize assignments to employee 0
    // Since we have 5 employees and 10 shifts, optimal would assign 2-3 shifts each to
    // employees 1-4, with minimal/no shifts to employee 0

    // The score should reflect the penalties incurred
    // SimpleScore format is just an integer
    let score: i64 = score_str.parse().unwrap_or(-999);
    println!("Parsed score: {}", score);

    // The optimal solution:
    // - penalizeId0: 0 if employee 0 has no shifts
    // - oneShiftPerEmployee: For 10 shifts with 5 employees (excluding emp 0 = 4 employees),
    //   if we use only 4 employees, each has 2-3 shifts, so pairs penalty =
    //   sum of (n choose 2) for n=2,2,3,3 = 1+1+3+3 = 8, or if n=2,2,2,4 = 1+1+1+6 = 9
    //   With 5 employees each having 2 shifts = 5 * 1 = 5
    //
    // Score should be negative (penalties) or close to 0

    println!("Test completed successfully - solver found a solution!");
    println!(
        "Note: Score of {} reflects oneShiftPerEmployee constraint violations",
        score
    );
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
