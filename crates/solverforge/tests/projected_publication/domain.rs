//! The capacity/assignment model and projections the projected-stream tests
//! score.

use solverforge::prelude::*;

/// Capacity available in one bucket.
#[problem_fact]
pub struct Capacity {
    /// Stable capacity id.
    #[planning_id]
    pub id: usize,
    /// Bucket this capacity belongs to.
    pub bucket: usize,
    /// Capacity amount.
    pub amount: i64,
}

/// Demand placed in one bucket.
#[planning_entity]
pub struct Assignment {
    /// Stable assignment id.
    #[planning_id]
    pub id: usize,
    /// Bucket this demand is placed in.
    pub bucket: usize,
    /// Demand amount.
    pub demand: i64,
}

/// Projected row: a signed capacity change for one bucket.
pub struct CapacityEntry {
    /// Bucket the change applies to.
    pub bucket: usize,
    /// Signed capacity change.
    pub delta: i64,
}

/// Joined row: one assignment's demand next to its bucket's capacity.
pub struct AssignmentCapacity {
    /// Shared bucket.
    pub bucket: usize,
    /// Assignment demand.
    pub demand: i64,
    /// Bucket capacity.
    pub capacity: i64,
}

/// Grouped row with an optional parent group.
pub struct GroupRow {
    /// Group id.
    pub group: usize,
    /// Parent group id, if any.
    pub parent: Option<usize>,
}

/// Capacities and assignments scored by the projected-stream tests.
#[planning_solution]
pub struct Plan {
    /// Bucket capacities.
    #[problem_fact_collection]
    pub capacities: Vec<Capacity>,
    /// Demand assignments.
    #[planning_entity_collection]
    pub assignments: Vec<Assignment>,
    /// The plan's score.
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

pub(crate) struct AssignmentEntries;

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

pub(crate) struct CapacityEntries;

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

pub(crate) struct GroupRows;

impl Projection<Assignment> for GroupRows {
    type Out = GroupRow;
    const MAX_EMITS: usize = 1;

    fn project<Sink>(&self, assignment: &Assignment, sink: &mut Sink)
    where
        Sink: ProjectionSink<Self::Out>,
    {
        sink.emit(GroupRow {
            group: assignment.bucket,
            parent: (assignment.demand >= 0).then_some(assignment.demand as usize),
        });
    }
}

pub(crate) struct CustomCapacityExtract;

impl solverforge::stream::CollectionExtract<Plan> for CustomCapacityExtract {
    type Item = Capacity;

    fn extract<'s>(&self, plan: &'s Plan) -> &'s [Capacity] {
        plan.capacities.as_slice()
    }

    // Capacities are problem facts: no planning-variable change alters them.
    fn change_source(&self) -> solverforge::stream::ChangeSource {
        solverforge::stream::ChangeSource::Static
    }
}
