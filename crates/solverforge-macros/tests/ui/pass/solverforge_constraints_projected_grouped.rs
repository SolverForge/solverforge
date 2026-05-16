use solverforge::prelude::*;

#[derive(Clone)]
struct Work {
    bucket: usize,
    demand: i64,
}

struct Entry {
    bucket: usize,
    delta: i64,
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

#[derive(Clone)]
struct Plan {
    work: Vec<Work>,
}

fn work(plan: &Plan) -> &[Work] {
    plan.work.as_slice()
}

#[solverforge_constraints]
fn constraints() -> impl ConstraintSet<Plan, SoftScore> {
    let g = ConstraintFactory::<Plan, SoftScore>::new();
    let demand_by_bucket = g
        .for_each(work as fn(&Plan) -> &[Work])
        .project(WorkEntryProjection)
        .group_by(|entry: &Entry| entry.bucket, sum(|entry: &Entry| entry.delta));

    (
        demand_by_bucket
            .penalize(|_bucket: &usize, demand: &i64| SoftScore::of(*demand))
            .named("linear demand"),
        demand_by_bucket
            .penalize(|_bucket: &usize, demand: &i64| SoftScore::of(*demand * *demand))
            .named("squared demand"),
    )
}

fn main() {
    let mut constraints = constraints();
    let plan = Plan {
        work: vec![
            Work {
                bucket: 0,
                demand: 2,
            },
            Work {
                bucket: 0,
                demand: 3,
            },
        ],
    };

    assert_eq!(constraints.initialize_all(&plan), SoftScore::of(-30));
}
