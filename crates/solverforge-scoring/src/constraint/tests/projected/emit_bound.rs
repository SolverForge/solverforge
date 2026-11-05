use super::support::*;

/// Emits one entry per unit of demand while declaring at most two.
struct EntryPerDemand;

impl Projection<Work> for EntryPerDemand {
    type Out = Entry;
    const MAX_EMITS: usize = 2;

    fn project<Sink>(&self, work: &Work, out: &mut Sink)
    where
        Sink: ProjectionSink<Self::Out>,
    {
        for bucket in 0..work.demand as usize {
            out.emit(Entry { bucket, delta: 1 });
        }
    }
}

fn entries_per_demand() -> impl IncrementalConstraint<Plan, SoftScore> {
    ConstraintFactory::<Plan, SoftScore>::new()
        .for_each(source(
            work as fn(&Plan) -> &[Work],
            ChangeSource::Descriptor(0),
        ))
        .project(EntryPerDemand)
        .group_by(
            |entry: &Entry| entry.bucket,
            sum(|entry: &Entry| entry.delta),
        )
        .penalize(|_bucket: &usize, delta: &i64| SoftScore::of(*delta))
        .named("entries")
}

fn plan(demand: i64) -> Plan {
    Plan {
        work: vec![Work {
            bucket: 0,
            demand,
            enabled: true,
        }],
        capacity: Vec::new(),
    }
}

#[test]
fn a_projection_may_emit_up_to_its_bound() {
    let mut constraint = entries_per_demand();
    let mut plan = plan(1);
    let mut total = constraint.initialize(&plan);
    assert_eq!(total, SoftScore::of(-1));

    total = total + constraint.on_retract(&plan, 0, 0);
    plan.work[0].demand = 2;
    total = total + constraint.on_insert(&plan, 0, 0);
    assert_eq!(total, SoftScore::of(-2));
    assert_eq!(total, constraint.evaluate(&plan));
}

#[test]
#[should_panic(expected = "EntryPerDemand` emitted 3 rows for one input; its MAX_EMITS is 2")]
fn emitting_past_the_bound_panics_on_initialization() {
    entries_per_demand().initialize(&plan(3));
}

#[test]
#[should_panic(expected = "EntryPerDemand` emitted 3 rows for one input; its MAX_EMITS is 2")]
fn emitting_past_the_bound_panics_on_an_incremental_insert() {
    let mut constraint = entries_per_demand();
    let mut plan = plan(2);
    constraint.initialize(&plan);
    constraint.on_retract(&plan, 0, 0);
    plan.work[0].demand = 3;
    constraint.on_insert(&plan, 0, 0);
}
