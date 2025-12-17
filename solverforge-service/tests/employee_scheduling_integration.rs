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
    Collector, DomainObjectDto, Joiner, ListAccessorDto, SolveRequest, SolveResponse,
    StreamComponent, TerminationConfig, WasmFunction,
};
use solverforge_service::{EmbeddedService, ServiceConfig};
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use std::time::Duration;

fn java_home() -> String {
    std::env::var("JAVA_HOME").unwrap_or_else(|_| "/usr/lib64/jvm/java-24-openjdk-24".to_string())
}
const SUBMODULE_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../solverforge-wasm-service");

/// Generate problem JSON with configurable scale.
/// Shifts are distributed across days with 3 shifts per day (morning, afternoon, night).
///
/// # Arguments
/// * `employee_count` - Number of employees
/// * `shift_count` - Number of shifts (will be rounded to multiple of 3)
fn generate_problem_json(employee_count: usize, shift_count: usize) -> String {
    // Available skills
    let skills = ["NURSE", "DOCTOR", "ADMIN"];

    let employees: Vec<String> = (0..employee_count)
        .map(|id| {
            // Each employee has 1-2 skills
            let primary_skill = skills[id % skills.len()];
            let skills_json = if id % 3 == 0 {
                // Every 3rd employee has 2 skills
                let secondary_skill = skills[(id + 1) % skills.len()];
                format!(r#"["{}", "{}"]"#, primary_skill, secondary_skill)
            } else {
                format!(r#"["{}"]"#, primary_skill)
            };

            // Generate some unavailable/undesired/desired dates (as epoch days)
            // For simplicity, use day indices: 0 = day 0, 1 = day 1, etc.
            let unavailable = if id % 5 == 0 {
                // Every 5th employee unavailable on days 2, 5
                r#"[2, 5]"#
            } else {
                r#"[]"#
            };

            let undesired = if id % 4 == 0 {
                // Every 4th employee undesired on days 1, 3
                r#"[1, 3]"#
            } else {
                r#"[]"#
            };

            let desired = if id % 3 == 0 {
                // Every 3rd employee desired on days 0, 4
                r#"[0, 4]"#
            } else {
                r#"[]"#
            };

            format!(
                r#"{{"name": "Employee{}", "skills": {}, "unavailableDates": {}, "undesiredDates": {}, "desiredDates": {}}}"#,
                id, skills_json, unavailable, undesired, desired
            )
        })
        .collect();

    // Generate shifts with ISO-8601 datetime: 3 shifts per day (8-hour shifts)
    // Morning: 06:00-14:00, Afternoon: 14:00-22:00, Night: 22:00-06:00
    // Each shift requires a skill (cycling through the same skills as employees)
    // Start date: 2025-01-01, using proper date arithmetic for multi-month spans
    use chrono::{Duration, NaiveDate};

    let locations = ["LOCATION_A", "LOCATION_B", "LOCATION_C"];
    let shifts_per_day = 3;
    let days = (shift_count + shifts_per_day - 1) / shifts_per_day;
    let start_date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();

    let mut shifts = Vec::new();
    let shift_id = |idx: usize| format!("SHIFT{}", idx);

    for day in 0..days {
        let date = start_date + Duration::days(day as i64);
        let next_date = date + Duration::days(1);
        let date_str = date.format("%Y-%m-%d").to_string();
        let next_date_str = next_date.format("%Y-%m-%d").to_string();

        // Morning shift: 06:00-14:00
        if shifts.len() < shift_count {
            let idx = shifts.len();
            let skill = skills[idx % skills.len()];
            let location = locations[idx % locations.len()];
            shifts.push(format!(
                r#"{{"id": "{}", "start": "{}T06:00", "end": "{}T14:00", "location": "{}", "requiredSkill": "{}"}}"#,
                shift_id(idx), date_str, date_str, location, skill
            ));
        }

        // Afternoon shift: 14:00-22:00
        if shifts.len() < shift_count {
            let idx = shifts.len();
            let skill = skills[idx % skills.len()];
            let location = locations[idx % locations.len()];
            shifts.push(format!(
                r#"{{"id": "{}", "start": "{}T14:00", "end": "{}T22:00", "location": "{}", "requiredSkill": "{}"}}"#,
                shift_id(idx), date_str, date_str, location, skill
            ));
        }

        // Night shift: 22:00-06:00 (next day)
        if shifts.len() < shift_count {
            let idx = shifts.len();
            let skill = skills[idx % skills.len()];
            let location = locations[idx % locations.len()];
            shifts.push(format!(
                r#"{{"id": "{}", "start": "{}T22:00", "end": "{}T06:00", "location": "{}", "requiredSkill": "{}"}}"#,
                shift_id(idx), date_str, next_date_str, location, skill
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
                    FieldDescriptor::new("name", FieldType::Primitive(PrimitiveType::String))
                        .with_planning_annotation(PlanningAnnotation::PlanningId),
                )
                .with_field(FieldDescriptor::new(
                    "skills",
                    FieldType::list(FieldType::Primitive(PrimitiveType::String)),
                ))
                .with_field(FieldDescriptor::new(
                    "unavailableDates",
                    FieldType::list(FieldType::Primitive(PrimitiveType::Date)),
                ))
                .with_field(FieldDescriptor::new(
                    "undesiredDates",
                    FieldType::list(FieldType::Primitive(PrimitiveType::Date)),
                ))
                .with_field(FieldDescriptor::new(
                    "desiredDates",
                    FieldType::list(FieldType::Primitive(PrimitiveType::Date)),
                )),
        )
        .add_class(
            DomainClass::new("Shift")
                .with_annotation(PlanningAnnotation::PlanningEntity)
                .with_field(
                    FieldDescriptor::new("id", FieldType::Primitive(PrimitiveType::String))
                        .with_planning_annotation(PlanningAnnotation::PlanningId),
                )
                .with_field(
                    FieldDescriptor::new("employee", FieldType::object("Employee"))
                        .with_planning_annotation(PlanningAnnotation::planning_variable(vec![
                            "employees".to_string(),
                        ])),
                )
                .with_field(FieldDescriptor::new(
                    "location",
                    FieldType::Primitive(PrimitiveType::String),
                ))
                .with_field(FieldDescriptor::new(
                    "start",
                    FieldType::Primitive(PrimitiveType::DateTime),
                ))
                .with_field(FieldDescriptor::new(
                    "end",
                    FieldType::Primitive(PrimitiveType::DateTime),
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

    // employee != null AND !employee.skills.contains(shift.requiredSkill)
    let predicate = Expr::and(
        Expr::is_not_null(employee.clone()),
        Expr::not(Expr::list_contains(
            employee.get("Employee", "skills"),
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

/// Build getMinuteOverlap weigher: returns minutes of overlap between two shifts
/// Formula: (min(end1, end2) - max(start1, start2)) / 60
fn build_get_minute_overlap_weigher() -> solverforge_core::wasm::PredicateDefinition {
    use solverforge_core::wasm::{Expr, FieldAccessExt};

    let shift1 = Expr::param(0);
    let shift2 = Expr::param(1);

    let start1 = shift1.clone().get("Shift", "start");
    let end1 = shift1.get("Shift", "end");
    let start2 = shift2.clone().get("Shift", "start");
    let end2 = shift2.get("Shift", "end");

    // overlap_start = max(start1, start2)
    let overlap_start =
        Expr::if_then_else(Expr::gt(start1.clone(), start2.clone()), start1, start2);

    // overlap_end = min(end1, end2)
    let overlap_end = Expr::if_then_else(Expr::lt(end1.clone(), end2.clone()), end1, end2);

    // overlap_minutes = (overlap_end - overlap_start) / 60
    let overlap_seconds = Expr::sub(overlap_end, overlap_start);
    let overlap_minutes = Expr::div(overlap_seconds, Expr::int(60));

    solverforge_core::wasm::PredicateDefinition::from_expression(
        "getMinuteOverlap",
        2,
        overlap_minutes,
    )
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

    // Same day: start1 / 86400 == start2 / 86400 (86400 seconds in a day)
    let day1 = Expr::div(shift1.get("Shift", "start"), Expr::int(86400));
    let day2 = Expr::div(shift2.get("Shift", "start"), Expr::int(86400));
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

    // Gap calculation with nested if-then-else (in seconds):
    // if end1 <= start2 then start2 - end1
    // else if end2 <= start1 then start1 - end2
    // else 999999 (large number for overlapping case - handled by shiftsOverlap)
    let gap_seconds = Expr::if_then_else(
        Expr::le(end1.clone(), start2.clone()),
        Expr::sub(start2.clone(), end1.clone()),
        Expr::if_then_else(
            Expr::le(end2.clone(), start1.clone()),
            Expr::sub(start1, end2),
            Expr::int(999999), // Large number for overlapping case
        ),
    );

    // Convert to hours: gap_seconds / 3600
    let gap_hours = Expr::div(gap_seconds, Expr::int(3600));
    let gap_too_small = Expr::lt(gap_hours, Expr::int(10));
    let predicate = Expr::and(same_employee, gap_too_small);

    solverforge_core::wasm::PredicateDefinition::from_expression(
        "lessThan10HoursBetween",
        2,
        predicate,
    )
}

/// Build getRestDeficit weigher: returns (10 hours in minutes) - actual gap in minutes
/// Formula: (10 * 60) - (gap_seconds / 60)
/// This represents how many minutes short of the required 10-hour rest
fn build_get_rest_deficit_weigher() -> solverforge_core::wasm::PredicateDefinition {
    use solverforge_core::wasm::{Expr, FieldAccessExt};

    let shift1 = Expr::param(0);
    let shift2 = Expr::param(1);

    let start1 = shift1.clone().get("Shift", "start");
    let end1 = shift1.get("Shift", "end");
    let start2 = shift2.clone().get("Shift", "start");
    let end2 = shift2.get("Shift", "end");

    // Gap calculation (in seconds):
    // if end1 <= start2 then start2 - end1
    // else if end2 <= start1 then start1 - end2
    // else 0 (overlapping case)
    let gap_seconds = Expr::if_then_else(
        Expr::le(end1.clone(), start2.clone()),
        Expr::sub(start2.clone(), end1.clone()),
        Expr::if_then_else(
            Expr::le(end2.clone(), start1.clone()),
            Expr::sub(start1, end2),
            Expr::int(0), // Overlapping case
        ),
    );

    // gap_minutes = gap_seconds / 60
    let gap_minutes = Expr::div(gap_seconds, Expr::int(60));

    // rest_deficit = (10 * 60) - gap_minutes = 600 - gap_minutes
    let rest_deficit = Expr::sub(Expr::int(600), gap_minutes);

    solverforge_core::wasm::PredicateDefinition::from_expression("getRestDeficit", 2, rest_deficit)
}

/// Build shiftOverlapsDate predicate: checks if a shift overlaps with a given date (epoch day)
/// A shift overlaps a date if: shift.start's day == date OR shift.end's day == date
fn build_shift_overlaps_date_predicate() -> solverforge_core::wasm::PredicateDefinition {
    use solverforge_core::wasm::{Expr, FieldAccessExt};

    let shift = Expr::param(0);
    let date = Expr::param(1); // LocalDate as epoch day (i64)

    // shift.start and shift.end are epoch seconds
    // Convert to epoch day by dividing by 86400
    let start_day = Expr::div(shift.clone().get("Shift", "start"), Expr::int(86400));
    let end_day = Expr::div(shift.get("Shift", "end"), Expr::int(86400));

    // shift overlaps date if start_day == date OR end_day == date
    let overlaps = Expr::or(Expr::eq(start_day, date.clone()), Expr::eq(end_day, date));

    solverforge_core::wasm::PredicateDefinition::from_expression("shiftOverlapsDate", 2, overlaps)
}

/// Build getShiftDateOverlapMinutes weigher: calculates overlap minutes between a shift and a date
/// The date is considered as [date 00:00:00, date 23:59:59]
/// Formula: max(0, min(shift_end, day_end) - max(shift_start, day_start)) / 60
fn build_get_shift_date_overlap_minutes_weigher() -> solverforge_core::wasm::PredicateDefinition {
    use solverforge_core::wasm::{Expr, FieldAccessExt};

    let shift = Expr::param(0);
    let date = Expr::param(1); // LocalDate as epoch day (i64)

    // Day boundaries in epoch seconds
    let day_start = Expr::mul(date.clone(), Expr::int(86400)); // date * 86400
    let day_end = Expr::add(
        Expr::mul(date, Expr::int(86400)),
        Expr::int(86399), // date * 86400 + 86399 (23:59:59)
    );

    let shift_start = shift.clone().get("Shift", "start");
    let shift_end = shift.get("Shift", "end");

    // overlap_start = max(shift_start, day_start)
    let overlap_start = Expr::if_then_else(
        Expr::gt(shift_start.clone(), day_start.clone()),
        shift_start,
        day_start,
    );

    // overlap_end = min(shift_end, day_end)
    let overlap_end = Expr::if_then_else(
        Expr::lt(shift_end.clone(), day_end.clone()),
        shift_end,
        day_end,
    );

    // overlap_seconds = max(0, overlap_end - overlap_start)
    let overlap_diff = Expr::sub(overlap_end, overlap_start);
    let overlap_seconds = Expr::if_then_else(
        Expr::gt(overlap_diff.clone(), Expr::int(0)),
        overlap_diff,
        Expr::int(0),
    );

    // overlap_minutes = overlap_seconds / 60
    let overlap_minutes = Expr::div(overlap_seconds, Expr::int(60));

    solverforge_core::wasm::PredicateDefinition::from_expression(
        "getShiftDateOverlapMinutes",
        2,
        overlap_minutes,
    )
}

/// Build get_Shift_employee_unavailableDates mapper: shift -> shift.employee.unavailableDates
/// Returns the employee's unavailable dates list from a shift, or null if no employee assigned
fn build_get_shift_employee_unavailable_dates() -> solverforge_core::wasm::PredicateDefinition {
    use solverforge_core::wasm::{Expr, FieldAccessExt};

    let shift = Expr::param(0);
    let employee = shift.clone().get("Shift", "employee");
    let dates = employee.clone().get("Employee", "unavailableDates");

    // If employee is null, return null (which flattenLast should handle gracefully)
    // Otherwise return the dates list
    let result = Expr::if_then_else(Expr::is_not_null(employee), dates, Expr::null());

    solverforge_core::wasm::PredicateDefinition::from_expression(
        "get_Shift_employee_unavailableDates",
        1,
        result,
    )
}

/// Build get_Shift_employee_undesiredDates mapper: shift -> shift.employee.undesiredDates
fn build_get_shift_employee_undesired_dates() -> solverforge_core::wasm::PredicateDefinition {
    use solverforge_core::wasm::{Expr, FieldAccessExt};

    let shift = Expr::param(0);
    let dates = shift
        .get("Shift", "employee")
        .get("Employee", "undesiredDates");

    solverforge_core::wasm::PredicateDefinition::from_expression(
        "get_Shift_employee_undesiredDates",
        1,
        dates,
    )
}

/// Build get_Shift_employee_desiredDates mapper: shift -> shift.employee.desiredDates
fn build_get_shift_employee_desired_dates() -> solverforge_core::wasm::PredicateDefinition {
    use solverforge_core::wasm::{Expr, FieldAccessExt};

    let shift = Expr::param(0);
    let dates = shift
        .get("Shift", "employee")
        .get("Employee", "desiredDates");

    solverforge_core::wasm::PredicateDefinition::from_expression(
        "get_Shift_employee_desiredDates",
        1,
        dates,
    )
}

/// Build shiftHasEmployee predicate: checks if shift has an assigned employee (not null)
fn build_shift_has_employee_predicate() -> solverforge_core::wasm::PredicateDefinition {
    use solverforge_core::wasm::{Expr, FieldAccessExt};

    let shift = Expr::param(0);
    let employee = shift.get("Shift", "employee");
    let has_employee = Expr::is_not_null(employee);

    solverforge_core::wasm::PredicateDefinition::from_expression(
        "shiftHasEmployee",
        1,
        has_employee,
    )
}

/// Build pick1 function: extracts first element from a 2-tuple (e.g., from groupBy result)
/// Used after groupBy(employee, count()) to get the employee from (Employee, count) tuple
fn build_pick1_function() -> solverforge_core::wasm::PredicateDefinition {
    use solverforge_core::wasm::Expr;

    // param(0) is the first element of the tuple
    let first = Expr::param(0);

    solverforge_core::wasm::PredicateDefinition::from_expression("pick1", 2, first)
}

/// Build pick2 function: extracts second element from a 2-tuple (e.g., from groupBy result)
/// Used after groupBy(employee, count()) to get the count from (Employee, count) tuple
fn build_pick2_function() -> solverforge_core::wasm::PredicateDefinition {
    use solverforge_core::wasm::Expr;

    // param(1) is the second element of the tuple
    let second = Expr::param(1);

    solverforge_core::wasm::PredicateDefinition::from_expression("pick2", 2, second)
}

/// Build scaleByFloat function: rounds a float value to an integer for scoring
/// Used for penalizing/rewarding by LoadBalance unfairness (which is a BigDecimal/float)
fn build_scale_by_float_function() -> solverforge_core::wasm::PredicateDefinition {
    use solverforge_core::wasm::{Expr, ValType};

    // param(0) is the float value - use hround host function to convert to int
    // For unfairness penalty, we round to the nearest integer
    let float_val = Expr::param(0);

    // Call hround host function to round float to int
    let int_val = Expr::host_call("hround", vec![float_val]);

    // The LoadBalance unfairness is passed as f32, so we need explicit param type
    solverforge_core::wasm::PredicateDefinition::from_expression_with_types(
        "scaleByFloat",
        vec![ValType::F32],
        int_val,
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
    // Using forEachUniquePair with equal joiner on employee for efficient self-join
    // The equal joiner ensures only shifts with same employee are paired
    // shiftsOverlap further filters to check time overlap
    constraints.insert(
        "noOverlappingShifts".to_string(),
        vec![
            StreamComponent::for_each_unique_pair_with_joiners(
                "Shift",
                vec![Joiner::equal(WasmFunction::new("get_Shift_employee"))],
            ),
            StreamComponent::filter(WasmFunction::new("shiftsOverlap")),
            StreamComponent::penalize_with_weigher(
                "1hard/0soft",
                WasmFunction::new("getMinuteOverlap"),
            ),
        ],
    );

    // Constraint 3: One shift per day per employee (HARD)
    constraints.insert(
        "oneShiftPerDay".to_string(),
        vec![
            StreamComponent::for_each_unique_pair_with_joiners(
                "Shift",
                vec![Joiner::equal(WasmFunction::new("get_Shift_employee"))],
            ),
            StreamComponent::filter(WasmFunction::new("sameEmployeeSameDay")),
            StreamComponent::penalize("1hard/0soft"),
        ],
    );

    // Constraint 4: At least 10 hours between shifts for same employee (HARD)
    constraints.insert(
        "atLeast10HoursBetweenTwoShifts".to_string(),
        vec![
            StreamComponent::for_each_unique_pair_with_joiners(
                "Shift",
                vec![Joiner::equal(WasmFunction::new("get_Shift_employee"))],
            ),
            StreamComponent::filter(WasmFunction::new("lessThan10HoursBetween")),
            StreamComponent::penalize_with_weigher(
                "1hard/0soft",
                WasmFunction::new("getRestDeficit"),
            ),
        ],
    );

    // Constraint 5: Balance employee shift assignments (SOFT)
    // Groups by employee with count, adds unassigned employees with 0 count,
    // then computes loadBalance unfairness as the penalty.
    // This encourages fair distribution of shifts across all employees.
    constraints.insert(
        "balanceEmployeeShiftAssignments".to_string(),
        vec![
            StreamComponent::for_each("Shift"),
            StreamComponent::group_by(
                vec![WasmFunction::new("get_Shift_employee")],
                vec![Collector::count()],
            ),
            StreamComponent::complement("Employee"),
            StreamComponent::group_by(
                vec![],
                vec![Collector::load_balance_with_load(
                    WasmFunction::new("pick1"),
                    WasmFunction::new("pick2"),
                )],
            ),
            StreamComponent::penalize_with_weigher(
                "0hard/1soft",
                WasmFunction::new("scaleByFloat"),
            ),
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
        .add_predicate(build_get_minute_overlap_weigher())
        .add_predicate(build_same_employee_same_day_predicate())
        .add_predicate(build_less_than_10_hours_between_predicate())
        .add_predicate(build_get_rest_deficit_weigher())
        .add_predicate(build_shift_overlaps_date_predicate())
        .add_predicate(build_get_shift_date_overlap_minutes_weigher())
        .add_predicate(build_get_shift_employee_unavailable_dates())
        .add_predicate(build_get_shift_employee_undesired_dates())
        .add_predicate(build_get_shift_employee_desired_dates())
        .add_predicate(build_shift_has_employee_predicate())
        .add_predicate(build_pick1_function())
        .add_predicate(build_pick2_function())
        .add_predicate(build_scale_by_float_function())
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
        .with_java_home(PathBuf::from(java_home()))
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
    let mut assignment_counts: HashMap<String, i32> = HashMap::new();

    for shift in shifts_array {
        if let Some(employee) = shift.get("employee") {
            if !employee.is_null() {
                if let Some(emp_name) = employee.get("name").and_then(|v| v.as_str()) {
                    *assignment_counts.entry(emp_name.to_string()).or_insert(0) += 1;

                    // Check skill mismatch - employee.skills must contain shift.requiredSkill
                    let emp_skills = employee
                        .get("skills")
                        .and_then(|v| v.as_array())
                        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
                        .unwrap_or_default();
                    let req_skill = shift.get("requiredSkill").and_then(|v| v.as_str());
                    if let Some(required) = req_skill {
                        if !emp_skills.contains(&required) {
                            skill_mismatches += 1;
                        }
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
