/* Planning ids in constraint analysis.

Every entity a match names carries the planning id its source renders: here
`shift-<id>` and `nurse-<id>` from test renderings, and none from a plain
source. Each operator family must hand the right source row to the
explanation, on both explanation paths: a full evaluation before
initialization, and the retained rows after it, including after a mutation
has moved rows around. Match scores keep summing to the constraint's score.

Roster: nurse 0 works shifts 0 and 1 (day 0), nurse 1 works shift 2 (day 1),
shift 3 is unassigned (day 1). Nurse 1 is unavailable on day 1.
*/

use solverforge_core::score::SoftScore;

use crate::api::analysis::DetailedConstraintMatch;
use crate::api::constraint_set::IncrementalConstraint;
use crate::stream::collection_extract::{source, ChangeSource, SourceExtract, SourcePlanningIds};
use crate::stream::collector::count;
use crate::stream::joiner::{equal, equal_bi};
use crate::stream::ConstraintFactory;

#[derive(Debug)]
struct Nurse {
    id: usize,
    unavailable_days: Vec<usize>,
}

// The self-join stream keys its pairs by entity, so shifts are `Hash + Eq`.
#[derive(Debug, Hash, PartialEq, Eq)]
struct Shift {
    id: usize,
    day: usize,
    nurse: Option<usize>,
}

struct Roster {
    shifts: Vec<Shift>,
    nurses: Vec<Nurse>,
}

struct ShiftIds;

impl SourcePlanningIds<Shift> for ShiftIds {
    fn planning_id(shift: &Shift) -> Option<String> {
        Some(format!("shift-{}", shift.id))
    }
}

struct NurseIds;

impl SourcePlanningIds<Nurse> for NurseIds {
    fn planning_id(nurse: &Nurse) -> Option<String> {
        Some(format!("nurse-{}", nurse.id))
    }
}

fn shifts(roster: &Roster) -> &[Shift] {
    roster.shifts.as_slice()
}

fn nurses(roster: &Roster) -> &[Nurse] {
    roster.nurses.as_slice()
}

fn shift_source() -> SourceExtract<fn(&Roster) -> &[Shift], ShiftIds> {
    SourceExtract::new(
        shifts as fn(&Roster) -> &[Shift],
        ChangeSource::Descriptor(0),
    )
}

fn nurse_source() -> SourceExtract<fn(&Roster) -> &[Nurse], NurseIds> {
    SourceExtract::new(
        nurses as fn(&Roster) -> &[Nurse],
        ChangeSource::Descriptor(1),
    )
}

fn roster() -> Roster {
    let shift = |id, day, nurse| Shift { id, day, nurse };
    Roster {
        shifts: vec![
            shift(0, 0, Some(0)),
            shift(1, 0, Some(0)),
            shift(2, 1, Some(1)),
            shift(3, 1, None),
        ],
        nurses: vec![
            Nurse {
                id: 0,
                unavailable_days: Vec::new(),
            },
            Nurse {
                id: 1,
                unavailable_days: vec![1],
            },
        ],
    }
}

fn factory() -> ConstraintFactory<Roster, SoftScore> {
    ConstraintFactory::<Roster, SoftScore>::new()
}

/// Each match as (score, planning ids of its entities), sorted.
fn ids(matches: &[DetailedConstraintMatch<'_, SoftScore>]) -> Vec<(i64, Vec<Option<String>>)> {
    let mut rows = matches
        .iter()
        .map(|detailed| {
            (
                detailed.score.score(),
                detailed
                    .justification
                    .entities
                    .iter()
                    .map(|entity| entity.planning_id.clone())
                    .collect(),
            )
        })
        .collect::<Vec<_>>();
    rows.sort();
    rows
}

fn id(name: &str) -> Option<String> {
    Some(name.to_string())
}

fn match_sum(matches: &[DetailedConstraintMatch<'_, SoftScore>]) -> SoftScore {
    matches
        .iter()
        .fold(SoftScore::of(0), |total, detailed| total + detailed.score)
}

/// Matches carry `expected` ids before and after initialization, and
/// `moved` ids after shift 1 moves from nurse 0 to nurse 1 on day 1.
fn assert_ids<C>(
    mut constraint: C,
    expected: Vec<(i64, Vec<Option<String>>)>,
    moved: Vec<(i64, Vec<Option<String>>)>,
) where
    C: IncrementalConstraint<Roster, SoftScore>,
{
    let mut plan = roster();
    let before = constraint.get_matches(&plan);
    assert_eq!(match_sum(&before), constraint.evaluate(&plan));
    assert_eq!(ids(&before), expected, "before initialization");
    drop(before);

    let total = constraint.initialize(&plan);
    let after = constraint.get_matches(&plan);
    assert_eq!(match_sum(&after), total);
    assert_eq!(ids(&after), expected, "after initialization");
    drop(after);

    constraint.on_retract(&plan, 1, 0);
    plan.shifts[1].nurse = Some(1);
    plan.shifts[1].day = 1;
    constraint.on_insert(&plan, 1, 0);
    let after_move = constraint.get_matches(&plan);
    assert_eq!(match_sum(&after_move), constraint.evaluate(&plan));
    assert_eq!(ids(&after_move), moved, "after the move");
}

#[test]
fn filtered_entities_report_their_planning_id() {
    let constraint = factory()
        .for_each(shift_source())
        .filter(|shift: &Shift| shift.nurse.is_none())
        .penalize(SoftScore::of(1))
        .named("Unassigned shift");
    assert_ids(
        constraint,
        vec![(-1, vec![id("shift-3")])],
        vec![(-1, vec![id("shift-3")])],
    );
}

#[test]
fn plain_sources_report_no_planning_id() {
    let constraint = factory()
        .for_each(source(
            shifts as fn(&Roster) -> &[Shift],
            ChangeSource::Descriptor(0),
        ))
        .filter(|shift: &Shift| shift.nurse.is_none())
        .penalize(SoftScore::of(1))
        .named("Unassigned shift");
    assert_ids(constraint, vec![(-1, vec![None])], vec![(-1, vec![None])]);
}

#[test]
fn self_joined_pairs_report_both_planning_ids() {
    let constraint = factory()
        .for_each(shift_source())
        .join(equal(|shift: &Shift| (shift.day, shift.nurse)))
        .filter(|left: &Shift, right: &Shift| left.nurse.is_some() && left.id < right.id)
        .penalize(SoftScore::of(1))
        .named("Two shifts on one day");
    assert_ids(
        constraint,
        vec![(-1, vec![id("shift-0"), id("shift-1")])],
        vec![(-1, vec![id("shift-1"), id("shift-2")])],
    );
}

#[test]
fn cross_joined_pairs_report_both_planning_ids() {
    let constraint = factory()
        .for_each(shift_source())
        .join((
            nurse_source(),
            equal_bi(|shift: &Shift| shift.nurse, |nurse: &Nurse| Some(nurse.id)),
        ))
        .filter(|shift: &Shift, nurse: &Nurse| nurse.unavailable_days.contains(&shift.day))
        .penalize(SoftScore::of(1))
        .named("Shift on an unavailable day");
    assert_ids(
        constraint,
        vec![(-1, vec![id("shift-2"), id("nurse-1")])],
        vec![
            (-1, vec![id("shift-1"), id("nurse-1")]),
            (-1, vec![id("shift-2"), id("nurse-1")]),
        ],
    );
}

#[test]
fn existence_filtered_entities_report_their_planning_id() {
    let constraint = factory()
        .for_each(shift_source())
        .if_exists((
            factory()
                .for_each(nurse_source())
                .filter(|nurse: &Nurse| !nurse.unavailable_days.is_empty()),
            equal_bi(|shift: &Shift| shift.nurse, |nurse: &Nurse| Some(nurse.id)),
        ))
        .penalize(SoftScore::of(1))
        .named("Shift of a nurse with unavailable days");
    assert_ids(
        constraint,
        vec![(-1, vec![id("shift-2")])],
        vec![(-1, vec![id("shift-1")]), (-1, vec![id("shift-2")])],
    );
}

#[test]
fn flattened_pairs_report_the_owners_planning_ids() {
    let constraint = factory()
        .for_each(shift_source())
        .join((
            nurse_source(),
            equal_bi(|shift: &Shift| shift.nurse, |nurse: &Nurse| Some(nurse.id)),
        ))
        .flatten_last(
            |nurse: &Nurse| nurse.unavailable_days.as_slice(),
            |day: &usize| *day,
            |shift: &Shift| shift.day,
        )
        .penalize(SoftScore::of(1))
        .named("Shift on an unavailable day (flattened)");
    assert_ids(
        constraint,
        vec![(-1, vec![id("shift-2"), id("nurse-1")])],
        vec![
            (-1, vec![id("shift-1"), id("nurse-1")]),
            (-1, vec![id("shift-2"), id("nurse-1")]),
        ],
    );
}

#[test]
fn groups_and_complements_report_their_contributors_planning_ids() {
    let constraint = factory()
        .for_each(shift_source())
        .group_by(|shift: &Shift| shift.nurse.unwrap_or(usize::MAX), count())
        .complement(nurse_source(), |nurse: &Nurse| nurse.id, |_nurse| 0usize)
        .penalize(|_nurse: &usize, shifts: &usize| SoftScore::of(*shifts as i64))
        .named("Shifts per nurse");
    assert_ids(
        constraint,
        vec![
            (-2, vec![id("nurse-0"), id("shift-0"), id("shift-1")]),
            (-1, vec![id("nurse-1"), id("shift-2")]),
        ],
        // Contributors keep contribution order: shift 1 joined nurse 1 last.
        vec![
            (-2, vec![id("nurse-1"), id("shift-2"), id("shift-1")]),
            (-1, vec![id("nurse-0"), id("shift-0")]),
        ],
    );
}
