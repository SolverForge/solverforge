use solverforge::prelude::*;

// P3: independent-key join chain through the attribute. The two joins use
// heterogeneous key types (u32 shift ids, String employee codes) with the
// second key inspecting the whole left row. The runtime twin lives in
// `crates/solverforge/tests/relational_join_publication.rs`.
struct RelSchedule {
    shifts: Vec<RelShift>,
    assignments: Vec<RelAssignment>,
    employees: Vec<RelEmployee>,
}

#[derive(Clone, Debug)]
struct RelShift {
    id: u32,
    night: bool,
}

#[derive(Clone, Debug)]
struct RelAssignment {
    shift_id: u32,
    employee_code: String,
}

#[derive(Clone, Debug)]
struct RelEmployee {
    code: String,
}

fn shifts(schedule: &RelSchedule) -> &[RelShift] {
    schedule.shifts.as_slice()
}

fn assignments(schedule: &RelSchedule) -> &[RelAssignment] {
    schedule.assignments.as_slice()
}

fn employees(schedule: &RelSchedule) -> &[RelEmployee] {
    schedule.employees.as_slice()
}

fn assignment_shift_id(assignment: &RelAssignment) -> u32 {
    assignment.shift_id
}

fn shift_id(shift: &RelShift) -> u32 {
    shift.id
}

fn row_employee_code(
    row: &solverforge::stream::relational::Concat<
        solverforge::stream::relational::Leaf<'_, RelAssignment>,
        RelShift,
    >,
) -> String {
    row.left.entity.employee_code.clone()
}

fn employee_code(employee: &RelEmployee) -> String {
    employee.code.clone()
}

#[solverforge_constraints]
fn constraints() -> impl ConstraintSet<RelSchedule, SoftScore> {
    let factory = ConstraintFactory::<RelSchedule, SoftScore>::new();
    (
        factory
            .for_each(assignments as fn(&RelSchedule) -> &[RelAssignment])
            .join((
                shifts as fn(&RelSchedule) -> &[RelShift],
                joiner::equal_bi(
                    assignment_shift_id as fn(&RelAssignment) -> u32,
                    shift_id as fn(&RelShift) -> u32,
                ),
            ))
            .join_on((
                employees as fn(&RelSchedule) -> &[RelEmployee],
                joiner::equal_on(
                    row_employee_code
                        as fn(
                            &solverforge::stream::relational::Concat<
                                solverforge::stream::relational::Leaf<'_, RelAssignment>,
                                RelShift,
                            >,
                        ) -> String,
                    employee_code as fn(&RelEmployee) -> String,
                ),
            ))
            .filter(|_assignment: &RelAssignment, shift: &RelShift, _employee: &RelEmployee| {
                shift.night
            })
            .penalize(
                |_assignment: &RelAssignment,
                 _shift: &RelShift,
                 _employee: &RelEmployee| { SoftScore::of(1) },
            )
            .named("night shift staffed"),
    )
}

fn main() {
    let mut constraints = constraints();
    let schedule = RelSchedule {
        shifts: vec![
            RelShift { id: 10, night: true },
            RelShift { id: 11, night: false },
        ],
        assignments: vec![
            RelAssignment {
                shift_id: 10,
                employee_code: "e0".to_string(),
            },
            RelAssignment {
                shift_id: 11,
                employee_code: "e1".to_string(),
            },
            RelAssignment {
                shift_id: 10,
                employee_code: "e0".to_string(),
            },
        ],
        employees: vec![
            RelEmployee { code: "e0".to_string() },
            RelEmployee { code: "e1".to_string() },
        ],
    };

    // (a0, s10-night, e0) and (a2, s10-night, e0) match; a1 is day shift.
    assert_eq!(constraints.initialize_all(&schedule), SoftScore::of(-2));
    assert_eq!(constraints.evaluate_all(&schedule), SoftScore::of(-2));
}
