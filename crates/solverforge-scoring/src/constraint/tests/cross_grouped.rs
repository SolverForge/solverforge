use crate::api::constraint_set::ConstraintSet;
use crate::stream::collection_extract::{source, ChangeSource};
use crate::stream::collector::sum;
use crate::stream::joiner::equal_bi;
use crate::stream::ConstraintFactory;
use solverforge_core::score::SoftScore;

#[derive(Clone)]
struct Employee {
    id: usize,
}

#[derive(Clone)]
struct Shift {
    employee_id: Option<usize>,
}

struct Schedule {
    shifts: Vec<Shift>,
    employees: Vec<Employee>,
}

fn schedule() -> Schedule {
    Schedule {
        shifts: vec![
            Shift {
                employee_id: Some(0),
            },
            Shift {
                employee_id: Some(0),
            },
        ],
        employees: vec![Employee { id: 0 }, Employee { id: 1 }],
    }
}

#[test]
fn shared_cross_grouped_set_updates_one_join_node_for_multiple_terminals() {
    let mut constraints = ConstraintFactory::<Schedule, SoftScore>::new()
        .for_each(source(
            (|schedule: &Schedule| schedule.shifts.as_slice()) as fn(&Schedule) -> &[Shift],
            ChangeSource::Descriptor(0),
        ))
        .join((
            source(
                (|schedule: &Schedule| schedule.employees.as_slice())
                    as fn(&Schedule) -> &[Employee],
                ChangeSource::Descriptor(1),
            ),
            equal_bi(
                |shift: &Shift| shift.employee_id,
                |employee: &Employee| Some(employee.id),
            ),
        ))
        .group_by(
            |_shift: &Shift, employee: &Employee| employee.id,
            sum(|(_shift, _employee): (&Shift, &Employee)| 1i64),
        )
        .penalize(|_employee_id: &usize, count: &i64| SoftScore::of(*count))
        .named("linear assigned shift count")
        .penalize(|_employee_id: &usize, count: &i64| SoftScore::of(count * count))
        .named("squared assigned shift count");
    let plan = schedule();

    assert_eq!(constraints.evaluate_all(&plan), SoftScore::of(-6));
    assert_eq!(constraints.initialize_all(&plan), SoftScore::of(-6));
    let metadata = constraints.constraint_metadata();
    assert_eq!(metadata[0].name(), "linear assigned shift count");
    assert_eq!(metadata[1].name(), "squared assigned shift count");

    assert_eq!(constraints.on_retract_all(&plan, 0, 0), SoftScore::of(4));
    assert_eq!(constraints.update_count(), 1);
    assert_eq!(constraints.changed_key_count(), 1);
    assert_eq!(constraints.on_insert_all(&plan, 0, 0), SoftScore::of(-4));
    assert_eq!(constraints.update_count(), 2);
    assert_eq!(constraints.changed_key_count(), 2);
}
