use solverforge::prelude::*;

struct Plan {
    shifts: Vec<(usize, usize)>,
}

fn shifts(plan: &Plan) -> &[(usize, usize)] {
    plan.shifts.as_slice()
}

#[solverforge_constraints]
fn constraints() -> impl ConstraintSet<Plan, SoftScore> {
    let by_employee = ConstraintFactory::<Plan, SoftScore>::new()
        .for_each(shifts as fn(&Plan) -> &[(usize, usize)])
        .group_by(|shift: &(usize, usize)| shift.0, count());
    let by_day = ConstraintFactory::<Plan, SoftScore>::new()
        .for_each(shifts as fn(&Plan) -> &[(usize, usize)])
        .group_by(|shift: &(usize, usize)| shift.1, count());

    (
        by_employee
            .penalize(|_employee_id: &usize, count: &usize| SoftScore::of(*count as i64))
            .named("employee linear"),
        by_day
            .penalize(|_day: &usize, count: &usize| SoftScore::of(*count as i64))
            .named("day linear"),
        by_employee
            .penalize(|_employee_id: &usize, count: &usize| {
                SoftScore::of((*count * *count) as i64)
            })
            .named("employee square"),
        by_day
            .penalize(|_day: &usize, count: &usize| SoftScore::of((*count * *count) as i64))
            .named("day square"),
    )
}

fn main() {
    let mut constraints = constraints();
    let plan = Plan {
        shifts: vec![(0, 0), (0, 1), (1, 1)],
    };

    assert_eq!(constraints.initialize_all(&plan), SoftScore::of(-16));
    let results = constraints.evaluate_each(&plan);
    assert_eq!(results[0].name, "employee linear");
    assert_eq!(results[1].name, "day linear");
    assert_eq!(results[2].name, "employee square");
    assert_eq!(results[3].name, "day square");
}
