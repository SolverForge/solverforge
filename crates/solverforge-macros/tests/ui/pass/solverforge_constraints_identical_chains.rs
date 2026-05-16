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
fn inline_constraints() -> impl ConstraintSet<Plan, SoftScore> {
    (
        ConstraintFactory::<Plan, SoftScore>::new()
            .for_each(shifts as fn(&Plan) -> &[Shift])
            .group_by(|shift: &Shift| shift.employee_id, count())
            .penalize(|_employee_id: &usize, count: &usize| SoftScore::of(*count as i64))
            .named("linear employee load"),
        ConstraintFactory::<Plan, SoftScore>::new()
            .for_each(shifts as fn(&Plan) -> &[Shift])
            .group_by(|shift: &Shift| shift.employee_id, count())
            .penalize(|_employee_id: &usize, count: &usize| {
                SoftScore::of((*count * *count) as i64)
            })
            .named("squared employee load"),
    )
}

#[solverforge_constraints]
fn binding_constraints() -> impl ConstraintSet<Plan, SoftScore> {
    let first = ConstraintFactory::<Plan, SoftScore>::new()
        .for_each(shifts as fn(&Plan) -> &[Shift])
        .group_by(|shift: &Shift| shift.employee_id, count());
    let second = ConstraintFactory::<Plan, SoftScore>::new()
        .for_each(shifts as fn(&Plan) -> &[Shift])
        .group_by(|shift: &Shift| shift.employee_id, count());

    (
        first
            .penalize(|_employee_id: &usize, count: &usize| SoftScore::of(*count as i64))
            .named("linear employee load"),
        second
            .penalize(|_employee_id: &usize, count: &usize| {
                SoftScore::of((*count * *count) as i64)
            })
            .named("squared employee load"),
    )
}

fn main() {
    let plan = Plan {
        shifts: vec![
            Shift { employee_id: 0 },
            Shift { employee_id: 0 },
            Shift { employee_id: 1 },
        ],
    };

    assert_eq!(inline_constraints().evaluate_all(&plan), SoftScore::of(-8));
    assert_eq!(binding_constraints().evaluate_all(&plan), SoftScore::of(-8));
}
