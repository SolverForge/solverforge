use std::collections::{HashMap, VecDeque};
use std::marker::PhantomData;

use solverforge_core::score::HardSoftScore;
use solverforge_core::ConstraintRef;

use crate::api::constraint_set::{IncrementalConstraint, IncrementalConstraintSealed};

type NodeId = usize;
type OwnerId = usize;
type Edge = (NodeId, NodeId);

pub struct ListPrecedenceMakespanConstraint<S> {
    constraint_ref: ConstraintRef,
    list_descriptor_index: usize,
    node_count: fn(&S) -> usize,
    node_duration: fn(&S, NodeId) -> usize,
    fixed_successors: fn(&S, NodeId, &mut Vec<NodeId>),
    list_owner_count: fn(&S) -> usize,
    list_len: fn(&S, OwnerId) -> usize,
    list_get: fn(&S, OwnerId, usize) -> Option<NodeId>,
    expected_owner: Option<fn(&S, NodeId) -> Option<OwnerId>>,
    state: Option<ListPrecedenceState>,
    _phantom: PhantomData<fn() -> S>,
}

impl<S> ListPrecedenceMakespanConstraint<S> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        constraint_ref: ConstraintRef,
        list_descriptor_index: usize,
        node_count: fn(&S) -> usize,
        node_duration: fn(&S, NodeId) -> usize,
        fixed_successors: fn(&S, NodeId, &mut Vec<NodeId>),
        list_owner_count: fn(&S) -> usize,
        list_len: fn(&S, OwnerId) -> usize,
        list_get: fn(&S, OwnerId, usize) -> Option<NodeId>,
    ) -> Self {
        Self {
            constraint_ref,
            list_descriptor_index,
            node_count,
            node_duration,
            fixed_successors,
            list_owner_count,
            list_len,
            list_get,
            expected_owner: None,
            state: None,
            _phantom: PhantomData,
        }
    }

    pub fn with_expected_owner(
        mut self,
        expected_owner: Option<fn(&S, NodeId) -> Option<OwnerId>>,
    ) -> Self {
        self.expected_owner = expected_owner;
        self
    }

    fn build_state(&self, solution: &S) -> ListPrecedenceState {
        let node_count = (self.node_count)(solution);
        let owner_count = (self.list_owner_count)(solution);
        let access = self.access();
        let durations = (0..node_count)
            .map(|node| usize_to_i64((self.node_duration)(solution, node)))
            .collect();
        let mut state = ListPrecedenceState::new(node_count, owner_count, durations);

        let mut successors = Vec::new();
        for node in 0..node_count {
            successors.clear();
            (self.fixed_successors)(solution, node, &mut successors);
            for &successor in &successors {
                if successor < node_count {
                    state.add_edge((node, successor));
                } else {
                    state.invalid_fixed_edges += 1;
                }
            }
        }

        for owner in 0..owner_count {
            state.add_owner_route(solution, owner, access);
        }
        state.refresh_score_full();
        state
    }

    fn score_from_state(&self, solution: &S) -> HardSoftScore {
        let state = self.build_state(solution);
        state.score
    }

    fn match_count_from_state(&self, solution: &S) -> usize {
        let state = self.build_state(solution);
        state.hard_penalty + usize::from(state.makespan > 0)
    }

    fn access(&self) -> ListPrecedenceAccess<S> {
        ListPrecedenceAccess {
            list_len: self.list_len,
            list_get: self.list_get,
            expected_owner: self.expected_owner,
            _phantom: PhantomData,
        }
    }
}

impl<S> IncrementalConstraintSealed for ListPrecedenceMakespanConstraint<S> {}

impl<S> IncrementalConstraint<S, HardSoftScore> for ListPrecedenceMakespanConstraint<S>
where
    S: Send + Sync,
{
    fn evaluate(&self, solution: &S) -> HardSoftScore {
        self.score_from_state(solution)
    }

    fn match_count(&self, solution: &S) -> usize {
        self.match_count_from_state(solution)
    }

    fn initialize(&mut self, solution: &S) -> HardSoftScore {
        let state = self.build_state(solution);
        let score = state.score;
        self.state = Some(state);
        score
    }

    fn on_insert(
        &mut self,
        solution: &S,
        entity_index: usize,
        descriptor_index: usize,
    ) -> HardSoftScore {
        if descriptor_index != self.list_descriptor_index {
            return HardSoftScore::ZERO;
        }
        let access = self.access();
        let Some(state) = self.state.as_mut() else {
            return HardSoftScore::ZERO;
        };
        if entity_index >= state.owner_edges.len() {
            return HardSoftScore::ZERO;
        }
        let before = state.score;
        let change = state.add_owner_route(solution, entity_index, access);
        let after = state.refresh_score_after_route_change(&change);
        after - before
    }

    fn on_retract(
        &mut self,
        _solution: &S,
        entity_index: usize,
        descriptor_index: usize,
    ) -> HardSoftScore {
        if descriptor_index != self.list_descriptor_index {
            return HardSoftScore::ZERO;
        }
        let Some(state) = self.state.as_mut() else {
            return HardSoftScore::ZERO;
        };
        if entity_index >= state.owner_edges.len() {
            return HardSoftScore::ZERO;
        }
        let before = state.score;
        let change = state.remove_owner_route(entity_index);
        let after = state.refresh_score_after_route_change(&change);
        after - before
    }

    fn reset(&mut self) {
        self.state = None;
    }

    fn constraint_ref(&self) -> &ConstraintRef {
        &self.constraint_ref
    }

    fn is_hard(&self) -> bool {
        true
    }

    fn weight(&self) -> HardSoftScore {
        HardSoftScore::ZERO
    }
}

struct ListPrecedenceState {
    node_count: usize,
    durations: Vec<i64>,
    edge_counts: Vec<Vec<(NodeId, usize)>>,
    successors: Vec<Vec<NodeId>>,
    predecessors: Vec<Vec<NodeId>>,
    assigned_counts: Vec<usize>,
    owner_elements: Vec<Vec<NodeId>>,
    owner_edges: Vec<Vec<Edge>>,
    owner_invalid_counts: Vec<usize>,
    owner_violation_counts: Vec<usize>,
    invalid_fixed_edges: usize,
    owner_invalid_total: usize,
    owner_violation_total: usize,
    assignment_penalty: usize,
    cycle_penalty: usize,
    cycle_added_edges: Vec<Edge>,
    earliest: Vec<i64>,
    finishes: Vec<i64>,
    score: HardSoftScore,
    hard_penalty: usize,
    makespan: i64,
}

#[derive(Default)]
struct RouteChange {
    added_edges: Vec<Edge>,
    removed_edges: Vec<Edge>,
}

impl RouteChange {
    fn is_empty(&self) -> bool {
        self.added_edges.is_empty() && self.removed_edges.is_empty()
    }

    fn seeds(&self) -> impl Iterator<Item = NodeId> + '_ {
        self.added_edges
            .iter()
            .chain(self.removed_edges.iter())
            .map(|&(_, to)| to)
    }
}

#[derive(Default)]
struct OwnerRouteSnapshot {
    elements: Vec<NodeId>,
    edges: Vec<Edge>,
    invalid_count: usize,
    violation_count: usize,
}

#[derive(Debug, PartialEq, Eq)]
enum GraphRefreshKind {
    Skipped,
    Incremental { visited: usize },
    CycleDetected,
    CycleRecovered,
    Full,
}

struct ListPrecedenceAccess<S> {
    list_len: fn(&S, OwnerId) -> usize,
    list_get: fn(&S, OwnerId, usize) -> Option<NodeId>,
    expected_owner: Option<fn(&S, NodeId) -> Option<OwnerId>>,
    _phantom: PhantomData<fn() -> S>,
}

impl<S> Copy for ListPrecedenceAccess<S> {}

impl<S> Clone for ListPrecedenceAccess<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl ListPrecedenceState {
    fn new(node_count: usize, owner_count: usize, durations: Vec<i64>) -> Self {
        Self {
            node_count,
            durations,
            edge_counts: vec![Vec::new(); node_count],
            successors: vec![Vec::new(); node_count],
            predecessors: vec![Vec::new(); node_count],
            assigned_counts: vec![0; node_count],
            owner_elements: vec![Vec::new(); owner_count],
            owner_edges: vec![Vec::new(); owner_count],
            owner_invalid_counts: vec![0; owner_count],
            owner_violation_counts: vec![0; owner_count],
            invalid_fixed_edges: 0,
            owner_invalid_total: 0,
            owner_violation_total: 0,
            assignment_penalty: node_count,
            cycle_penalty: 0,
            cycle_added_edges: Vec::new(),
            earliest: vec![0; node_count],
            finishes: vec![0; node_count],
            score: HardSoftScore::ZERO,
            hard_penalty: 0,
            makespan: 0,
        }
    }

    fn add_edge(&mut self, edge: Edge) -> bool {
        if let Some((_, count)) = self.edge_counts[edge.0]
            .iter_mut()
            .find(|(to, _)| *to == edge.1)
        {
            *count += 1;
            false
        } else {
            self.edge_counts[edge.0].push((edge.1, 1));
            self.successors[edge.0].push(edge.1);
            self.predecessors[edge.1].push(edge.0);
            true
        }
    }

    fn remove_edge(&mut self, edge: Edge) -> bool {
        let Some(pos) = self.edge_counts[edge.0]
            .iter()
            .position(|(to, _)| *to == edge.1)
        else {
            return false;
        };
        let count = &mut self.edge_counts[edge.0][pos].1;
        *count -= 1;
        if *count == 0 {
            self.edge_counts[edge.0].swap_remove(pos);
            remove_node(&mut self.successors[edge.0], edge.1);
            remove_node(&mut self.predecessors[edge.1], edge.0);
            true
        } else {
            false
        }
    }

    fn adjust_assignment(&mut self, node: NodeId, new_count: usize) {
        let old_count = self.assigned_counts[node];
        if old_count == new_count {
            return;
        }
        let old_penalty = assignment_penalty(old_count);
        let new_penalty = assignment_penalty(new_count);
        if new_penalty >= old_penalty {
            self.assignment_penalty += new_penalty - old_penalty;
        } else {
            self.assignment_penalty -= old_penalty - new_penalty;
        }
        self.assigned_counts[node] = new_count;
    }

    fn add_owner_route<S>(
        &mut self,
        solution: &S,
        owner: OwnerId,
        access: ListPrecedenceAccess<S>,
    ) -> RouteChange {
        let snapshot = self.owner_route_snapshot(solution, owner, access);
        self.replace_owner_route(owner, snapshot)
    }

    fn remove_owner_route(&mut self, owner: OwnerId) -> RouteChange {
        self.replace_owner_route(owner, OwnerRouteSnapshot::default())
    }

    fn owner_route_snapshot<S>(
        &self,
        solution: &S,
        owner: OwnerId,
        access: ListPrecedenceAccess<S>,
    ) -> OwnerRouteSnapshot {
        let mut snapshot = OwnerRouteSnapshot::default();
        let len = (access.list_len)(solution, owner);
        let mut previous = None;
        for pos in 0..len {
            let Some(node) = (access.list_get)(solution, owner, pos) else {
                snapshot.invalid_count += 1;
                previous = None;
                continue;
            };
            if node >= self.node_count {
                snapshot.invalid_count += 1;
                previous = None;
                continue;
            }

            snapshot.elements.push(node);
            if access
                .expected_owner
                .and_then(|expected_owner| expected_owner(solution, node))
                .is_some_and(|expected| expected != owner)
            {
                snapshot.violation_count += 1;
            }
            if let Some(from) = previous {
                let edge = (from, node);
                snapshot.edges.push(edge);
            }
            previous = Some(node);
        }
        snapshot
    }

    fn replace_owner_route(&mut self, owner: OwnerId, snapshot: OwnerRouteSnapshot) -> RouteChange {
        let mut change = RouteChange::default();
        let OwnerRouteSnapshot {
            elements,
            edges,
            invalid_count,
            violation_count,
        } = snapshot;

        let old_elements = std::mem::take(&mut self.owner_elements[owner]);
        self.diff_owner_assignments(&old_elements, &elements);
        self.owner_elements[owner] = elements;

        let old_edges = std::mem::take(&mut self.owner_edges[owner]);
        self.diff_owner_edges(&old_edges, &edges, &mut change);
        self.owner_edges[owner] = edges;

        self.owner_invalid_total -= self.owner_invalid_counts[owner];
        self.owner_invalid_total += invalid_count;
        self.owner_invalid_counts[owner] = invalid_count;

        self.owner_violation_total -= self.owner_violation_counts[owner];
        self.owner_violation_total += violation_count;
        self.owner_violation_counts[owner] = violation_count;

        change
    }

    fn diff_owner_assignments(&mut self, old_elements: &[NodeId], new_elements: &[NodeId]) {
        let mut counts = HashMap::<NodeId, (usize, usize)>::new();
        for &node in old_elements {
            counts.entry(node).or_default().0 += 1;
        }
        for &node in new_elements {
            counts.entry(node).or_default().1 += 1;
        }

        for (node, (old_count, new_count)) in counts {
            if old_count == new_count {
                continue;
            }
            let current = self.assigned_counts[node];
            let updated = current.saturating_sub(old_count).saturating_add(new_count);
            self.adjust_assignment(node, updated);
        }
    }

    fn diff_owner_edges(
        &mut self,
        old_edges: &[Edge],
        new_edges: &[Edge],
        change: &mut RouteChange,
    ) {
        let mut counts = HashMap::<Edge, (usize, usize)>::new();
        for &edge in old_edges {
            counts.entry(edge).or_default().0 += 1;
        }
        for &edge in new_edges {
            counts.entry(edge).or_default().1 += 1;
        }

        for (edge, (old_count, new_count)) in counts {
            if old_count > new_count {
                for _ in 0..(old_count - new_count) {
                    if self.remove_edge(edge) {
                        change.removed_edges.push(edge);
                    }
                }
            } else if new_count > old_count {
                for _ in 0..(new_count - old_count) {
                    if self.add_edge(edge) {
                        change.added_edges.push(edge);
                    }
                }
            }
        }
    }

    fn refresh_score_full(&mut self) -> HardSoftScore {
        self.rebuild_graph_summary();
        self.refresh_score_from_cached_graph()
    }

    fn refresh_score_after_route_change(&mut self, change: &RouteChange) -> HardSoftScore {
        self.refresh_graph_after_route_change(change);
        self.refresh_score_from_cached_graph()
    }

    fn refresh_score_from_cached_graph(&mut self) -> HardSoftScore {
        let hard_penalty = self.invalid_fixed_edges
            + self.owner_invalid_total
            + self.owner_violation_total
            + self.assignment_penalty
            + self.cycle_penalty;
        self.hard_penalty = hard_penalty;
        self.score = HardSoftScore::of(-usize_to_i64(hard_penalty), self.makespan.saturating_neg());
        self.score
    }

    fn refresh_graph_after_route_change(&mut self, change: &RouteChange) -> GraphRefreshKind {
        if change.is_empty() {
            return GraphRefreshKind::Skipped;
        }

        if self.cycle_penalty > 0 {
            if self.recover_cached_cycle(change) {
                return GraphRefreshKind::CycleRecovered;
            }
            self.rebuild_graph_summary();
            return GraphRefreshKind::Full;
        }

        if self.added_edges_introduce_cycle(&change.added_edges) {
            if change.removed_edges.is_empty() {
                self.mark_cyclic_from_route_change(change);
            } else {
                self.mark_cyclic_without_cache();
            }
            return GraphRefreshKind::CycleDetected;
        }

        let mut queued = vec![false; self.node_count];
        let mut queue = VecDeque::new();
        for node in change.seeds() {
            if node < self.node_count && !queued[node] {
                queued[node] = true;
                queue.push_back(node);
            }
        }

        let mut visited = 0;
        while let Some(node) = queue.pop_front() {
            queued[node] = false;
            visited += 1;

            let new_earliest = self.predecessors[node]
                .iter()
                .map(|&predecessor| {
                    self.earliest[predecessor].saturating_add(self.durations[predecessor])
                })
                .max()
                .unwrap_or(0);
            if new_earliest == self.earliest[node] {
                continue;
            }

            self.replace_earliest(node, new_earliest);
            for &successor in &self.successors[node] {
                if !queued[successor] {
                    queued[successor] = true;
                    queue.push_back(successor);
                }
            }
        }
        self.makespan = self.max_finish();
        GraphRefreshKind::Incremental { visited }
    }

    fn rebuild_graph_summary(&mut self) {
        let mut indegree: Vec<usize> = self.predecessors.iter().map(Vec::len).collect();
        let mut earliest = vec![0i64; self.node_count];
        let mut finishes = vec![0i64; self.node_count];
        let mut ready = VecDeque::new();
        for (node, &degree) in indegree.iter().enumerate() {
            if degree == 0 {
                ready.push_back(node);
            }
        }

        let mut processed = 0usize;
        let mut makespan = 0i64;
        while let Some(node) = ready.pop_front() {
            processed += 1;
            let finish = earliest[node].saturating_add(self.durations[node]);
            finishes[node] = finish;
            makespan = makespan.max(finish);
            for &successor in &self.successors[node] {
                earliest[successor] = earliest[successor].max(finish);
                indegree[successor] -= 1;
                if indegree[successor] == 0 {
                    ready.push_back(successor);
                }
            }
        }

        if processed < self.node_count {
            self.mark_cyclic_without_cache();
        } else {
            self.earliest = earliest;
            self.finishes = finishes;
            self.cycle_penalty = 0;
            self.cycle_added_edges.clear();
            self.makespan = makespan;
        }
    }

    fn mark_cyclic_from_route_change(&mut self, change: &RouteChange) {
        self.cycle_added_edges = change.added_edges.clone();
        self.cycle_penalty = self.node_count;
        self.makespan = 0;
    }

    fn mark_cyclic_without_cache(&mut self) {
        self.earliest.fill(0);
        self.finishes.fill(0);
        self.cycle_added_edges.clear();
        self.cycle_penalty = self.node_count;
        self.makespan = 0;
    }

    fn recover_cached_cycle(&mut self, change: &RouteChange) -> bool {
        if self.cycle_added_edges.is_empty() || !change.added_edges.is_empty() {
            return false;
        }
        if self
            .cycle_added_edges
            .iter()
            .any(|&edge| self.contains_edge(edge))
        {
            return false;
        }
        self.cycle_penalty = 0;
        self.cycle_added_edges.clear();
        self.makespan = self.max_finish();
        true
    }

    fn replace_earliest(&mut self, node: NodeId, new_earliest: i64) {
        let old_finish = self.finishes[node];
        self.earliest[node] = new_earliest;
        let new_finish = new_earliest.saturating_add(self.durations[node]);
        self.finishes[node] = new_finish;
        if new_finish >= self.makespan {
            self.makespan = new_finish;
        } else if old_finish == self.makespan {
            self.makespan = self.max_finish();
        }
    }

    fn max_finish(&self) -> i64 {
        self.finishes.iter().copied().max().unwrap_or(0)
    }

    fn added_edges_introduce_cycle(&self, added_edges: &[Edge]) -> bool {
        if added_edges.is_empty() {
            return false;
        }

        let mut visited = vec![0usize; self.node_count];
        let mut stack = Vec::new();
        for (visit_id, &(from, to)) in added_edges.iter().enumerate() {
            if self.reaches_with_scratch(to, from, visit_id + 1, &mut visited, &mut stack) {
                return true;
            }
        }
        false
    }

    fn reaches_with_scratch(
        &self,
        start: NodeId,
        target: NodeId,
        visit_id: usize,
        visited: &mut [usize],
        stack: &mut Vec<NodeId>,
    ) -> bool {
        if start == target {
            return true;
        }
        stack.clear();
        stack.push(start);
        while let Some(node) = stack.pop() {
            if visited[node] == visit_id {
                continue;
            }
            visited[node] = visit_id;
            for &successor in &self.successors[node] {
                if successor == target {
                    return true;
                }
                if visited[successor] != visit_id {
                    stack.push(successor);
                }
            }
        }
        false
    }

    fn contains_edge(&self, edge: Edge) -> bool {
        self.edge_counts[edge.0]
            .iter()
            .any(|&(to, count)| to == edge.1 && count > 0)
    }
}

fn assignment_penalty(count: usize) -> usize {
    match count {
        0 => 1,
        1 => 0,
        extra => extra - 1,
    }
}

fn usize_to_i64(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn remove_node(nodes: &mut Vec<NodeId>, node: NodeId) {
    let Some(pos) = nodes.iter().position(|&candidate| candidate == node) else {
        return;
    };
    nodes.swap_remove(pos);
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use crate::api::constraint_set::IncrementalConstraint;
    use crate::director::Director;
    use crate::ScoreDirector;
    use solverforge_core::domain::PlanningSolution;
    use solverforge_core::score::HardSoftScore;

    use super::*;

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

    #[test]
    fn evaluates_fixed_and_list_precedence_makespan() {
        let constraint = constraint();

        assert_eq!(
            constraint.evaluate(&plan(vec![vec![0], vec![1]])),
            HardSoftScore::of(0, -5)
        );
    }

    #[test]
    fn acyclic_graph_route_change_uses_incremental_descendant_refresh() {
        let mut state = graph_state(4, vec![2, 3, 5, 7], &[(0, 1), (1, 2), (0, 3)]);
        assert_eq!(state.cycle_penalty, 0);
        assert_eq!(state.makespan, 10);

        let mut change = RouteChange::default();
        if state.remove_edge((1, 2)) {
            change.removed_edges.push((1, 2));
        }

        assert_eq!(
            state.refresh_graph_after_route_change(&change),
            GraphRefreshKind::Incremental { visited: 1 }
        );
        assert_eq!(state.cycle_penalty, 0);
        assert_eq!(state.earliest[2], 0);
        assert_eq!(state.earliest[3], 2);
        assert_eq!(state.makespan, 9);
    }

    #[test]
    fn cycle_introducing_route_change_marks_cyclic_without_graph_rebuild() {
        let mut state = graph_state(3, vec![1, 1, 1], &[(0, 1), (1, 2)]);
        assert_eq!(state.cycle_penalty, 0);
        assert_eq!(state.makespan, 3);

        let mut change = RouteChange::default();
        if state.add_edge((2, 0)) {
            change.added_edges.push((2, 0));
        }

        assert_eq!(
            state.refresh_graph_after_route_change(&change),
            GraphRefreshKind::CycleDetected
        );
        assert_eq!(state.cycle_penalty, 3);
        assert_eq!(state.makespan, 0);
    }

    #[test]
    fn cycle_retraction_recovers_cached_acyclic_state_without_graph_rebuild() {
        let mut state = graph_state(3, vec![1, 1, 1], &[(0, 1), (1, 2)]);

        let mut cycle_change = RouteChange::default();
        if state.add_edge((2, 0)) {
            cycle_change.added_edges.push((2, 0));
        }
        assert_eq!(
            state.refresh_graph_after_route_change(&cycle_change),
            GraphRefreshKind::CycleDetected
        );
        assert_eq!(state.cycle_penalty, 3);
        assert_eq!(state.makespan, 0);

        let mut undo_change = RouteChange::default();
        if state.remove_edge((2, 0)) {
            undo_change.removed_edges.push((2, 0));
        }
        assert_eq!(
            state.refresh_graph_after_route_change(&undo_change),
            GraphRefreshKind::CycleRecovered
        );
        assert_eq!(state.cycle_penalty, 0);
        assert_eq!(state.earliest, vec![0, 1, 2]);
        assert_eq!(state.makespan, 3);
    }

    #[test]
    fn cycle_introduced_with_removed_edges_recovers_by_full_rebuild() {
        let mut state = graph_state(3, vec![10, 10, 1], &[(0, 1), (1, 2)]);
        assert_eq!(state.makespan, 21);

        let mut cycle_change = RouteChange::default();
        if state.remove_edge((0, 1)) {
            cycle_change.removed_edges.push((0, 1));
        }
        if state.add_edge((2, 1)) {
            cycle_change.added_edges.push((2, 1));
        }
        assert_eq!(
            state.refresh_graph_after_route_change(&cycle_change),
            GraphRefreshKind::CycleDetected
        );
        assert_eq!(state.cycle_penalty, 3);
        assert!(state.cycle_added_edges.is_empty());

        let mut undo_change = RouteChange::default();
        if state.remove_edge((2, 1)) {
            undo_change.removed_edges.push((2, 1));
        }
        assert_eq!(
            state.refresh_graph_after_route_change(&undo_change),
            GraphRefreshKind::Full
        );
        assert_eq!(state.cycle_penalty, 0);
        assert_eq!(state.earliest, vec![0, 0, 10]);
        assert_eq!(state.makespan, 11);
    }

    #[test]
    fn owner_route_replacement_diffs_unchanged_prefix_edges() {
        let constraint = constraint();
        let access = constraint.access();
        let mut state = ListPrecedenceState::new(4, 1, vec![1, 1, 1, 1]);
        state.add_owner_route(&four_task_plan(vec![vec![0, 1, 2]]), 0, access);
        state.refresh_score_full();

        let mut change = state.add_owner_route(&four_task_plan(vec![vec![0, 1, 3]]), 0, access);
        change.added_edges.sort_unstable();
        change.removed_edges.sort_unstable();

        assert_eq!(change.removed_edges, vec![(1, 2)]);
        assert_eq!(change.added_edges, vec![(1, 3)]);
        assert_eq!(state.assigned_counts, vec![1, 1, 0, 1]);
        assert_eq!(state.owner_edges[0], vec![(0, 1), (1, 3)]);
    }

    #[test]
    fn cyclic_state_with_unmatched_change_uses_full_graph_refresh() {
        let mut state = graph_state(4, vec![1, 1, 1, 1], &[(0, 1), (1, 2)]);

        let mut cycle_change = RouteChange::default();
        if state.add_edge((2, 0)) {
            cycle_change.added_edges.push((2, 0));
        }
        assert_eq!(
            state.refresh_graph_after_route_change(&cycle_change),
            GraphRefreshKind::CycleDetected
        );

        let mut unrelated_change = RouteChange::default();
        if state.add_edge((2, 3)) {
            unrelated_change.added_edges.push((2, 3));
        }
        assert_eq!(
            state.refresh_graph_after_route_change(&unrelated_change),
            GraphRefreshKind::Full
        );
        assert_eq!(state.cycle_penalty, 4);
        assert_eq!(state.makespan, 0);
    }

    #[test]
    fn partial_cycle_penalizes_whole_precedence_schedule() {
        let state = graph_state(4, vec![1, 1, 1, 10], &[(0, 1), (1, 0)]);

        assert_eq!(state.cycle_penalty, 4);
        assert_eq!(state.makespan, 0);
        assert_eq!(state.score, HardSoftScore::of(-8, 0));
    }

    #[test]
    fn penalizes_cycles_incrementally() {
        let mut director = ScoreDirector::new(plan(vec![vec![0, 1]]), (constraint(),));

        assert_eq!(director.calculate_score(), HardSoftScore::of(0, -5));
        director.before_variable_changed(0, 0);
        director.working_solution_mut().routes[0].tasks = vec![1, 0];
        director.after_variable_changed(0, 0);

        let score = director.calculate_score();
        assert_eq!(score, HardSoftScore::of(-2, 0));
        assert_eq!(Director::fresh_score(&director), Some(score));
    }

    #[test]
    fn repeated_route_updates_keep_incremental_score_fresh() {
        let mut director = ScoreDirector::new(plan(vec![vec![0], vec![1]]), (constraint(),));

        let score = director.calculate_score();
        assert_eq!(Director::fresh_score(&director), Some(score));

        for (owner, tasks) in [
            (0, vec![0, 1]),
            (1, Vec::new()),
            (0, vec![1, 0]),
            (1, vec![0, 1]),
        ] {
            director.before_variable_changed(0, owner);
            director.working_solution_mut().routes[owner].tasks = tasks;
            director.after_variable_changed(0, owner);

            let score = director.calculate_score();
            assert_eq!(Director::fresh_score(&director), Some(score));
        }
    }

    #[test]
    fn retract_removes_cached_owner_route_before_insert() {
        let mut constraint = constraint();
        let mut solution = plan(vec![vec![0], vec![1]]);
        let mut score = constraint.initialize(&solution);
        assert_eq!(score, HardSoftScore::of(0, -5));

        score = score + constraint.on_retract(&solution, 0, 0);
        assert_eq!(score, HardSoftScore::of(-1, -5));

        solution.routes[0].tasks = vec![0, 1];
        score = score + constraint.on_insert(&solution, 0, 0);

        assert_eq!(score, HardSoftScore::of(-1, -5));
        assert_eq!(constraint.evaluate(&solution), score);
    }

    #[test]
    fn incremental_route_update_only_reads_changed_owner_route() {
        reset_access_counts();
        let mut director =
            ScoreDirector::new(plan(vec![vec![0], vec![1]]), (counted_constraint(),));

        assert_eq!(director.calculate_score(), HardSoftScore::of(0, -5));
        assert_eq!(NODE_COUNT_CALLS.load(Ordering::Relaxed), 1);
        assert_eq!(NODE_DURATION_CALLS.load(Ordering::Relaxed), 2);
        assert_eq!(FIXED_SUCCESSOR_CALLS.load(Ordering::Relaxed), 2);
        assert_eq!(OWNER_COUNT_CALLS.load(Ordering::Relaxed), 1);
        assert_eq!(LIST_LEN_CALLS[0].load(Ordering::Relaxed), 1);
        assert_eq!(LIST_LEN_CALLS[1].load(Ordering::Relaxed), 1);
        assert_eq!(LIST_GET_CALLS[0].load(Ordering::Relaxed), 1);
        assert_eq!(LIST_GET_CALLS[1].load(Ordering::Relaxed), 1);

        reset_access_counts();
        director.before_variable_changed(0, 0);
        director.working_solution_mut().routes[0].tasks = vec![0, 1];
        director.after_variable_changed(0, 0);

        assert_eq!(director.calculate_score(), HardSoftScore::of(-1, -5));
        assert_eq!(NODE_COUNT_CALLS.load(Ordering::Relaxed), 0);
        assert_eq!(NODE_DURATION_CALLS.load(Ordering::Relaxed), 0);
        assert_eq!(FIXED_SUCCESSOR_CALLS.load(Ordering::Relaxed), 0);
        assert_eq!(OWNER_COUNT_CALLS.load(Ordering::Relaxed), 0);
        assert_eq!(LIST_LEN_CALLS[0].load(Ordering::Relaxed), 1);
        assert_eq!(LIST_LEN_CALLS[1].load(Ordering::Relaxed), 0);
        assert_eq!(LIST_GET_CALLS[0].load(Ordering::Relaxed), 2);
        assert_eq!(LIST_GET_CALLS[1].load(Ordering::Relaxed), 0);
        assert_eq!(
            Director::fresh_score(&director),
            Some(director.calculate_score())
        );
    }

    #[test]
    fn penalizes_missing_duplicate_and_wrong_owner_assignments() {
        let constraint = owner_constraint();

        assert_eq!(
            constraint.evaluate(&plan(vec![vec![0, 1], vec![1]])),
            HardSoftScore::of(-2, -5)
        );
    }

    #[test]
    fn ignores_unrelated_descriptor_changes() {
        let mut director = ScoreDirector::new(plan(vec![vec![0], vec![1]]), (constraint(),));

        let score = director.calculate_score();
        director.before_variable_changed(1, 0);
        director.working_solution_mut().routes[0].tasks = vec![1, 0];
        director.after_variable_changed(1, 0);

        assert_eq!(director.calculate_score(), score);
    }
}
