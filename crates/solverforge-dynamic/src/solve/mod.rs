//! Solver implementation for dynamic solutions using real solver infrastructure.

#[cfg(test)]
mod tests;

use std::any::TypeId;
use std::sync::atomic::AtomicBool;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use solverforge_config::SolverConfig;
use solverforge_core::domain::SolutionDescriptor;
use solverforge_core::score::HardSoftScore;
use solverforge_scoring::director::typed::TypedScoreDirector;
use solverforge_scoring::ConstraintSet;
use solverforge_solver::phase::construction::{BestFitForager, ConstructionHeuristicPhase};
use solverforge_solver::phase::localsearch::{
    AcceptedCountForager, LocalSearchPhase, SimulatedAnnealingAcceptor,
};
use solverforge_solver::Solver;
use tracing::info;

use crate::constraint_set::DynamicConstraintSet;
use crate::moves::{
    DynamicChangeMove, DynamicEitherMove, DynamicEntityPlacer, DynamicMoveSelector,
};
use crate::solution::DynamicSolution;

/// Default time limit in seconds.
const DEFAULT_TIME_LIMIT_SECS: u64 = 30;

/// Configuration for the solver.
#[derive(Debug, Clone)]
pub struct SolveConfig {
    /// Maximum time to spend solving.
    pub time_limit: Duration,
}

impl Default for SolveConfig {
    fn default() -> Self {
        Self {
            time_limit: Duration::from_secs(DEFAULT_TIME_LIMIT_SECS),
        }
    }
}

impl SolveConfig {
    /// Creates a new solve config with the given time limit.
    pub fn with_time_limit(time_limit: Duration) -> Self {
        Self { time_limit }
    }
}

/// Result of solving.
#[derive(Debug, Clone)]
pub struct SolveResult {
    /// The best solution found.
    pub solution: DynamicSolution,
    /// The score of the best solution.
    pub score: HardSoftScore,
    /// Total time spent solving.
    pub duration: Duration,
    /// Number of steps taken.
    pub steps: u64,
    /// Number of moves evaluated.
    pub moves_evaluated: u64,
}

impl SolveResult {
    /// Returns true if the solution is feasible (no hard constraint violations).
    pub fn is_feasible(&self) -> bool {
        self.score.hard() >= 0
    }
}

/// Solves the given problem using the real solver infrastructure.
///
/// Uses ConstructionHeuristicPhase + LocalSearchPhase with Late Acceptance.
/// Loads configuration from solver.toml if available.
pub fn solve(
    solution: DynamicSolution,
    constraints: DynamicConstraintSet,
    config: SolveConfig,
) -> SolveResult {
    let terminate = AtomicBool::new(false);
    let snapshot = Mutex::new(None);
    solve_with_controls(solution, constraints, config, &terminate, &snapshot)
}

/// Solves with external termination flag and best solution snapshot.
///
/// - `terminate`: Set to true to stop solving early
/// - `snapshot`: Receives best solution updates during solving
pub fn solve_with_controls(
    solution: DynamicSolution,
    constraints: DynamicConstraintSet,
    config: SolveConfig,
    terminate: &AtomicBool,
    snapshot: &Mutex<Option<DynamicSolution>>,
) -> SolveResult {
    // Initialize console output (identical to native Rust solver)
    solverforge_console::init();

    // Load solver.toml config if available
    let solver_config = SolverConfig::load("solver.toml").unwrap_or_default();

    let start = Instant::now();

    // Count total entities across all classes
    let entity_count: usize = solution.entities.iter().map(|v| v.len()).sum();
    // Count total values across all value ranges
    let value_count: usize = solution
        .descriptor
        .value_ranges
        .values()
        .map(|vr| vr.len())
        .sum();

    info!(
        event = "solve_start",
        entity_count = entity_count,
        value_count = value_count,
        constraint_count = constraints.constraint_count(),
        time_limit_secs = config.time_limit.as_secs(),
    );

    // Create solution descriptor (required by ScoreDirector)
    let descriptor = SolutionDescriptor::new("DynamicSolution", TypeId::of::<DynamicSolution>());

    // Entity counter function for TypedScoreDirector
    fn entity_counter(s: &DynamicSolution, idx: usize) -> usize {
        s.entities.get(idx).map(|v| v.len()).unwrap_or(0)
    }

    // Create typed score director with incremental constraint set
    let score_director =
        TypedScoreDirector::with_descriptor(solution, constraints, descriptor, entity_counter);

    // Create construction phase with BestFitForager to evaluate scores
    let construction: ConstructionHeuristicPhase<
        DynamicSolution,
        DynamicChangeMove,
        DynamicEntityPlacer,
        BestFitForager<DynamicSolution, DynamicChangeMove>,
    > = ConstructionHeuristicPhase::new(DynamicEntityPlacer::new(), BestFitForager::new());

    // Get step limit from solver.toml config
    let step_limit = solver_config
        .termination
        .as_ref()
        .and_then(|t| t.step_count_limit);

    // Create local search phase with Simulated Annealing + unified move selector
    let local_search: LocalSearchPhase<
        DynamicSolution,
        DynamicEitherMove,
        DynamicMoveSelector,
        SimulatedAnnealingAcceptor,
        AcceptedCountForager<DynamicSolution>,
    > = LocalSearchPhase::new(
        DynamicMoveSelector::new(),
        SimulatedAnnealingAcceptor::default(),
        AcceptedCountForager::new(4),
        step_limit,
    );

    // Build termination based on config - use time_limit from solver.toml or config
    let time_limit = solver_config
        .termination
        .as_ref()
        .and_then(|c| c.time_limit())
        .unwrap_or(config.time_limit);

    // Wire best-solution snapshot: write to the Mutex whenever a better solution is found.
    // This allows Python to poll intermediate results via SolverManager.get_best_solution().
    let snapshot_callback: Box<dyn Fn(&DynamicSolution) + Send + Sync> =
        Box::new(move |solution: &DynamicSolution| {
            if let Ok(mut guard) = snapshot.try_lock() {
                *guard = Some(solution.clone());
            }
        });

    // Create and run solver with snapshot callback
    let solver_result = Solver::new(((), construction, local_search))
        .with_time_limit(time_limit)
        .with_terminate(terminate)
        .with_best_solution_callback(snapshot_callback)
        .solve(score_director);

    let duration = start.elapsed();
    let score = solver_result.solution.score.unwrap_or(HardSoftScore::ZERO);

    info!(
        event = "solve_end",
        score = %score,
        feasible = score.hard() >= 0,
        duration_ms = duration.as_millis(),
        steps = solver_result.stats.step_count,
        moves_evaluated = solver_result.stats.moves_evaluated,
    );

    SolveResult {
        solution: solver_result.solution,
        score,
        duration,
        steps: solver_result.stats.step_count,
        moves_evaluated: solver_result.stats.moves_evaluated,
    }
}
