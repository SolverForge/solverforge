use solverforge_macros::{planning_entity, planning_solution};

#[planning_entity]
struct Task {
    #[planning_id]
    id: String,
}

#[planning_solution]
struct Plan {
    #[planning_entity_collection]
    tasks: Vec<Task>,
}

fn main() {}
