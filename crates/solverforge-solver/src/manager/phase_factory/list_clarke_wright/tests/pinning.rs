use std::any::TypeId;

use solverforge_core::domain::{
    EntityCollectionExtractor, EntityDescriptor, PlanningSolution, SolutionDescriptor,
};
use solverforge_core::score::SoftScore;
use solverforge_scoring::ScoreDirector;

use crate::builder::usize_element_source_key;
use crate::manager::phase_factory::list_clarke_wright::ListClarkeWrightPhase;
use crate::phase::Phase;
use crate::scope::SolverScope;

#[derive(Clone)]
struct Route {
    visits: Vec<usize>,
    pinned: bool,
}

#[derive(Clone)]
struct Plan {
    customers: Vec<usize>,
    routes: Vec<Route>,
    score: Option<SoftScore>,
}

impl PlanningSolution for Plan {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

#[test]
fn clarke_wright_does_not_use_an_empty_pinned_owner() {
    let input = Plan {
        customers: vec![10],
        routes: vec![
            Route {
                visits: vec![],
                pinned: true,
            },
            Route {
                visits: vec![],
                pinned: false,
            },
        ],
        score: None,
    };
    let descriptor = SolutionDescriptor::new("Plan", TypeId::of::<Plan>()).with_entity(
        EntityDescriptor::new("Route", TypeId::of::<Route>(), "routes")
            .with_extractor(Box::new(EntityCollectionExtractor::new(
                "Route",
                "routes",
                |plan: &Plan| &plan.routes,
                |plan: &mut Plan| &mut plan.routes,
            )))
            .with_pin_predicate(|entity| entity.downcast_ref::<Route>().unwrap().pinned),
    );
    let director = ScoreDirector::simple(input, descriptor, |plan, _| plan.routes.len());
    let mut scope = SolverScope::new(director);
    let mut phase = ListClarkeWrightPhase::new(
        |plan: &Plan| plan.customers.len(),
        |plan: &Plan| {
            plan.routes
                .iter()
                .flat_map(|r| r.visits.iter().copied())
                .collect()
        },
        |plan: &Plan| plan.routes.len(),
        |plan: &Plan, owner| plan.routes[owner].visits.len(),
        |plan: &mut Plan, owner, route| plan.routes[owner].visits = route,
        |plan: &Plan, index| plan.customers[index],
        usize_element_source_key,
        |_plan: &Plan, _owner| 0,
        |_plan: &Plan, _owner, left, right| (left as i64 - right as i64).abs(),
        |_plan: &Plan, _owner, _route: &[usize]| true,
        0,
    );

    phase.solve(&mut scope);

    assert!(scope.working_solution().routes[0].visits.is_empty());
    assert_eq!(scope.working_solution().routes[1].visits, vec![10]);
}
