//! End-to-end integration tests for solving constraint problems
//!
//! These tests require Java 24 and Maven to be installed.
//! Run with: cargo test -p solverforge-service --test solve_integration -- --ignored

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use indexmap::IndexMap;
use solverforge_core::{
    DomainAccessor, DomainObjectDto, DomainObjectMapper, FieldDescriptor, ListAccessorDto,
    SolveRequest, SolveResponse, SolverPlanningAnnotation as PA, TerminationConfig,
};
use solverforge_service::{EmbeddedService, ServiceConfig};
use std::path::PathBuf;
use std::time::Duration;

fn java_home() -> String {
    std::env::var("JAVA_HOME").unwrap_or_else(|_| "/usr/lib64/jvm/java-24-openjdk-24".to_string())
}
const SUBMODULE_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../solverforge-wasm-service");

/// WebAssembly Text format module matching the Java test's expectations.
/// This defines functions for:
/// - Memory allocation/deallocation
/// - Schedule parsing/serialization (delegates to host)
/// - List operations (delegates to host)
/// - Field accessors (getEmployee, setEmployee, etc.)
/// - Constraint predicates (isEmployeeId0, etc.)
const TEST_WAT: &str = r#"
(module
    (type (;0;) (func (param i32) (result i32)))
    (type (;1;) (func (result i32)))
    (type (;2;) (func (param i32 i32) (result i32)))
    (type (;3;) (func (param i32 i32 i32)))
    (type (;4;) (func (param i32 i32)))
    (type (;5;) (func (param i32) (result i32)))
    (type (;6;) (func (param f32) (result i32)))
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
    (func (export "parseSchedule") (param $length i32) (param $schedule i32) (result i32)
        (local.get $length) (local.get $schedule) (call $hparseSchedule)
    )
    (func (export "scheduleString") (param $schedule i32) (result i32)
        (local.get $schedule) (call $hscheduleString)
    )
    (func (export "newList") (result i32)
        (call $hnewList)
    )
    (func (export "round") (param $value f32) (result i32)
        (local.get $value) (call $hround)
    )
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
    (func (export "getEmployee") (param $shift i32) (result i32)
        (local.get $shift) (i32.load)
    )
    (func (export "getShiftEmployeeId") (param $shift i32) (result i32)
        (local.get $shift) (i32.load) (i32.load)
    )
    (func (export "getEmployeeId") (param $employee i32) (result i32)
        (local.get $employee) (i32.load)
    )
    (func (export "getEmployeePlus2") (param $employee i32) (result i32)
        (i32.add (local.get $employee) (i32.load) (i32.const 2))
    )
    (func (export "setEmployee") (param $shift i32) (param $employee i32) (result)
        (local.get $shift) (local.get $employee) (i32.store)
    )
    (func (export "getEmployees") (param $schedule i32) (result i32)
        (local.get $schedule) (i32.load)
    )
    (func (export "setEmployees") (param $schedule i32) (param $employees i32) (result)
        (local.get $schedule) (local.get $employees) (i32.store)
    )
    (func (export "getShifts") (param $schedule i32) (result i32)
        (i32.add (local.get $schedule) (i32.const 4)) (i32.load)
    )
    (func (export "setShifts") (param $schedule i32) (param $shifts i32) (result)
        (i32.add (local.get $schedule) (i32.const 4)) (local.get $shifts) (i32.store)
    )
    (func (export "isEmployeeId0") (param $shift i32) (param $employee i32) (result i32)
        (i32.eq (local.get $shift) (i32.load) (i32.load) (i32.const 0))
    )
    (func (export "scaleByCount") (param $count i32) (result i32)
        (local.get $count)
    )
    (func (export "scaleByFloat") (param $value f32) (result i32)
        (local.get $value) (call $hround)
    )
    (func (export "scaleByCountItemSquared") (param $list i32) (result i32)
        (local $x i32) (i32.mul (local.get $list) (i32.const 0) (call $hgetItem) (local.tee $x) (local.get $x))
    )
    (func (export "compareInt") (param $a i32) (param $b i32) (result i32)
        (i32.sub (local.get $a) (local.get $b))
    )
    (func (export "sameParity") (param $a i32) (param $b i32) (result i32)
        (local.get $a) (i32.const 2) (i32.rem_u) (local.get $b) (i32.const 2) (i32.rem_u) (i32.eq)
    )
    (func (export "parity") (param $a i32) (result i32)
        (local.get $a) (i32.const 2) (i32.rem_u)
    )
    (func (export "id") (param $a i32) (result i32)
        (local.get $a)
    )
    (func (export "pick1") (param $a i32) (param $b i32) (result i32)
        (local.get $a)
    )
    (func (export "pick2") (param $a i32) (param $b i32) (result i32)
        (local.get $b)
    )
    (func (export "alloc") (param $size i32) (result i32)
        (local $out i32) (i32.const 0) (i32.load) (local.set $out) (i32.const 0) (i32.add (local.get $out) (local.get $size)) (i32.store) (local.get $out)
    )
    (func (export "dealloc") (param $pointer i32) (result)
        return
    )
    (func (export "_start") (result)
        (i32.const 0) (i32.const 32) (i32.store)
    )
)
"#;

/// Build the domain model for the test
/// Uses IndexMap to preserve field insertion order for WASM memory layout.
fn build_test_domain() -> IndexMap<String, DomainObjectDto> {
    let mut domain = IndexMap::new();

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

/// Build constraints for the test
fn build_test_constraints() -> IndexMap<String, Vec<solverforge_core::StreamComponent>> {
    use solverforge_core::{StreamComponent, WasmFunction};

    let mut constraints = IndexMap::new();

    // penalizeId0: forEach(Shift).join(Employee).filter(isEmployeeId0).penalize(1)
    constraints.insert(
        "penalizeId0".to_string(),
        vec![
            StreamComponent::for_each("Shift"),
            StreamComponent::join("Employee"),
            StreamComponent::filter(WasmFunction::new("isEmployeeId0")),
            StreamComponent::penalize("1"),
        ],
    );

    constraints
}

/// Compile WAT to WASM and base64 encode
fn compile_test_wasm() -> String {
    let wasm_bytes = wat::parse_str(TEST_WAT).expect("Failed to parse WAT");
    BASE64.encode(&wasm_bytes)
}

#[test]
fn test_solve_simple_problem() {
    env_logger::try_init().ok();

    // Start the service using local submodule
    let config = ServiceConfig::new()
        .with_startup_timeout(Duration::from_secs(120))
        .with_java_home(PathBuf::from(java_home()))
        .with_submodule_dir(PathBuf::from(SUBMODULE_DIR));

    let service = EmbeddedService::start(config).expect("Failed to start service");
    println!("Service started on {}", service.url());

    // Build the solve request
    let domain = build_test_domain();
    let constraints = build_test_constraints();
    let wasm_base64 = compile_test_wasm();

    let list_accessor = ListAccessorDto::new(
        "newList", "getItem", "setItem", "size", "append", "insert", "remove", "dealloc",
    );

    let problem_json = r#"{"employees": [{"id": 0}, {"id": 1}], "shifts": [{}, {}]}"#;

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
    .with_termination(TerminationConfig::new().with_move_count_limit(10));

    // Send to solver
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(60))
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
    println!("Response body: {}", response_text);

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

    // Verify solution structure
    let shifts = solution.get("shifts").expect("Solution should have shifts");
    let shifts_array = shifts.as_array().expect("shifts should be an array");
    assert_eq!(shifts_array.len(), 2, "Should have 2 shifts");

    // Verify each shift has an employee assigned
    for (i, shift) in shifts_array.iter().enumerate() {
        let employee = shift.get("employee");
        assert!(
            employee.is_some() && !employee.unwrap().is_null(),
            "Shift {} should have an employee assigned",
            i
        );
    }

    // With only the penalizeId0 constraint, optimal solution avoids employee 0
    // Score should be 0 (no penalties) when both shifts use employee 1
    assert_eq!(
        result.score, "0",
        "Score should be 0 when avoiding employee 0"
    );
}

#[test]
fn test_solve_request_json_structure() {
    // Verify the JSON structure matches what Java expects
    let domain = build_test_domain();
    let constraints = build_test_constraints();
    let wasm_base64 = compile_test_wasm();

    let list_accessor = ListAccessorDto::new(
        "newList", "getItem", "setItem", "size", "append", "insert", "remove", "dealloc",
    );

    let request = SolveRequest::new(
        domain,
        constraints,
        wasm_base64,
        "alloc".to_string(),
        "dealloc".to_string(),
        list_accessor,
        r#"{"employees": [], "shifts": []}"#.to_string(),
    )
    .with_termination(TerminationConfig::new().with_move_count_limit(10));

    let json = serde_json::to_string_pretty(&request).unwrap();
    println!("Generated JSON:\n{}", json);

    // Verify key fields exist
    assert!(json.contains("\"domain\""));
    assert!(json.contains("\"constraints\""));
    assert!(json.contains("\"wasm\""));
    assert!(json.contains("\"allocator\""));
    assert!(json.contains("\"deallocator\""));
    assert!(json.contains("\"listAccessor\""));
    assert!(json.contains("\"problem\""));
    assert!(json.contains("\"termination\""));

    // Verify list accessor uses Java-compatible field names
    assert!(json.contains("\"new\":"));
    assert!(json.contains("\"get\":"));
    assert!(json.contains("\"set\":"));
    assert!(json.contains("\"length\":"));

    // Verify annotations use correct names
    assert!(
        json.contains("\"annotation\": \"PlanningId\"")
            || json.contains("\"annotation\":\"PlanningId\"")
    );
    assert!(
        json.contains("\"annotation\": \"PlanningVariable\"")
            || json.contains("\"annotation\":\"PlanningVariable\"")
    );
    assert!(
        json.contains("\"annotation\": \"PlanningScore\"")
            || json.contains("\"annotation\":\"PlanningScore\"")
    );
}
