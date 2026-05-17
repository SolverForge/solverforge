use solverforge::prelude::*;
use solverforge::prelude::ConstraintSet as CSet;
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

fn fixed_named(name: String) -> impl ConstraintSet<Plan, SoftScore> {
    ConstraintFactory::<Plan, SoftScore>::new()
        .for_each(shifts as fn(&Plan) -> &[usize])
        .penalize(SoftScore::ONE)
        .named(name.as_str())
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
fn moved_name_constraints() -> impl ConstraintSet<Plan, SoftScore> {
    let by_employee = ConstraintFactory::<Plan, SoftScore>::new()
        .for_each(shifts as fn(&Plan) -> &[usize])
        .group_by(|employee_id: &usize| *employee_id, count());
    let name = String::from("moved fixed");

    (
        by_employee
            .penalize(|_employee_id: &usize, count: &usize| SoftScore::of(*count as i64))
            .named(name.as_str()),
        fixed_named(name),
        by_employee
            .penalize(|_employee_id: &usize, count: &usize| {
                SoftScore::of((*count * *count) as i64)
            })
            .named("after move"),
    )
}

#[solverforge_constraints]
fn empty_tuple_constraints() -> impl ConstraintSet<Plan, SoftScore> {
    let by_employee = ConstraintFactory::<Plan, SoftScore>::new()
        .for_each(shifts as fn(&Plan) -> &[usize])
        .group_by(|employee_id: &usize| *employee_id, count());

    (
        by_employee
            .penalize(|_employee_id: &usize, count: &usize| SoftScore::of(*count as i64))
            .named("empty linear"),
        (),
        by_employee
            .penalize(|_employee_id: &usize, count: &usize| {
                SoftScore::of((*count * *count) as i64)
            })
            .named("empty squared"),
    )
}

#[solverforge_constraints]
fn alias_terminal_constraints() -> impl CSet<Plan, SoftScore> {
    let by_employee = ConstraintFactory::<Plan, SoftScore>::new()
        .for_each(shifts as fn(&Plan) -> &[usize])
        .group_by(|employee_id: &usize| *employee_id, count());

    (
        by_employee
            .penalize(|_employee_id: &usize, count: &usize| SoftScore::of(*count as i64))
            .named("alias linear"),
        by_employee
            .penalize(|_employee_id: &usize, count: &usize| {
                SoftScore::of((*count * *count) as i64)
            })
            .named("alias squared"),
    )
}

#[solverforge_constraints]
fn alias_empty_tuple_constraints() -> impl CSet<Plan, SoftScore> {
    let by_employee = ConstraintFactory::<Plan, SoftScore>::new()
        .for_each(shifts as fn(&Plan) -> &[usize])
        .group_by(|employee_id: &usize| *employee_id, count());

    (
        by_employee
            .penalize(|_employee_id: &usize, count: &usize| SoftScore::of(*count as i64))
            .named("alias empty linear"),
        (),
        by_employee
            .penalize(|_employee_id: &usize, count: &usize| {
                SoftScore::of((*count * *count) as i64)
            })
            .named("alias empty squared"),
    )
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

    let moved_constraints = moved_name_constraints();
    let moved_results = moved_constraints.evaluate_each(&plan);
    assert_eq!(moved_results.len(), 3);
    assert_eq!(moved_results[0].name, "moved fixed");
    assert_eq!(moved_results[1].name, "moved fixed");
    assert_eq!(moved_results[2].name, "after move");

    let empty_constraints = empty_tuple_constraints();
    let empty_results = empty_constraints.evaluate_each(&plan);
    assert_eq!(empty_results.len(), 2);
    assert_eq!(empty_results[0].name, "empty linear");
    assert_eq!(empty_results[1].name, "empty squared");

    let alias_constraints = alias_terminal_constraints();
    let alias_results = alias_constraints.evaluate_each(&plan);
    assert_eq!(alias_results.len(), 2);
    assert_eq!(alias_results[0].name, "alias linear");
    assert_eq!(alias_results[1].name, "alias squared");

    let alias_empty_constraints = alias_empty_tuple_constraints();
    let alias_empty_results = alias_empty_constraints.evaluate_each(&plan);
    assert_eq!(alias_empty_results.len(), 2);
    assert_eq!(alias_empty_results[0].name, "alias empty linear");
    assert_eq!(alias_empty_results[1].name, "alias empty squared");
}
