use solverforge_config::{
    CartesianProductConfig, ChangeMoveConfig, MoveSelectorConfig, NearbyChangeMoveConfig,
    NearbySwapMoveConfig, PillarChangeMoveConfig, PillarSwapMoveConfig, RecreateHeuristicType,
    RuinRecreateMoveSelectorConfig, SwapMoveConfig, UnionMoveSelectorConfig, VariableTargetConfig,
};
use solverforge_core::domain::{
    EntityCollectionExtractor, EntityDescriptor, PlanningSolution, SolutionDescriptor,
};
use solverforge_core::score::SoftScore;
use solverforge_scoring::ScoreDirector;
use std::any::TypeId;

use super::*;
use crate::heuristic::r#move::Move;
use crate::heuristic::selector::decorator::FilteringMoveSelector;
use crate::heuristic::selector::move_selector::{collect_cursor_indices, MoveCandidateRef};
use crate::heuristic::selector::MoveSelector;

include!("tests/support.rs");
include!("tests/change_swap.rs");
include!("tests/nearby_ruin.rs");
include!("tests/pillar_cartesian.rs");
