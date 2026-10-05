use solverforge::prelude::*;

struct Schedule {
    shifts: Vec<Option<usize>>,
    employees: Vec<usize>,
    days_off: Vec<Option<usize>>,
}

fn shifts(schedule: &Schedule) -> &[Option<usize>] {
    schedule.shifts.as_slice()
}

fn employees(schedule: &Schedule) -> &[usize] {
    schedule.employees.as_slice()
}

fn days_off(schedule: &Schedule) -> &[Option<usize>] {
    schedule.days_off.as_slice()
}

#[solverforge_constraints]
fn constraints() -> impl ConstraintSet<Schedule, SoftScore> {
    let factory = ConstraintFactory::<Schedule, SoftScore>::new();
    (
        factory
            .for_each(shifts as fn(&Schedule) -> &[Option<usize>])
            .join((
                employees as fn(&Schedule) -> &[usize],
                joiner::equal_bi(
                    |shift: &Option<usize>| *shift,
                    |employee: &usize| Some(*employee),
                ),
            ))
            .join((
                days_off as fn(&Schedule) -> &[Option<usize>],
                joiner::filtering_on(
                    |row: &solverforge::stream::relational::operator::Pair<
                        solverforge::stream::relational::Leaf<'_, Option<usize>>,
                        solverforge::stream::relational::Leaf<'_, usize>,
                    >,
                     day_off: &solverforge::stream::relational::Leaf<'_, Option<usize>>| {
                        row.left.entity == day_off.entity
                    },
                ),
            ))
            .penalize(|shift: &Option<usize>, _employee: &usize, day_off: &Option<usize>| {
                SoftScore::of((*shift == *day_off) as i64)
            })
            .named("shift matches day off"),
    )
}

fn main() {
    let mut constraints = constraints();
    let schedule = Schedule {
        shifts: vec![Some(0), Some(1)],
        employees: vec![0, 1],
        days_off: vec![Some(0), Some(2)],
    };

    // (shift Some(0), employee 0, day_off Some(0)) matches; the shift-1 row
    // pairs with no day-off entry.
    assert_eq!(constraints.initialize_all(&schedule), SoftScore::of(-1));
    assert_eq!(constraints.evaluate_all(&schedule), SoftScore::of(-1));
}
