use solverforge::prelude::*;

struct Entry {
    bucket: usize,
    delta: i64,
}

struct Employee {
    id: usize,
}

struct WorkEntryProjection;

impl Projection<(usize, i64)> for WorkEntryProjection {
    type Out = Entry;
    const MAX_EMITS: usize = 1;

    fn project<Sink>(&self, work: &(usize, i64), out: &mut Sink)
    where
        Sink: ProjectionSink<Self::Out>,
    {
        out.emit(Entry {
            bucket: work.0,
            delta: work.1,
        });
    }
}

struct Plan {
    work: Vec<(usize, i64)>,
    employees: Vec<Employee>,
}

fn work(plan: &Plan) -> &[(usize, i64)] {
    plan.work.as_slice()
}

fn employees(plan: &Plan) -> &[Employee] {
    plan.employees.as_slice()
}

#[solverforge_constraints]
fn constraints() -> impl ConstraintSet<Plan, SoftScore> {
    let g = ConstraintFactory::<Plan, SoftScore>::new();
    let demand_by_bucket = g
        .for_each(work as fn(&Plan) -> &[(usize, i64)])
        .project(WorkEntryProjection)
        .group_by(|entry: &Entry| entry.bucket, sum(|entry: &Entry| entry.delta))
        .complement(
            employees as fn(&Plan) -> &[Employee],
            |employee: &Employee| employee.id,
            |_employee: &Employee| 0i64,
        );

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
        work: vec![(0, 2), (0, 3)],
        employees: vec![Employee { id: 0 }, Employee { id: 1 }],
    };

    assert_eq!(constraints.initialize_all(&plan), SoftScore::of(-30));
}
