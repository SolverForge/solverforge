pub(super) fn route_after_list_change_into(
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

pub(super) fn route_after_sublist_change_into(
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

pub(super) fn route_after_sublist_swap_into(
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

pub(super) fn route_after_permutation_into(
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

pub(super) fn changed_route_edges_into(
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

pub(super) fn added_edges_introduce_cycle(
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
pub(super) fn route_delta_has_cycle_into(
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
    pub(super) fixed_successors: &'a [Vec<usize>],
    pub(super) successors: &'a [Vec<usize>],
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
