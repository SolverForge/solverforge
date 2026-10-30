use super::*;

impl ListPrecedenceState {
    /// Snapshots the owner's route into the reusable scratch buffers. The
    /// scratch slots must not be aliased by `replace_owner_route_scratch`'s
    /// diff bookkeeping (it swaps them into the owner route storage).
    fn owner_route_snapshot<S>(
        &mut self,
        solution: &S,
        owner: OwnerId,
        access: ListPrecedenceAccess<S>,
    ) {
        self.scratch_elements.clear();
        self.scratch_edges.clear();
        let len = (access.list_len)(solution, owner);
        let mut previous = None;
        let mut invalid_count = 0;
        let mut violation_count = 0;
        for pos in 0..len {
            let Some(node) = (access.list_get)(solution, owner, pos) else {
                invalid_count += 1;
                previous = None;
                continue;
            };
            if node >= self.node_count {
                invalid_count += 1;
                previous = None;
                continue;
            }

            self.scratch_elements.push(node);
            if access
                .expected_owner
                .and_then(|expected_owner| expected_owner(solution, node))
                .is_some_and(|expected| expected != owner)
            {
                violation_count += 1;
            }
            if let Some(from) = previous {
                let edge = (from, node);
                self.scratch_edges.push(edge);
            }
            previous = Some(node);
        }
        self.scratch_invalid_count = invalid_count;
        self.scratch_violation_count = violation_count;
    }

    pub(super) fn take_route_change_scratch(&mut self) -> RouteChange {
        std::mem::take(&mut self.scratch_route_change)
    }

    pub(super) fn restore_route_change_scratch(&mut self, change: RouteChange) {
        self.scratch_route_change = change;
    }

    pub(super) fn replace_owner_route_scratch_with<S>(
        &mut self,
        change: &mut RouteChange,
        solution: &S,
        owner: OwnerId,
        access: ListPrecedenceAccess<S>,
    ) {
        self.owner_route_snapshot(solution, owner, access);
        self.replace_owner_route_scratch(change, owner);
    }

    pub(super) fn flush_pending_retractions(
        &mut self,
        change: &mut RouteChange,
        inserted_owner: OwnerId,
    ) {
        for index in 0..self.pending_retractions.len() {
            let owner = self.pending_retractions[index];
            self.pending_retracted[owner] = false;
            if owner != inserted_owner {
                self.replace_owner_route_scratch_with_removal(change, owner);
                self.defer_removed_edges(change);
            }
        }
        self.pending_retractions.clear();
    }

    pub(super) fn defer_removed_edges(&mut self, change: &RouteChange) {
        if change.removed_edges.is_empty() {
            return;
        }
        self.deferred_seed_stamp = self.deferred_seed_stamp.wrapping_add(1);
        if self.deferred_seed_stamp == 0 {
            self.deferred_seed_seen
                .iter_mut()
                .for_each(|stamp| *stamp = 0);
            self.deferred_seed_stamp = 1;
        }
        if self.deferred_seed_seen.len() < self.node_count {
            self.deferred_seed_seen.resize(self.node_count, 0);
        }
        for edge in &change.removed_edges {
            let node = edge.1;
            if self.deferred_seed_seen[node] == self.deferred_seed_stamp {
                continue;
            }
            self.deferred_seed_seen[node] = self.deferred_seed_stamp;
            self.deferred_seeds.push(edge.1);
        }
    }

    fn replace_owner_route_scratch_with_removal(
        &mut self,
        change: &mut RouteChange,
        owner: OwnerId,
    ) {
        self.scratch_elements.clear();
        self.scratch_edges.clear();
        self.replace_owner_route_scratch(change, owner);
    }

    fn replace_owner_route_scratch(&mut self, change: &mut RouteChange, owner: OwnerId) {
        change.added_edges.clear();
        change.removed_edges.clear();
        let previous_invalid = self.owner_invalid_counts[owner];
        let previous_violation = self.owner_violation_counts[owner];

        let mut old_elements = std::mem::take(&mut self.owner_elements[owner]);
        let mut new_elements = std::mem::take(&mut self.scratch_elements);
        self.diff_owner_assignments(&old_elements, &new_elements);
        std::mem::swap(&mut old_elements, &mut new_elements);
        self.scratch_elements = new_elements;
        self.owner_elements[owner] = old_elements;

        let mut old_edges = std::mem::take(&mut self.owner_edges[owner]);
        let mut new_edges = std::mem::take(&mut self.scratch_edges);
        self.diff_owner_edges(&old_edges, &new_edges, change);
        std::mem::swap(&mut old_edges, &mut new_edges);
        self.scratch_edges = new_edges;
        self.owner_edges[owner] = old_edges;

        self.owner_invalid_total -= previous_invalid;
        self.owner_invalid_total += self.scratch_invalid_count;
        self.owner_invalid_counts[owner] = self.scratch_invalid_count;

        self.owner_violation_total -= previous_violation;
        self.owner_violation_total += self.scratch_violation_count;
        self.owner_violation_counts[owner] = self.scratch_violation_count;
    }

    fn diff_owner_assignments(&mut self, old_elements: &[NodeId], new_elements: &[NodeId]) {
        self.scratch_assignment_counts.clear();
        self.scratch_assignment_keys.clear();
        for &node in old_elements {
            if self
                .scratch_assignment_counts
                .insert(node, (1, 0))
                .is_none()
            {
                self.scratch_assignment_keys.push(node);
            } else {
                self.scratch_assignment_counts
                    .get_mut(&node)
                    .expect("just inserted")
                    .0 += 1;
            }
        }
        for &node in new_elements {
            match self.scratch_assignment_counts.get_mut(&node) {
                Some(entry) => entry.1 += 1,
                None => {
                    self.scratch_assignment_counts.insert(node, (0, 1));
                    self.scratch_assignment_keys.push(node);
                }
            }
        }

        for index in 0..self.scratch_assignment_keys.len() {
            let node = self.scratch_assignment_keys[index];
            let (old_count, new_count) = self.scratch_assignment_counts[&node];
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
        // Common prefix/suffix shrink the diff to the touched middle; the
        // middle itself is diffed by counting per edge in reusable scratch.
        let mut prefix = 0;
        let common = old_edges.len().min(new_edges.len());
        while prefix < common && old_edges[prefix] == new_edges[prefix] {
            prefix += 1;
        }
        let mut suffix = 0;
        while suffix < common - prefix
            && old_edges[old_edges.len() - 1 - suffix] == new_edges[new_edges.len() - 1 - suffix]
        {
            suffix += 1;
        }
        let old_middle = &old_edges[prefix..old_edges.len() - suffix];
        let new_middle = &new_edges[prefix..new_edges.len() - suffix];

        self.scratch_edge_keys.clear();
        for &edge in old_middle {
            match self.scratch_edge_counts.get_mut(&edge) {
                Some(entry) => entry.0 += 1,
                None => {
                    self.scratch_edge_counts.insert(edge, (1, 0));
                    self.scratch_edge_keys.push(edge);
                }
            }
        }
        for &edge in new_middle {
            match self.scratch_edge_counts.get_mut(&edge) {
                Some(entry) => entry.1 += 1,
                None => {
                    self.scratch_edge_counts.insert(edge, (0, 1));
                    self.scratch_edge_keys.push(edge);
                }
            }
        }
        for index in 0..self.scratch_edge_keys.len() {
            let edge = self.scratch_edge_keys[index];
            let (old_count, new_count) = self.scratch_edge_counts[&edge];
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
        self.scratch_edge_counts.clear();
    }
}
