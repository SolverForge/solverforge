use std::sync::atomic::Ordering;

use solverforge::prelude::*;
use solverforge::stream::ConstraintFactory;
use solverforge::SolverConfig;

use super::DummyEntity;

#[planning_solution(
    constraints = "define_explicit_constraints",
    config = "solver_config_for_explicit_solution",
    solver_toml = "../../../fixtures/configurable_solvable_solver.toml"
)]
pub struct ExplicitConfigurableSolution {
    #[planning_entity_collection]
    pub entities: Vec<DummyEntity>,

    #[planning_score]
    pub score: Option<HardSoftScore>,

    pub time_limit_secs: u64,
}

fn define_explicit_constraints(
) -> impl ConstraintSet<ExplicitConfigurableSolution, HardSoftScore> {
    (
        ConstraintFactory::<ExplicitConfigurableSolution, HardSoftScore>::new()
            .for_each(ExplicitConfigurableSolution::entities())
            .penalize(|_: &DummyEntity| HardSoftScore::of(0, 0))
            .named("noop"),
    )
}

fn solver_config_for_explicit_solution(
    solution: &ExplicitConfigurableSolution,
    config: SolverConfig,
) -> SolverConfig {
    crate::LAST_EXPLICIT_BASE_RANDOM_SEED
        .store(config.random_seed.unwrap_or_default(), Ordering::SeqCst);
    crate::LAST_EXPLICIT_BASE_PHASE_COUNT.store(config.phases.len(), Ordering::SeqCst);

    let config = config.with_termination_seconds(solution.time_limit_secs);

    crate::LAST_EXPLICIT_FINAL_RANDOM_SEED
        .store(config.random_seed.unwrap_or_default(), Ordering::SeqCst);
    crate::LAST_EXPLICIT_FINAL_PHASE_COUNT.store(config.phases.len(), Ordering::SeqCst);
    crate::LAST_EXPLICIT_FINAL_TERMINATION_SECONDS.store(
        config
            .time_limit()
            .map(|duration| duration.as_secs())
            .unwrap_or(0),
        Ordering::SeqCst,
    );

    config
}
