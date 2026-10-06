/* Fluent grouped-over-join: `group_by(...).join((collection, key))`.

A group (key + aggregate) is related to a second collection by the group key
and scored through the shared operator terminal — the grouped result is an
upstream row producer, not a finalized score. This is the fluent surface the
operator-level `group_producer` tests exercise directly.
*/

use solverforge_core::score::SoftScore;

use crate::api::constraint_set::IncrementalConstraint;
use crate::stream::collection_extract::{source, ChangeSource};
use crate::stream::collector::count;
use crate::stream::ConstraintFactory;

#[derive(Clone, Hash, PartialEq, Eq, Debug)]
struct Shift {
    employee_id: u32,
}

#[derive(Clone, Hash, PartialEq, Eq, Debug)]
struct Employee {
    id: u32,
    budget: i64,
}

#[derive(Clone)]
struct Schedule {
    shifts: Vec<Shift>,
    employees: Vec<Employee>,
}

fn shifts(s: &Schedule) -> &[Shift] {
    s.shifts.as_slice()
}
fn employees(s: &Schedule) -> &[Employee] {
    s.employees.as_slice()
}

// Reward each employee's budget times the number of shifts assigned to them.
fn budget_weighted_coverage() -> impl IncrementalConstraint<Schedule, SoftScore> {
    ConstraintFactory::<Schedule, SoftScore>::new()
        .for_each(source(
            shifts as fn(&Schedule) -> &[Shift],
            ChangeSource::Descriptor(0),
        ))
        .group_by(|s: &Shift| s.employee_id, count())
        .join((
            source(
                employees as fn(&Schedule) -> &[Employee],
                ChangeSource::Descriptor(1),
            ),
            |e: &Employee| e.id,
        ))
        .reward(|_emp_id: &u32, count: &usize, employee: &Employee| {
            SoftScore::of(*count as i64 * employee.budget)
        })
        .named("budget weighted coverage")
}

fn schedule() -> Schedule {
    Schedule {
        shifts: vec![
            Shift { employee_id: 0 },
            Shift { employee_id: 0 },
            Shift { employee_id: 1 },
        ],
        employees: vec![Employee { id: 0, budget: 3 }, Employee { id: 1, budget: 5 }],
    }
}

#[test]
fn grouped_result_joins_by_key_and_scores() {
    let c = budget_weighted_coverage();
    let s = schedule();
    // employee 0: count 2 x budget 3 = 6; employee 1: count 1 x budget 5 = 5.
    assert_eq!(c.evaluate(&s), SoftScore::of(11));
    assert_eq!(c.match_count(&s), 2);
}

#[test]
fn group_join_incremental_matches_full_evaluation() {
    let mut c = budget_weighted_coverage();
    let mut s = schedule();
    let mut running = c.initialize(&s);
    assert_eq!(running, c.evaluate(&s));

    // Add a shift for employee 1: count 2 x 5 = 10, up 5.
    s.shifts.push(Shift { employee_id: 1 });
    running = running + c.on_insert(&s, 3, 0);
    assert_eq!(running, c.evaluate(&s));
    assert_eq!(c.match_count(&s), 2);

    // Add a new employee with budget 2 and no shifts: absent group -> default 0.
    s.employees.push(Employee { id: 2, budget: 2 });
    running = running + c.on_insert(&s, 2, 1);
    assert_eq!(running, c.evaluate(&s));
}

// A source filter placed before `group_by` stays binding for the grouped row.
fn filtered_budget_coverage() -> impl IncrementalConstraint<Schedule, SoftScore> {
    ConstraintFactory::<Schedule, SoftScore>::new()
        .for_each(source(
            shifts as fn(&Schedule) -> &[Shift],
            ChangeSource::Descriptor(0),
        ))
        // Only employee 0's shifts count toward the group.
        .filter(|s: &Shift| s.employee_id == 0)
        .group_by(|s: &Shift| s.employee_id, count())
        .join((
            source(
                employees as fn(&Schedule) -> &[Employee],
                ChangeSource::Descriptor(1),
            ),
            |e: &Employee| e.id,
        ))
        .reward(|_emp_id: &u32, count: &usize, employee: &Employee| {
            SoftScore::of(*count as i64 * employee.budget)
        })
        .named("filtered budget coverage")
}

#[test]
fn source_filter_stays_binding_after_grouping() {
    let c = filtered_budget_coverage();
    let s = schedule();
    // Only employee 0's two shifts group; employee 1 has no group, so it is
    // absent from the join. employee 0: 2 x 3 = 6.
    assert_eq!(c.evaluate(&s), SoftScore::of(6));
    assert_eq!(c.match_count(&s), 1);
}
