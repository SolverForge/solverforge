use solverforge::prelude::*;

// readme:begin
/// A vehicle that drives to its stops in list order.
#[planning_entity]
pub struct Vehicle {
    /// Stable vehicle id.
    #[planning_id]
    pub id: usize,

    /// The stops the vehicle makes, in order.
    #[planning_list_variable(element_collection = "stops", element_group_fn = "job_stops")]
    pub stops: Vec<usize>,
}

/// A job's pick-up and drop-off, in that order; a single stop has no group.
pub(super) fn job_stops(plan: &super::Deliveries, stop: usize, members: &mut Vec<usize>) {
    if let Some(job) = plan.stops[stop].job {
        members.extend([2 * job, 2 * job + 1]);
    }
}
// readme:end
