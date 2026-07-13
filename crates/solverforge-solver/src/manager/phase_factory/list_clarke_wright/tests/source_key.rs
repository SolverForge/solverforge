//! Public Clarke-Wright source-key contract regressions.

use std::any::TypeId;

use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};
use solverforge_core::score::SoftScore;
use solverforge_scoring::ScoreDirector;

use super::super::ListClarkeWrightPhase;
use crate::builder::context::{bind_runtime_list_source, ListConstructionKernelError};
use crate::phase::Phase;
use crate::scope::SolverScope;

/// Deliberately has no `PartialEq`, `Eq`, `Hash`, or `Debug` implementation.
#[derive(Clone, Copy)]
struct Customer(usize);

impl From<Customer> for usize {
    fn from(value: Customer) -> Self {
        value.0
    }
}

#[derive(Clone)]
struct Plan {
    score: Option<SoftScore>,
    elements: Vec<Customer>,
    routes: Vec<Vec<usize>>,
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

fn element_count(plan: &Plan) -> usize {
    plan.elements.len()
}

fn assigned(plan: &Plan) -> Vec<Customer> {
    plan.routes
        .iter()
        .flatten()
        .copied()
        .map(Customer)
        .collect()
}

fn entity_count(plan: &Plan) -> usize {
    plan.routes.len()
}

fn route_len(plan: &Plan, entity: usize) -> usize {
    plan.routes[entity].len()
}

fn assign_route(plan: &mut Plan, entity: usize, route: Vec<usize>) {
    plan.routes[entity] = route;
}

fn index_to_element(plan: &Plan, source_index: usize) -> Customer {
    plan.elements[source_index]
}

fn customer_source_key(_: &Plan, customer: &Customer) -> usize {
    customer.0
}

fn depot(_: &Plan, _: usize) -> usize {
    0
}

fn distance(_: &Plan, _: usize, from: usize, to: usize) -> i64 {
    from.abs_diff(to) as i64
}

fn feasible(_: &Plan, _: usize, _: &[usize]) -> bool {
    true
}

fn phase() -> ListClarkeWrightPhase<Plan, Customer> {
    ListClarkeWrightPhase::new(
        element_count,
        assigned,
        entity_count,
        route_len,
        assign_route,
        index_to_element,
        customer_source_key,
        depot,
        distance,
        feasible,
        0,
    )
}

#[test]
fn public_clarke_wright_needs_no_payload_equality_or_hashing() {
    let plan = Plan {
        score: None,
        elements: vec![Customer(1), Customer(2), Customer(3)],
        routes: vec![Vec::new()],
    };
    let director = ScoreDirector::simple(
        plan,
        SolutionDescriptor::new("Plan", TypeId::of::<Plan>()),
        |_, _| 0,
    );
    let mut scope = SolverScope::new(director);
    let mut phase = phase();

    phase.solve(&mut scope);

    assert_eq!(scope.working_solution().routes, vec![vec![1, 2, 3]]);
}

#[test]
fn non_injective_public_source_key_fails_before_phase_execution() {
    let plan = Plan {
        score: None,
        elements: vec![Customer(7), Customer(7)],
        routes: vec![Vec::new()],
    };

    assert!(matches!(
        bind_runtime_list_source(&phase(), &plan),
        Err(ListConstructionKernelError::DuplicateDeclaredElement {
            first_source_index: 0,
            duplicate_source_index: 1,
        })
    ));
}
