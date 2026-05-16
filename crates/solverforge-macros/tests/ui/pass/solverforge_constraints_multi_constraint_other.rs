use solverforge::prelude::*;

struct Plan {
    shifts: Vec<usize>,
}

fn shifts(plan: &Plan) -> &[usize] {
    plan.shifts.as_slice()
}

fn fixed_constraints() -> impl ConstraintSet<Plan, SoftScore> {
    let g = ConstraintFactory::<Plan, SoftScore>::new();
    let fixed_one = g
        .for_each(shifts as fn(&Plan) -> &[usize])
        .penalize(SoftScore::ONE)
        .named("fixed one");
    let fixed_two = ConstraintFactory::<Plan, SoftScore>::new()
        .for_each(shifts as fn(&Plan) -> &[usize])
        .reward(SoftScore::ONE)
        .named("fixed two");

    (fixed_one, fixed_two)
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
        fixed_constraints(),
        by_employee
            .penalize(|_employee_id: &usize, count: &usize| {
                SoftScore::of((*count * *count) as i64)
            })
            .named("squared employee load"),
    )
}

fn main() {
    let constraints = constraints();
    let plan = Plan {
        shifts: vec![0, 0, 1],
    };

    let results = constraints.evaluate_each(&plan);
    assert_eq!(results.len(), 4);
    assert_eq!(results[0].name, "linear employee load");
    assert_eq!(results[1].name, "fixed one");
    assert_eq!(results[2].name, "fixed two");
    assert_eq!(results[3].name, "squared employee load");

    let analyses = constraints.evaluate_detailed(&plan);
    assert_eq!(analyses.len(), 4);
    assert_eq!(analyses[0].constraint_ref.name, "linear employee load");
    assert_eq!(analyses[1].constraint_ref.name, "fixed one");
    assert_eq!(analyses[2].constraint_ref.name, "fixed two");
    assert_eq!(analyses[3].constraint_ref.name, "squared employee load");

    let metadata = constraints.constraint_metadata();
    assert_eq!(metadata.len(), 4);
    assert_eq!(metadata[0].name(), "linear employee load");
    assert_eq!(metadata[1].name(), "fixed one");
    assert_eq!(metadata[2].name(), "fixed two");
    assert_eq!(metadata[3].name(), "squared employee load");
}
