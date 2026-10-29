use std::sync::atomic::{AtomicUsize, Ordering};

use crate::api::constraint_set::IncrementalConstraint;
use crate::director::Director;
use crate::ScoreDirector;
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::HardSoftScore;

use super::*;

mod graph;
mod scoring;

#[derive(Clone, Debug)]
struct Task {
    duration: usize,
    next: Option<usize>,
    owner: Option<usize>,
}

#[derive(Clone, Debug)]
struct Route {
    tasks: Vec<usize>,
}

#[derive(Clone, Debug)]
struct Plan {
    tasks: Vec<Task>,
    routes: Vec<Route>,
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

fn node_count(plan: &Plan) -> usize {
    plan.tasks.len()
}

fn node_duration(plan: &Plan, node: usize) -> usize {
    plan.tasks[node].duration
}

fn fixed_successors(plan: &Plan, node: usize, out: &mut Vec<usize>) {
    if let Some(next) = plan.tasks[node].next {
        out.push(next);
    }
}

fn owner_count(plan: &Plan) -> usize {
    plan.routes.len()
}

fn list_len(plan: &Plan, owner: usize) -> usize {
    plan.routes[owner].tasks.len()
}

fn list_get(plan: &Plan, owner: usize, pos: usize) -> Option<usize> {
    plan.routes[owner].tasks.get(pos).copied()
}

fn expected_owner(plan: &Plan, node: usize) -> Option<usize> {
    plan.tasks[node].owner
}

fn constraint() -> ListPrecedenceMakespanConstraint<Plan> {
    ListPrecedenceMakespanConstraint::new(
        ConstraintRef::new("", "listPrecedenceMakespan"),
        0,
        node_count,
        node_duration,
        fixed_successors,
        owner_count,
        list_len,
        list_get,
    )
}

fn owner_constraint() -> ListPrecedenceMakespanConstraint<Plan> {
    constraint().with_expected_owner(Some(expected_owner))
}

static NODE_COUNT_CALLS: AtomicUsize = AtomicUsize::new(0);
static NODE_DURATION_CALLS: AtomicUsize = AtomicUsize::new(0);
static FIXED_SUCCESSOR_CALLS: AtomicUsize = AtomicUsize::new(0);
static OWNER_COUNT_CALLS: AtomicUsize = AtomicUsize::new(0);
static LIST_LEN_CALLS: [AtomicUsize; 2] = [AtomicUsize::new(0), AtomicUsize::new(0)];
static LIST_GET_CALLS: [AtomicUsize; 2] = [AtomicUsize::new(0), AtomicUsize::new(0)];

fn reset_access_counts() {
    NODE_COUNT_CALLS.store(0, Ordering::Relaxed);
    NODE_DURATION_CALLS.store(0, Ordering::Relaxed);
    FIXED_SUCCESSOR_CALLS.store(0, Ordering::Relaxed);
    OWNER_COUNT_CALLS.store(0, Ordering::Relaxed);
    for counter in &LIST_LEN_CALLS {
        counter.store(0, Ordering::Relaxed);
    }
    for counter in &LIST_GET_CALLS {
        counter.store(0, Ordering::Relaxed);
    }
}

fn counted_node_count(plan: &Plan) -> usize {
    NODE_COUNT_CALLS.fetch_add(1, Ordering::Relaxed);
    node_count(plan)
}

fn counted_node_duration(plan: &Plan, node: usize) -> usize {
    NODE_DURATION_CALLS.fetch_add(1, Ordering::Relaxed);
    node_duration(plan, node)
}

fn counted_fixed_successors(plan: &Plan, node: usize, out: &mut Vec<usize>) {
    FIXED_SUCCESSOR_CALLS.fetch_add(1, Ordering::Relaxed);
    fixed_successors(plan, node, out);
}

fn counted_owner_count(plan: &Plan) -> usize {
    OWNER_COUNT_CALLS.fetch_add(1, Ordering::Relaxed);
    owner_count(plan)
}

fn counted_list_len(plan: &Plan, owner: usize) -> usize {
    LIST_LEN_CALLS[owner].fetch_add(1, Ordering::Relaxed);
    list_len(plan, owner)
}

fn counted_list_get(plan: &Plan, owner: usize, pos: usize) -> Option<usize> {
    LIST_GET_CALLS[owner].fetch_add(1, Ordering::Relaxed);
    list_get(plan, owner, pos)
}

fn counted_constraint() -> ListPrecedenceMakespanConstraint<Plan> {
    ListPrecedenceMakespanConstraint::new(
        ConstraintRef::new("", "countedListPrecedenceMakespan"),
        0,
        counted_node_count,
        counted_node_duration,
        counted_fixed_successors,
        counted_owner_count,
        counted_list_len,
        counted_list_get,
    )
}

fn graph_state(node_count: usize, durations: Vec<i64>, edges: &[Edge]) -> ListPrecedenceState {
    let mut state = ListPrecedenceState::new(node_count, 0, durations);
    for &edge in edges {
        state.add_edge(edge);
    }
    state.refresh_score_full();
    state
}

fn plan(routes: Vec<Vec<usize>>) -> Plan {
    Plan {
        tasks: vec![
            Task {
                duration: 2,
                next: Some(1),
                owner: Some(0),
            },
            Task {
                duration: 3,
                next: None,
                owner: Some(1),
            },
        ],
        routes: routes.into_iter().map(|tasks| Route { tasks }).collect(),
        score: None,
    }
}

fn four_task_plan(routes: Vec<Vec<usize>>) -> Plan {
    Plan {
        tasks: (0..4)
            .map(|_| Task {
                duration: 1,
                next: None,
                owner: Some(0),
            })
            .collect(),
        routes: routes.into_iter().map(|tasks| Route { tasks }).collect(),
        score: None,
    }
}
