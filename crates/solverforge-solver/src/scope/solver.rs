// Solver-level scope.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use rand::rngs::StdRng;
use rand::SeedableRng;

use solverforge_config::{EnvironmentMode, TerminationConfig};
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::ParseableScore;
use solverforge_scoring::{requires_weighted_mode, Director};

use crate::constraint_weights::validate_director_constraint_weights;
use crate::heuristic::r#move::Move;
use crate::manager::{SolverLifecycleState, SolverRuntime, SolverTerminalReason};
use crate::phase::construction::{
    ConstructionFrontier, ConstructionGroupSlotId, ConstructionListElementId, ConstructionSlotId,
};
use crate::realtime::ProblemChange;
use crate::runtime_build_error::{RuntimeBuildError, RuntimeBuildResult};
use crate::stats::{
    CandidatePullTelemetry, CandidateTraceConstructionTarget, CandidateTraceDisposition,
    CandidateTraceHeader, CandidateTracePullToken, CandidateTraceRecordDecision,
    CandidateTraceSource, CandidateTraceTelemetry, SolverStats,
};

include!("solver/progress.rs");
include!("solver/scope_core.rs");
include!("solver/phase_budget.rs");
include!("solver/scope_progress.rs");
include!("solver/scope_trace.rs");
include!("solver/problem_change.rs");
include!("solver/weight_mode.rs");

impl<S: PlanningSolution, D: Director<S>, ProgressCb> SolverScope<'_, S, D, ProgressCb> {
    /// Closes the running phase's statistics record. Bound-free so a phase
    /// scope can close its record when it drops.
    pub(crate) fn end_phase_stats(&mut self) {
        self.stats.end_phase();
    }
}
