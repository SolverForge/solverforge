use solverforge::prelude::*;
use solverforge::stream::ConstraintFactory;

use super::{Color, Node};

#[planning_solution(
    constraints = "define_constraints",
    solver_toml = "../../solver.toml"
)]
pub struct GraphColoring {
    #[problem_fact_collection]
    pub colors: Vec<Color>,

    #[planning_entity_collection]
    pub nodes: Vec<Node>,

    #[planning_score]
    pub score: Option<HardSoftScore>,
}

fn define_constraints() -> impl ConstraintSet<GraphColoring, HardSoftScore> {
    let unassigned = ConstraintFactory::<GraphColoring, HardSoftScore>::new()
        .for_each(GraphColoring::nodes())
        .unassigned()
        .penalize(HardSoftScore::ONE_HARD)
        .named("Unassigned color");

    let adjacent_same_color = ConstraintFactory::<GraphColoring, HardSoftScore>::new()
        .for_each(GraphColoring::nodes())
        .join((
            ConstraintFactory::<GraphColoring, HardSoftScore>::new()
                .for_each(GraphColoring::nodes()),
            |left: &Node, right: &Node| {
                left.id < right.id
                    && left.neighbors.contains(&right.id)
                    && left.color_idx.is_some()
                    && left.color_idx == right.color_idx
            },
        ))
        .penalize(HardSoftScore::ONE_HARD)
        .named("Adjacent color conflict");

    (unassigned, adjacent_same_color)
}
