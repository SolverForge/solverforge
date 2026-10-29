use super::*;

impl ListPrecedenceState {
    pub(super) fn refresh_score_full(&mut self) -> HardSoftScore {
        self.rebuild_graph_summary();
        self.refresh_score_from_cached_graph()
    }

    pub(super) fn refresh_score_after_route_change(
        &mut self,
        change: &RouteChange,
    ) -> HardSoftScore {
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

    pub(super) fn refresh_graph_after_route_change(
        &mut self,
        change: &RouteChange,
    ) -> GraphRefreshKind {
        if change.is_empty() && self.deferred_seeds.is_empty() {
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

        let node_count = self.node_count;
        if self.scratch_queued.len() < node_count {
            self.scratch_queued.resize(node_count, 0);
        }
        self.scratch_queue.clear();
        for node in change.seeds() {
            if node < node_count && self.scratch_queued[node] != self.scratch_visited_stamp {
                self.scratch_queued[node] = self.scratch_visited_stamp;
                self.scratch_queue.push(node);
            }
        }
        for node in self.deferred_seeds.iter().copied() {
            if self.scratch_queued[node] != self.scratch_visited_stamp {
                self.scratch_queued[node] = self.scratch_visited_stamp;
                self.scratch_queue.push(node);
            }
        }
        self.deferred_seeds.clear();

        let mut head = 0usize;
        let mut visited = 0;
        while head < self.scratch_queue.len() {
            let node = self.scratch_queue[head];
            head += 1;
            // Mirror the former VecDeque semantics exactly: pop releases the
            // queued marker so the node can be re-queued and re-processed if
            // a later update in the same refresh shifts its earliest again.
            self.scratch_queued[node] = 0;
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
                if successor < node_count
                    && self.scratch_queued[successor] != self.scratch_visited_stamp
                {
                    self.scratch_queued[successor] = self.scratch_visited_stamp;
                    self.scratch_queue.push(successor);
                }
            }
        }
        self.scratch_visited_stamp = self.scratch_visited_stamp.wrapping_add(1);
        if self.scratch_visited_stamp == 0 {
            self.scratch_queued.iter_mut().for_each(|stamp| *stamp = 0);
            self.scratch_visited_stamp = 1;
        }
        self.scratch_queue_head = 0;
        GraphRefreshKind::Incremental { visited }
    }

    fn rebuild_graph_summary(&mut self) {
        self.clear_deferred_seeds();
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
        let mut max_finish_nodes = 0usize;
        while let Some(node) = ready.pop_front() {
            processed += 1;
            let finish = earliest[node].saturating_add(self.durations[node]);
            finishes[node] = finish;
            if finish > makespan {
                makespan = finish;
                max_finish_nodes = 1;
            } else if finish == makespan {
                max_finish_nodes += 1;
            }
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
            self.max_finish_nodes = max_finish_nodes;
        }
    }

    fn mark_cyclic_from_route_change(&mut self, change: &RouteChange) {
        self.clear_deferred_seeds();
        self.cycle_added_edges = change.added_edges.clone();
        self.cycle_penalty = self.node_count;
        self.makespan = 0;
        self.max_finish_nodes = self.node_count;
    }

    fn mark_cyclic_without_cache(&mut self) {
        self.clear_deferred_seeds();
        self.earliest.fill(0);
        self.finishes.fill(0);
        self.cycle_added_edges.clear();
        self.cycle_penalty = self.node_count;
        self.makespan = 0;
        self.max_finish_nodes = self.node_count;
    }

    fn recover_cached_cycle(&mut self, change: &RouteChange) -> bool {
        self.clear_deferred_seeds();
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
        self.recount_max_finish();
        true
    }

    fn replace_earliest(&mut self, node: NodeId, new_earliest: i64) {
        let old_finish = self.finishes[node];
        self.earliest[node] = new_earliest;
        let new_finish = new_earliest.saturating_add(self.durations[node]);
        self.finishes[node] = new_finish;
        if new_finish > self.makespan {
            self.makespan = new_finish;
            self.max_finish_nodes = 1;
        } else if new_finish == self.makespan {
            if old_finish != self.makespan {
                self.max_finish_nodes += 1;
            }
        } else if old_finish == self.makespan {
            self.max_finish_nodes -= 1;
            if self.max_finish_nodes == 0 {
                self.recount_max_finish();
            }
        }
    }

    /// Full O(N) recount; only runs when the last max-finish node drops
    /// (rare: the makespan must strictly shrink to zero achievers) and
    /// after bulk graph rebuilds.
    /// All non-incremental refresh paths rescan the full graph, so the
    /// deferred-retract seeds carry no information once one of them runs.
    pub(super) fn clear_deferred_seeds(&mut self) {
        self.deferred_seeds.clear();
    }

    fn recount_max_finish(&mut self) {
        let mut max = 0i64;
        let mut count = 0usize;
        for &finish in &self.finishes {
            if finish > max {
                max = finish;
                count = 1;
            } else if finish == max {
                count += 1;
            }
        }
        self.makespan = max;
        self.max_finish_nodes = count;
    }

    fn added_edges_introduce_cycle(&mut self, added_edges: &[Edge]) -> bool {
        if added_edges.is_empty() {
            return false;
        }

        // Probe visits share NO stamps with the BFS refresh mask: they use
        // their own mask + counter so a probe can never suppress a BFS push
        // (and vice versa) across events.
        if self.probe_queued.len() < self.node_count {
            self.probe_queued.resize(self.node_count, 0);
        }
        // One fresh stamp per probe: a probe's visited marks never leak into
        // the next probe, and stale stamps from previous refreshes read as
        // unvisited because the counter only moves forward (the BFS mask
        // follows the same scheme; no O(N) clearing per refresh).
        for &(from, to) in added_edges {
            self.probe_visited_stamp = self.probe_visited_stamp.wrapping_add(1);
            if self.probe_visited_stamp == 0 {
                self.probe_queued.iter_mut().for_each(|stamp| *stamp = 0);
                self.probe_visited_stamp = 1;
            }
            let visit_stamp = self.probe_visited_stamp;
            if self.reaches_with_scratch(to, from, visit_stamp) {
                return true;
            }
        }
        false
    }

    // DFS probe over successors with a generation-stamped visit mask; the
    // stack lives in scratch storage so repeated probes never reallocate.
    // Stamps must stay < u32::MAX probes per refresh call; the caller resets
    // scratch_queued whenever scratch_visited_stamp wraps to keep stamps unique.
    fn reaches_with_scratch(&mut self, start: NodeId, target: NodeId, visit_id: usize) -> bool {
        if start == target {
            return true;
        }
        self.scratch_probe_stack.clear();
        self.scratch_probe_stack.push(start);
        while let Some(node) = self.scratch_probe_stack.pop() {
            if self.probe_queued[node] == visit_id {
                continue;
            }
            self.probe_queued[node] = visit_id;
            for &successor in &self.successors[node] {
                if successor == target {
                    return true;
                }
                if self.probe_queued[successor] != visit_id {
                    self.scratch_probe_stack.push(successor);
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
