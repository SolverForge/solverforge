use solverforge::prelude::*;

use super::Task;

#[planning_solution]
pub struct Plan {
    #[planning_entity_collection]
    pub tasks: Vec<Task>,

    #[constraint_weights(default = 1)]
    pub weights: ConstraintWeights,

    #[planning_score]
    pub score: Option<HardSoftScore>,
}
