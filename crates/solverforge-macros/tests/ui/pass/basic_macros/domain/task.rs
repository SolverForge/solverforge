use solverforge::prelude::*;

#[planning_entity]
pub struct Task {
    #[planning_id]
    pub id: String,
    #[planning_pin]
    pub pinned: bool,
    #[planning_variable(allows_unassigned = true)]
    pub worker_idx: Option<usize>,
}
