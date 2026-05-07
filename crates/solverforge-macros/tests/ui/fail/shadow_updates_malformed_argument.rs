use solverforge::prelude::*;

#[planning_solution]
#[shadow_variable_updates(list_owner = routes)]
pub struct Plan {
    #[planning_score]
    pub score: Option<HardSoftScore>,
}

fn main() {}
