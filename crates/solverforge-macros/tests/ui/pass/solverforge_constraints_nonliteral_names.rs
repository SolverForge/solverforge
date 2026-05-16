use solverforge::prelude::*;

const LINEAR_NAME: &str = "linear employee load";

struct Plan {
    shifts: Vec<usize>,
}

fn shifts(plan: &Plan) -> &[usize] {
    plan.shifts.as_slice()
}

#[solverforge_constraints]
fn shared_constraints() -> impl ConstraintSet<Plan, SoftScore> {
    let g = ConstraintFactory::<Plan, SoftScore>::new();
    let by_employee = g
        .for_each(shifts as fn(&Plan) -> &[usize])
        .group_by(|employee_id: &usize| *employee_id, count());
    let squared_name = "squared employee load";

    (
        by_employee
            .penalize(|_employee_id: &usize, count: &usize| SoftScore::of(*count as i64))
            .named(LINEAR_NAME),
        by_employee
            .penalize(|_employee_id: &usize, count: &usize| {
                SoftScore::of((*count * *count) as i64)
            })
            .named(squared_name),
    )
}

#[solverforge_constraints]
fn passthrough_constraints() -> impl ConstraintSet<Plan, SoftScore> {
    let name = "fixed";

    (
        ConstraintFactory::<Plan, SoftScore>::new()
            .for_each(shifts as fn(&Plan) -> &[usize])
            .penalize(SoftScore::ONE)
            .named(name),
    )
}

fn main() {
    let mut shared = shared_constraints();
    let mut passthrough = passthrough_constraints();
    let plan = Plan {
        shifts: vec![0, 0, 1],
    };

    assert_eq!(shared.initialize_all(&plan), SoftScore::of(-8));
    assert_eq!(passthrough.initialize_all(&plan), SoftScore::of(-3));
    let results = shared.evaluate_each(&plan);
    assert_eq!(results[0].name, LINEAR_NAME);
    assert_eq!(results[1].name, "squared employee load");
}
