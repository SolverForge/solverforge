use solverforge::prelude::*;
use solverforge::stream::ConstraintFactory;
use solverforge::{MoveThreadCount, SolverConfig};

use super::{Route, Visit};

#[planning_solution(constraints = "define_constraints", config = "parallel_config")]
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

    pub move_threads: usize,
}

// The visit terms read only list shadows, so a director whose element rows
// lag their shadows scores every candidate wrong. Position-dependent costs
// make order inside a route matter as well as ownership, so list change,
// swap, sublist, reversal, and k-opt moves all compete.
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
            .for_each(RoutePlan::visits())
            .filter(|visit: &Visit| visit.previous.is_some())
            .penalize(|visit: &Visit| {
                let previous = visit.previous.unwrap_or_default() as i64;
                HardSoftScore::of_soft((previous - visit.id as i64).abs() % 7)
            })
            .named("leg cost"),
        ConstraintFactory::<RoutePlan, HardSoftScore>::new()
            .for_each(RoutePlan::routes())
            .penalize(|route: &Route| HardSoftScore::of_soft((route.visits.len() as i64).pow(2)))
            .named("balance"),
    )
}

const PARALLEL_SOLVER_TOML: &str = r#"
random_seed = 11

[candidate_trace]
max_entries = 1_000_000

[[phases]]
type = "construction_heuristic"

[[phases]]
type = "local_search"

[phases.termination]
step_count_limit = 40
"#;

fn parallel_config(solution: &RoutePlan, _base: SolverConfig) -> SolverConfig {
    let mut config = SolverConfig::from_toml_str(PARALLEL_SOLVER_TOML)
        .expect("parallel list local-search fixture config must parse");
    config.move_thread_count = MoveThreadCount::Count(solution.move_threads);
    config
}
