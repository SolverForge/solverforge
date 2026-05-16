use solverforge::prelude::*;

struct Plan {
    shifts: Vec<(usize, i64, bool)>,
}

fn shifts(plan: &Plan) -> &[(usize, i64, bool)] {
    plan.shifts.as_slice()
}

#[solverforge_constraints]
fn constraints() -> impl ConstraintSet<Plan, SoftScore> {
    let g = ConstraintFactory::<Plan, SoftScore>::new();
    let nurse_presence = g
        .for_each(shifts as fn(&Plan) -> &[(usize, i64, bool)])
        .filter(|shift: &(usize, i64, bool)| shift.2)
        .group_by(
            |shift: &(usize, i64, bool)| shift.0,
            indexed_presence(|shift: &(usize, i64, bool)| shift.1),
        );

    (
        nurse_presence
            .penalize(|_nurse_id: &usize, presence: &IndexedPresence| {
                SoftScore::of(
                    presence
                        .runs()
                        .runs()
                        .iter()
                        .map(|run| run.point_count().saturating_sub(2) as i64)
                        .sum(),
                )
            })
            .named("consecutive work bounds"),
        nurse_presence
            .penalize(|_nurse_id: &usize, presence: &IndexedPresence| {
                SoftScore::of(
                    presence
                        .complement_runs(0..5)
                        .runs()
                        .iter()
                        .map(|run| run.point_count().saturating_sub(1) as i64)
                        .sum(),
                )
            })
            .named("consecutive off bounds"),
        nurse_presence
            .penalize(|_nurse_id: &usize, presence: &IndexedPresence| {
                SoftScore::of(if presence.any_in(5..7) { 1 } else { 0 })
            })
            .named("working weekends"),
    )
}

fn main() {
    let mut constraints = constraints();
    let plan = Plan {
        shifts: vec![(0, 0, true), (0, 1, true), (0, 2, true)],
    };

    assert_eq!(constraints.initialize_all(&plan), SoftScore::of(-2));
}
