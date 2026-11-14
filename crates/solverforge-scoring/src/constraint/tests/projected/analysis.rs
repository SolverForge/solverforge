/* Constraint analysis of projected grouped and complemented constraints.

A projected row is justified by the entities it was projected from, read
through its source, so a projected group names the work items that feed it
(with their planning ids) and a complemented group adds its target first. The
match scores must sum to the constraint's score before and after
initialization.

Work rows report the planning id `work-<bucket>-<demand>`; capacity rows come
from a plain source and report none.
*/

use super::support::*;
use crate::api::analysis::DetailedConstraintMatch;
use crate::stream::collection_extract::{SourceExtract, SourcePlanningIds};

struct WorkIds;

impl SourcePlanningIds<Work> for WorkIds {
    fn planning_id(work: &Work) -> Option<String> {
        Some(format!("work-{}-{}", work.bucket, work.demand))
    }
}

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

fn work_source() -> SourceExtract<fn(&Plan) -> &[Work], WorkIds> {
    SourceExtract::new(work as fn(&Plan) -> &[Work], ChangeSource::Descriptor(0))
}

/// One justification entity: its display and planning id.
type Entity = (String, Option<String>);

/// Each match as (score, justification entities), sorted.
fn summary(matches: &[DetailedConstraintMatch<'_, SoftScore>]) -> Vec<(i64, Vec<Entity>)> {
    let mut rows = matches
        .iter()
        .map(|detailed| {
            (
                detailed.score.score(),
                detailed
                    .justification
                    .entities
                    .iter()
                    .map(|entity| (entity.display.clone(), entity.planning_id.clone()))
                    .collect(),
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

fn work_entity(bucket: usize, demand: i64) -> Entity {
    let work = Work {
        bucket,
        demand,
        enabled: true,
    };
    (format!("{work:?}"), Some(format!("work-{bucket}-{demand}")))
}

fn capacity_entity(bucket: usize) -> Entity {
    (
        format!(
            "{:?}",
            Capacity {
                bucket,
                capacity: 0
            }
        ),
        None,
    )
}

fn assert_matches<C>(mut constraint: C, expected: Vec<(i64, Vec<Entity>)>)
where
    C: IncrementalConstraint<Plan, SoftScore>,
{
    let plan = plan();
    let before = constraint.get_matches(&plan);
    assert_eq!(match_sum(&before), constraint.evaluate(&plan));
    assert_eq!(summary(&before), expected, "before initialization");
    drop(before);
    let total = constraint.initialize(&plan);
    let after = constraint.get_matches(&plan);
    assert_eq!(match_sum(&after), total);
    assert_eq!(summary(&after), expected, "after initialization");
}

#[test]
fn projected_grouped_constraint_reports_each_costly_group_with_its_owners() {
    let constraint = ConstraintFactory::<Plan, SoftScore>::new()
        .for_each(work_source())
        .project(WorkEntryProjection)
        .group_by(
            |entry: &Entry| entry.bucket,
            sum(|entry: &Entry| entry.delta),
        )
        .penalize(|_bucket: &usize, demand: &i64| SoftScore::of(*demand))
        .named("projected demand");
    // bucket 0: 5; bucket 1: 2 - 2 = 0, no match.
    assert_matches(constraint, vec![(-5, vec![work_entity(0, 5)])]);
}

#[test]
fn projected_complemented_constraint_justifies_each_group_by_its_target_and_owners() {
    let constraint = ConstraintFactory::<Plan, SoftScore>::new()
        .for_each(work_source())
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
    assert_matches(
        constraint,
        vec![
            (-5, vec![capacity_entity(0), work_entity(0, 5)]),
            (-3, vec![capacity_entity(2)]),
        ],
    );
}
