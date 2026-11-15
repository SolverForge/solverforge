use solverforge::prelude::*;

/// A machine that processes its tasks in list order.
#[planning_entity]
pub struct Machine {
    /// Stable machine id.
    #[planning_id]
    pub id: usize,

    /// The tasks the machine processes, in order.
    #[planning_list_variable(element_collection = "tasks")]
    pub tasks: Vec<usize>,
}
