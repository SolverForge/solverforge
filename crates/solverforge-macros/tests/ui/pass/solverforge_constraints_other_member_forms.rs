use solverforge::prelude::*;
use solverforge::IncrementalConstraint;

struct Plan {
    shifts: Vec<usize>,
}

fn shifts(plan: &Plan) -> &[usize] {
    plan.shifts.as_slice()
}

fn fixed_one() -> impl IncrementalConstraint<Plan, SoftScore> {
    ConstraintFactory::<Plan, SoftScore>::new()
        .for_each(shifts as fn(&Plan) -> &[usize])
        .penalize(SoftScore::ONE)
        .named("fixed one")
}

fn fixed_constraints() -> impl ConstraintSet<Plan, SoftScore> {
    let fixed_two = ConstraintFactory::<Plan, SoftScore>::new()
        .for_each(shifts as fn(&Plan) -> &[usize])
        .penalize(SoftScore::ONE)
        .named("fixed duplicate");
    let fixed_three = ConstraintFactory::<Plan, SoftScore>::new()
        .for_each(shifts as fn(&Plan) -> &[usize])
        .reward(SoftScore::ONE)
        .named("fixed duplicate");

    (fixed_two, fixed_three)
}

#[solverforge_constraints]
fn call_single_constraints() -> impl ConstraintSet<Plan, SoftScore> {
    let by_employee = ConstraintFactory::<Plan, SoftScore>::new()
        .for_each(shifts as fn(&Plan) -> &[usize])
        .group_by(|employee_id: &usize| *employee_id, count());

    (
        by_employee
            .penalize(|_employee_id: &usize, count: &usize| SoftScore::of(*count as i64))
            .named("call linear"),
        fixed_one(),
        fixed_one(),
        by_employee
            .penalize(|_employee_id: &usize, count: &usize| {
                SoftScore::of((*count * *count) as i64)
            })
            .named("call squared"),
    )
}

#[solverforge_constraints]
fn bound_set_constraints() -> impl ConstraintSet<Plan, SoftScore> {
    let by_employee = ConstraintFactory::<Plan, SoftScore>::new()
        .for_each(shifts as fn(&Plan) -> &[usize])
        .group_by(|employee_id: &usize| *employee_id, count());
    let fixed = fixed_constraints();

    (
        by_employee
            .penalize(|_employee_id: &usize, count: &usize| SoftScore::of(*count as i64))
            .named("bound linear"),
        fixed,
        by_employee
            .penalize(|_employee_id: &usize, count: &usize| {
                SoftScore::of((*count * *count) as i64)
            })
            .named("bound squared"),
    )
}

fn main() {
    let plan = Plan {
        shifts: vec![0, 0, 1],
    };

    let call_constraints = call_single_constraints();
    let call_results = call_constraints.evaluate_each(&plan);
    assert_eq!(call_results.len(), 4);
    assert_eq!(call_results[0].name, "call linear");
    assert_eq!(call_results[1].name, "fixed one");
    assert_eq!(call_results[2].name, "fixed one");
    assert_eq!(call_results[3].name, "call squared");

    let call_metadata = call_constraints.constraint_metadata();
    assert_eq!(call_metadata.len(), 3);
    assert_eq!(call_metadata[0].name(), "call linear");
    assert_eq!(call_metadata[1].name(), "fixed one");
    assert_eq!(call_metadata[2].name(), "call squared");

    let bound_constraints = bound_set_constraints();
    let bound_results = bound_constraints.evaluate_each(&plan);
    assert_eq!(bound_results.len(), 4);
    assert_eq!(bound_results[0].name, "bound linear");
    assert_eq!(bound_results[1].name, "fixed duplicate");
    assert_eq!(bound_results[2].name, "fixed duplicate");
    assert_eq!(bound_results[3].name, "bound squared");

    let bound_metadata = bound_constraints.constraint_metadata();
    assert_eq!(bound_metadata.len(), 3);
    assert_eq!(bound_metadata[0].name(), "bound linear");
    assert_eq!(bound_metadata[1].name(), "fixed duplicate");
    assert_eq!(bound_metadata[2].name(), "bound squared");
}
