use solverforge::prelude::*;
use solverforge::stream::ConstraintFactory;
use solverforge::SolverConfig;

use super::{Route, Visit};

#[planning_solution(constraints = "define_constraints", config = "realtime_config")]
#[shadow_variable_updates(
    list_owner = "routes",
    inverse_field = "route",
    index_field = "index",
    previous_field = "previous",
    next_field = "next"
)]
pub struct RoutePlan {
    #[planning_entity_collection]
    pub routes: Vec<Route>,

    #[problem_fact_collection]
    pub visits: Vec<Visit>,

    #[planning_score]
    pub score: Option<HardSoftScore>,
}

// Visit terms read only list shadows, so a stale element binding or shadow
// row after a problem change scores the changed solution wrong.
fn define_constraints() -> impl ConstraintSet<RoutePlan, HardSoftScore> {
    (
        ConstraintFactory::<RoutePlan, HardSoftScore>::new()
            .for_each(RoutePlan::visits())
            .filter(|visit: &Visit| visit.route.is_none())
            .penalize(HardSoftScore::ONE_HARD)
            .named("unrouted visit"),
        ConstraintFactory::<RoutePlan, HardSoftScore>::new()
            .for_each(RoutePlan::visits())
            .filter(|visit: &Visit| visit.index.is_some())
            .penalize(|visit: &Visit| {
                let index = visit.index.unwrap_or_default() as i64;
                let route = visit.route.unwrap_or_default() as i64;
                HardSoftScore::of_soft((visit.demand * (index + 2) + route * 5) % 13)
            })
            .named("position cost"),
        ConstraintFactory::<RoutePlan, HardSoftScore>::new()
            .for_each(RoutePlan::routes())
            .penalize(|route: &Route| HardSoftScore::of_soft((route.visits.len() as i64).pow(2)))
            .named("balance"),
    )
}

// No solver-level termination: the job runs until it is cancelled. Full
// assert checks every incremental score against a fresh calculation.
const REALTIME_TOML: &str = r#"
environment_mode = "full_assert"
random_seed = 11

[[phases]]
type = "construction_heuristic"

[[phases]]
type = "local_search"
"#;

fn realtime_config(_solution: &RoutePlan, _base: SolverConfig) -> SolverConfig {
    SolverConfig::from_toml_str(REALTIME_TOML).expect("real-time list fixture config must parse")
}
