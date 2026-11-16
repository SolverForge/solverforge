use solverforge::prelude::*;
use solverforge::stream::joiner::equal_bi;
use solverforge::stream::ConstraintFactory;
use solverforge::SolverConfig;

use super::{Stop, Vehicle};

/// Vehicles carrying jobs from a pick-up to a drop-off.
#[planning_solution(constraints = "define_constraints", config = "deliveries_config")]
#[shadow_variable_updates(
    list_owner = "vehicles",
    inverse_field = "vehicle",
    index_field = "index",
    previous_field = "previous"
)]
pub struct Deliveries {
    /// The vehicles.
    #[planning_entity_collection]
    pub vehicles: Vec<Vehicle>,

    /// The stops: every job's pick-up and drop-off, then single stops.
    #[problem_fact_collection]
    pub stops: Vec<Stop>,

    /// The plan's score.
    #[planning_score]
    pub score: Option<HardSoftScore>,

    /// Which solver configuration the fixture runs.
    pub scenario: usize,
}

/// Every stop is made; a job stays on one vehicle with its pick-up first;
/// driving distance is soft.
pub fn define_constraints() -> impl ConstraintSet<Deliveries, HardSoftScore> {
    (
        ConstraintFactory::<Deliveries, HardSoftScore>::new()
            .for_each(Deliveries::stops())
            .filter(|stop: &Stop| stop.vehicle.is_none())
            .penalize(HardSoftScore::ONE_HARD)
            .named("unassigned stop"),
        ConstraintFactory::<Deliveries, HardSoftScore>::new()
            .for_each(Deliveries::stops())
            .filter(|stop: &Stop| stop.pickup)
            .join((
                ConstraintFactory::<Deliveries, HardSoftScore>::new()
                    .for_each(Deliveries::stops()),
                equal_bi(|pickup: &Stop| pickup.job, |stop: &Stop| stop.job),
            ))
            .filter(|pickup: &Stop, dropoff: &Stop| {
                !dropoff.pickup
                    && (pickup.vehicle != dropoff.vehicle || pickup.index > dropoff.index)
            })
            .penalize(HardSoftScore::ONE_HARD)
            .named("broken job"),
        ConstraintFactory::<Deliveries, HardSoftScore>::new()
            .for_each(Deliveries::stops())
            .join((
                ConstraintFactory::<Deliveries, HardSoftScore>::new()
                    .for_each(Deliveries::stops()),
                equal_bi(|stop: &Stop| stop.previous, |before: &Stop| Some(before.id)),
            ))
            .penalize(|stop: &Stop, before: &Stop| {
                HardSoftScore::of_soft((stop.location - before.location).abs())
            })
            .named("driving"),
        ConstraintFactory::<Deliveries, HardSoftScore>::new()
            .for_each(Deliveries::stops())
            .filter(|stop: &Stop| stop.vehicle.is_some() && stop.previous.is_none())
            .penalize(|stop: &Stop| HardSoftScore::of_soft(stop.location.abs()))
            .named("leaving the depot"),
    )
}

// Every scenario checks each incremental score against a fresh one.
const DEFAULTS: &str = r#"
environment_mode = "full_assert"
random_seed = 11

[termination]
step_count_limit = 40
"#;

const EXPLICIT: &str = r#"
environment_mode = "full_assert"
random_seed = 13

[[phases]]
type = "construction_heuristic"
construction_heuristic_type = "list_regret_insertion"
max_member_distance = 3

[[phases]]
type = "local_search"

[phases.move_selector]
type = "union_move_selector"

[[phases.move_selector.selectors]]
type = "list_group_change_move_selector"
max_member_distance = 4

[[phases.move_selector.selectors]]
type = "list_group_swap_move_selector"

[[phases.move_selector.selectors]]
type = "list_ruin_move_selector"
min_ruin_count = 1
max_ruin_count = 3
moves_per_step = 4
max_group_owners = 1

[[phases.move_selector.selectors]]
type = "list_change_move_selector"

[phases.termination]
step_count_limit = 40
"#;

const ROUND_ROBIN: &str = r#"
[[phases]]
type = "construction_heuristic"
construction_heuristic_type = "list_round_robin"
"#;

fn deliveries_config(plan: &Deliveries, _base: SolverConfig) -> SolverConfig {
    let toml = [DEFAULTS, EXPLICIT, ROUND_ROBIN][plan.scenario];
    SolverConfig::from_toml_str(toml).expect("deliveries fixture config must parse")
}
