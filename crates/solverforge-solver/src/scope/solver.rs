// Solver-level scope.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use rand::rngs::StdRng;
use rand::SeedableRng;

use solverforge_config::EnvironmentMode;
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::{Director, RecordingDirector};

use crate::heuristic::r#move::Move;
use crate::manager::{SolverLifecycleState, SolverRuntime, SolverTerminalReason};
use crate::phase::construction::{
    ConstructionFrontier, ConstructionGroupSlotId, ConstructionListElementId, ConstructionSlotId,
};
use crate::stats::SolverStats;

include!("solver/progress.rs");
include!("solver/scope_core.rs");
include!("solver/scope_progress.rs");
