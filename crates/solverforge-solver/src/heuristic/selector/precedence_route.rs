use std::marker::PhantomData;

mod delta;
use delta::{
    added_edges_introduce_cycle, changed_route_edges_into, route_after_list_change_into,
    route_after_permutation_into, route_after_sublist_change_into, route_after_sublist_swap_into,
    route_delta_has_cycle_into,
};

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
pub(crate) fn node_index<V: PartialEq>(elements: &[V], needle: &V) -> Option<usize> {
    elements.iter().position(|element| element == needle)
}
