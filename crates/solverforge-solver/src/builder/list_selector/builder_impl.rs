use std::fmt::Debug;

use solverforge_config::MoveSelectorConfig;
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::{ListMoveUnion, MoveArena, SequentialCompositeMove};
use crate::heuristic::selector::decorator::{
    CartesianProductCursor, CartesianProductSelector, VecUnionSelector,
};
use crate::heuristic::selector::k_opt::KOptConfig;
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;
use crate::heuristic::selector::{
    move_selector::{CandidateId, MoveCandidateRef, MoveCursor, MoveStreamContext},
    FromSolutionEntitySelector, MoveSelector,
};

use super::super::context::{IntraDistanceAdapter, ListVariableSlot};
use super::leaf::{ListLeafCursor, ListLeafSelector};

include!("builder_impl/types.rs");
include!("builder_impl/builder.rs");
