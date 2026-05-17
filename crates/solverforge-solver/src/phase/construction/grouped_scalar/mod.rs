mod assignment_block;
mod assignment_candidate;
mod assignment_cycle;
mod assignment_edge;
mod assignment_entity;
mod assignment_family;
mod assignment_index;
mod assignment_pair;
mod assignment_path;
mod assignment_required_batch;
mod assignment_state;
mod assignment_stream;
mod assignment_value_cycle;
mod assignment_value_index;
mod assignment_value_release;
mod assignment_value_run;
mod move_build;
mod phase;
mod placement;
mod placer;
mod placer_stream;

#[cfg(test)]
pub(crate) use assignment_candidate::selector_assignment_moves;
pub(crate) use assignment_candidate::ScalarAssignmentMoveOptions;
#[cfg(test)]
pub(crate) use assignment_stream::rematch_assignment_moves;
#[cfg(test)]
pub(crate) use assignment_stream::value_block_reassignment_assignment_moves;
#[cfg(test)]
pub(crate) use assignment_stream::value_window_assignment_moves;
pub(crate) use assignment_stream::ScalarAssignmentMoveCursor;
pub(crate) use move_build::compound_move_for_group_candidate;
pub(crate) use phase::build_scalar_group_construction;
pub(crate) use placement::scalar_group_move_strength;
