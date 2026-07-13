use std::any::TypeId;

use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};
use solverforge_core::score::{HardSoftScore, SoftScore};
use solverforge_scoring::{ConstraintMetadata, Director, ScoreDirector};

use super::*;
use crate::builder::context::{bind_runtime_list_source, ListConstructionKernelError};
use crate::builder::usize_element_source_key;
use crate::phase::Phase;
use crate::scope::SolverScope;

#[path = "tests/parity.rs"]
mod parity;

#[derive(Clone, Debug)]
struct Plan {
    elements: Vec<usize>,
    routes: Vec<Vec<usize>>,
    score: Option<HardSoftScore>,
}

impl PlanningSolution for Plan {
    type Score = HardSoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

fn descriptor() -> SolutionDescriptor {
    SolutionDescriptor::new("Plan", TypeId::of::<Plan>())
}

fn element_count(plan: &Plan) -> usize {
    plan.elements.len()
}

fn assigned(plan: &Plan) -> Vec<usize> {
    plan.routes.iter().flatten().copied().collect()
}

fn entity_count(plan: &Plan) -> usize {
    plan.routes.len()
}

fn list_len(plan: &Plan, entity: usize) -> usize {
    plan.routes[entity].len()
}

fn list_insert(plan: &mut Plan, entity: usize, position: usize, element: usize) {
    plan.routes[entity].insert(position, element);
}

fn list_remove(plan: &mut Plan, entity: usize, position: usize) -> usize {
    plan.routes[entity].remove(position)
}

fn index_to_element(plan: &Plan, source_index: usize) -> usize {
    plan.elements[source_index]
}

fn phase() -> ListRegretInsertionPhase<Plan, usize> {
    ListRegretInsertionPhase::new(
        element_count,
        assigned,
        entity_count,
        list_len,
        list_insert,
        list_remove,
        index_to_element,
        usize_element_source_key,
        0,
    )
}

struct TestDirector {
    working_solution: Plan,
    descriptor: SolutionDescriptor,
    score_fn: fn(&Plan) -> HardSoftScore,
}

impl TestDirector {
    fn new(working_solution: Plan, score_fn: fn(&Plan) -> HardSoftScore) -> Self {
        Self {
            working_solution,
            descriptor: descriptor(),
            score_fn,
        }
    }
}

impl Director<Plan> for TestDirector {
    fn working_solution(&self) -> &Plan {
        &self.working_solution
    }

    fn working_solution_mut(&mut self) -> &mut Plan {
        &mut self.working_solution
    }

    fn calculate_score(&mut self) -> HardSoftScore {
        let score = (self.score_fn)(&self.working_solution);
        self.working_solution.set_score(Some(score));
        score
    }

    fn solution_descriptor(&self) -> &SolutionDescriptor {
        &self.descriptor
    }

    fn clone_working_solution(&self) -> Plan {
        self.working_solution.clone()
    }

    fn before_variable_changed(&mut self, _: usize, _: usize) {}

    fn after_variable_changed(&mut self, _: usize, _: usize) {}

    fn entity_count(&self, descriptor_index: usize) -> Option<usize> {
        (descriptor_index == 0).then_some(self.working_solution.routes.len())
    }

    fn total_entity_count(&self) -> Option<usize> {
        Some(self.working_solution.routes.len())
    }

    fn constraint_metadata(&self) -> Vec<ConstraintMetadata<'_>> {
        Vec::new()
    }
}

fn director(plan: Plan, score: fn(&Plan) -> HardSoftScore) -> TestDirector {
    TestDirector::new(plan, score)
}

fn zero_score(_: &Plan) -> HardSoftScore {
    HardSoftScore::ZERO
}

fn descending_weight_score(plan: &Plan) -> HardSoftScore {
    let weighted = plan
        .routes
        .iter()
        .map(|route| {
            let len = route.len();
            route
                .iter()
                .enumerate()
                .map(|(position, &element)| (len - position) as i64 * element as i64)
                .sum::<i64>()
        })
        .sum();
    HardSoftScore::of(0, weighted)
}

fn single_owner(_: &Plan, _: &usize) -> Option<usize> {
    Some(0)
}

fn unit_duration(_: &Plan, _: usize) -> usize {
    1
}

fn long_chain_successor(_: &Plan, element: usize, out: &mut Vec<usize>) {
    match element {
        0 => out.push(2),
        2 => out.push(3),
        _ => {}
    }
}

#[test]
fn duplicate_declared_source_key_fails_before_regret_construction() {
    let plan = Plan {
        elements: vec![7, 7],
        routes: vec![Vec::new()],
        score: None,
    };
    assert!(matches!(
        bind_runtime_list_source(&phase(), &plan),
        Err(ListConstructionKernelError::DuplicateDeclaredElement {
            first_source_index: 0,
            duplicate_source_index: 1,
        })
    ));
}

#[test]
fn public_regret_completes_mandatory_construction_past_ordinary_limits() {
    let plan = Plan {
        elements: vec![0, 1, 2],
        routes: vec![Vec::new()],
        score: None,
    };
    let mut scope = SolverScope::new(director(plan, zero_score));
    scope.start_solving();
    scope.inphase_step_count_limit = Some(0);
    let mut phase = phase();

    phase.solve(&mut scope);

    assert_eq!(scope.working_solution().routes[0].len(), 3);
}

#[test]
fn owner_restricted_regret_uses_best_insertion_below_work_budget() {
    let plan = Plan {
        elements: (0..47).collect(),
        routes: vec![Vec::new()],
        score: None,
    };
    let mut scope = SolverScope::new(director(plan, descending_weight_score));
    scope.start_solving();
    let mut phase = phase().with_element_owner_fn(Some(single_owner));

    phase.solve(&mut scope);

    let route = &scope.working_solution().routes[0];
    assert_eq!(route.len(), 47);
    assert_eq!(route.first(), Some(&46));
    assert_eq!(route.last(), Some(&0));
}

#[test]
fn owner_restricted_regret_uses_bounded_append_above_work_budget() {
    let plan = Plan {
        elements: (0..300).collect(),
        routes: vec![Vec::new()],
        score: None,
    };
    let mut scope = SolverScope::new(director(plan, descending_weight_score));
    scope.start_solving();
    let mut phase = phase().with_element_owner_fn(Some(single_owner));

    phase.solve(&mut scope);

    let route = &scope.working_solution().routes[0];
    assert_eq!(route.len(), 300);
    assert_eq!(route.first(), Some(&0));
    assert_eq!(route.last(), Some(&299));
}

#[test]
fn precedence_downstream_breaks_regret_ties_without_reconstructing_source_order() {
    let plan = Plan {
        elements: vec![1, 0, 2, 3],
        routes: vec![Vec::new()],
        score: None,
    };
    let mut scope = SolverScope::new(director(plan, zero_score));
    scope.start_solving();
    let mut phase = phase().with_precedence_hooks(Some(unit_duration), Some(long_chain_successor));

    phase.solve(&mut scope);

    assert_eq!(scope.working_solution().routes[0].last(), Some(&0));
}

#[derive(Clone, Copy)]
struct Opaque(usize);

#[derive(Clone)]
struct OpaquePlan {
    elements: Vec<Opaque>,
    routes: Vec<Vec<Opaque>>,
    score: Option<SoftScore>,
}

impl PlanningSolution for OpaquePlan {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

fn opaque_phase() -> ListRegretInsertionPhase<OpaquePlan, Opaque> {
    ListRegretInsertionPhase::new(
        |plan| plan.elements.len(),
        |plan| plan.routes.iter().flatten().copied().collect(),
        |plan| plan.routes.len(),
        |plan, entity| plan.routes[entity].len(),
        |plan, entity, position, value| plan.routes[entity].insert(position, value),
        |plan, entity, position| plan.routes[entity].remove(position),
        |plan, source_index| plan.elements[source_index],
        |_, value| value.0,
        0,
    )
}

#[test]
fn regret_requires_only_the_explicit_source_key_not_payload_equality_or_hashing() {
    let plan = OpaquePlan {
        elements: vec![Opaque(1), Opaque(2), Opaque(3)],
        routes: vec![Vec::new()],
        score: None,
    };
    let descriptor = SolutionDescriptor::new("OpaquePlan", TypeId::of::<OpaquePlan>());
    let director = ScoreDirector::simple(plan, descriptor, |plan, _| plan.routes.len());
    let mut scope = SolverScope::new(director);
    let mut phase = opaque_phase();

    phase.solve(&mut scope);

    assert_eq!(
        scope.working_solution().routes[0]
            .iter()
            .map(|value| value.0)
            .collect::<Vec<_>>(),
        vec![3, 2, 1]
    );
}
