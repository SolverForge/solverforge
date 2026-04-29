use std::fmt::{self, Debug};

use solverforge_config::{
    AcceptorConfig, ChangeMoveConfig, ListReverseMoveConfig, LocalSearchConfig, MoveSelectorConfig,
    NearbyListChangeMoveConfig, NearbyListSwapMoveConfig, VariableTargetConfig, VndConfig,
};
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::{ParseableScore, Score};

use crate::heuristic::r#move::{
    ListMoveUnion, Move, MoveArena, MoveTabuSignature, ScalarMoveUnion, SequentialCompositeMove,
};
use crate::heuristic::selector::decorator::{
    CartesianProductCursor, CartesianProductSelector, LimitedMoveCursor, MappedMoveCursor,
    VecUnionSelector,
};
use crate::heuristic::selector::move_selector::{
    collect_cursor_indices, CandidateId, MoveCandidateRef, MoveCursor, MoveSelector,
};
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;
use crate::phase::dynamic_vnd::DynamicVndPhase;
use crate::phase::localsearch::{
    AcceptedCountForager, LocalSearchPhase, SimulatedAnnealingAcceptor,
};

use super::acceptor::{AcceptorBuilder, AnyAcceptor};
use super::context::ModelContext;
use super::forager::{AnyForager, ForagerBuilder};
use super::list_selector::{ListLeafSelector, ListMoveSelectorBuilder};
use super::scalar_selector::{build_scalar_flat_selector, ScalarLeafSelector};

include!("selector/conflict_repair.rs");
include!("selector/types.rs");
include!("selector/families.rs");
include!("selector/build.rs");
