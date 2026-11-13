/* Constraint analysis of grouped and complemented constraints.

Every grouped and complemented builder must report one match per group whose
weight is not zero, justified by the group's contributors (and, for a
complement, by its target entity; a defaulted key by its target alone), and
the match scores must sum to the constraint's score: before initialization,
after it, and across mutations. Shared grouped nodes report matches for every
terminal they carry, scaled by each terminal's weight multiplier.

Roster: nurse 0 works shifts 0 and 1, nurse 1 works shift 2, shift 3 is
unassigned, nurse 2 works nothing.
*/

use solverforge_core::score::SoftScore;

use crate::api::analysis::DetailedConstraintMatch;
use crate::api::constraint_set::{ConstraintSet, IncrementalConstraint};
use crate::stream::collection_extract::{source, ChangeSource};
use crate::stream::collector::{count, sum};
use crate::stream::joiner::equal_bi;
use crate::stream::ConstraintFactory;

const UNASSIGNED: usize = usize::MAX;

#[derive(Clone, Debug)]
struct Nurse {
    id: usize,
}

#[derive(Clone, Debug)]
struct Shift {
    // Read only through `Debug`: it tells the shifts apart in justifications.
    #[allow(dead_code)]
    id: usize,
    nurse: Option<usize>,
}

#[derive(Clone)]
struct Roster {
    shifts: Vec<Shift>,
    nurses: Vec<Nurse>,
}

fn shifts(roster: &Roster) -> &[Shift] {
    roster.shifts.as_slice()
}

fn nurses(roster: &Roster) -> &[Nurse] {
    roster.nurses.as_slice()
}

fn roster() -> Roster {
    Roster {
        shifts: vec![
            Shift {
                id: 0,
                nurse: Some(0),
            },
            Shift {
                id: 1,
                nurse: Some(0),
            },
            Shift {
                id: 2,
                nurse: Some(1),
            },
            Shift { id: 3, nurse: None },
        ],
        nurses: vec![Nurse { id: 0 }, Nurse { id: 1 }, Nurse { id: 2 }],
    }
}

fn factory() -> ConstraintFactory<Roster, SoftScore> {
    ConstraintFactory::<Roster, SoftScore>::new()
}

fn shift_source() -> impl crate::stream::collection_extract::CollectionExtract<Roster, Item = Shift>
{
    source(
        shifts as fn(&Roster) -> &[Shift],
        ChangeSource::Descriptor(0),
    )
}

fn nurse_source() -> impl crate::stream::collection_extract::CollectionExtract<Roster, Item = Nurse>
{
    source(
        nurses as fn(&Roster) -> &[Nurse],
        ChangeSource::Descriptor(1),
    )
}

fn nurse_key(shift: &Shift) -> usize {
    shift.nurse.unwrap_or(UNASSIGNED)
}

/// Each match as (score, displayed justification entities), sorted.
fn summary(matches: &[DetailedConstraintMatch<'_, SoftScore>]) -> Vec<(i64, Vec<String>)> {
    let mut rows = matches
        .iter()
        .map(|detailed| {
            (
                detailed.score.score(),
                detailed
                    .justification
                    .entities
                    .iter()
                    .map(|entity| entity.display.clone())
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>();
    rows.sort();
    rows
}

fn match_sum(matches: &[DetailedConstraintMatch<'_, SoftScore>]) -> SoftScore {
    matches
        .iter()
        .fold(SoftScore::of(0), |total, detailed| total + detailed.score)
}

fn shift(id: usize, nurse: Option<usize>) -> String {
    format!("{:?}", Shift { id, nurse })
}

fn nurse(id: usize) -> String {
    format!("{:?}", Nurse { id })
}

/// Matches agree with the score before initialization, after it, after a
/// retraction and after the re-insertion, and are the expected rows.
fn assert_lifecycle<C>(mut constraint: C, expected: Vec<(i64, Vec<String>)>)
where
    C: IncrementalConstraint<Roster, SoftScore>,
{
    let mut plan = roster();
    let before = constraint.get_matches(&plan);
    assert_eq!(match_sum(&before), constraint.evaluate(&plan));
    assert_eq!(summary(&before), expected, "before initialization");
    drop(before);

    let total = constraint.initialize(&plan);
    let after = constraint.get_matches(&plan);
    assert_eq!(match_sum(&after), total);
    assert_eq!(summary(&after), expected, "after initialization");
    drop(after);

    // Unassign shift 1: nurse 0 drops to one shift.
    let retracted = constraint.on_retract(&plan, 1, 0);
    plan.shifts[1].nurse = None;
    let inserted = constraint.on_insert(&plan, 1, 0);
    let moved = constraint.get_matches(&plan);
    assert_eq!(match_sum(&moved), total + retracted + inserted);
    assert_eq!(match_sum(&moved), constraint.evaluate(&plan));
    drop(moved);

    constraint.on_retract(&plan, 1, 0);
    plan.shifts[1].nurse = Some(0);
    constraint.on_insert(&plan, 1, 0);
    let restored = constraint.get_matches(&plan);
    assert_eq!(match_sum(&restored), total);
    assert_eq!(summary(&restored), expected, "after restoring");
}

#[test]
fn grouped_constraint_reports_each_costly_group_with_its_contributors() {
    let constraint = factory()
        .for_each(shift_source())
        .group_by(nurse_key, count())
        .penalize(|_nurse: &usize, shifts: &usize| SoftScore::of(shifts.saturating_sub(1) as i64))
        .named("Shifts per nurse beyond one");
    assert_lifecycle(
        constraint,
        vec![(-1, vec![shift(0, Some(0)), shift(1, Some(0))])],
    );
}

#[test]
fn grouped_join_constraint_reports_the_group_and_the_joined_entity() {
    let constraint = factory()
        .for_each(shift_source())
        .group_by(nurse_key, count())
        .join((nurse_source(), |nurse: &Nurse| nurse.id))
        .penalize(|_id: &usize, shifts: &usize, _nurse: &Nurse| {
            SoftScore::of(shifts.saturating_sub(1) as i64)
        })
        .named("Shifts per known nurse beyond one");
    assert_lifecycle(
        constraint,
        vec![(-1, vec![shift(0, Some(0)), shift(1, Some(0)), nurse(0)])],
    );
}

#[test]
fn complemented_constraint_justifies_defaulted_keys_by_their_target() {
    let constraint = factory()
        .for_each(shift_source())
        .group_by(nurse_key, count())
        .complement(nurse_source(), |nurse: &Nurse| nurse.id, |_nurse| 0usize)
        .penalize(|_id: &usize, shifts: &usize| SoftScore::of(2 - *shifts as i64))
        .named("Distance from two shifts");
    // nurse 0: 2 shifts -> 0 (no match); nurse 1: 1 -> -1; nurse 2: default 0 -> -2.
    assert_lifecycle(
        constraint,
        vec![
            (-2, vec![nurse(2)]),
            (-1, vec![nurse(1), shift(2, Some(1))]),
        ],
    );
}

fn cross_join() -> impl Fn(&Shift) -> Option<usize> + Send + Sync + Clone {
    |shift: &Shift| shift.nurse
}

#[test]
fn cross_grouped_constraint_reports_every_contributing_pair() {
    let constraint = factory()
        .for_each(shift_source())
        .join((
            nurse_source(),
            equal_bi(cross_join(), |nurse: &Nurse| Some(nurse.id)),
        ))
        .group_by(
            |_shift: &Shift, nurse: &Nurse| nurse.id,
            sum(|(_shift, _nurse): (&Shift, &Nurse)| 1i64),
        )
        .penalize(|_id: &usize, shifts: &i64| SoftScore::of(shifts - 1))
        .named("Joined shifts per nurse beyond one");
    assert_lifecycle(
        constraint,
        vec![(
            -1,
            vec![shift(0, Some(0)), nurse(0), shift(1, Some(0)), nurse(0)],
        )],
    );
}

#[test]
fn cross_complemented_grouped_constraint_justifies_defaulted_keys_by_their_target() {
    let constraint = factory()
        .for_each(shift_source())
        .join((
            nurse_source(),
            equal_bi(cross_join(), |nurse: &Nurse| Some(nurse.id)),
        ))
        .group_by(
            |_shift: &Shift, nurse: &Nurse| nurse.id,
            sum(|(_shift, _nurse): (&Shift, &Nurse)| 1i64),
        )
        .complement(nurse_source(), |nurse: &Nurse| nurse.id, |_nurse| 0i64)
        .penalize(|_id: &usize, shifts: &i64| SoftScore::of(2 - shifts))
        .named("Joined distance from two shifts");
    assert_lifecycle(
        constraint,
        vec![
            (-2, vec![nurse(2)]),
            (-1, vec![nurse(1), shift(2, Some(1)), nurse(1)]),
        ],
    );
}

#[test]
fn shared_grouped_terminals_each_report_their_matches_under_their_multiplier() {
    let mut constraints = factory()
        .for_each(shift_source())
        .group_by(nurse_key, count())
        .penalize(|_nurse: &usize, shifts: &usize| SoftScore::of(shifts.saturating_sub(1) as i64))
        .named("Shifts beyond one")
        .reward(|nurse: &usize, shifts: &usize| {
            SoftScore::of(if *nurse == UNASSIGNED {
                0
            } else {
                *shifts as i64
            })
        })
        .named("Assigned shifts");
    let plan = roster();
    constraints.initialize_all(&plan);

    let analyses = constraints.evaluate_detailed(&plan);
    assert_eq!(analyses.len(), 2);
    for analysis in &analyses {
        assert_eq!(match_sum(&analysis.matches), analysis.score);
    }
    assert_eq!(
        summary(&analyses[0].matches),
        vec![(-1, vec![shift(0, Some(0)), shift(1, Some(0))])]
    );
    assert_eq!(
        summary(&analyses[1].matches),
        vec![
            (1, vec![shift(2, Some(1))]),
            (2, vec![shift(0, Some(0)), shift(1, Some(0))]),
        ]
    );

    let weighted = constraints.evaluate_detailed_weighted(&plan, &[3, 2]);
    assert_eq!(weighted[0].score, SoftScore::of(-3));
    assert_eq!(weighted[1].score, SoftScore::of(6));
    for analysis in &weighted {
        assert_eq!(match_sum(&analysis.matches), analysis.score);
    }
    let disabled = constraints.evaluate_detailed_weighted(&plan, &[0, 2]);
    assert_eq!(disabled[0].score, SoftScore::of(0));
    assert_eq!(
        summary(&disabled[0].matches),
        vec![(0, vec![shift(0, Some(0)), shift(1, Some(0))])],
        "a disabled terminal still names its groups, at score zero"
    );
}
