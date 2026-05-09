use solverforge::prelude::*;
use solverforge::stream::joiner::equal_bi;
use solverforge::stream::ConstraintFactory;

use super::{Route, Visit};

#[planning_solution(
    constraints = "define_constraints",
    solver_toml = "../../solver.toml"
)]
pub struct TourPlan {
    #[problem_fact_collection]
    pub visits: Vec<Visit>,

    #[planning_list_element_collection(owner = "routes")]
    pub visit_values: Vec<usize>,

    #[planning_entity_collection]
    pub routes: Vec<Route>,

    #[planning_score]
    pub score: Option<HardSoftScore>,
}

fn define_constraints() -> impl ConstraintSet<TourPlan, HardSoftScore> {
    let all_visits_assigned = ConstraintFactory::<TourPlan, HardSoftScore>::new()
        .for_each(TourPlan::visits())
        .if_not_exists((
            ConstraintFactory::<TourPlan, HardSoftScore>::new()
                .for_each(TourPlan::routes())
                .flattened(|route: &Route| &route.visits),
            equal_bi(|visit: &Visit| visit.id, |assigned: &usize| *assigned),
        ))
        .penalize(HardSoftScore::ONE_HARD)
        .named("All visits assigned");

    let compact_route = ConstraintFactory::<TourPlan, HardSoftScore>::new()
        .for_each(TourPlan::routes())
        .penalize(|route: &Route| HardSoftScore::of_soft(route.visits.len() as i64))
        .named("Route length");

    (all_visits_assigned, compact_route)
}
