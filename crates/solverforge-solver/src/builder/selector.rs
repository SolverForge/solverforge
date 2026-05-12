use std::fmt::Debug;

use solverforge_config::{AcceptorConfig, LocalSearchConfig, LocalSearchType, MoveSelectorConfig};
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::{ParseableScore, Score};

use crate::heuristic::r#move::{Move, ScalarMoveUnion};
use crate::heuristic::selector::decorator::{CartesianProductSelector, VecUnionSelector};
use crate::heuristic::selector::move_selector::{
    CandidateId, MoveCandidateRef, MoveCursor, MoveSelector, MoveStreamContext,
};
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;
use crate::phase::localsearch::LocalSearchPhase;
use crate::phase::localsearch::VndPhase;
use crate::phase::Phase;
use crate::scope::{ProgressCallback, SolverScope};

use super::acceptor::{AcceptorBuilder, AnyAcceptor};
use super::context::RuntimeModel;
use super::forager::{AnyForager, ForagerBuilder};
use super::list_selector::ListMoveSelectorBuilder;
use super::scalar_selector::build_scalar_flat_selector;

include!("selector/conflict_repair.rs");
include!("selector/grouped_scalar.rs");
mod types;
use types::LeafSelector;
pub use types::{
    CartesianChildCursor, CartesianChildSelector, Neighborhood, NeighborhoodCursor,
    NeighborhoodLeaf, NeighborhoodLeafCursor, NeighborhoodMove,
};
include!("selector/families.rs");
include!("selector/build.rs");
