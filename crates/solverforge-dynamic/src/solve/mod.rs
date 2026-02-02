//! Solver implementation for dynamic solutions using real solver infrastructure.

#[cfg(test)]
mod tests;

use std::any::TypeId;
use std::sync::atomic::AtomicBool;
use std::sync::{Mutex, Once};
use std::time::{Duration, Instant};

use solverforge_core::domain::SolutionDescriptor;
use solverforge_core::score::HardSoftScore;
use solverforge_scoring::director::typed::TypedScoreDirector;
use solverforge_scoring::ConstraintSet;
use solverforge_solver::phase::construction::{BestFitForager, ConstructionHeuristicPhase};
use solverforge_solver::phase::localsearch::{
    FirstAcceptedForager, LateAcceptanceAcceptor, LocalSearchPhase,
};
use solverforge_solver::Solver;
use tracing::info;

use crate::constraint_set::DynamicConstraintSet;
use crate::moves::{DynamicChangeMove, DynamicEntityPlacer, DynamicMoveSelector};
use crate::solution::DynamicSolution;

/// Configuration for the solver.
#[derive(Debug, Clone)]
pub struct SolveConfig {
    /// Maximum time to spend solving.
    pub time_limit: Duration,
    /// Late acceptance history size.
    pub late_acceptance_size: usize,
}

impl Default for SolveConfig {
    fn default() -> Self {
        Self {
            time_limit: Duration::from_secs(30),
            late_acceptance_size: 400,
        }
    }
}

impl SolveConfig {
    /// Creates a new solve config with the given time limit.
    pub fn with_time_limit(time_limit: Duration) -> Self {
        Self {
            time_limit,
            ..Default::default()
        }
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
pub fn solve(
    solution: DynamicSolution,
    constraints: DynamicConstraintSet,
    config: SolveConfig,
) -> SolveResult {
    let terminate = AtomicBool::new(false);
    let snapshot = Mutex::new(None);
    solve_with_controls(solution, constraints, config, &terminate, &snapshot)
}

/// Initialize tracing once for the process.
static TRACING_INIT: Once = Once::new();

fn init_tracing() {
    TRACING_INIT.call_once(|| {
        use tracing_subscriber::EnvFilter;
        let subscriber = tracing_subscriber::fmt()
            .with_target(false)
            .with_level(true)
            .with_writer(std::io::stderr)
            .with_env_filter(
                EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
            )
            .finish();
        let _ = tracing::subscriber::set_global_default(subscriber);
    });
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
    init_tracing();
    let start = Instant::now();

    eprintln!(
        "[DEBUG] solve_start: entity_classes={}, constraint_count={}, time_limit_secs={}",
        solution.descriptor.entity_classes.len(),
        constraints.constraint_count(),
        config.time_limit.as_secs(),
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

    // Create local search phase with Late Acceptance
    // Use FirstAcceptedForager to take sidesteps for plateau exploration
    let local_search: LocalSearchPhase<
        DynamicSolution,
        DynamicChangeMove,
        DynamicMoveSelector,
        LateAcceptanceAcceptor<DynamicSolution>,
        FirstAcceptedForager<DynamicSolution>,
    > = LocalSearchPhase::new(
        DynamicMoveSelector::new(),
        LateAcceptanceAcceptor::new(400), // Standard late acceptance size
        FirstAcceptedForager::new(),      // Take first accepted (enables plateau exploration)
        None,                             // No step limit - rely on time limit
    );

    // Create and run solver with both phases
    eprintln!(
        "[DEBUG] Creating solver with time_limit={:?}",
        config.time_limit
    );
    // Note: snapshot updates not implemented yet - solver runs to completion
    let _ = snapshot; // silence unused warning
    let mut solver = Solver::new(((), construction, local_search))
        .with_time_limit(config.time_limit)
        .with_terminate(terminate);

    eprintln!("[DEBUG] Starting solver.solve()");
    let start_solve = Instant::now();
    let result_solution = solver.solve(score_director);
    eprintln!("[DEBUG] Solver finished in {:?}", start_solve.elapsed());
    eprintln!("[DEBUG] Result solution score: {:?}", result_solution.score);

    // Debug: print final solution
    eprintln!("[DEBUG] Final solution entities:");
    for (class_idx, entities) in result_solution.entities.iter().enumerate() {
        for (entity_idx, entity) in entities.iter().enumerate() {
            eprintln!(
                "[DEBUG]   class={} entity={}: {:?}",
                class_idx, entity_idx, entity.fields
            );
        }
    }
    let duration = start.elapsed();
    let score = result_solution.score.unwrap_or(HardSoftScore::ZERO);

    info!(
        event = "solve_end",
        score = %score,
        feasible = score.hard() >= 0,
        duration_ms = duration.as_millis(),
    );

    SolveResult {
        solution: result_solution,
        score,
        duration,
        steps: 0,           // TODO: Get from solver stats
        moves_evaluated: 0, // TODO: Get from solver stats
    }
}
