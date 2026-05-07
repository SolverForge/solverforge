use std::any::{Any, TypeId};
use std::sync::atomic::{AtomicBool, Ordering};

use solverforge_config::{
    CartesianProductConfig, ChangeMoveConfig, ConstructionHeuristicConfig,
    ConstructionHeuristicType, MoveSelectorConfig, NearbyChangeMoveConfig, NearbySwapMoveConfig,
    PillarChangeMoveConfig, PillarSwapMoveConfig, RecreateHeuristicType,
    RuinRecreateMoveSelectorConfig, SwapMoveConfig, VariableTargetConfig,
};
use solverforge_core::domain::{
    EntityCollectionExtractor, EntityDescriptor, PlanningSolution, ProblemFactDescriptor,
    SolutionDescriptor, ValueRangeType, VariableDescriptor,
};
use solverforge_core::score::SoftScore;
use solverforge_scoring::{Director, ScoreDirector};

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::decorator::FilteringMoveSelector;
use crate::heuristic::selector::move_selector::{
    collect_cursor_indices, MoveCandidateRef, MoveCursor, MoveSelector,
};
use crate::phase::localsearch::{FirstAcceptedForager, HillClimbingAcceptor, LocalSearchPhase};
use crate::phase::Phase;
use crate::scope::SolverScope;

use super::{build_descriptor_construction, build_descriptor_move_selector, scalar_work_remaining};

include!("mod/support.rs");
include!("mod/construction.rs");
include!("mod/selectors.rs");
include!("mod/ruin_recreate.rs");
