// `ProblemSpec` trait for parameterizing `run_solver` over problem types.

use std::sync::atomic::AtomicBool;
use std::time::Duration;

use solverforge_config::SolverConfig;
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::{ParseableScore, Score};
use solverforge_scoring::{ConstraintSet, ScoreDirector};

use crate::run::AnyTermination;
use crate::scope::ProgressCallback;
use crate::solver::SolveResult;

/// Parameterizes `run_solver` over standard-variable and list-variable problems.
///
/// Implementors supply problem-specific trivial-case detection, logging,
/// default time limit, and the actual construction + local search execution.
pub trait ProblemSpec<S, C>
where
    S: PlanningSolution,
    S::Score: Score + ParseableScore,
    C: ConstraintSet<S, S::Score>,
{
    // Returns `true` if the problem is trivially empty and solving can be skipped.
    fn is_trivial(&self, solution: &S) -> bool;

    // Default solver time limit in seconds (used when config has no termination).
    fn default_time_limit_secs(&self) -> u64;

    // Logs the problem scale (entity count, value count, etc.).
    fn log_scale(&self, solution: &S);

    // Builds the construction + local search phases and runs the solver.
    fn build_and_solve(
        self,
        director: ScoreDirector<S, C>,
        config: &SolverConfig,
        time_limit: Duration,
        termination: AnyTermination<S, ScoreDirector<S, C>>,
        terminate: Option<&AtomicBool>,
        callback: impl ProgressCallback<S>,
    ) -> SolveResult<S>;
}
