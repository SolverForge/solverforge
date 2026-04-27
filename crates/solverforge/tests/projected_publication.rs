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

#[derive(Clone)]
pub struct CapacityEntry {
    pub bucket: usize,
    pub delta: i64,
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
    fn attach_descriptor_scalar_hooks(
        _descriptor: &mut solverforge::__internal::SolutionDescriptor,
    ) {
    }

    fn attach_runtime_scalar_hooks(
        context: solverforge::__internal::ScalarVariableContext<Self>,
    ) -> solverforge::__internal::ScalarVariableContext<Self> {
        context
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

fn assignment_entries(assignment: &Assignment) -> Vec<CapacityEntry> {
    vec![CapacityEntry {
        bucket: assignment.bucket,
        delta: assignment.demand,
    }]
}

fn capacity_entries(capacity: &Capacity) -> Vec<CapacityEntry> {
    vec![CapacityEntry {
        bucket: capacity.bucket,
        delta: -capacity.amount,
    }]
}

#[test]
fn projected_stream_is_public_and_infers_output_type() {
    use PlanConstraintStreams;

    let constraint = ConstraintFactory::<Plan, HardSoftScore>::new()
        .assignments()
        .project(assignment_entries as fn(&Assignment) -> Vec<CapacityEntry>)
        .merge(
            ConstraintFactory::<Plan, HardSoftScore>::new()
                .capacities()
                .project(capacity_entries as fn(&Capacity) -> Vec<CapacityEntry>),
        )
        .group_by(
            |entry: &CapacityEntry| entry.bucket,
            sum(|entry: &CapacityEntry| entry.delta),
        )
        .penalize_hard_with(|delta: &i64| HardSoftScore::of_hard((*delta).max(0)))
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
