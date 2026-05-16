use solverforge::prelude::*;

#[derive(Clone)]
struct Shift {
    employee_id: usize,
}

#[derive(Clone)]
struct Plan {
    shifts: Vec<Shift>,
}

fn shifts(plan: &Plan) -> &[Shift] {
    plan.shifts.as_slice()
}

#[solverforge_constraints]
fn constraints() -> impl ConstraintSet<Plan, SoftScore> {
    let g = ConstraintFactory::<Plan, SoftScore>::new();
    let by_employee = g
        .for_each(shifts as fn(&Plan) -> &[Shift])
        .group_by(|shift: &Shift| shift.employee_id, count());

    (
        by_employee
            .penalize(|_employee_id: &usize, count: &usize| SoftScore::of(*count as i64))
            .named("linear employee load"),
        by_employee
            .penalize(|_employee_id: &usize, count: &usize| {
                SoftScore::of((*count * *count) as i64)
            })
            .named("squared employee load"),
    )
}

fn main() {
    let mut constraints = constraints();
    let plan = Plan {
        shifts: vec![
            Shift { employee_id: 0 },
            Shift { employee_id: 0 },
            Shift { employee_id: 1 },
        ],
    };

    assert_eq!(constraints.initialize_all(&plan), SoftScore::of(-8));
}
