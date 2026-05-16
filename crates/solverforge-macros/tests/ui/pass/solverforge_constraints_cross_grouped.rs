use solverforge::prelude::*;

#[derive(Clone)]
struct Shift {
    employee_id: Option<usize>,
}

#[derive(Clone)]
struct Employee {
    id: usize,
}

#[derive(Clone)]
struct Schedule {
    shifts: Vec<Shift>,
    employees: Vec<Employee>,
}

fn shifts(schedule: &Schedule) -> &[Shift] {
    schedule.shifts.as_slice()
}

fn employees(schedule: &Schedule) -> &[Employee] {
    schedule.employees.as_slice()
}

#[solverforge_constraints]
fn constraints() -> impl ConstraintSet<Schedule, SoftScore> {
    let g = ConstraintFactory::<Schedule, SoftScore>::new();
    let assigned_by_employee = g
        .for_each(shifts as fn(&Schedule) -> &[Shift])
        .join((
            employees as fn(&Schedule) -> &[Employee],
            joiner::equal_bi(
                |shift: &Shift| shift.employee_id,
                |employee: &Employee| Some(employee.id),
            ),
        ))
        .group_by(
            |_shift: &Shift, employee: &Employee| employee.id,
            sum(|(_shift, _employee): (&Shift, &Employee)| 1i64),
        );

    (
        assigned_by_employee
            .penalize(|_employee_id: &usize, count: &i64| SoftScore::of(*count))
            .named("linear assigned shifts"),
        assigned_by_employee
            .reward(|_employee_id: &usize, count: &i64| SoftScore::of(*count * 2))
            .named("coverage reward"),
    )
}

fn main() {
    let mut constraints = constraints();
    let schedule = Schedule {
        shifts: vec![
            Shift {
                employee_id: Some(0),
            },
            Shift {
                employee_id: Some(0),
            },
        ],
        employees: vec![Employee { id: 0 }, Employee { id: 1 }],
    };

    assert_eq!(constraints.initialize_all(&schedule), SoftScore::of(2));
}
