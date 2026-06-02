use std::marker::PhantomData;

pub(crate) struct PrecedenceRouteHooks<S, V> {
    element_count: fn(&S) -> usize,
    index_to_element: fn(&S, usize) -> V,
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
        element_count: fn(&S) -> usize,
        index_to_element: fn(&S, usize) -> V,
        fixed_successors: fn(&S, V, &mut Vec<V>),
        entity_count: fn(&S) -> usize,
        list_len: fn(&S, usize) -> usize,
        list_get: fn(&S, usize, usize) -> Option<V>,
    ) -> Self {
        Self {
            element_count,
            index_to_element,
            fixed_successors,
            entity_count,
            list_len,
            list_get,
            _phantom: PhantomData,
        }
    }

    pub(crate) fn build_graph(&self, solution: &S) -> PrecedenceRouteGraph
    where
        V: Clone + PartialEq,
    {
        let elements = (0..(self.element_count)(solution))
            .map(|index| (self.index_to_element)(solution, index))
            .collect::<Vec<_>>();
        self.build_graph_with_elements(solution, &elements)
    }

    pub(crate) fn build_graph_with_elements(
        &self,
        solution: &S,
        elements: &[V],
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
        };

        let mut raw_successors = Vec::new();
        for (from_idx, element) in elements.iter().enumerate() {
            raw_successors.clear();
            (self.fixed_successors)(solution, element.clone(), &mut raw_successors);
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

        for owner in 0..(self.entity_count)(solution) {
            let len = (self.list_len)(solution, owner);
            let mut nodes = Vec::with_capacity(len);
            let mut previous = None;
            for pos in 0..len {
                let Some(element) = (self.list_get)(solution, owner, pos) else {
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
}

#[derive(Default)]
pub(crate) struct PrecedenceRouteGraph {
    fixed_successors: Vec<Vec<usize>>,
    fixed_predecessors: Vec<Vec<usize>>,
    successors: Vec<Vec<usize>>,
    predecessors: Vec<Vec<usize>>,
    route_nodes: Vec<Vec<usize>>,
}

impl PrecedenceRouteGraph {
    pub(super) fn successors(&self) -> &[Vec<usize>] {
        &self.successors
    }

    pub(super) fn predecessors(&self) -> &[Vec<usize>] {
        &self.predecessors
    }

    pub(super) fn route(&self, entity: usize) -> Option<&[usize]> {
        self.route_nodes.get(entity).map(Vec::as_slice)
    }

    pub(super) fn fixed_successors(&self, node: usize) -> &[usize] {
        self.fixed_successors
            .get(node)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub(super) fn fixed_predecessors(&self, node: usize) -> &[usize] {
        self.fixed_predecessors
            .get(node)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub(super) fn node_route_position(&self, node: usize) -> Option<(usize, usize)> {
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

    pub(super) fn intra_list_change_introduces_cycle(
        &self,
        entity: usize,
        source: usize,
        dest: usize,
    ) -> bool {
        let Some(route) = self.route(entity) else {
            return false;
        };
        self.route_move_introduces_cycle(route, &route_after_list_change(route, source, dest))
    }

    pub(super) fn intra_list_swap_introduces_cycle(
        &self,
        entity: usize,
        first: usize,
        second: usize,
    ) -> bool {
        let Some(route) = self.route(entity) else {
            return false;
        };
        let mut after = route.to_vec();
        after.swap(first, second);
        self.route_move_introduces_cycle(route, &after)
    }

    pub(super) fn intra_list_reverse_introduces_cycle(
        &self,
        entity: usize,
        start: usize,
        end: usize,
    ) -> bool {
        let Some(route) = self.route(entity) else {
            return false;
        };
        let mut after = route.to_vec();
        after[start..end].reverse();
        self.route_move_introduces_cycle(route, &after)
    }

    pub(super) fn intra_sublist_change_introduces_cycle(
        &self,
        entity: usize,
        source_start: usize,
        source_end: usize,
        dest: usize,
    ) -> bool {
        let Some(route) = self.route(entity) else {
            return false;
        };
        self.route_move_introduces_cycle(
            route,
            &route_after_sublist_change(route, source_start, source_end, dest),
        )
    }

    pub(super) fn intra_sublist_swap_introduces_cycle(
        &self,
        entity: usize,
        first_start: usize,
        first_end: usize,
        second_start: usize,
        second_end: usize,
    ) -> bool {
        let Some(route) = self.route(entity) else {
            return false;
        };
        let Some(after) =
            route_after_sublist_swap(route, first_start, first_end, second_start, second_end)
        else {
            return false;
        };
        self.route_move_introduces_cycle(route, &after)
    }

    pub(super) fn intra_list_permutation_introduces_cycle(
        &self,
        entity: usize,
        start: usize,
        end: usize,
        permutation: &[usize],
    ) -> bool {
        let Some(route) = self.route(entity) else {
            return false;
        };
        self.route_move_introduces_cycle(
            route,
            &route_after_permutation(route, start, end, permutation),
        )
    }

    pub(super) fn multi_intra_list_swaps_introduce_cycle(
        &self,
        swaps: &[(usize, usize, usize)],
    ) -> bool {
        let mut removed = Vec::new();
        let mut added = Vec::new();
        for &(entity, first, second) in swaps {
            let Some(route) = self.route(entity) else {
                continue;
            };
            if first >= route.len() || second >= route.len() {
                continue;
            }
            let mut after = route.to_vec();
            after.swap(first, second);
            let (route_removed, route_added) = changed_route_edges(route, &after);
            removed.extend(route_removed);
            added.extend(route_added);
        }
        route_delta_has_cycle(&self.fixed_successors, &self.successors, &removed, &added)
    }

    pub(crate) fn insertion_introduces_cycle(
        &self,
        previous: Option<usize>,
        element: usize,
        next: Option<usize>,
    ) -> bool {
        let mut removed = Vec::new();
        let mut added = Vec::new();
        if let Some(previous) = previous {
            added.push((previous, element));
            if let Some(next) = next {
                removed.push((previous, next));
            }
        }
        if let Some(next) = next {
            added.push((element, next));
        }
        route_delta_has_cycle(&self.fixed_successors, &self.successors, &removed, &added)
    }

    fn push_edge(&mut self, from: usize, to: usize) {
        if !self.successors[from].contains(&to) {
            self.successors[from].push(to);
            self.predecessors[to].push(from);
        }
    }

    fn route_move_introduces_cycle(&self, before: &[usize], after: &[usize]) -> bool {
        let (removed, added) = changed_route_edges(before, after);
        added_edges_introduce_cycle(&self.fixed_successors, &self.successors, &removed, &added)
    }
}

fn route_after_list_change(route: &[usize], source: usize, dest: usize) -> Vec<usize> {
    let mut after = route.to_vec();
    let node = after.remove(source);
    let insert_pos = if dest > source { dest - 1 } else { dest };
    after.insert(insert_pos, node);
    after
}

fn route_after_sublist_change(
    route: &[usize],
    source_start: usize,
    source_end: usize,
    dest: usize,
) -> Vec<usize> {
    let mut after = route.to_vec();
    let moved = after.drain(source_start..source_end).collect::<Vec<_>>();
    after.splice(dest..dest, moved);
    after
}

fn route_after_sublist_swap(
    route: &[usize],
    first_start: usize,
    first_end: usize,
    second_start: usize,
    second_end: usize,
) -> Option<Vec<usize>> {
    let (left_start, left_end, right_start, right_end) = if first_start <= second_start {
        (first_start, first_end, second_start, second_end)
    } else {
        (second_start, second_end, first_start, first_end)
    };
    if left_end > right_start || right_end > route.len() {
        return None;
    }

    let mut after = Vec::with_capacity(route.len());
    after.extend_from_slice(&route[..left_start]);
    after.extend_from_slice(&route[right_start..right_end]);
    after.extend_from_slice(&route[left_end..right_start]);
    after.extend_from_slice(&route[left_start..left_end]);
    after.extend_from_slice(&route[right_end..]);
    Some(after)
}

fn route_after_permutation(
    route: &[usize],
    start: usize,
    end: usize,
    permutation: &[usize],
) -> Vec<usize> {
    let mut after = route.to_vec();
    for (offset, &source_offset) in permutation.iter().enumerate() {
        after[start + offset] = route[start + source_offset];
    }
    debug_assert_eq!(start + permutation.len(), end);
    after
}

fn changed_route_edges(
    before: &[usize],
    after: &[usize],
) -> (Vec<(usize, usize)>, Vec<(usize, usize)>) {
    let before_edges = route_edges(before);
    let after_edges = route_edges(after);
    let removed = before_edges
        .iter()
        .copied()
        .filter(|edge| !after_edges.contains(edge))
        .collect();
    let added = after_edges
        .iter()
        .copied()
        .filter(|edge| !before_edges.contains(edge))
        .collect();
    (removed, added)
}

fn route_edges(route: &[usize]) -> Vec<(usize, usize)> {
    route
        .windows(2)
        .map(|window| (window[0], window[1]))
        .collect()
}

fn added_edges_introduce_cycle(
    fixed_successors: &[Vec<usize>],
    successors: &[Vec<usize>],
    removed: &[(usize, usize)],
    added: &[(usize, usize)],
) -> bool {
    let mut active_added = Vec::new();
    let mut visited = vec![false; successors.len()];
    let mut stack = Vec::new();
    for &(from, to) in added {
        if edge_active(
            fixed_successors,
            successors,
            removed,
            &active_added,
            from,
            to,
        ) {
            continue;
        }
        visited.fill(false);
        stack.clear();
        if reaches_with_route_delta(
            to,
            from,
            fixed_successors,
            successors,
            removed,
            &active_added,
            &mut visited,
            &mut stack,
        ) {
            return true;
        }
        active_added.push((from, to));
    }
    false
}

fn route_delta_has_cycle(
    fixed_successors: &[Vec<usize>],
    successors: &[Vec<usize>],
    removed: &[(usize, usize)],
    added: &[(usize, usize)],
) -> bool {
    let node_count = successors.len();
    let mut adjacency = vec![Vec::new(); node_count];
    let mut indegree = vec![0usize; node_count];

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
            if !adjacency[from].contains(&to) {
                adjacency[from].push(to);
                indegree[to] += 1;
            }
        }
    }

    for &(from, to) in added {
        if from >= node_count || to >= node_count {
            continue;
        }
        if !adjacency[from].contains(&to) {
            adjacency[from].push(to);
            indegree[to] += 1;
        }
    }

    let mut ready = Vec::new();
    for (node, &degree) in indegree.iter().enumerate() {
        if degree == 0 {
            ready.push(node);
        }
    }

    let mut visited = 0usize;
    while let Some(node) = ready.pop() {
        visited += 1;
        for &successor in &adjacency[node] {
            indegree[successor] -= 1;
            if indegree[successor] == 0 {
                ready.push(successor);
            }
        }
    }

    visited != node_count
}

fn reaches_with_route_delta(
    source: usize,
    target: usize,
    fixed_successors: &[Vec<usize>],
    successors: &[Vec<usize>],
    removed: &[(usize, usize)],
    added: &[(usize, usize)],
    visited: &mut [bool],
    stack: &mut Vec<usize>,
) -> bool {
    if source >= successors.len() || target >= successors.len() {
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
        for &successor in &successors[node] {
            if removed.contains(&(node, successor))
                && !fixed_successors
                    .get(node)
                    .is_some_and(|nodes| nodes.contains(&successor))
            {
                continue;
            }
            if !visited.get(successor).copied().unwrap_or(true) {
                stack.push(successor);
            }
        }
        for &(from, to) in added {
            if from == node && !visited.get(to).copied().unwrap_or(true) {
                stack.push(to);
            }
        }
    }
    false
}

fn edge_active(
    fixed_successors: &[Vec<usize>],
    successors: &[Vec<usize>],
    removed: &[(usize, usize)],
    added: &[(usize, usize)],
    from: usize,
    to: usize,
) -> bool {
    added.contains(&(from, to))
        || successors.get(from).is_some_and(|nodes| {
            nodes.contains(&to)
                && (!removed.contains(&(from, to))
                    || fixed_successors
                        .get(from)
                        .is_some_and(|fixed| fixed.contains(&to)))
        })
}

pub(crate) fn node_index<V: PartialEq>(elements: &[V], needle: &V) -> Option<usize> {
    elements.iter().position(|element| element == needle)
}
