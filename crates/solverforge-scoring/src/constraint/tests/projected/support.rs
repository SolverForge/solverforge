pub(super) use std::marker::PhantomData;
pub(super) use std::sync::atomic::{AtomicUsize, Ordering};

pub(super) use solverforge_core::score::SoftScore;
pub(super) use solverforge_core::{ConstraintRef, ImpactType};

pub(super) use crate::api::constraint_set::{ConstraintSet, IncrementalConstraint};
pub(super) use crate::constraint::projected::{
    ProjectedComplementedGroupedNodeState, ProjectedComplementedGroupedTerminalScorer,
    ProjectedGroupedNodeState, ProjectedGroupedTerminalScorer,
    SharedProjectedComplementedGroupedConstraintSet, SharedProjectedGroupedConstraintSet,
};
pub(super) use crate::director::score_director::ScoreDirector;
pub(super) use crate::director::Director;
pub(super) use crate::stream::collection_extract::{source, ChangeSource};
pub(super) use crate::stream::collector::{sum, Accumulator, Collector};
pub(super) use crate::stream::filter::FnBiFilter;
pub(super) use crate::stream::joiner::equal;
pub(super) use crate::stream::{
    ConstraintFactory, ProjectedBiConstraintStream, Projection, ProjectionSink,
};
use solverforge_core::domain::PlanningSolution;

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Work {
    pub(super) bucket: usize,
    pub(super) demand: i64,
    pub(super) enabled: bool,
}

#[derive(Clone)]
pub(super) struct Capacity {
    pub(super) bucket: usize,
    pub(super) capacity: i64,
}

#[derive(Clone)]
pub(super) struct Plan {
    pub(super) work: Vec<Work>,
    pub(super) capacity: Vec<Capacity>,
}

impl PlanningSolution for Plan {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        None
    }

    fn set_score(&mut self, _score: Option<Self::Score>) {}
}

pub(super) struct Entry {
    pub(super) bucket: usize,
    pub(super) delta: i64,
}

pub(super) struct MoveOnlyEntry {
    pub(super) bucket: usize,
    pub(super) delta: i64,
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub(super) struct MoveOnlyBucket(pub(super) usize);

pub(super) struct MoveOnlyWorkEntryProjection;

impl Projection<Work> for MoveOnlyWorkEntryProjection {
    type Out = MoveOnlyEntry;
    const MAX_EMITS: usize = 1;

    fn project<Sink>(&self, work: &Work, out: &mut Sink)
    where
        Sink: ProjectionSink<Self::Out>,
    {
        out.emit(MoveOnlyEntry {
            bucket: work.bucket,
            delta: work.demand,
        });
    }
}

pub(super) struct MoveOnlyDelta(pub(super) i64);

pub(super) struct MoveOnlyDeltaCollector;

impl Collector<&Entry> for MoveOnlyDeltaCollector {
    type Value = MoveOnlyDelta;
    type Result = i64;
    type Accumulator = MoveOnlyDeltaAccumulator;

    fn extract(&self, entry: &Entry) -> Self::Value {
        MoveOnlyDelta(entry.delta)
    }

    fn create_accumulator(&self) -> Self::Accumulator {
        MoveOnlyDeltaAccumulator { total: 0 }
    }
}

impl Collector<&MoveOnlyEntry> for MoveOnlyDeltaCollector {
    type Value = MoveOnlyDelta;
    type Result = i64;
    type Accumulator = MoveOnlyDeltaAccumulator;

    fn extract(&self, entry: &MoveOnlyEntry) -> Self::Value {
        MoveOnlyDelta(entry.delta)
    }

    fn create_accumulator(&self) -> Self::Accumulator {
        MoveOnlyDeltaAccumulator { total: 0 }
    }
}

pub(super) struct MoveOnlyDeltaAccumulator {
    total: i64,
}

impl Accumulator<MoveOnlyDelta, i64> for MoveOnlyDeltaAccumulator {
    type Retraction = i64;

    fn accumulate(&mut self, value: MoveOnlyDelta) -> Self::Retraction {
        self.total += value.0;
        value.0
    }

    fn retract(&mut self, value: Self::Retraction) {
        self.total -= value;
    }

    fn with_result<T>(&self, f: impl FnOnce(&i64) -> T) -> T {
        f(&self.total)
    }

    fn reset(&mut self) {
        self.total = 0;
    }
}

pub(super) struct WorkTwoEntries;

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

pub(super) struct WorkEntryProjection;

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

pub(super) struct OrderedWorkEntries;

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

pub(super) struct OptionalSecondWorkEntry;

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

pub(super) struct CapacityEntryProjection;

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

pub(super) struct CountedProjection;

pub(super) static PROJECTION_CALLS: AtomicUsize = AtomicUsize::new(0);

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

pub(super) fn work(plan: &Plan) -> &[Work] {
    plan.work.as_slice()
}

pub(super) fn capacity(plan: &Plan) -> &[Capacity] {
    plan.capacity.as_slice()
}

pub(super) fn projected_asymmetric_self_join_constraint(
) -> impl IncrementalConstraint<Plan, SoftScore> {
    ConstraintFactory::<Plan, SoftScore>::new()
        .for_each(source(
            work as fn(&Plan) -> &[Work],
            ChangeSource::Descriptor(0),
        ))
        .project(OptionalSecondWorkEntry)
        .join(equal(|entry: &Entry| entry.bucket))
        .filter(|left: &Entry, right: &Entry| left.delta < right.delta)
        .penalize(|left: &Entry, right: &Entry| SoftScore::of(left.delta * 10 + right.delta))
        .named("projected asymmetric duplicate bucket")
}

pub(super) fn projected_asymmetric_self_join_plan() -> Plan {
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

pub(super) fn fresh_projected_asymmetric_self_join_score(plan: &Plan) -> SoftScore {
    (projected_asymmetric_self_join_constraint(),).evaluate_all(plan)
}

pub(super) fn assert_projected_director_matches_fresh<C>(director: &mut ScoreDirector<Plan, C>)
where
    C: ConstraintSet<Plan, SoftScore>,
{
    let cached = director.calculate_score();
    let fresh = director
        .constraints()
        .evaluate_all(director.working_solution());
    assert_eq!(cached, fresh);
}
