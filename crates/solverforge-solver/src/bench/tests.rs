//! The `__bench` handles enumerate exactly the stream the standalone
//! list-change oracle enumerates, through the compiled runtime's cursor.

use std::any::TypeId;

use solverforge_config::{
    ListChangeMoveConfig, LocalSearchConfig, MoveSelectorConfig, PhaseConfig, SelectionOrder,
    SolverConfig,
};
use solverforge_core::domain::{
    EntityCollectionExtractor, EntityDescriptor, PlanningSolution, SolutionDescriptor,
    VariableDescriptor,
};
use solverforge_core::score::SoftScore;
use solverforge_scoring::ScoreDirector;

use super::{CandidateVisitor, CompiledRuntimeRunner};
use crate::builder::{ListVariableSlot, RuntimeModel, SearchContext, VariableSlot};
use crate::heuristic::r#move::Move;
use crate::heuristic::selector::entity::FromSolutionEntitySelector;
use crate::heuristic::selector::list_change::ListChangeMoveSelector;
use crate::heuristic::selector::move_selector::{
    MoveCandidateRef, MoveSelector, MoveStreamContext,
};
use crate::heuristic::selector::DefaultCrossEntityDistanceMeter;
use crate::stats::CandidateTraceIdentity;

type Meter = DefaultCrossEntityDistanceMeter;

#[derive(Clone, Debug)]
struct Routes {
    routes: Vec<Vec<usize>>,
    score: Option<SoftScore>,
}

impl PlanningSolution for Routes {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

fn descriptor() -> SolutionDescriptor {
    SolutionDescriptor::new("Routes", TypeId::of::<Routes>()).with_entity(
        EntityDescriptor::new("Route", TypeId::of::<Vec<usize>>(), "routes")
            .with_extractor(Box::new(EntityCollectionExtractor::new(
                "Route",
                "routes",
                |plan: &Routes| &plan.routes,
                |plan: &mut Routes| &mut plan.routes,
            )))
            .with_variable(VariableDescriptor::list("visits")),
    )
}

fn director() -> ScoreDirector<Routes, ()> {
    let plan = Routes {
        routes: vec![vec![0, 1, 2], vec![3, 4], vec![5, 6, 7]],
        score: None,
    };
    ScoreDirector::simple(plan, descriptor(), |plan, _| plan.routes.len())
}

fn route_count(plan: &Routes) -> usize {
    plan.routes.len()
}

fn element_count(plan: &Routes) -> usize {
    plan.routes.iter().map(Vec::len).sum()
}

fn assigned(plan: &Routes) -> Vec<usize> {
    plan.routes.iter().flatten().copied().collect()
}

fn len(plan: &Routes, route: usize) -> usize {
    plan.routes[route].len()
}

fn get(plan: &Routes, route: usize, position: usize) -> Option<usize> {
    plan.routes.get(route)?.get(position).copied()
}

fn set(plan: &mut Routes, route: usize, position: usize, value: usize) {
    plan.routes[route][position] = value;
}

fn remove(plan: &mut Routes, route: usize, position: usize) -> Option<usize> {
    (position < plan.routes[route].len()).then(|| plan.routes[route].remove(position))
}

fn take(plan: &mut Routes, route: usize, position: usize) -> usize {
    plan.routes[route].remove(position)
}

fn insert(plan: &mut Routes, route: usize, position: usize, value: usize) {
    plan.routes[route].insert(position, value);
}

fn reverse(plan: &mut Routes, route: usize, start: usize, end: usize) {
    plan.routes[route][start..end].reverse();
}

fn drain(plan: &mut Routes, route: usize, start: usize, end: usize) -> Vec<usize> {
    plan.routes[route].drain(start..end).collect()
}

fn splice(plan: &mut Routes, route: usize, position: usize, values: Vec<usize>) {
    plan.routes[route].splice(position..position, values);
}

fn element(_: &Routes, index: usize) -> usize {
    index
}

fn key(_: &Routes, element: &usize) -> usize {
    *element
}

fn context() -> SearchContext<Routes, usize, Meter, Meter> {
    let slot = ListVariableSlot::new(
        "Route",
        element_count,
        assigned,
        len,
        remove,
        take,
        insert,
        get,
        set,
        reverse,
        drain,
        splice,
        take,
        insert,
        element,
        key,
        route_count,
        Meter::default(),
        Meter::default(),
        "visits",
        0,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    );
    SearchContext::new(
        descriptor(),
        RuntimeModel::new(vec![VariableSlot::List(slot)]),
        Some(7),
    )
    .expect("the test model binds to its descriptor")
}

fn list_change_config() -> SolverConfig {
    SolverConfig {
        random_seed: Some(7),
        phases: vec![PhaseConfig::LocalSearch(LocalSearchConfig {
            move_selector: Some(MoveSelectorConfig::ListChangeMoveSelector(
                ListChangeMoveConfig {
                    selection_order: Some(SelectionOrder::Original),
                    ..ListChangeMoveConfig::default()
                },
            )),
            ..LocalSearchConfig::default()
        })],
        ..SolverConfig::default()
    }
}

struct Identity;

impl CandidateVisitor<Routes> for Identity {
    type Output = CandidateTraceIdentity;

    fn visit<M: Move<Routes>>(self, candidate: MoveCandidateRef<'_, Routes, M>) -> Self::Output {
        candidate
            .candidate_trace_identity()
            .expect("list-change candidates expose a logical identity")
    }
}

#[test]
fn compiled_list_change_cursor_matches_the_standalone_oracle() {
    let mut director = director();
    let mut neighborhood = CompiledRuntimeRunner::compile(&list_change_config(), context())
        .expect("the configuration compiles")
        .into_local_search_neighborhood()
        .expect("the first phase is an acceptor/forager local search");
    let mut compiled = Vec::new();
    let mut cursor = neighborhood.open_cursor(&director, MoveStreamContext::new(0, 7, None));
    while let Some(id) = cursor.next_candidate(&mut director) {
        compiled.push(
            cursor
                .visit_candidate(id, Identity)
                .expect("live candidate"),
        );
        assert!(cursor.release_candidate(id));
    }
    drop(cursor);

    let oracle = ListChangeMoveSelector::<Routes, usize, _>::new(
        FromSolutionEntitySelector::new(0),
        len,
        get,
        remove,
        insert,
        "visits",
        0,
    );
    let expected = oracle
        .iter_moves(&director)
        .map(|mov| {
            mov.candidate_trace_identity()
                .expect("list-change moves expose a logical identity")
        })
        .collect::<Vec<_>>();

    assert!(!expected.is_empty());
    assert_eq!(compiled, expected);
}

#[test]
fn handoffs_reject_a_phase_of_the_other_kind() {
    let runner = CompiledRuntimeRunner::compile(&list_change_config(), context())
        .expect("the configuration compiles");
    assert!(runner.into_scalar_construction_placer().is_none());
}
