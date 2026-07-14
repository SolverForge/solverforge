use solverforge::prelude::*;

#[planning_entity]
pub struct Visit {
    #[planning_id]
    pub id: usize,

    #[index_shadow_variable(source_variable_name = "visits")]
    pub index: usize,
}

fn main() {}
