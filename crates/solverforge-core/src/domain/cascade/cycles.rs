//! Dependency cycles among the nodes of a stalled propagation.

use super::{CascadePropagation, UNVISITED};
use crate::domain::PlanningSolution;

impl CascadePropagation {
    /// Marks the members of every dependency cycle among the unfinished
    /// nodes of the cone (strongly connected components with more than one
    /// node, or a node that depends on itself). Runs only when propagation
    /// stalls on a cycle.
    #[cold]
    pub(super) fn mark_cycles<S: PlanningSolution>(&mut self, solution: &S) {
        let mut next_order = 0;
        let mut components = Vec::new();
        // Each frame holds a node and its unfinished outputs still to visit.
        let mut frames: Vec<(usize, Vec<usize>)> = Vec::new();
        for index in 0..self.cone.len() {
            let root = self.cone[index];
            if self.done[root] || self.order[root] != UNVISITED {
                continue;
            }
            self.enter(
                solution,
                root,
                &mut next_order,
                &mut components,
                &mut frames,
            );
            while let Some((node, outputs)) = frames.last_mut() {
                let node = *node;
                if let Some(output) = outputs.pop() {
                    if self.order[output] == UNVISITED {
                        self.enter(
                            solution,
                            output,
                            &mut next_order,
                            &mut components,
                            &mut frames,
                        );
                    } else if self.on_stack[output] {
                        self.low[node] = self.low[node].min(self.order[output]);
                    }
                    continue;
                }
                frames.pop();
                if let Some((parent, _)) = frames.last() {
                    self.low[*parent] = self.low[*parent].min(self.low[node]);
                }
                if self.low[node] == self.order[node] {
                    let start = components
                        .iter()
                        .rposition(|&member| member == node)
                        .expect("component root is on the stack");
                    let self_loop = self.outputs(solution, node).contains(&node);
                    let is_cycle = components.len() - start > 1 || self_loop;
                    for member in components.drain(start..) {
                        self.on_stack[member] = false;
                        self.cycle[member] = is_cycle;
                    }
                }
            }
        }
    }

    fn enter<S: PlanningSolution>(
        &mut self,
        solution: &S,
        node: usize,
        next_order: &mut usize,
        components: &mut Vec<usize>,
        frames: &mut Vec<(usize, Vec<usize>)>,
    ) {
        self.order[node] = *next_order;
        self.low[node] = *next_order;
        *next_order += 1;
        self.on_stack[node] = true;
        components.push(node);
        let mut outputs = self.outputs(solution, node);
        outputs.retain(|&output| self.in_cone[output] && !self.done[output]);
        frames.push((node, outputs));
    }

    /// The nodes that the next elements and the dependents of `node`'s
    /// members belong to.
    fn outputs<S: PlanningSolution>(&self, solution: &S, node: usize) -> Vec<usize> {
        if self.node.is_empty() {
            self.outputs_of::<false, S>(solution, node)
        } else {
            self.outputs_of::<true, S>(solution, node)
        }
    }

    fn outputs_of<const GROUPED: bool, S: PlanningSolution>(
        &self,
        solution: &S,
        node: usize,
    ) -> Vec<usize> {
        let mut outputs = Vec::new();
        for index in 0..self.member_count::<GROUPED>(node) {
            let member = self.member::<GROUPED>(node, index);
            if let Some(next) = solution.cascading_next(member) {
                outputs.push(self.node_of::<GROUPED>(next));
            }
            outputs.extend(
                self.dependents[self.offsets[member]..self.offsets[member + 1]]
                    .iter()
                    .map(|&dependent| self.node_of::<GROUPED>(dependent)),
            );
        }
        outputs
    }
}
