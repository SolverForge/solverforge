//! Public dynamic runner helpers.

use solverforge_config::SolverConfig;
use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};
use solverforge_core::score::{ParseableScore, Score};
use solverforge_scoring::{ConstraintSet, ScoreDirector};
use solverforge_solver::{run_solver_with_config_parts, Phase, SolverRuntime};

/// Run a solver from already-built runtime parts.
///
/// This is the binding-oriented entrypoint: descriptor, constraints, config,
/// and phase construction are values supplied by the caller instead of
/// macro-generated `fn() -> T` factories.
#[allow(clippy::too_many_arguments)]
pub fn run_dynamic_solver_with_config<S, C, P, BuildPhases>(
    solution: S,
    constraints: C,
    descriptor: SolutionDescriptor,
    entity_count_by_descriptor: fn(&S, usize) -> usize,
    runtime: SolverRuntime<S>,
    config: SolverConfig,
    default_time_limit_secs: u64,
    is_trivial: fn(&S) -> bool,
    log_scale: fn(&S),
    build_phases: BuildPhases,
) -> S
where
    S: PlanningSolution,
    S::Score: Score + ParseableScore,
    C: ConstraintSet<S, S::Score>,
    P: Phase<S, ScoreDirector<S, C>, solverforge_solver::run::ChannelProgressCallback<S>>
        + Send
        + std::fmt::Debug,
    BuildPhases: Fn(&SolverConfig, &SolutionDescriptor) -> P,
{
    run_solver_with_config_parts(
        solution,
        constraints,
        descriptor,
        entity_count_by_descriptor,
        runtime,
        config,
        default_time_limit_secs,
        is_trivial,
        log_scale,
        build_phases,
    )
}
