use solverforge::prelude::*;
use solverforge::stream::ConstraintFactory;
use solverforge::IncrementalConstraint;

#[problem_fact]
pub struct Capacity {
    #[planning_id]
    pub id: usize,
    pub bucket: usize,
    pub amount: i64,
}

#[planning_entity]
pub struct Assignment {
    #[planning_id]
    pub id: usize,
    pub bucket: usize,
    pub demand: i64,
}

pub struct CapacityEntry {
    pub bucket: usize,
    pub delta: i64,
}

pub struct AssignmentCapacity {
    pub bucket: usize,
    pub demand: i64,
    pub capacity: i64,
}

#[planning_solution]
pub struct Plan {
    #[problem_fact_collection]
    pub capacities: Vec<Capacity>,
    #[planning_entity_collection]
    pub assignments: Vec<Assignment>,
    #[planning_score]
    pub score: Option<HardSoftScore>,
}

impl solverforge::__internal::PlanningModelSupport for Plan {
    fn attach_descriptor_hooks(_descriptor: &mut solverforge::__internal::SolutionDescriptor) {}

    fn attach_runtime_scalar_hooks(
        slot: solverforge::__internal::ScalarVariableSlot<Self>,
    ) -> solverforge::__internal::ScalarVariableSlot<Self> {
        slot
    }

    fn validate_model(_descriptor: &solverforge::__internal::SolutionDescriptor) {}

    fn update_entity_shadows(
        _solution: &mut Self,
        _descriptor_index: usize,
        _entity_index: usize,
    ) -> bool {
        false
    }

    fn update_all_shadows(_solution: &mut Self) -> bool {
        false
    }
}

struct AssignmentEntries;

impl Projection<Assignment> for AssignmentEntries {
    type Out = CapacityEntry;
    const MAX_EMITS: usize = 1;

    fn project<Sink>(&self, assignment: &Assignment, sink: &mut Sink)
    where
        Sink: ProjectionSink<Self::Out>,
    {
        sink.emit(CapacityEntry {
            bucket: assignment.bucket,
            delta: assignment.demand,
        });
    }
}

struct CapacityEntries;

impl Projection<Capacity> for CapacityEntries {
    type Out = CapacityEntry;
    const MAX_EMITS: usize = 1;

    fn project<Sink>(&self, capacity: &Capacity, sink: &mut Sink)
    where
        Sink: ProjectionSink<Self::Out>,
    {
        sink.emit(CapacityEntry {
            bucket: capacity.bucket,
            delta: -capacity.amount,
        });
    }
}

#[test]
fn projected_stream_is_public_and_infers_output_type() {
    let constraint = ConstraintFactory::<Plan, HardSoftScore>::new()
        .for_each(Plan::assignments())
        .project(AssignmentEntries)
        .merge(
            ConstraintFactory::<Plan, HardSoftScore>::new()
                .for_each(Plan::capacities())
                .project(CapacityEntries),
        )
        .group_by(
            |entry: &CapacityEntry| entry.bucket,
            sum(|entry: &CapacityEntry| entry.delta),
        )
        .penalize(|_bucket: &usize, delta: &i64| HardSoftScore::of_hard((*delta).max(0)))
        .named("capacity shortage");

    let plan = Plan {
        capacities: vec![Capacity {
            id: 0,
            bucket: 0,
            amount: 3,
        }],
        assignments: vec![Assignment {
            id: 0,
            bucket: 0,
            demand: 5,
        }],
        score: None,
    };

    assert_eq!(constraint.evaluate(&plan), HardSoftScore::of(-2, 0));
}

#[test]
fn cross_join_project_is_public_and_infers_output_type() {
    let constraint = ConstraintFactory::<Plan, HardSoftScore>::new()
        .for_each(Plan::assignments())
        .join((
            ConstraintFactory::<Plan, HardSoftScore>::new().for_each(Plan::capacities()),
            joiner::equal_bi(
                |assignment: &Assignment| assignment.bucket,
                |capacity: &Capacity| capacity.bucket,
            ),
        ))
        .project(
            |assignment: &Assignment, capacity: &Capacity| AssignmentCapacity {
                bucket: assignment.bucket,
                demand: assignment.demand,
                capacity: capacity.amount,
            },
        )
        .penalize(|row: &AssignmentCapacity| {
            HardSoftScore::of_hard((row.demand - row.capacity).max(0))
        })
        .named("assignment capacity shortage");

    let plan = Plan {
        capacities: vec![Capacity {
            id: 0,
            bucket: 0,
            amount: 3,
        }],
        assignments: vec![Assignment {
            id: 0,
            bucket: 0,
            demand: 5,
        }],
        score: None,
    };

    assert_eq!(constraint.evaluate(&plan), HardSoftScore::of(-2, 0));
}

#[test]
fn cross_join_accepts_filtered_source_stream_target() {
    let constraint = ConstraintFactory::<Plan, HardSoftScore>::new()
        .for_each(Plan::assignments())
        .join((
            ConstraintFactory::<Plan, HardSoftScore>::new()
                .for_each(Plan::capacities())
                .filter(|capacity: &Capacity| capacity.amount > 5),
            joiner::equal_bi(
                |assignment: &Assignment| assignment.bucket,
                |capacity: &Capacity| capacity.bucket,
            ),
        ))
        .penalize(|_assignment: &Assignment, capacity: &Capacity| {
            HardSoftScore::of_hard(capacity.amount)
        })
        .named("filtered capacity target");

    let plan = Plan {
        capacities: vec![
            Capacity {
                id: 0,
                bucket: 0,
                amount: 3,
            },
            Capacity {
                id: 1,
                bucket: 0,
                amount: 8,
            },
        ],
        assignments: vec![Assignment {
            id: 0,
            bucket: 0,
            demand: 5,
        }],
        score: None,
    };

    assert_eq!(constraint.evaluate(&plan), HardSoftScore::of(-8, 0));
}

#[test]
fn cross_join_accepts_generated_source_target() {
    let constraint = ConstraintFactory::<Plan, HardSoftScore>::new()
        .for_each(Plan::assignments())
        .join((
            Plan::capacities(),
            joiner::equal_bi(
                |assignment: &Assignment| assignment.bucket,
                |capacity: &Capacity| capacity.bucket,
            ),
        ))
        .penalize(|_assignment: &Assignment, capacity: &Capacity| {
            HardSoftScore::of_hard(capacity.amount)
        })
        .named("generated capacity target");

    let plan = Plan {
        capacities: vec![Capacity {
            id: 0,
            bucket: 0,
            amount: 3,
        }],
        assignments: vec![Assignment {
            id: 0,
            bucket: 0,
            demand: 5,
        }],
        score: None,
    };

    assert_eq!(constraint.evaluate(&plan), HardSoftScore::of(-3, 0));
}

#[test]
fn cross_join_group_by_is_public_without_projecting_first() {
    let constraint = ConstraintFactory::<Plan, HardSoftScore>::new()
        .for_each(Plan::assignments())
        .join((
            ConstraintFactory::<Plan, HardSoftScore>::new().for_each(Plan::capacities()),
            joiner::equal_bi(
                |assignment: &Assignment| assignment.bucket,
                |capacity: &Capacity| capacity.bucket,
            ),
        ))
        .group_by(
            |_assignment: &Assignment, capacity: &Capacity| capacity.bucket,
            sum(|(_assignment, _capacity): (&Assignment, &Capacity)| 1i64),
        )
        .penalize(|_bucket: &usize, count: &i64| HardSoftScore::of_hard(*count))
        .named("assignments per capacity");

    let plan = Plan {
        capacities: vec![Capacity {
            id: 0,
            bucket: 0,
            amount: 3,
        }],
        assignments: vec![Assignment {
            id: 0,
            bucket: 0,
            demand: 5,
        }],
        score: None,
    };

    assert_eq!(constraint.evaluate(&plan), HardSoftScore::of(-1, 0));
}

#[test]
fn cross_join_project_group_by_complement_is_public() {
    let constraint = ConstraintFactory::<Plan, HardSoftScore>::new()
        .for_each(Plan::assignments())
        .join((
            ConstraintFactory::<Plan, HardSoftScore>::new().for_each(Plan::capacities()),
            joiner::equal_bi(
                |assignment: &Assignment| assignment.bucket,
                |capacity: &Capacity| capacity.bucket,
            ),
        ))
        .project(
            |assignment: &Assignment, capacity: &Capacity| CapacityEntry {
                bucket: assignment.bucket,
                delta: assignment.demand - capacity.amount,
            },
        )
        .group_by(
            |entry: &CapacityEntry| entry.bucket,
            sum(|entry: &CapacityEntry| entry.delta),
        )
        .complement(
            Plan::capacities(),
            |capacity: &Capacity| capacity.bucket,
            |_| 0i64,
        )
        .penalize(|bucket: &usize, delta: &i64| {
            HardSoftScore::of_hard((*bucket as i64 * 10) + *delta)
        })
        .named("assignment capacity shortage including empty capacity");

    let plan = Plan {
        capacities: vec![
            Capacity {
                id: 0,
                bucket: 0,
                amount: 3,
            },
            Capacity {
                id: 1,
                bucket: 1,
                amount: 4,
            },
        ],
        assignments: vec![Assignment {
            id: 0,
            bucket: 0,
            demand: 5,
        }],
        score: None,
    };

    assert_eq!(constraint.evaluate(&plan), HardSoftScore::of(-12, 0));
}
