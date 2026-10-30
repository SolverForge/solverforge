use std::collections::{HashMap, VecDeque};
use std::marker::PhantomData;

use solverforge_core::score::HardSoftScore;
use solverforge_core::ConstraintRef;

use crate::api::constraint_set::{IncrementalConstraint, SingleConstraint};

mod graph;
mod route;

type NodeId = usize;
type OwnerId = usize;
type Edge = (NodeId, NodeId);

/// Stock incremental constraint that scores list-variable routes against fixed
/// precedence edges, over `HardSoftScore`.
///
/// Node `n` must run after every node that precedes it in a route and after
/// every fixed predecessor. The constraint builds one precedence graph from the
/// fixed edges plus every adjacent pair of every owner's route, and scores:
///
/// - hard: one point per out-of-range fixed successor, per route slot holding
///   no node or an out-of-range node, per node in the wrong owner (see
///   [`with_expected_owner`](Self::with_expected_owner)), per node assigned zero
///   times (and `count - 1` for a node assigned `count > 1` times), plus
///   `node_count` while the graph has a cycle;
/// - soft: minus the makespan, the latest finish over the graph where a node
///   finishes at the latest predecessor finish plus its duration (`0` while the
///   graph is cyclic).
///
/// `on_retract` defers the owner and returns zero; the matching `on_insert`
/// diffs the owner's old and new route edges and refreshes the graph from the
/// touched nodes, returning the whole delta. Notifications for other
/// descriptors, and before `initialize`, return zero.
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
    /// Creates the constraint.
    ///
    /// - `list_descriptor_index`: descriptor whose entities own the routes.
    /// - `node_count`: number of nodes; node ids are `0..node_count`.
    /// - `node_duration`: duration of a node.
    /// - `fixed_successors`: pushes the fixed successors of a node.
    /// - `list_owner_count`, `list_len`, `list_get`: the owners, the length of an
    ///   owner's route and the node at a route position (`None` for an empty slot).
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

    /// Adds fixed-owner validation: when `expected_owner` returns `Some(owner)`
    /// for a node routed by a different owner, that slot costs one hard point.
    /// `None` disables the check.
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
            let mut change = state.take_route_change_scratch();
            state.replace_owner_route_scratch_with(&mut change, solution, owner, access);
            state.restore_route_change_scratch(change);
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

impl<S> SingleConstraint for ListPrecedenceMakespanConstraint<S> {}

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
        let mut change = state.take_route_change_scratch();
        state.flush_pending_retractions(&mut change, entity_index);
        state.replace_owner_route_scratch_with(&mut change, solution, entity_index, access);
        let after = state.refresh_score_after_route_change(&change);
        state.restore_route_change_scratch(change);
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
        if !state.pending_retracted[entity_index] {
            state.pending_retracted[entity_index] = true;
            state.pending_retractions.push(entity_index);
        }
        // Preserve the matching owner baseline for a net-edge diff. Other
        // pending owners are removed before the next insert is scored, so
        // interleaved notifications retain their retract/insert semantics.
        HardSoftScore::ZERO
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
    max_finish_nodes: usize,
    scratch_queued: Vec<usize>,
    scratch_queue: Vec<NodeId>,
    scratch_queue_head: usize,
    scratch_visited_stamp: usize,
    probe_queued: Vec<usize>,
    probe_visited_stamp: usize,
    scratch_probe_stack: Vec<NodeId>,
    scratch_elements: Vec<NodeId>,
    scratch_edges: Vec<Edge>,
    scratch_invalid_count: usize,
    scratch_violation_count: usize,
    scratch_assignment_counts: HashMap<NodeId, (usize, usize)>,
    scratch_assignment_keys: Vec<NodeId>,
    scratch_edge_counts: HashMap<Edge, (usize, usize)>,
    scratch_edge_keys: Vec<Edge>,
    scratch_route_change: RouteChange,
    deferred_seeds: Vec<NodeId>,
    deferred_seed_seen: Vec<usize>,
    deferred_seed_stamp: usize,
    pending_retracted: Vec<bool>,
    pending_retractions: Vec<OwnerId>,
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
            max_finish_nodes: 0,
            scratch_queued: Vec::new(),
            scratch_queue: Vec::new(),
            scratch_queue_head: 0,
            scratch_visited_stamp: 1,
            probe_queued: Vec::new(),
            probe_visited_stamp: 1,
            scratch_probe_stack: Vec::new(),
            scratch_elements: Vec::new(),
            scratch_edges: Vec::new(),
            scratch_invalid_count: 0,
            scratch_violation_count: 0,
            scratch_assignment_counts: HashMap::new(),
            scratch_assignment_keys: Vec::new(),
            scratch_edge_counts: HashMap::new(),
            scratch_edge_keys: Vec::new(),
            scratch_route_change: RouteChange::default(),
            deferred_seeds: Vec::new(),
            deferred_seed_seen: Vec::new(),
            deferred_seed_stamp: 0,
            pending_retracted: vec![false; owner_count],
            pending_retractions: Vec::with_capacity(owner_count),
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
mod tests;
