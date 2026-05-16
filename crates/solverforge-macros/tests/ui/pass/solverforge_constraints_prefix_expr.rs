use std::sync::atomic::{AtomicUsize, Ordering};

use solverforge::prelude::*;

static VALIDATED: AtomicUsize = AtomicUsize::new(0);

struct Plan {
    shifts: Vec<usize>,
}

fn shifts(plan: &Plan) -> &[usize] {
    plan.shifts.as_slice()
}

#[solverforge_constraints]
fn constraints() -> impl ConstraintSet<Plan, SoftScore> {
    if cfg!(debug_assertions) {
        VALIDATED.fetch_add(1, Ordering::SeqCst);
    }
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
    VALIDATED.store(0, Ordering::SeqCst);
    let mut constraints = constraints();
    assert_eq!(VALIDATED.load(Ordering::SeqCst), 1);
    let plan = Plan {
        shifts: vec![0, 0, 1],
    };

    assert_eq!(constraints.initialize_all(&plan), SoftScore::of(-8));
}
