use solverforge::prelude::*;

struct Schedule {
    shifts: Vec<Option<usize>>,
    employees: Vec<usize>,
}

fn shifts(schedule: &Schedule) -> &[Option<usize>] {
    schedule.shifts.as_slice()
}

fn employees(schedule: &Schedule) -> &[usize] {
    schedule.employees.as_slice()
}

#[solverforge_constraints]
fn constraints() -> impl ConstraintSet<Schedule, SoftScore> {
    let g = ConstraintFactory::<Schedule, SoftScore>::new();
    let assigned_by_employee = g
        .for_each(shifts as fn(&Schedule) -> &[Option<usize>])
        .join((
            employees as fn(&Schedule) -> &[usize],
            joiner::equal_bi(
                |shift: &Option<usize>| *shift,
                |employee: &usize| Some(*employee),
            ),
        ))
        .group_by(
            |_shift: &Option<usize>, employee: &usize| *employee,
            sum(|(_shift, _employee): (&Option<usize>, &usize)| 1i64),
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
        shifts: vec![Some(0), Some(0)],
        employees: vec![0, 1],
    };

    assert_eq!(constraints.initialize_all(&schedule), SoftScore::of(2));
}
