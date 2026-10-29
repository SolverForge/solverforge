use crate::heuristic::selector::entity::FromSolutionEntitySelector;
use crate::manager::solver_manager::manager::Solvable;
use crate::heuristic::selector::value_selector::StaticValueSelector;
use crate::phase::construction::placer::QueuedEntityPlacer;
use crate::manager::solver_manager::manager::SolverManager;
use std::borrow::Cow;
use super::*;
use std::any::TypeId;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use solverforge_core::domain::SolutionDescriptor;
use solverforge_core::score::SoftScore;
use solverforge_scoring::Director;

use crate::heuristic::selector::EntityReference;
use crate::heuristic::selector::move_selector::{
    ArenaMoveCursor, CandidateId, CandidateStore, MoveCandidateRef, MoveCursor,
};
use crate::manager::{
    SolverEvent, SolverLifecycleState, SolverRuntime, SolverTerminalReason,
};
use crate::phase::construction::{
    BestFitForager, EntityPlacerCursor, FirstFitForager, Placement,
    
};
use crate::test_utils::{
    create_simple_nqueens_director, get_queen_row, set_queen_row, NQueensSolution,
};

include!("tests/support.rs");
include!("tests/selection.rs");
include!("tests/streaming.rs");
include!("tests/interruption.rs");
include!("tests/lifecycle.rs");
