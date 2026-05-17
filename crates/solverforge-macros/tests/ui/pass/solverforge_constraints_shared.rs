use solverforge::prelude::*;

struct Plan {
    shifts: Vec<usize>,
}

fn shifts(plan: &Plan) -> &[usize] {
    plan.shifts.as_slice()
}

#[solverforge_constraints]
fn constraints() -> impl ConstraintSet<Plan, SoftScore> {
    let g = ConstraintFactory::<Plan, SoftScore>::new();
    let by_employee = g
        .for_each(shifts as fn(&Plan) -> &[usize])
        .group_by(|employee_id: &usize| *employee_id, count());

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
        shifts: vec![0, 0, 1],
    };

    assert_eq!(constraints.initialize_all(&plan), SoftScore::of(-8));
}
