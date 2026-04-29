use std::any::TypeId;

use solverforge_config::{
    AcceptorConfig, CartesianProductConfig, ChangeMoveConfig, ForagerConfig, LateAcceptanceConfig,
    LimitedNeighborhoodConfig, ListChangeMoveConfig, ListReverseMoveConfig,
    ListRuinMoveSelectorConfig, LocalSearchConfig, MoveSelectorConfig,
    RuinRecreateMoveSelectorConfig, SwapMoveConfig, TerminationConfig, UnionMoveSelectorConfig,
    VariableTargetConfig,
};
use solverforge_core::domain::{
    EntityCollectionExtractor, EntityDescriptor, PlanningSolution, SolutionDescriptor,
    VariableDescriptor,
};
use solverforge_core::score::SoftScore;
use solverforge_core::ConstraintRef;
use solverforge_scoring::{DetailedConstraintMatch, IncrementalConstraint, ScoreDirector};

use super::*;
use crate::builder::list_selector::ListLeafSelector;
use crate::builder::scalar_selector::ScalarLeafSelector;
use crate::builder::{ListVariableContext, ScalarVariableContext, ValueSource, VariableContext};
use crate::heuristic::selector::decorator::FilteringMoveSelector;
use crate::heuristic::selector::move_selector::{
    collect_cursor_indices, MoveCandidateRef, MoveCursor,
};
use crate::CrossEntityDistanceMeter;

include!("tests/support.rs");
include!("tests/defaults.rs");
include!("tests/cartesian.rs");
include!("tests/grouped_scalar.rs");
include!("tests/phases.rs");
