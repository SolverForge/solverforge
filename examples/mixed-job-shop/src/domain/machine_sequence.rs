use solverforge::prelude::*;

#[planning_entity]
pub struct MachineSequence {
    #[planning_id]
    pub id: usize,

    #[planning_list_variable(element_collection = "operation_values")]
    pub operations: Vec<usize>,
}
