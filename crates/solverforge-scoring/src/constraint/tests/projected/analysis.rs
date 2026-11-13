/* Constraint analysis of projected grouped and complemented constraints.

Projected rows carry no entity of their own, so a projected group justifies
itself with nothing beyond its complement target; what must hold is one match
per group whose weight is not zero, with match scores summing to the
constraint's score before and after initialization.
*/

use super::support::*;
use crate::api::analysis::DetailedConstraintMatch;

fn plan() -> Plan {
    Plan {
        work: vec![
            Work {
                bucket: 0,
                demand: 5,
                enabled: true,
            },
            Work {
                bucket: 1,
                demand: 2,
                enabled: true,
            },
            Work {
                bucket: 1,
                demand: -2,
                enabled: true,
            },
        ],
        capacity: vec![
            Capacity {
                bucket: 0,
                capacity: 0,
            },
            Capacity {
                bucket: 2,
                capacity: 0,
            },
        ],
    }
}

fn scores(matches: &[DetailedConstraintMatch<'_, SoftScore>]) -> Vec<(i64, usize)> {
    let mut rows = matches
        .iter()
        .map(|detailed| {
            (
                detailed.score.score(),
                detailed.justification.entities.len(),
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

fn assert_matches<C>(mut constraint: C, expected: Vec<(i64, usize)>)
where
    C: IncrementalConstraint<Plan, SoftScore>,
{
    let plan = plan();
    let before = constraint.get_matches(&plan);
    assert_eq!(match_sum(&before), constraint.evaluate(&plan));
    assert_eq!(scores(&before), expected, "before initialization");
    drop(before);
    let total = constraint.initialize(&plan);
    let after = constraint.get_matches(&plan);
    assert_eq!(match_sum(&after), total);
    assert_eq!(scores(&after), expected, "after initialization");
}

#[test]
fn projected_grouped_constraint_reports_each_costly_group() {
    let constraint = ConstraintFactory::<Plan, SoftScore>::new()
        .for_each(source(
            work as fn(&Plan) -> &[Work],
            ChangeSource::Descriptor(0),
        ))
        .project(WorkEntryProjection)
        .group_by(
            |entry: &Entry| entry.bucket,
            sum(|entry: &Entry| entry.delta),
        )
        .penalize(|_bucket: &usize, demand: &i64| SoftScore::of(*demand))
        .named("projected demand");
    // bucket 0: 5; bucket 1: 2 - 2 = 0, no match.
    assert_matches(constraint, vec![(-5, 0)]);
}

#[test]
fn projected_complemented_constraint_justifies_each_group_by_its_target() {
    let constraint = ConstraintFactory::<Plan, SoftScore>::new()
        .for_each(source(
            work as fn(&Plan) -> &[Work],
            ChangeSource::Descriptor(0),
        ))
        .project(WorkEntryProjection)
        .group_by(
            |entry: &Entry| entry.bucket,
            sum(|entry: &Entry| entry.delta),
        )
        .complement(
            source(
                capacity as fn(&Plan) -> &[Capacity],
                ChangeSource::Descriptor(1),
            ),
            |capacity: &Capacity| capacity.bucket,
            |_capacity: &Capacity| 3i64,
        )
        .penalize(|_bucket: &usize, demand: &i64| SoftScore::of(*demand))
        .named("projected demand per capacity");
    // capacity bucket 0: real 5; capacity bucket 2: default 3. Bucket 1 has
    // no capacity row, so it is no complemented row at all.
    assert_matches(constraint, vec![(-5, 1), (-3, 1)]);
}
