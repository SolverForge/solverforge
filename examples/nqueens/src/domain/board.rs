use solverforge::prelude::*;
use solverforge::stream::ConstraintFactory;

use super::{Queen, Row};

#[planning_solution(
    constraints = "define_constraints",
    solver_toml = "../../solver.toml"
)]
pub struct Board {
    #[problem_fact_collection]
    pub rows: Vec<Row>,

    #[planning_entity_collection]
    pub queens: Vec<Queen>,

    #[planning_score]
    pub score: Option<HardSoftScore>,
}

fn define_constraints() -> impl ConstraintSet<Board, HardSoftScore> {
    let unassigned = ConstraintFactory::<Board, HardSoftScore>::new()
        .for_each(Board::queens())
        .unassigned()
        .penalize_hard()
        .named("Unassigned queen");

    let conflict = ConstraintFactory::<Board, HardSoftScore>::new()
        .for_each(Board::queens())
        .join((
            ConstraintFactory::<Board, HardSoftScore>::new().for_each(Board::queens()),
            |left: &Queen, right: &Queen| {
                if left.column >= right.column {
                    return false;
                }
                let (Some(left_row), Some(right_row)) = (left.row_idx, right.row_idx) else {
                    return false;
                };
                left_row == right_row
                    || left_row.abs_diff(right_row) == left.column.abs_diff(right.column)
            },
        ))
        .penalize_hard()
        .named("Queen conflict");

    (unassigned, conflict)
}
