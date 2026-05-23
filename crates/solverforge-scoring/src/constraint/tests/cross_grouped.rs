use crate::api::constraint_set::ConstraintSet;
use crate::constraint::cross_grouped::{
    CrossGroupedNodeState, CrossGroupedTerminalScorer, SharedCrossGroupedConstraintSet,
};
use crate::stream::collection_extract::{source, ChangeSource};
use crate::stream::collector::sum;
use solverforge_core::score::SoftScore;
use solverforge_core::{ConstraintRef, ImpactType};

struct Employee {
    id: usize,
}

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
    let state = CrossGroupedNodeState::new(
        source(
            (|schedule: &Schedule| schedule.shifts.as_slice()) as fn(&Schedule) -> &[Shift],
            ChangeSource::Descriptor(0),
        ),
        source(
            (|schedule: &Schedule| schedule.employees.as_slice()) as fn(&Schedule) -> &[Employee],
            ChangeSource::Descriptor(1),
        ),
        |shift: &Shift| shift.employee_id,
        |employee: &Employee| Some(employee.id),
        |_schedule: &Schedule,
         _shift: &Shift,
         _employee: &Employee,
         _shift_idx: usize,
         _employee_idx: usize| true,
        |_shift: &Shift, employee: &Employee| employee.id,
        sum(|(_shift, _employee): (&Shift, &Employee)| 1i64),
    );
    let scorers = (
        CrossGroupedTerminalScorer::new(
            ConstraintRef::new("", "linear assigned shift count"),
            ImpactType::Penalty,
            |_employee_id: &usize, count: &i64| SoftScore::of(*count),
            false,
        ),
        CrossGroupedTerminalScorer::new(
            ConstraintRef::new("", "squared assigned shift count"),
            ImpactType::Penalty,
            |_employee_id: &usize, count: &i64| SoftScore::of(count * count),
            false,
        ),
    );
    let mut constraints = SharedCrossGroupedConstraintSet::new(state, scorers);
    let plan = schedule();

    assert_eq!(constraints.evaluate_all(&plan), SoftScore::of(-6));
    assert_eq!(constraints.initialize_all(&plan), SoftScore::of(-6));
    let metadata = constraints.constraint_metadata();
    assert_eq!(metadata[0].name(), "linear assigned shift count");
    assert_eq!(metadata[1].name(), "squared assigned shift count");

    assert_eq!(constraints.on_retract_all(&plan, 0, 0), SoftScore::of(4));
    assert_eq!(constraints.state().update_count(), 1);
    assert_eq!(constraints.state().changed_key_count(), 1);
    assert_eq!(constraints.on_insert_all(&plan, 0, 0), SoftScore::of(-4));
    assert_eq!(constraints.state().update_count(), 2);
    assert_eq!(constraints.state().changed_key_count(), 2);
}
