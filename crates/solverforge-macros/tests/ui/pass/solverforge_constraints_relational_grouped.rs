use solverforge::prelude::*;

// Shared grouped work plus a post-group join, all through the attribute.
// The same grouped stream feeds two terminals (the compiler shares one
// grouped node), and a second stream joins its grouped result to a collection.
struct Schedule {
    shifts: Vec<Option<usize>>,
    employees: Vec<usize>,
    budgets: Vec<i64>,
}

fn shifts(schedule: &Schedule) -> &[Option<usize>] {
    schedule.shifts.as_slice()
}

fn employees(schedule: &Schedule) -> &[usize] {
    schedule.employees.as_slice()
}

fn budgets(schedule: &Schedule) -> &[i64] {
    schedule.budgets.as_slice()
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

    let budgeted = ConstraintFactory::<Schedule, SoftScore>::new()
        .for_each(shifts as fn(&Schedule) -> &[Option<usize>])
        .group_by(
            |shift: &Option<usize>| shift.unwrap_or(usize::MAX),
            count(),
        )
        .join((
            budgets as fn(&Schedule) -> &[i64],
            |_budget: &i64| 0usize,
        ));

    (
        assigned_by_employee
            .penalize(|_employee_id: &usize, count: &i64| SoftScore::of(*count))
            .named("linear assigned shifts"),
        assigned_by_employee
            .reward(|_employee_id: &usize, count: &i64| SoftScore::of(*count * 2))
            .named("coverage reward"),
        budgeted
            .reward(|_key: &usize, count: &usize, _budget: &i64| SoftScore::of(*count as i64))
            .named("budgeted coverage"),
    )
}

fn main() {
    let constraints = constraints();
    let schedule = Schedule {
        shifts: vec![Some(0), Some(0)],
        employees: vec![0, 1],
        budgets: vec![10],
    };
    // Just needs to build and score deterministically; exact value is covered
    // by the runtime facade tests.
    let _ = constraints.constraint_count();
    let _ = schedule;
}
