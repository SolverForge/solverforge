use solverforge::prelude::*;

/// A route with a list variable and a scalar variable.
#[planning_entity]
pub struct Route {
    /// Stable route id.
    #[planning_id]
    pub id: usize,

    /// Visits on this route, in order.
    #[planning_list_variable(element_collection = "visits")]
    pub visits: Vec<usize>,

    /// The visit this route starts with.
    #[planning_variable(value_range_provider = "visits", allows_unassigned = true)]
    pub first_visit: Option<usize>,
}
