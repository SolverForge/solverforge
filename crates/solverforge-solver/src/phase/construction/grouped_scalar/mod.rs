mod assignment_candidate;
mod assignment_construction;
mod assignment_path;
mod assignment_rematch;
mod assignment_state;
mod candidate;
mod move_build;
mod phase;
mod selection;

#[cfg(test)]
pub(crate) use assignment_candidate::required_assignment_moves;
pub(crate) use assignment_candidate::{selector_assignment_moves, ScalarAssignmentMoveOptions};
#[cfg(test)]
pub(crate) use assignment_rematch::rematch_assignment_moves;
pub(crate) use move_build::compound_move_for_group_candidate;
pub(crate) use phase::solve_grouped_scalar_construction;
