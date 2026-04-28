use std::sync::atomic::{AtomicUsize, Ordering};

use solverforge_core::score::SoftScore;

use crate::api::constraint_set::{ConstraintSet, IncrementalConstraint};
use crate::director::score_director::ScoreDirector;
use crate::director::{Director, RecordingDirector};
use crate::stream::collection_extract::{source, ChangeSource};
use crate::stream::collector::sum;
use crate::stream::joiner::equal;
use crate::stream::{ConstraintFactory, Projection, ProjectionSink};
use solverforge_core::domain::PlanningSolution;

#[derive(Clone, Debug, PartialEq)]
struct Work {
    bucket: usize,
    demand: i64,
    enabled: bool,
}

#[derive(Clone)]
struct Capacity {
    bucket: usize,
    capacity: i64,
}

#[derive(Clone)]
struct Plan {
    work: Vec<Work>,
    capacity: Vec<Capacity>,
}

impl PlanningSolution for Plan {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        None
    }

    fn set_score(&mut self, _score: Option<Self::Score>) {}
}

#[derive(Clone)]
struct Entry {
    bucket: usize,
    delta: i64,
}

struct WorkTwoEntries;

impl Projection<Work> for WorkTwoEntries {
    type Out = Entry;
    const MAX_EMITS: usize = 2;

    fn project<Sink>(&self, work: &Work, out: &mut Sink)
    where
        Sink: ProjectionSink<Self::Out>,
    {
        if !work.enabled {
            return;
        }
        out.emit(Entry {
            bucket: work.bucket,
            delta: work.demand,
        });
        out.emit(Entry {
            bucket: work.bucket + 1,
            delta: work.demand,
        });
    }
}

struct WorkEntryProjection;

impl Projection<Work> for WorkEntryProjection {
    type Out = Entry;
    const MAX_EMITS: usize = 1;

    fn project<Sink>(&self, work: &Work, out: &mut Sink)
    where
        Sink: ProjectionSink<Self::Out>,
    {
        out.emit(Entry {
            bucket: work.bucket,
            delta: work.demand,
        });
    }
}

struct OrderedWorkEntries;

impl Projection<Work> for OrderedWorkEntries {
    type Out = Entry;
    const MAX_EMITS: usize = 2;

    fn project<Sink>(&self, work: &Work, out: &mut Sink)
    where
        Sink: ProjectionSink<Self::Out>,
    {
        out.emit(Entry {
            bucket: work.bucket,
            delta: work.demand,
        });
        out.emit(Entry {
            bucket: work.bucket,
            delta: work.demand + 10,
        });
    }
}

struct OptionalSecondWorkEntry;

impl Projection<Work> for OptionalSecondWorkEntry {
    type Out = Entry;
    const MAX_EMITS: usize = 2;

    fn project<Sink>(&self, work: &Work, out: &mut Sink)
    where
        Sink: ProjectionSink<Self::Out>,
    {
        out.emit(Entry {
            bucket: work.bucket,
            delta: work.demand,
        });
        if work.enabled {
            out.emit(Entry {
                bucket: work.bucket,
                delta: work.demand + 10,
            });
        }
    }
}

struct CapacityEntryProjection;

impl Projection<Capacity> for CapacityEntryProjection {
    type Out = Entry;
    const MAX_EMITS: usize = 1;

    fn project<Sink>(&self, capacity: &Capacity, out: &mut Sink)
    where
        Sink: ProjectionSink<Self::Out>,
    {
        out.emit(Entry {
            bucket: capacity.bucket,
            delta: -capacity.capacity,
        });
    }
}

struct CountedProjection;

impl Projection<Work> for CountedProjection {
    type Out = Entry;
    const MAX_EMITS: usize = 1;

    fn project<Sink>(&self, work: &Work, out: &mut Sink)
    where
        Sink: ProjectionSink<Self::Out>,
    {
        PROJECTION_CALLS.fetch_add(1, Ordering::SeqCst);
        out.emit(Entry {
            bucket: work.bucket,
            delta: work.demand,
        });
    }
}

fn work(plan: &Plan) -> &[Work] {
    plan.work.as_slice()
}

fn capacity(plan: &Plan) -> &[Capacity] {
    plan.capacity.as_slice()
}

fn projected_asymmetric_self_join_constraint() -> impl IncrementalConstraint<Plan, SoftScore> {
    ConstraintFactory::<Plan, SoftScore>::new()
        .for_each(source(
            work as fn(&Plan) -> &[Work],
            ChangeSource::Descriptor(0),
        ))
        .project(OptionalSecondWorkEntry)
        .join(equal(|entry: &Entry| entry.bucket))
        .filter(|left: &Entry, right: &Entry| left.delta < right.delta)
        .penalize_with(|left: &Entry, right: &Entry| SoftScore::of(left.delta * 10 + right.delta))
        .named("projected asymmetric duplicate bucket")
}

fn projected_asymmetric_self_join_plan() -> Plan {
    Plan {
        work: vec![
            Work {
                bucket: 0,
                demand: 2,
                enabled: true,
            },
            Work {
                bucket: 0,
                demand: 20,
                enabled: true,
            },
            Work {
                bucket: 1,
                demand: 40,
                enabled: true,
            },
        ],
        capacity: Vec::new(),
    }
}

fn fresh_projected_asymmetric_self_join_score(plan: &Plan) -> SoftScore {
    (projected_asymmetric_self_join_constraint(),).evaluate_all(plan)
}

fn assert_projected_director_matches_fresh<C>(director: &mut ScoreDirector<Plan, C>)
where
    C: ConstraintSet<Plan, SoftScore>,
{
    let cached = director.calculate_score();
    let fresh = director
        .constraints()
        .evaluate_all(director.working_solution());
    assert_eq!(cached, fresh);
}

#[test]
fn projected_allows_zero_and_multiple_outputs() {
    let constraint = ConstraintFactory::<Plan, SoftScore>::new()
        .for_each(source(
            work as fn(&Plan) -> &[Work],
            ChangeSource::Descriptor(0),
        ))
        .project(WorkTwoEntries)
        .penalize_with(|entry: &Entry| SoftScore::of(entry.delta))
        .named("projected work");

    let plan = Plan {
        work: vec![
            Work {
                bucket: 0,
                demand: 3,
                enabled: true,
            },
            Work {
                bucket: 0,
                demand: 100,
                enabled: false,
            },
        ],
        capacity: Vec::new(),
    };

    assert_eq!(constraint.match_count(&plan), 2);
    assert_eq!(constraint.evaluate(&plan), SoftScore::of(-6));
}

#[test]
fn projected_grouping_merges_multiple_sources() {
    let constraint = ConstraintFactory::<Plan, SoftScore>::new()
        .for_each(source(
            work as fn(&Plan) -> &[Work],
            ChangeSource::Descriptor(0),
        ))
        .project(WorkEntryProjection)
        .merge(
            ConstraintFactory::<Plan, SoftScore>::new()
                .for_each(source(
                    capacity as fn(&Plan) -> &[Capacity],
                    ChangeSource::Descriptor(1),
                ))
                .project(CapacityEntryProjection),
        )
        .group_by(
            |entry: &Entry| entry.bucket,
            sum(|entry: &Entry| entry.delta),
        )
        .penalize_with(|delta: &i64| SoftScore::of((*delta).max(0)))
        .named("capacity shortage");

    let plan = Plan {
        work: vec![
            Work {
                bucket: 0,
                demand: 5,
                enabled: true,
            },
            Work {
                bucket: 1,
                demand: 7,
                enabled: true,
            },
        ],
        capacity: vec![
            Capacity {
                bucket: 0,
                capacity: 3,
            },
            Capacity {
                bucket: 1,
                capacity: 10,
            },
        ],
    };

    assert_eq!(constraint.match_count(&plan), 2);
    assert_eq!(constraint.evaluate(&plan), SoftScore::of(-2));
}

#[test]
fn projected_rows_can_self_join_by_key() {
    let mut constraint = ConstraintFactory::<Plan, SoftScore>::new()
        .for_each(source(
            work as fn(&Plan) -> &[Work],
            ChangeSource::Descriptor(0),
        ))
        .project(WorkEntryProjection)
        .join(equal(|entry: &Entry| entry.bucket))
        .filter(|left: &Entry, right: &Entry| left.delta < right.delta)
        .penalize_with(|_left: &Entry, _right: &Entry| SoftScore::of(1))
        .named("projected duplicate bucket");

    let mut plan = Plan {
        work: vec![
            Work {
                bucket: 0,
                demand: 1,
                enabled: true,
            },
            Work {
                bucket: 0,
                demand: 2,
                enabled: true,
            },
            Work {
                bucket: 1,
                demand: 3,
                enabled: true,
            },
        ],
        capacity: Vec::new(),
    };

    let mut total = constraint.initialize(&plan);
    assert_eq!(constraint.match_count(&plan), 1);
    assert_eq!(total, SoftScore::of(-1));

    total = total + constraint.on_retract(&plan, 2, 0);
    plan.work[2].bucket = 0;
    total = total + constraint.on_insert(&plan, 2, 0);

    assert_eq!(constraint.match_count(&plan), 3);
    assert_eq!(total, SoftScore::of(-3));
    assert_eq!(total, constraint.evaluate(&plan));
}

#[test]
fn projected_self_join_reuses_row_slots_after_repeated_updates() {
    let mut constraint = ConstraintFactory::<Plan, SoftScore>::new()
        .for_each(source(
            work as fn(&Plan) -> &[Work],
            ChangeSource::Descriptor(0),
        ))
        .project(WorkEntryProjection)
        .join(equal(|entry: &Entry| entry.bucket))
        .filter(|left: &Entry, right: &Entry| left.delta < right.delta)
        .penalize_with(|_left: &Entry, _right: &Entry| SoftScore::of(1))
        .named("projected duplicate bucket");

    let mut plan = Plan {
        work: vec![
            Work {
                bucket: 0,
                demand: 1,
                enabled: true,
            },
            Work {
                bucket: 0,
                demand: 2,
                enabled: true,
            },
            Work {
                bucket: 1,
                demand: 3,
                enabled: true,
            },
        ],
        capacity: Vec::new(),
    };

    let mut total = constraint.initialize(&plan);
    let initial_row_storage_len = constraint.debug_row_storage_len();
    assert_eq!(initial_row_storage_len, 3);

    for bucket in [0, 1, 2, 0, 3, 0] {
        total = total + constraint.on_retract(&plan, 2, 0);
        plan.work[2].bucket = bucket;
        total = total + constraint.on_insert(&plan, 2, 0);

        assert_eq!(total, constraint.evaluate(&plan));
        assert_eq!(constraint.debug_row_storage_len(), initial_row_storage_len);
        assert_eq!(constraint.debug_free_row_count(), 0);
    }
}

#[test]
fn projected_self_join_preserves_projection_order_when_reusing_slots() {
    let mut constraint = ConstraintFactory::<Plan, SoftScore>::new()
        .for_each(source(
            work as fn(&Plan) -> &[Work],
            ChangeSource::Descriptor(0),
        ))
        .project(OrderedWorkEntries)
        .join(equal(|entry: &Entry| entry.bucket))
        .filter(|left: &Entry, right: &Entry| left.delta < right.delta)
        .penalize_with(|left: &Entry, right: &Entry| SoftScore::of(left.delta * 10 + right.delta))
        .named("projected ordered duplicate bucket");

    let plan = Plan {
        work: vec![Work {
            bucket: 0,
            demand: 1,
            enabled: true,
        }],
        capacity: Vec::new(),
    };

    let mut total = constraint.initialize(&plan);
    let initial_row_storage_len = constraint.debug_row_storage_len();
    assert_eq!(total, SoftScore::of(-21));
    assert_eq!(total, constraint.evaluate(&plan));

    for _ in 0..4 {
        total = total + constraint.on_retract(&plan, 0, 0);
        total = total + constraint.on_insert(&plan, 0, 0);

        assert_eq!(total, SoftScore::of(-21));
        assert_eq!(total, constraint.evaluate(&plan));
        assert_eq!(constraint.debug_row_storage_len(), initial_row_storage_len);
        assert_eq!(constraint.debug_free_row_count(), 0);
    }
}

#[test]
fn projected_self_join_reuses_slots_across_cardinality_changes() {
    let mut constraint = ConstraintFactory::<Plan, SoftScore>::new()
        .for_each(source(
            work as fn(&Plan) -> &[Work],
            ChangeSource::Descriptor(0),
        ))
        .project(OptionalSecondWorkEntry)
        .join(equal(|entry: &Entry| entry.bucket))
        .filter(|left: &Entry, right: &Entry| left.delta < right.delta)
        .penalize_with(|left: &Entry, right: &Entry| SoftScore::of(left.delta * 10 + right.delta))
        .named("projected cardinality changing duplicate bucket");

    let mut plan = Plan {
        work: vec![Work {
            bucket: 0,
            demand: 2,
            enabled: true,
        }],
        capacity: Vec::new(),
    };

    let mut total = constraint.initialize(&plan);
    assert_eq!(total, SoftScore::of(-32));
    assert_eq!(constraint.debug_row_storage_len(), 2);

    total = total + constraint.on_retract(&plan, 0, 0);
    plan.work[0].enabled = false;
    total = total + constraint.on_insert(&plan, 0, 0);
    assert_eq!(total, SoftScore::ZERO);
    assert_eq!(total, constraint.evaluate(&plan));
    assert_eq!(constraint.debug_row_storage_len(), 2);
    assert_eq!(constraint.debug_free_row_count(), 1);

    total = total + constraint.on_retract(&plan, 0, 0);
    plan.work[0].enabled = true;
    total = total + constraint.on_insert(&plan, 0, 0);
    assert_eq!(total, SoftScore::of(-32));
    assert_eq!(total, constraint.evaluate(&plan));
    assert_eq!(constraint.debug_row_storage_len(), 2);
    assert_eq!(constraint.debug_free_row_count(), 0);
}

#[test]
fn projected_self_join_score_director_cached_score_matches_fresh_after_updates() {
    let mut director = ScoreDirector::new(
        projected_asymmetric_self_join_plan(),
        (projected_asymmetric_self_join_constraint(),),
    );
    assert_projected_director_matches_fresh(&mut director);

    for (entity_index, bucket, demand, enabled) in [
        (2, 0, 40, true),
        (1, 0, 20, false),
        (1, 0, 5, true),
        (0, 2, 2, true),
        (2, 0, 30, false),
        (2, 0, 30, true),
    ] {
        director.before_variable_changed(0, entity_index);
        {
            let work = &mut director.working_solution_mut().work[entity_index];
            work.bucket = bucket;
            work.demand = demand;
            work.enabled = enabled;
        }
        director.after_variable_changed(0, entity_index);
        assert_projected_director_matches_fresh(&mut director);
    }
}

#[test]
fn projected_self_join_nested_recording_director_undo_restores_cached_score() {
    let initial_plan = projected_asymmetric_self_join_plan();
    let mut inner = ScoreDirector::new(
        initial_plan.clone(),
        (projected_asymmetric_self_join_constraint(),),
    );
    assert_projected_director_matches_fresh(&mut inner);

    {
        let mut outer = RecordingDirector::new(&mut inner);
        let old_outer_work = outer.working_solution().work[1].clone();
        outer.before_variable_changed(0, 1);
        {
            let work = &mut outer.working_solution_mut().work[1];
            work.bucket = 0;
            work.demand = 5;
            work.enabled = true;
        }
        outer.after_variable_changed(0, 1);
        outer.register_undo(Box::new(move |plan: &mut Plan| {
            plan.work[1] = old_outer_work;
        }));
        assert_eq!(
            outer.calculate_score(),
            fresh_projected_asymmetric_self_join_score(outer.working_solution())
        );

        {
            let mut nested = RecordingDirector::new(&mut outer);
            let old_nested_work = nested.working_solution().work[2].clone();
            nested.before_variable_changed(0, 2);
            {
                let work = &mut nested.working_solution_mut().work[2];
                work.bucket = 0;
                work.demand = 30;
                work.enabled = false;
            }
            nested.after_variable_changed(0, 2);
            nested.register_undo(Box::new(move |plan: &mut Plan| {
                plan.work[2] = old_nested_work;
            }));

            assert_eq!(
                nested.calculate_score(),
                fresh_projected_asymmetric_self_join_score(nested.working_solution())
            );
            nested.undo_changes();
        }

        assert_eq!(
            outer.calculate_score(),
            fresh_projected_asymmetric_self_join_score(outer.working_solution())
        );
        outer.undo_changes();
    }

    assert_eq!(inner.working_solution().work, initial_plan.work);
    assert_projected_director_matches_fresh(&mut inner);
}

#[test]
fn projected_merged_descriptor_sources_update_only_owning_slot() {
    let mut constraint = ConstraintFactory::<Plan, SoftScore>::new()
        .for_each(source(
            work as fn(&Plan) -> &[Work],
            ChangeSource::Descriptor(0),
        ))
        .project(WorkEntryProjection)
        .merge(
            ConstraintFactory::<Plan, SoftScore>::new()
                .for_each(source(
                    capacity as fn(&Plan) -> &[Capacity],
                    ChangeSource::Descriptor(1),
                ))
                .project(CapacityEntryProjection),
        )
        .group_by(
            |entry: &Entry| entry.bucket,
            sum(|entry: &Entry| entry.delta),
        )
        .penalize_with(|delta: &i64| SoftScore::of((*delta).max(0)))
        .named("capacity shortage");

    let mut plan = Plan {
        work: vec![Work {
            bucket: 0,
            demand: 5,
            enabled: true,
        }],
        capacity: vec![Capacity {
            bucket: 0,
            capacity: 3,
        }],
    };

    let mut total = constraint.initialize(&plan);
    assert_eq!(total, SoftScore::of(-2));

    total = total + constraint.on_retract(&plan, 0, 1);
    plan.capacity[0].capacity = 8;
    total = total + constraint.on_insert(&plan, 0, 1);

    assert_eq!(total, SoftScore::of(0));
    assert_eq!(total, constraint.evaluate(&plan));
}

#[test]
fn projected_merged_descriptor_sources_keep_same_entity_index_slots_distinct() {
    let mut constraint = ConstraintFactory::<Plan, SoftScore>::new()
        .for_each(source(
            work as fn(&Plan) -> &[Work],
            ChangeSource::Descriptor(0),
        ))
        .project(WorkEntryProjection)
        .merge(
            ConstraintFactory::<Plan, SoftScore>::new()
                .for_each(source(
                    capacity as fn(&Plan) -> &[Capacity],
                    ChangeSource::Descriptor(1),
                ))
                .project(CapacityEntryProjection),
        )
        .penalize_with(|entry: &Entry| SoftScore::of(entry.delta))
        .named("merged projected rows");

    let mut plan = Plan {
        work: vec![Work {
            bucket: 0,
            demand: 5,
            enabled: true,
        }],
        capacity: vec![Capacity {
            bucket: 0,
            capacity: 3,
        }],
    };

    let mut total = constraint.initialize(&plan);
    assert_eq!(total, SoftScore::of(-2));

    total = total + constraint.on_retract(&plan, 0, 0);
    plan.work[0].demand = 8;
    total = total + constraint.on_insert(&plan, 0, 0);

    assert_eq!(total, SoftScore::of(-5));
    assert_eq!(total, constraint.evaluate(&plan));
}

#[test]
#[should_panic(expected = "cannot localize entity indexes")]
fn projected_unknown_source_panics_on_localized_callback() {
    let mut constraint = ConstraintFactory::<Plan, SoftScore>::new()
        .for_each(work as fn(&Plan) -> &[Work])
        .project(WorkEntryProjection)
        .penalize_with(|entry: &Entry| SoftScore::of(entry.delta))
        .named("unknown projected");

    let plan = Plan {
        work: vec![Work {
            bucket: 0,
            demand: 5,
            enabled: true,
        }],
        capacity: Vec::new(),
    };

    constraint.initialize(&plan);
    constraint.on_retract(&plan, 0, 0);
}

#[test]
fn projected_retracts_previous_outputs_before_update() {
    let mut constraint = ConstraintFactory::<Plan, SoftScore>::new()
        .for_each(source(
            work as fn(&Plan) -> &[Work],
            ChangeSource::Descriptor(0),
        ))
        .project(WorkEntryProjection)
        .group_by(
            |entry: &Entry| entry.bucket,
            sum(|entry: &Entry| entry.delta),
        )
        .penalize_with(|delta: &i64| SoftScore::of((*delta).max(0)))
        .named("demand");

    let mut plan = Plan {
        work: vec![Work {
            bucket: 0,
            demand: 5,
            enabled: true,
        }],
        capacity: Vec::new(),
    };

    let mut total = constraint.initialize(&plan);
    assert_eq!(total, SoftScore::of(-5));
    total = total + constraint.on_retract(&plan, 0, 0);
    plan.work[0].demand = 2;
    total = total + constraint.on_insert(&plan, 0, 0);

    assert_eq!(total, SoftScore::of(-2));
    assert_eq!(total, constraint.evaluate(&plan));
}

static PROJECTION_CALLS: AtomicUsize = AtomicUsize::new(0);

#[test]
fn projected_updates_only_the_changed_descriptor_entity() {
    PROJECTION_CALLS.store(0, Ordering::SeqCst);
    let mut constraint = ConstraintFactory::<Plan, SoftScore>::new()
        .for_each(source(
            work as fn(&Plan) -> &[Work],
            ChangeSource::Descriptor(0),
        ))
        .project(CountedProjection)
        .penalize_with(|entry: &Entry| SoftScore::of(entry.delta))
        .named("counted");

    let mut plan = Plan {
        work: vec![
            Work {
                bucket: 0,
                demand: 1,
                enabled: true,
            },
            Work {
                bucket: 0,
                demand: 2,
                enabled: true,
            },
            Work {
                bucket: 0,
                demand: 3,
                enabled: true,
            },
        ],
        capacity: vec![Capacity {
            bucket: 0,
            capacity: 1,
        }],
    };

    let mut total = constraint.initialize(&plan);
    assert_eq!(PROJECTION_CALLS.load(Ordering::SeqCst), 3);

    total = total + constraint.on_retract(&plan, 0, 1);
    plan.capacity[0].capacity = 2;
    total = total + constraint.on_insert(&plan, 0, 1);
    assert_eq!(PROJECTION_CALLS.load(Ordering::SeqCst), 3);
    assert_eq!(total, constraint.evaluate(&plan));
    PROJECTION_CALLS.store(3, Ordering::SeqCst);

    total = total + constraint.on_retract(&plan, 1, 0);
    plan.work[1].demand = 20;
    total = total + constraint.on_insert(&plan, 1, 0);
    assert_eq!(PROJECTION_CALLS.load(Ordering::SeqCst), 4);
    assert_eq!(total, constraint.evaluate(&plan));
}
