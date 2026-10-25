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

// Streams over the visits do not observe their list shadows, so every term
// reads the owning route. Position-dependent costs make order inside a route
// matter as well as ownership, so list change, swap, sublist, reversal, and
// k-opt moves all compete.
fn define_constraints() -> impl ConstraintSet<RoutePlan, HardSoftScore> {
    (
        ConstraintFactory::<RoutePlan, HardSoftScore>::new()
            .for_each(RoutePlan::routes())
            .penalize(|route: &Route| {
                let cost = route
                    .visits
                    .iter()
                    .enumerate()
                    .map(|(position, &visit)| (visit as i64 * (position as i64 + 3)) % 11)
                    .sum();
                HardSoftScore::of_soft(cost)
            })
            .named("position cost"),
        ConstraintFactory::<RoutePlan, HardSoftScore>::new()
            .for_each(RoutePlan::routes())
            .penalize(|route: &Route| {
                let legs = route
                    .visits
                    .windows(2)
                    .map(|leg| (leg[0] as i64 - leg[1] as i64).abs() % 7)
                    .sum();
                HardSoftScore::of_soft(legs)
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
