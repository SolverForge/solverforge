/* Higher-arity condition composition and complement key-skipping.

Proves two acceptance points at the surfaces the fluent chain now reaches:

- Comparison and predicate conditions compose inside a chain (not only
  equality), at the arity the fluent API can express.
- `complement_with_key` treats a `None` key as "skip": a None-keyed source
  entity contributes no complement row, while `Some` keys match targets.
*/

use solverforge_core::score::SoftScore;

use crate::api::constraint_set::IncrementalConstraint;
use crate::stream::collection_extract::{source, ChangeSource};
use crate::stream::collector::count;
use crate::stream::joiner::less_than;
use crate::stream::ConstraintFactory;

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct Shift {
    employee_id: Option<usize>,
    start: i64,
    end: i64,
}

#[derive(Clone, Hash, PartialEq, Eq, Debug)]
struct Employee {
    id: usize,
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

// Count shifts per employee, complementing absent employees with 0. The None
// key (unassigned shift) must contribute no complement row.
fn complement_skips_none_keys() -> impl IncrementalConstraint<Schedule, SoftScore> {
    ConstraintFactory::<Schedule, SoftScore>::new()
        .for_each(source(
            shifts as fn(&Schedule) -> &[Shift],
            ChangeSource::Descriptor(0),
        ))
        .group_by(|s: &Shift| s.employee_id.unwrap_or(usize::MAX), count())
        .complement_with_key(
            source(
                employees as fn(&Schedule) -> &[Employee],
                ChangeSource::Descriptor(1),
            ),
            |s: &Shift| s.employee_id, // Option<usize>: None = skip
            |e: &Employee| e.id,
            |_e| 0usize,
        )
        .penalize(|_id: &usize, count: &usize| SoftScore::of(*count as i64))
        .named("complement with key")
}

#[test]
fn complement_with_key_skips_none_keyed_entities() {
    let schedule = Schedule {
        shifts: vec![
            Shift {
                employee_id: Some(0),
                start: 0,
                end: 1,
            },
            Shift {
                employee_id: Some(0),
                start: 1,
                end: 2,
            },
            Shift {
                employee_id: None, // skipped: no complement row
                start: 2,
                end: 3,
            },
        ],
        employees: vec![Employee { id: 0 }, Employee { id: 1 }],
    };
    let c = complement_skips_none_keys();
    // employee 0: 2 -> -2; employee 1: 0 (default) -> 0; None key contributes
    // nothing. Total -2.
    assert_eq!(c.evaluate(&schedule), SoftScore::of(-2));
    assert_eq!(c.match_count(&schedule), 2);
}

// A comparison condition inside a chain: assignments to employees whose id is
// less than the assignment's shift start.
fn comparison_condition_chain() -> impl IncrementalConstraint<Schedule, SoftScore> {
    ConstraintFactory::<Schedule, SoftScore>::new()
        .for_each(source(
            shifts as fn(&Schedule) -> &[Shift],
            ChangeSource::Descriptor(0),
        ))
        .join((
            source(
                employees as fn(&Schedule) -> &[Employee],
                ChangeSource::Descriptor(1),
            ),
            // matches when shift.start < employee.id
            less_than(|s: &Shift| s.start, |e: &Employee| e.id as i64),
        ))
        .penalize(SoftScore::of(1))
        .named("comparison chain")
}

#[test]
fn comparison_condition_composes_in_a_chain() {
    let schedule = Schedule {
        shifts: vec![
            Shift {
                employee_id: Some(0),
                start: 5,
                end: 6,
            },
            Shift {
                employee_id: Some(0),
                start: 1,
                end: 2,
            },
        ],
        employees: vec![Employee { id: 0 }, Employee { id: 3 }],
    };
    let c = comparison_condition_chain();
    // Relationship is shift.start < employee.id.
    // start 5: 5<0 false, 5<3 false. start 1: 1<0 false, 1<3 true.
    // Exactly one matching pair.
    assert_eq!(c.evaluate(&schedule), SoftScore::of(-1));
    assert_eq!(c.match_count(&schedule), 1);
}
