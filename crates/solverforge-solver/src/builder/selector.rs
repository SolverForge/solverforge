use std::fmt::Debug;

use solverforge_config::{
    AcceptorConfig, ChangeMoveConfig, ListReverseMoveConfig, LocalSearchConfig, MoveSelectorConfig,
    NearbyListChangeMoveConfig, NearbyListSwapMoveConfig, VariableTargetConfig, VndConfig,
};
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::{ParseableScore, Score};

use crate::heuristic::r#move::{Move, ScalarMoveUnion, SequentialCompositeMove};
use crate::heuristic::selector::decorator::{CartesianProductSelector, VecUnionSelector};
use crate::heuristic::selector::move_selector::{
    CandidateId, MoveCandidateRef, MoveCursor, MoveSelector,
};
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;
use crate::phase::dynamic_vnd::DynamicVndPhase;
use crate::phase::localsearch::{
    AcceptedCountForager, LocalSearchPhase, SimulatedAnnealingAcceptor,
};

use super::acceptor::{AcceptorBuilder, AnyAcceptor};
use super::context::RuntimeModel;
use super::forager::{AnyForager, ForagerBuilder};
use super::list_selector::ListMoveSelectorBuilder;
use super::scalar_selector::build_scalar_flat_selector;

include!("selector/conflict_repair.rs");
include!("selector/coverage_repair.rs");
include!("selector/grouped_scalar.rs");
mod types;
use types::LeafSelector;
pub use types::{
    CartesianChildCursor, CartesianChildSelector, Neighborhood, NeighborhoodCursor,
    NeighborhoodLeaf, NeighborhoodLeafCursor, NeighborhoodMove,
};
include!("selector/families.rs");
include!("selector/build.rs");
