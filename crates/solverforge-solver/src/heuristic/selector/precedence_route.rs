use std::marker::PhantomData;

pub(crate) struct PrecedenceRouteHooks<S, V> {
    fixed_successors: fn(&S, V, &mut Vec<V>),
    entity_count: fn(&S) -> usize,
    list_len: fn(&S, usize) -> usize,
    list_get: fn(&S, usize, usize) -> Option<V>,
    _phantom: PhantomData<fn() -> (S, V)>,
}

impl<S, V> Clone for PrecedenceRouteHooks<S, V> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S, V> Copy for PrecedenceRouteHooks<S, V> {}

impl<S, V> PrecedenceRouteHooks<S, V> {
    pub(crate) fn new(
        fixed_successors: fn(&S, V, &mut Vec<V>),
        entity_count: fn(&S) -> usize,
        list_len: fn(&S, usize) -> usize,
        list_get: fn(&S, usize, usize) -> Option<V>,
    ) -> Self {
        Self {
            fixed_successors,
            entity_count,
            list_len,
            list_get,
            _phantom: PhantomData,
        }
    }

    pub(crate) fn build_graph_with_elements(
        &self,
        solution: &S,
        elements: &[V],
    ) -> PrecedenceRouteGraph
    where
        V: Clone + PartialEq,
    {
        build_precedence_route_graph(
            solution,
            elements,
            |solution, element, successors| (self.fixed_successors)(solution, element, successors),
            self.entity_count,
            self.list_len,
            self.list_get,
        )
    }
}

/// Builds the immutable route/fixed-successor graph from one physical list
/// access protocol. Static function-pointer hooks and the compiled runtime
/// carrier both use this one graph construction path.
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_precedence_route_graph<S, V>(
    solution: &S,
    elements: &[V],
    mut fixed_successors: impl FnMut(&S, V, &mut Vec<V>),
    entity_count: impl Fn(&S) -> usize,
    list_len: impl Fn(&S, usize) -> usize,
    list_get: impl Fn(&S, usize, usize) -> Option<V>,
) -> PrecedenceRouteGraph
where
    V: Clone + PartialEq,
{
    let node_count = elements.len();
    let mut graph = PrecedenceRouteGraph {
        fixed_successors: vec![Vec::new(); node_count],
        fixed_predecessors: vec![Vec::new(); node_count],
        successors: vec![Vec::new(); node_count],
        predecessors: vec![Vec::new(); node_count],
        route_nodes: Vec::new(),
        ..PrecedenceRouteGraph::default()
    };

    let mut raw_successors = Vec::new();
    for (from_idx, element) in elements.iter().enumerate() {
        raw_successors.clear();
        fixed_successors(solution, element.clone(), &mut raw_successors);
        for successor in &raw_successors {
            if let Some(to_idx) = node_index(elements, successor) {
                graph.push_edge(from_idx, to_idx);
                if !graph.fixed_successors[from_idx].contains(&to_idx) {
                    graph.fixed_successors[from_idx].push(to_idx);
                    graph.fixed_predecessors[to_idx].push(from_idx);
                }
            }
        }
    }

    for owner in 0..entity_count(solution) {
        let len = list_len(solution, owner);
        let mut nodes = Vec::with_capacity(len);
        let mut previous = None;
        for position in 0..len {
            let Some(element) = list_get(solution, owner, position) else {
                previous = None;
                continue;
            };
            let Some(node) = node_index(elements, &element) else {
                previous = None;
                continue;
            };
            if let Some(from) = previous {
                graph.push_edge(from, node);
            }
            nodes.push(node);
            previous = Some(node);
        }
        graph.route_nodes.push(nodes);
    }

    graph
}

#[derive(Default)]
pub(crate) struct PrecedenceRouteGraph {
    fixed_successors: Vec<Vec<usize>>,
    fixed_predecessors: Vec<Vec<usize>>,
    successors: Vec<Vec<usize>>,
    predecessors: Vec<Vec<usize>>,
    route_nodes: Vec<Vec<usize>>,
    /* Reusable scratch for hypothetical-move cycle checks: never allocates
    per candidate. `after_route` holds the hypothesized route; the cycle
    topologies read the live successor lists plus the small edge delta. */
    scratch_after_route: Vec<usize>,
    scratch_before_edges: Vec<(usize, usize)>,
    scratch_after_edges: Vec<(usize, usize)>,
    scratch_removed: Vec<(usize, usize)>,
    scratch_added: Vec<(usize, usize)>,
    scratch_indegree: Vec<usize>,
    scratch_ready: Vec<usize>,
    scratch_visited: Vec<bool>,
    scratch_stack: Vec<usize>,
    scratch_active_added: Vec<(usize, usize)>,
}

impl PrecedenceRouteGraph {
    pub(crate) fn successors(&self) -> &[Vec<usize>] {
        &self.successors
    }

    pub(crate) fn predecessors(&self) -> &[Vec<usize>] {
        &self.predecessors
    }

    pub(crate) fn route(&self, entity: usize) -> Option<&[usize]> {
        self.route_nodes.get(entity).map(Vec::as_slice)
    }

    pub(crate) fn fixed_successors(&self, node: usize) -> &[usize] {
        self.fixed_successors
            .get(node)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub(crate) fn fixed_predecessors(&self, node: usize) -> &[usize] {
        self.fixed_predecessors
            .get(node)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub(crate) fn node_route_position(&self, node: usize) -> Option<(usize, usize)> {
        self.route_nodes
            .iter()
            .enumerate()
            .find_map(|(entity, route)| {
                route
                    .iter()
                    .position(|&candidate| candidate == node)
                    .map(|position| (entity, position))
            })
    }

    pub(crate) fn intra_list_change_introduces_cycle(
        &mut self,
        entity: usize,
        source: usize,
        dest: usize,
    ) -> bool {
        let Some(route) = self.route(entity) else {
            return false;
        };
        let route = route.to_vec();
        route_after_list_change_into(&route, source, dest, &mut self.scratch_after_route);
        self.route_move_introduces_cycle(&route)
    }

    pub(crate) fn intra_list_swap_introduces_cycle(
        &mut self,
        entity: usize,
        first: usize,
        second: usize,
    ) -> bool {
        let Some(route) = self.route(entity) else {
            return false;
        };
        let route = route.to_vec();
        self.scratch_after_route.clear();
        self.scratch_after_route.extend_from_slice(&route);
        self.scratch_after_route.swap(first, second);
        self.route_move_introduces_cycle(&route)
    }

    pub(crate) fn intra_list_reverse_introduces_cycle(
        &mut self,
        entity: usize,
        start: usize,
        end: usize,
    ) -> bool {
        let Some(route) = self.route(entity) else {
            return false;
        };
        let route = route.to_vec();
        self.scratch_after_route.clear();
        self.scratch_after_route.extend_from_slice(&route);
        self.scratch_after_route[start..end].reverse();
        self.route_move_introduces_cycle(&route)
    }

    pub(crate) fn intra_sublist_change_introduces_cycle(
        &mut self,
        entity: usize,
        source_start: usize,
        source_end: usize,
        dest: usize,
    ) -> bool {
        let Some(route) = self.route(entity) else {
            return false;
        };
        let route = route.to_vec();
        route_after_sublist_change_into(
            &route,
            source_start,
            source_end,
            dest,
            &mut self.scratch_after_route,
        );
        self.route_move_introduces_cycle(&route)
    }

    pub(crate) fn intra_sublist_swap_introduces_cycle(
        &mut self,
        entity: usize,
        first_start: usize,
        first_end: usize,
        second_start: usize,
        second_end: usize,
    ) -> bool {
        let Some(route) = self.route(entity) else {
            return false;
        };
        let route = route.to_vec();
        if !route_after_sublist_swap_into(
            &route,
            first_start,
            first_end,
            second_start,
            second_end,
            &mut self.scratch_after_route,
        ) {
            return false;
        }
        self.route_move_introduces_cycle(&route)
    }

    pub(crate) fn intra_list_permutation_introduces_cycle(
        &mut self,
        entity: usize,
        start: usize,
        end: usize,
        permutation: &[usize],
    ) -> bool {
        let Some(route) = self.route(entity) else {
            return false;
        };
        let route = route.to_vec();
        route_after_permutation_into(
            &route,
            start,
            end,
            permutation,
            &mut self.scratch_after_route,
        );
        self.route_move_introduces_cycle(&route)
    }

    pub(crate) fn multi_intra_list_swaps_introduce_cycle(
        &mut self,
        swaps: &[(usize, usize, usize)],
    ) -> bool {
        self.scratch_removed.clear();
        self.scratch_added.clear();
        for &(entity, first, second) in swaps {
            let Some(route) = self.route(entity) else {
                continue;
            };
            if first >= route.len() || second >= route.len() {
                continue;
            }
            let route = route.to_vec();
            self.scratch_after_route.clear();
            self.scratch_after_route.extend_from_slice(&route);
            self.scratch_after_route.swap(first, second);
            changed_route_edges_into(
                &route,
                &self.scratch_after_route,
                &mut self.scratch_before_edges,
                &mut self.scratch_after_edges,
                &mut self.scratch_removed,
                &mut self.scratch_added,
            );
        }
        route_delta_has_cycle_into(
            &self.fixed_successors,
            &self.successors,
            &self.scratch_removed,
            &self.scratch_added,
            &mut self.scratch_indegree,
            &mut self.scratch_ready,
            &mut self.scratch_visited,
        )
    }

    pub(crate) fn insertion_introduces_cycle(
        &mut self,
        previous: Option<usize>,
        element: usize,
        next: Option<usize>,
    ) -> bool {
        self.scratch_removed.clear();
        self.scratch_added.clear();
        if let Some(previous) = previous {
            self.scratch_added.push((previous, element));
            if let Some(next) = next {
                self.scratch_removed.push((previous, next));
            }
        }
        if let Some(next) = next {
            self.scratch_added.push((element, next));
        }
        route_delta_has_cycle_into(
            &self.fixed_successors,
            &self.successors,
            &self.scratch_removed,
            &self.scratch_added,
            &mut self.scratch_indegree,
            &mut self.scratch_ready,
            &mut self.scratch_visited,
        )
    }

    fn push_edge(&mut self, from: usize, to: usize) {
        if !self.successors[from].contains(&to) {
            self.successors[from].push(to);
            self.predecessors[to].push(from);
        }
    }

    /* `after` must have been staged in `scratch_after_route` by the caller. */
    fn route_move_introduces_cycle(&mut self, before: &[usize]) -> bool {
        changed_route_edges_into(
            before,
            &self.scratch_after_route,
            &mut self.scratch_before_edges,
            &mut self.scratch_after_edges,
            &mut self.scratch_removed,
            &mut self.scratch_added,
        );
        added_edges_introduce_cycle(
            &self.fixed_successors,
            &self.successors,
            &self.scratch_removed,
            &self.scratch_added,
            &mut self.scratch_active_added,
            &mut self.scratch_visited,
            &mut self.scratch_stack,
        )
    }
}

/* Hypothetical route mutation writers: each stages the post-move route into
the caller-owned scratch (no per-candidate allocation), keeping the same
arithmetic as the value-returning forms they replace.

`in_place` scratch may alias `route`'s contents — they are copied first. */
fn route_after_list_change_into(
    route: &[usize],
    source: usize,
    dest: usize,
    scratch: &mut Vec<usize>,
) {
    scratch.clear();
    scratch.extend_from_slice(route);
    let node = scratch.remove(source);
    let insert_pos = if dest > source { dest - 1 } else { dest };
    scratch.insert(insert_pos, node);
}

fn route_after_sublist_change_into(
    route: &[usize],
    source_start: usize,
    source_end: usize,
    dest: usize,
    scratch: &mut Vec<usize>,
) {
    scratch.clear();
    scratch.extend_from_slice(route);
    let moved = scratch.drain(source_start..source_end).collect::<Vec<_>>();
    scratch.splice(dest..dest, moved);
}

fn route_after_sublist_swap_into(
    route: &[usize],
    first_start: usize,
    first_end: usize,
    second_start: usize,
    second_end: usize,
    scratch: &mut Vec<usize>,
) -> bool {
    let (left_start, left_end, right_start, right_end) = if first_start <= second_start {
        (first_start, first_end, second_start, second_end)
    } else {
        (second_start, second_end, first_start, first_end)
    };
    if left_end > right_start || right_end > route.len() {
        return false;
    }

    scratch.clear();
    scratch.reserve(route.len());
    scratch.extend_from_slice(&route[..left_start]);
    scratch.extend_from_slice(&route[right_start..right_end]);
    scratch.extend_from_slice(&route[left_end..right_start]);
    scratch.extend_from_slice(&route[left_start..left_end]);
    scratch.extend_from_slice(&route[right_end..]);
    true
}

fn route_after_permutation_into(
    route: &[usize],
    start: usize,
    end: usize,
    permutation: &[usize],
    scratch: &mut Vec<usize>,
) {
    scratch.clear();
    scratch.extend_from_slice(route);
    for (offset, &source_offset) in permutation.iter().enumerate() {
        scratch[start + offset] = route[start + source_offset];
    }
    debug_assert_eq!(start + permutation.len(), end);
}

/* Edge diff into reusable buffers; contains() checks run on the small
per-route edge vectors (routes are short; the win is zero allocation). */
fn route_edges_into(route: &[usize], edges: &mut Vec<(usize, usize)>) {
    edges.clear();
    edges.extend(route.windows(2).map(|window| (window[0], window[1])));
}

fn changed_route_edges_into(
    before: &[usize],
    after: &[usize],
    before_edges: &mut Vec<(usize, usize)>,
    after_edges: &mut Vec<(usize, usize)>,
    removed: &mut Vec<(usize, usize)>,
    added: &mut Vec<(usize, usize)>,
) {
    route_edges_into(before, before_edges);
    route_edges_into(after, after_edges);
    removed.clear();
    added.clear();
    for edge in before_edges.iter().copied() {
        if !after_edges.contains(&edge) {
            removed.push(edge);
        }
    }
    for edge in after_edges.iter().copied() {
        if !before_edges.contains(&edge) {
            added.push(edge);
        }
    }
}

fn added_edges_introduce_cycle(
    fixed_successors: &[Vec<usize>],
    successors: &[Vec<usize>],
    removed: &[(usize, usize)],
    added: &[(usize, usize)],
    active_added: &mut Vec<(usize, usize)>,
    visited: &mut Vec<bool>,
    stack: &mut Vec<usize>,
) -> bool {
    let route_delta = RouteDeltaView {
        fixed_successors,
        successors,
        removed,
        added: &[],
    };
    active_added.clear();
    if visited.len() < successors.len() {
        visited.resize(successors.len(), false);
    }
    for &(from, to) in added {
        if route_delta.with_added(active_added).edge_active(from, to) {
            continue;
        }
        visited.fill(false);
        stack.clear();
        if reaches_with_route_delta(
            to,
            from,
            route_delta.with_added(active_added),
            visited,
            stack,
        ) {
            return true;
        }
        active_added.push((from, to));
    }
    false
}

/* Topological cycle check over the live successor lists plus the delta:
indegree is recomputed into a reusable buffer; adjacency stays implicit
(the live lists minus removed plus added, with fixed successors pinned). */
fn route_delta_has_cycle_into(
    fixed_successors: &[Vec<usize>],
    successors: &[Vec<usize>],
    removed: &[(usize, usize)],
    added: &[(usize, usize)],
    indegree: &mut Vec<usize>,
    ready: &mut Vec<usize>,
    processed: &mut Vec<bool>,
) -> bool {
    let node_count = successors.len();
    if indegree.len() < node_count {
        indegree.resize(node_count, 0);
    }
    indegree
        .iter_mut()
        .take(node_count)
        .for_each(|degree| *degree = 0);
    if processed.len() < node_count {
        processed.resize(node_count, false);
    }

    for (from, nodes) in successors.iter().enumerate() {
        for &to in nodes {
            if to >= node_count {
                continue;
            }
            if removed.contains(&(from, to))
                && !fixed_successors
                    .get(from)
                    .is_some_and(|fixed| fixed.contains(&to))
            {
                continue;
            }
            indegree[to] += 1;
        }
    }
    for &(from, to) in added {
        if from >= node_count || to >= node_count {
            continue;
        }
        if !successors
            .get(from)
            .is_some_and(|nodes| nodes.contains(&to))
            || removed.contains(&(from, to))
        {
            indegree[to] += 1;
        }
    }

    ready.clear();
    for (node, &degree) in indegree.iter().enumerate() {
        if degree == 0 {
            ready.push(node);
        }
    }

    let mut visited_count = 0usize;
    while let Some(node) = ready.pop() {
        visited_count += 1;
        let stack_node = node;
        for &successor in successors.get(stack_node).map(Vec::as_slice).unwrap_or(&[]) {
            if successor >= node_count {
                continue;
            }
            if removed.contains(&(stack_node, successor))
                && !fixed_successors
                    .get(stack_node)
                    .is_some_and(|fixed| fixed.contains(&successor))
            {
                continue;
            }
            indegree[successor] -= 1;
            if indegree[successor] == 0 {
                ready.push(successor);
            }
        }
        for &(from, to) in added {
            if from == stack_node && to < node_count {
                let live = successors
                    .get(from)
                    .is_some_and(|nodes| nodes.contains(&to))
                    && !(removed.contains(&(from, to))
                        && !fixed_successors
                            .get(from)
                            .is_some_and(|fixed| fixed.contains(&to)));
                if !live {
                    indegree[to] -= 1;
                    if indegree[to] == 0 {
                        ready.push(to);
                    }
                }
            }
        }
    }

    visited_count != node_count
}

#[derive(Clone, Copy)]
struct RouteDeltaView<'a> {
    fixed_successors: &'a [Vec<usize>],
    successors: &'a [Vec<usize>],
    removed: &'a [(usize, usize)],
    added: &'a [(usize, usize)],
}

impl<'a> RouteDeltaView<'a> {
    fn with_added(self, added: &'a [(usize, usize)]) -> Self {
        Self { added, ..self }
    }

    fn edge_active(self, from: usize, to: usize) -> bool {
        self.added.contains(&(from, to))
            || self.successors.get(from).is_some_and(|nodes| {
                nodes.contains(&to)
                    && (!self.removed.contains(&(from, to))
                        || self
                            .fixed_successors
                            .get(from)
                            .is_some_and(|fixed| fixed.contains(&to)))
            })
    }
}

fn reaches_with_route_delta(
    source: usize,
    target: usize,
    route_delta: RouteDeltaView<'_>,
    visited: &mut [bool],
    stack: &mut Vec<usize>,
) -> bool {
    if source >= route_delta.successors.len() || target >= route_delta.successors.len() {
        return false;
    }
    stack.push(source);
    while let Some(node) = stack.pop() {
        if node == target {
            return true;
        }
        if visited[node] {
            continue;
        }
        visited[node] = true;
        for &successor in &route_delta.successors[node] {
            if route_delta.removed.contains(&(node, successor))
                && !route_delta
                    .fixed_successors
                    .get(node)
                    .is_some_and(|nodes| nodes.contains(&successor))
            {
                continue;
            }
            if !visited.get(successor).copied().unwrap_or(true) {
                stack.push(successor);
            }
        }
        for &(from, to) in route_delta.added {
            if from == node && !visited.get(to).copied().unwrap_or(true) {
                stack.push(to);
            }
        }
    }
    false
}

pub(crate) fn node_index<V: PartialEq>(elements: &[V], needle: &V) -> Option<usize> {
    elements.iter().position(|element| element == needle)
}
