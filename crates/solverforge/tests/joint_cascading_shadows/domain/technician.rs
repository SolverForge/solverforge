use solverforge::prelude::*;

/// A technician who drives to the visits in list order.
#[planning_entity]
pub struct Technician {
    /// Stable technician id.
    #[planning_id]
    pub id: usize,

    /// The visits the technician makes, in order.
    #[planning_list_variable(element_collection = "visits")]
    pub visits: Vec<usize>,
}
