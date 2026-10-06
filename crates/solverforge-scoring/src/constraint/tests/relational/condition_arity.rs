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
use crate::stream::joiner::{equal_bi, less_than, overlapping};
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

// Equality composed with a comparison in one relationship: the equality keys
// the probe, the comparison stays an exact residual check.
fn equality_plus_comparison() -> impl IncrementalConstraint<Schedule, SoftScore> {
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
            equal_bi(|s: &Shift| s.employee_id, |e: &Employee| Some(e.id))
                .and(less_than(|s: &Shift| s.start, |_e: &Employee| 3i64)),
        ))
        .penalize(SoftScore::of(1))
        .named("equality plus comparison")
}

#[test]
fn equality_and_comparison_compose() {
    let schedule = Schedule {
        shifts: vec![
            Shift {
                employee_id: Some(0),
                start: 5,
                end: 6,
            }, // start >= 3: excluded by the comparison
            Shift {
                employee_id: Some(0),
                start: 1,
                end: 2,
            }, // start < 3: included
            Shift {
                employee_id: None,
                start: 0,
                end: 1,
            }, // no employee key: excluded by equality
        ],
        employees: vec![Employee { id: 0 }, Employee { id: 1 }],
    };
    let c = equality_plus_comparison();
    // Only the second shift joins employee 0 with start < 3.
    assert_eq!(c.evaluate(&schedule), SoftScore::of(-1));
    assert_eq!(c.match_count(&schedule), 1);
}

// Interval overlap in a chain: a shift overlaps an employee's availability
// window. Bounds are authored over entities and executed over leaf views.
fn overlap_condition_chain() -> impl IncrementalConstraint<Schedule, SoftScore> {
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
            overlapping(
                |s: &Shift| s.start,
                |s: &Shift| s.end,
                |e: &Employee| e.id as i64,
                |e: &Employee| e.id as i64 + 2,
            ),
        ))
        .penalize(SoftScore::of(1))
        .named("overlap chain")
}

#[test]
fn overlap_condition_composes_in_a_chain() {
    let schedule = Schedule {
        shifts: vec![
            Shift {
                employee_id: Some(0),
                start: 0,
                end: 2,
            }, // [0,2) overlaps employee 0's [0,2)
            Shift {
                employee_id: Some(0),
                start: 5,
                end: 6,
            }, // [5,6) overlaps nothing
        ],
        employees: vec![Employee { id: 0 }],
    };
    let c = overlap_condition_chain();
    // Only the first shift overlaps the single employee window [0,2).
    assert_eq!(c.evaluate(&schedule), SoftScore::of(-1));
    assert_eq!(c.match_count(&schedule), 1);
}

// Named key fns over nested rows: explicit lifetimes keep them higher-ranked.
type PairRow<'a> = crate::stream::relational::operator::Pair<
    crate::stream::relational::Leaf<'a, Shift>,
    crate::stream::relational::Leaf<'a, Employee>,
>;
fn pair_right_id(row: &PairRow<'_>) -> i64 {
    row.right.entity.id as i64
}
fn pair_right_start(row: &PairRow<'_>) -> i64 {
    row.left.entity.start
}
fn pair_right_end(row: &PairRow<'_>) -> i64 {
    row.left.entity.end
}
fn leaf_employee_id(row: &crate::stream::relational::Leaf<'_, Employee>) -> i64 {
    row.entity.id as i64
}
fn leaf_employee_end(row: &crate::stream::relational::Leaf<'_, Employee>) -> i64 {
    row.entity.id as i64 + 2
}

// A comparison and an overlap at the SECOND relationship of a tri chain, so
// non-equality conditions are proven at an arity above the first join.
fn later_arity_comparison() -> impl IncrementalConstraint<Schedule, SoftScore> {
    ConstraintFactory::<Schedule, SoftScore>::new()
        .for_each(source(
            shifts as fn(&Schedule) -> &[Shift],
            ChangeSource::Descriptor(0),
        ))
        // relationship 1: equality on the employee id
        .join((
            source(
                employees as fn(&Schedule) -> &[Employee],
                ChangeSource::Descriptor(1),
            ),
            equal_bi(|s: &Shift| s.employee_id, |e: &Employee| Some(e.id)),
        ))
        // relationship 2: comparison over the (shift, employee) row
        .join((
            source(
                employees as fn(&Schedule) -> &[Employee],
                ChangeSource::Descriptor(1),
            ),
            less_than(pair_right_id, leaf_employee_id),
        ))
        .penalize(SoftScore::of(1))
        .named("later arity comparison")
}

// The same second relationship written as an overlap.
fn later_arity_overlap() -> impl IncrementalConstraint<Schedule, SoftScore> {
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
            equal_bi(|s: &Shift| s.employee_id, |e: &Employee| Some(e.id)),
        ))
        .join((
            source(
                employees as fn(&Schedule) -> &[Employee],
                ChangeSource::Descriptor(1),
            ),
            overlapping(
                pair_right_start,
                pair_right_end,
                leaf_employee_id,
                leaf_employee_end,
            ),
        ))
        .penalize(SoftScore::of(1))
        .named("later arity overlap")
}

#[test]
fn comparison_composes_at_a_later_fluent_arity() {
    let schedule = Schedule {
        shifts: vec![
            Shift {
                employee_id: Some(0),
                start: 0,
                end: 2,
            },
            Shift {
                employee_id: Some(1),
                start: 0,
                end: 2,
            },
        ],
        employees: vec![Employee { id: 0 }, Employee { id: 1 }, Employee { id: 2 }],
    };
    // relationship 1 pairs shift.employee_id with employee.id.
    //   shift 0 -> employee 0; shift 1 -> employee 1.
    // relationship 2 is employee.id < other.id over the pair row.
    //   pair (s0,e0): 0 < {0,1,2} -> 2 matches.
    //   pair (s1,e1): 1 < {0,1,2} -> 1 match.
    assert_eq!(
        later_arity_comparison().evaluate(&schedule),
        SoftScore::of(-3)
    );
    assert_eq!(later_arity_comparison().match_count(&schedule), 3);
}

#[test]
fn overlap_composes_at_a_later_fluent_arity() {
    let schedule = Schedule {
        shifts: vec![Shift {
            employee_id: Some(0),
            start: 0,
            end: 2,
        }],
        employees: vec![Employee { id: 0 }, Employee { id: 1 }, Employee { id: 2 }],
    };
    // pair (s0,e0): shift window [0,2) vs each employee's [id, id+2).
    //   id 0: [0,2) overlaps [0,2) -> yes
    //   id 1: [0,2) vs [1,3) -> yes
    //   id 2: [0,2) vs [2,4) -> no (half-open)
    assert_eq!(later_arity_overlap().evaluate(&schedule), SoftScore::of(-2));
    assert_eq!(later_arity_overlap().match_count(&schedule), 2);
}
