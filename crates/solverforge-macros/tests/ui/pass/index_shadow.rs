use solverforge::prelude::*;

#[planning_entity]
struct Visit {
    #[planning_id]
    id: usize,

    #[index_shadow_variable(source_variable_name = "visits")]
    index: Option<usize>,
}

fn main() {
    let descriptor = Visit::entity_descriptor("visits");
    let index = descriptor
        .find_variable("index")
        .expect("generated index shadow descriptor");
    assert_eq!(format!("{:?}", index.variable_type), "Shadow(Index)");
}
