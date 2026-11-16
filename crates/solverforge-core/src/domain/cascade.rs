//! Propagation of cascading list shadows across list owners.
//!
//! A cascading value depends on the element's previous element in its list
//! and on the elements it declares it depends on, which other owners' lists
//! may hold. Elements that declare each other peers form a joint group,
//! computed together: two technicians that start a visit together each wait
//! on the other's arrival. Propagation works on nodes, a node being a joint
//! group or an element of none. The graph has an edge to a node from the
//! previous element and from each dependency of every member. After list
//! changes, [`CascadePropagation::propagate`] visits the downstream cone of
//! the changed elements and recomputes it in dependency order: a node is
//! recomputed at most once, only after every input in the cone is final, and
//! only when an input value changed (or a member is a changed element), so
//! propagation stops where recomputed values equal the old ones. Elements on
//! a dependency cycle have no defined value and are unset.

mod cycles;
mod groups;

use super::PlanningSolution;

const UNVISITED: usize = usize::MAX;

/// Reusable propagation state for one solution's cascading shadow.
///
/// Holds the reverse index of the declared dependencies and the joint
/// groups, built by [`rebuild_dependencies`](Self::rebuild_dependencies) and
/// valid while the dependencies, the peers and the element count stay
/// unchanged, plus per-node scratch that every propagation leaves cleared.
#[derive(Default)]
pub struct CascadePropagation {
    // Elements that declare a dependency on each element, as compressed
    // rows: the dependents of `e` are `dependents[offsets[e]..offsets[e + 1]]`.
    offsets: Vec<usize>,
    dependents: Vec<usize>,
    // Joint groups, empty when no element declares peers. A group is the
    // node of its lowest element, `node[e]`; its members are
    // `members[member_offsets[n]..member_offsets[n + 1]]`, empty for an
    // element that is not a node.
    node: Vec<usize>,
    member_offsets: Vec<usize>,
    members: Vec<usize>,
    // Per-node scratch, cleared after each propagation.
    in_cone: Vec<bool>,
    dirty: Vec<bool>,
    done: Vec<bool>,
    cycle: Vec<bool>,
    pending_inputs: Vec<usize>,
    order: Vec<usize>,
    low: Vec<usize>,
    on_stack: Vec<bool>,
    cone: Vec<usize>,
    stack: Vec<usize>,
    ready: Vec<usize>,
    scratch: Vec<usize>,
}

impl CascadePropagation {
    /// Propagation state for `solution`, with its dependency index built.
    pub fn new<S: PlanningSolution>(solution: &S) -> Self {
        let mut propagation = Self::default();
        propagation.rebuild_dependencies(solution);
        propagation
    }

    /// Rebuilds the reverse dependency index and the joint groups from
    /// `solution` and sizes the per-node scratch to its element count.
    ///
    /// # Panics
    ///
    /// When an element declares a dependency or a peer that does not exist.
    pub fn rebuild_dependencies<S: PlanningSolution>(&mut self, solution: &S) {
        let count = solution.cascading_element_count();
        self.offsets.clear();
        self.offsets.resize(count + 1, 0);
        for element in 0..count {
            self.scratch.clear();
            solution.cascading_dependencies(element, &mut self.scratch);
            for &dependency in &self.scratch {
                assert!(
                    dependency < count,
                    "cascading element {element} depends on element {dependency}, but there are only {count} elements"
                );
                self.offsets[dependency + 1] += 1;
            }
        }
        for element in 0..count {
            self.offsets[element + 1] += self.offsets[element];
        }
        self.dependents.clear();
        self.dependents.resize(self.offsets[count], 0);
        let mut fill = self.offsets.clone();
        for element in 0..count {
            self.scratch.clear();
            solution.cascading_dependencies(element, &mut self.scratch);
            for &dependency in &self.scratch {
                self.dependents[fill[dependency]] = element;
                fill[dependency] += 1;
            }
        }
        self.scratch.clear();
        self.rebuild_groups(solution, count);
        for flags in [
            &mut self.in_cone,
            &mut self.dirty,
            &mut self.done,
            &mut self.cycle,
            &mut self.on_stack,
        ] {
            flags.clear();
            flags.resize(count, false);
        }
        self.pending_inputs.clear();
        self.pending_inputs.resize(count, 0);
        self.order.clear();
        self.order.resize(count, UNVISITED);
        self.low.clear();
        self.low.resize(count, UNVISITED);
    }

    /// Recomputes every cascading value of `solution` from scratch.
    pub fn refresh_all<S: PlanningSolution>(solution: &mut S) {
        let mut propagation = Self::new(solution);
        let count = solution.cascading_element_count();
        propagation.propagate(solution, 0..count, |_, _| {});
    }

    /// Recomputes the cascading values downstream of `changed`, the elements
    /// whose list position or list membership changed.
    ///
    /// Every member of a node holding an element of `changed` is recomputed;
    /// another node of the downstream cone is recomputed when one of its
    /// inputs changed value, or when a member is held by a list but unset (it
    /// may have left a dependency cycle). Each node is recomputed at most
    /// once, after all of its inputs in the cone, its members in ascending
    /// element order. Before an element's value is written, `before_write`
    /// is called with the solution still holding the old value and the
    /// element index.
    pub fn propagate<S, I, W>(&mut self, solution: &mut S, changed: I, before_write: W)
    where
        S: PlanningSolution,
        I: IntoIterator<Item = usize>,
        W: FnMut(&S, usize),
    {
        // A model without peers runs the element-per-node instance, which
        // compiles the group lookups away.
        if self.node.is_empty() {
            self.propagate_nodes::<false, S, I, W>(solution, changed, before_write);
        } else {
            self.propagate_nodes::<true, S, I, W>(solution, changed, before_write);
        }
    }

    fn propagate_nodes<const GROUPED: bool, S, I, W>(
        &mut self,
        solution: &mut S,
        changed: I,
        mut before_write: W,
    ) where
        S: PlanningSolution,
        I: IntoIterator<Item = usize>,
        W: FnMut(&S, usize),
    {
        for element in changed {
            let node = self.node_of::<GROUPED>(element);
            self.dirty[node] = true;
            if !self.in_cone[node] {
                self.in_cone[node] = true;
                self.cone.push(node);
                self.stack.push(node);
            }
        }
        while let Some(node) = self.stack.pop() {
            for index in 0..self.member_count::<GROUPED>(node) {
                let member = self.member::<GROUPED>(node, index);
                if let Some(next) = solution.cascading_next(member) {
                    self.reach(self.node_of::<GROUPED>(next));
                }
                for position in self.offsets[member]..self.offsets[member + 1] {
                    self.reach(self.node_of::<GROUPED>(self.dependents[position]));
                }
            }
        }

        self.ready.extend(
            self.cone
                .iter()
                .copied()
                .filter(|&node| self.pending_inputs[node] == 0),
        );
        let mut finished = 0;
        loop {
            while let Some(node) = self.ready.pop() {
                self.done[node] = true;
                finished += 1;
                let stale = self.dirty[node]
                    || (0..self.member_count::<GROUPED>(node)).any(|index| {
                        let member = self.member::<GROUPED>(node, index);
                        solution.cascading_assigned(member) && solution.cascading_unset(member)
                    });
                if stale {
                    for index in 0..self.member_count::<GROUPED>(node) {
                        let member = self.member::<GROUPED>(node, index);
                        let assigned = solution.cascading_assigned(member);
                        if solution.update_cascading_shadow(member, assigned, |solution| {
                            before_write(solution, member)
                        }) {
                            self.mark_outputs_dirty::<GROUPED, S>(solution, member);
                        }
                    }
                }
                self.release_outputs::<GROUPED, S>(solution, node);
            }
            if finished == self.cone.len() {
                break;
            }
            // The remaining nodes wait on a dependency cycle. Cycle members
            // are unset; the nodes downstream of them are then computed from
            // the unset values.
            self.mark_cycles(solution);
            for position in 0..self.cone.len() {
                let node = self.cone[position];
                if self.cycle[node] && !self.done[node] {
                    self.done[node] = true;
                    finished += 1;
                    for index in 0..self.member_count::<GROUPED>(node) {
                        let member = self.member::<GROUPED>(node, index);
                        if solution.update_cascading_shadow(member, false, |solution| {
                            before_write(solution, member)
                        }) {
                            self.mark_outputs_dirty::<GROUPED, S>(solution, member);
                        }
                    }
                }
            }
            for position in 0..self.cone.len() {
                let node = self.cone[position];
                if self.cycle[node] {
                    self.release_outputs::<GROUPED, S>(solution, node);
                }
            }
        }

        for &node in &self.cone {
            self.in_cone[node] = false;
            self.dirty[node] = false;
            self.done[node] = false;
            self.cycle[node] = false;
            self.pending_inputs[node] = 0;
            self.order[node] = UNVISITED;
            self.low[node] = UNVISITED;
        }
        self.cone.clear();
    }

    fn reach(&mut self, node: usize) {
        self.pending_inputs[node] += 1;
        if !self.in_cone[node] {
            self.in_cone[node] = true;
            self.cone.push(node);
            self.stack.push(node);
        }
    }

    fn mark_outputs_dirty<const GROUPED: bool, S: PlanningSolution>(
        &mut self,
        solution: &S,
        element: usize,
    ) {
        if let Some(next) = solution.cascading_next(element) {
            let node = self.node_of::<GROUPED>(next);
            self.dirty[node] = true;
        }
        for position in self.offsets[element]..self.offsets[element + 1] {
            let node = self.node_of::<GROUPED>(self.dependents[position]);
            self.dirty[node] = true;
        }
    }

    fn release_outputs<const GROUPED: bool, S: PlanningSolution>(
        &mut self,
        solution: &S,
        node: usize,
    ) {
        for index in 0..self.member_count::<GROUPED>(node) {
            let member = self.member::<GROUPED>(node, index);
            if let Some(next) = solution.cascading_next(member) {
                self.release(self.node_of::<GROUPED>(next));
            }
            for position in self.offsets[member]..self.offsets[member + 1] {
                self.release(self.node_of::<GROUPED>(self.dependents[position]));
            }
        }
    }

    fn release(&mut self, node: usize) {
        self.pending_inputs[node] -= 1;
        if self.pending_inputs[node] == 0 && !self.done[node] && !self.cycle[node] {
            self.ready.push(node);
        }
    }
}
