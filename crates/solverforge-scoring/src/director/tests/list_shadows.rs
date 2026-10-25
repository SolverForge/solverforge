// Element streams that read list shadows, scored through ScoreDirector.

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::SoftScore;
use solverforge_core::{ConstraintRef, ImpactType};

use crate::api::constraint_set::IncrementalConstraint;
use crate::constraint::incremental::IncrementalUniConstraint;
use crate::director::score_director::ScoreDirector;
use crate::stream::collection_extract::{source, ChangeSource, SourceExtract};

const ROUTES: usize = 0;
const STOPS: usize = 1;

#[derive(Clone, Debug, Default)]
struct Stop {
    weight: i64,
    route: Option<usize>,
    index: Option<usize>,
}

#[derive(Clone, Debug)]
struct Plan {
    routes: Vec<Vec<usize>>,
    stops: Vec<Stop>,
    score: Option<SoftScore>,
}

impl PlanningSolution for Plan {
    type Score = SoftScore;

    fn score(&self) -> Option<SoftScore> {
        self.score
    }

    fn set_score(&mut self, score: Option<SoftScore>) {
        self.score = score;
    }

    fn update_entity_shadows(&mut self, descriptor_index: usize, entity_index: usize) {
        if descriptor_index != ROUTES {
            return;
        }
        for (position, &stop) in self.routes[entity_index].iter().enumerate() {
            self.stops[stop].route = Some(entity_index);
            self.stops[stop].index = Some(position);
        }
    }

    fn update_all_shadows(&mut self) {
        for stop in 0..self.stops.len() {
            self.clear_list_shadows(stop);
        }
        for route in 0..self.routes.len() {
            self.update_entity_shadows(ROUTES, route);
        }
    }

    fn list_shadow_elements(
        &self,
        descriptor_index: usize,
        entity_index: usize,
    ) -> Option<(usize, &[usize])> {
        (descriptor_index == ROUTES).then(|| (STOPS, self.routes[entity_index].as_slice()))
    }

    fn clear_list_shadows(&mut self, element: usize) {
        self.stops[element].route = None;
        self.stops[element].index = None;
    }
}

type StopCost = IncrementalUniConstraint<
    Plan,
    Stop,
    SourceExtract<fn(&Plan) -> &[Stop]>,
    fn(&Plan, &Stop) -> bool,
    fn(&Stop) -> SoftScore,
    SoftScore,
>;

// Unrouted stops cost 10; routed ones cost by route and weighted position,
// so any reordering changes the total.
fn stop_cost() -> StopCost {
    fn stops(plan: &Plan) -> &[Stop] {
        &plan.stops
    }
    fn every(_: &Plan, _: &Stop) -> bool {
        true
    }
    fn cost(stop: &Stop) -> SoftScore {
        match (stop.route, stop.index) {
            (Some(route), Some(index)) => {
                SoftScore::of(route as i64 * 3 + stop.weight * (index as i64 + 1))
            }
            _ => SoftScore::of(10),
        }
    }
    IncrementalUniConstraint::new(
        ConstraintRef::new("", "stop cost"),
        ImpactType::Penalty,
        source(
            stops as fn(&Plan) -> &[Stop],
            ChangeSource::Descriptor(STOPS),
        ),
        every as fn(&Plan, &Stop) -> bool,
        cost as fn(&Stop) -> SoftScore,
        false,
    )
}

fn director(routes: Vec<Vec<usize>>, stops: usize) -> ScoreDirector<Plan, (StopCost,)> {
    let plan = Plan {
        routes,
        stops: (0..stops)
            .map(|stop| Stop {
                weight: stop as i64 * 2 + 1,
                ..Stop::default()
            })
            .collect(),
        score: None,
    };
    let mut director = ScoreDirector::new(plan, (stop_cost(),));
    director.calculate_score();
    director
}

fn assert_matches_fresh(director: &mut ScoreDirector<Plan, (StopCost,)>) {
    let mut fresh = director.working_solution().clone();
    fresh.update_all_shadows();
    assert_eq!(
        director.calculate_score(),
        stop_cost().evaluate(&fresh),
        "routes {:?}",
        director.working_solution().routes
    );
}

// Moves `stop` from `from` to the end of `to`, notifying owners in `order`.
fn relocate(
    director: &mut ScoreDirector<Plan, (StopCost,)>,
    stop: usize,
    from: Option<usize>,
    to: Option<usize>,
    order: &[usize],
) {
    for &owner in order {
        director.before_variable_changed(ROUTES, owner);
    }
    let routes = &mut director.working_solution_mut().routes;
    if let Some(from) = from {
        routes[from].retain(|&held| held != stop);
    }
    if let Some(to) = to {
        routes[to].push(stop);
    }
    for &owner in order {
        director.after_variable_changed(ROUTES, owner);
    }
}

#[test]
fn rows_follow_stops_moved_between_routes_in_either_notification_order() {
    let mut director = director(vec![vec![0, 1, 2], vec![3]], 4);
    assert_matches_fresh(&mut director);

    relocate(&mut director, 1, Some(0), Some(1), &[0, 1]);
    assert_matches_fresh(&mut director);

    relocate(&mut director, 3, Some(1), Some(0), &[0, 1]);
    assert_matches_fresh(&mut director);

    relocate(&mut director, 0, Some(0), Some(1), &[1, 0]);
    assert_matches_fresh(&mut director);
}

#[test]
fn rows_follow_stops_that_leave_and_rejoin_every_route() {
    let mut director = director(vec![vec![0, 1], vec![]], 3);
    assert_matches_fresh(&mut director);

    // Stop 2 starts unrouted and joins a route.
    relocate(&mut director, 2, None, Some(1), &[1]);
    assert_matches_fresh(&mut director);

    // Stop 0 leaves every route: its shadows clear and the remaining stop
    // shifts position.
    relocate(&mut director, 0, Some(0), None, &[0]);
    assert!(director.working_solution().stops[0].route.is_none());
    assert_matches_fresh(&mut director);

    relocate(&mut director, 0, None, Some(0), &[0]);
    assert_matches_fresh(&mut director);
}

#[test]
fn reordering_inside_one_route_rescores_every_shifted_stop() {
    let mut director = director(vec![vec![0, 1, 2, 3]], 4);

    director.before_variable_changed(ROUTES, 0);
    director.working_solution_mut().routes[0].reverse();
    director.after_variable_changed(ROUTES, 0);

    assert_matches_fresh(&mut director);
}
