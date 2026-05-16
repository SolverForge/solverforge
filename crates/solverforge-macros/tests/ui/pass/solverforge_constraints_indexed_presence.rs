use solverforge::prelude::*;

#[derive(Clone)]
struct Shift {
    nurse_id: usize,
    day: i64,
    assigned: bool,
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
    let nurse_presence = g
        .for_each(shifts as fn(&Plan) -> &[Shift])
        .filter(|shift: &Shift| shift.assigned)
        .group_by(
            |shift: &Shift| shift.nurse_id,
            indexed_presence(|shift: &Shift| shift.day),
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
        shifts: vec![
            Shift {
                nurse_id: 0,
                day: 0,
                assigned: true,
            },
            Shift {
                nurse_id: 0,
                day: 1,
                assigned: true,
            },
            Shift {
                nurse_id: 0,
                day: 2,
                assigned: true,
            },
        ],
    };

    assert_eq!(constraints.initialize_all(&plan), SoftScore::of(-2));
}
