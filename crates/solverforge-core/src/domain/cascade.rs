//! Propagation of cascading list shadows across list owners.
//!
//! A cascading value depends on the element's previous element in its list
//! and on the elements it declares it depends on, which other owners' lists
//! may hold. The dependency graph has an edge from an element's previous
//! element and from each of its dependencies to the element. After list
//! changes, [`CascadePropagation::propagate`] visits the downstream cone of
//! the changed elements and recomputes it in dependency order: an element is
//! recomputed at most once, only after every input in the cone is final, and
//! only when an input value changed (or it is a changed element itself), so
//! propagation stops where a recomputed value equals the old one. Elements
//! on a dependency cycle have no defined value and are unset.

use super::PlanningSolution;

const UNVISITED: usize = usize::MAX;

/// Reusable propagation state for one solution's cascading shadow.
///
/// Holds the reverse index of the declared dependencies, built by
/// [`rebuild_dependencies`](Self::rebuild_dependencies) and valid while the
/// dependencies and the element count stay unchanged, plus per-element
/// scratch that every propagation leaves cleared.
#[derive(Default)]
pub struct CascadePropagation {
    // Elements that declare a dependency on each element, as compressed
    // rows: the dependents of `e` are `dependents[offsets[e]..offsets[e + 1]]`.
    offsets: Vec<usize>,
    dependents: Vec<usize>,
    // Per-element scratch, cleared after each propagation.
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

    /// Rebuilds the reverse dependency index from `solution` and sizes the
    /// per-element scratch to its element count.
    ///
    /// # Panics
    ///
    /// When an element declares a dependency on an element index that does
    /// not exist.
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
    /// Every element of `changed` is recomputed; another element of the
    /// downstream cone is recomputed when one of its inputs changed value, or
    /// when it is held by a list but unset (it may have left a dependency
    /// cycle). Each element is recomputed at most once, after all of its
    /// inputs in the cone. Before an element's value is written,
    /// `before_write` is called with the solution still holding the old value
    /// and the element index.
    pub fn propagate<S, I, W>(&mut self, solution: &mut S, changed: I, mut before_write: W)
    where
        S: PlanningSolution,
        I: IntoIterator<Item = usize>,
        W: FnMut(&S, usize),
    {
        for element in changed {
            self.dirty[element] = true;
            if !self.in_cone[element] {
                self.in_cone[element] = true;
                self.cone.push(element);
                self.stack.push(element);
            }
        }
        while let Some(element) = self.stack.pop() {
            if let Some(next) = solution.cascading_next(element) {
                self.reach(next);
            }
            for position in self.offsets[element]..self.offsets[element + 1] {
                self.reach(self.dependents[position]);
            }
        }

        self.ready.extend(
            self.cone
                .iter()
                .copied()
                .filter(|&element| self.pending_inputs[element] == 0),
        );
        let mut finished = 0;
        loop {
            while let Some(element) = self.ready.pop() {
                self.done[element] = true;
                finished += 1;
                let assigned = solution.cascading_assigned(element);
                if (self.dirty[element] || (assigned && solution.cascading_unset(element)))
                    && solution.update_cascading_shadow(element, assigned, |solution| {
                        before_write(solution, element)
                    })
                {
                    self.mark_outputs_dirty(solution, element);
                }
                self.release_outputs(solution, element);
            }
            if finished == self.cone.len() {
                break;
            }
            // The remaining elements wait on a dependency cycle. Cycle
            // members are unset; the elements downstream of them are then
            // computed from the unset values.
            self.mark_cycles(solution);
            for index in 0..self.cone.len() {
                let element = self.cone[index];
                if self.cycle[element] && !self.done[element] {
                    self.done[element] = true;
                    finished += 1;
                    if solution.update_cascading_shadow(element, false, |solution| {
                        before_write(solution, element)
                    }) {
                        self.mark_outputs_dirty(solution, element);
                    }
                }
            }
            for index in 0..self.cone.len() {
                let element = self.cone[index];
                if self.cycle[element] {
                    self.release_outputs(solution, element);
                }
            }
        }

        for &element in &self.cone {
            self.in_cone[element] = false;
            self.dirty[element] = false;
            self.done[element] = false;
            self.cycle[element] = false;
            self.pending_inputs[element] = 0;
            self.order[element] = UNVISITED;
            self.low[element] = UNVISITED;
        }
        self.cone.clear();
    }

    fn reach(&mut self, element: usize) {
        self.pending_inputs[element] += 1;
        if !self.in_cone[element] {
            self.in_cone[element] = true;
            self.cone.push(element);
            self.stack.push(element);
        }
    }

    fn mark_outputs_dirty<S: PlanningSolution>(&mut self, solution: &S, element: usize) {
        if let Some(next) = solution.cascading_next(element) {
            self.dirty[next] = true;
        }
        for position in self.offsets[element]..self.offsets[element + 1] {
            self.dirty[self.dependents[position]] = true;
        }
    }

    fn release_outputs<S: PlanningSolution>(&mut self, solution: &S, element: usize) {
        if let Some(next) = solution.cascading_next(element) {
            self.release(next);
        }
        for position in self.offsets[element]..self.offsets[element + 1] {
            self.release(self.dependents[position]);
        }
    }

    fn release(&mut self, element: usize) {
        self.pending_inputs[element] -= 1;
        if self.pending_inputs[element] == 0 && !self.done[element] && !self.cycle[element] {
            self.ready.push(element);
        }
    }

    /// Marks the members of every dependency cycle among the unfinished
    /// elements of the cone (strongly connected components with more than
    /// one element, or an element that depends on itself). Runs only when
    /// propagation stalls on a cycle.
    #[cold]
    fn mark_cycles<S: PlanningSolution>(&mut self, solution: &S) {
        let mut next_order = 0;
        let mut components = Vec::new();
        // Each frame holds an element and its unfinished outputs still to visit.
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
            while let Some((element, outputs)) = frames.last_mut() {
                let element = *element;
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
                        self.low[element] = self.low[element].min(self.order[output]);
                    }
                    continue;
                }
                frames.pop();
                if let Some((parent, _)) = frames.last() {
                    self.low[*parent] = self.low[*parent].min(self.low[element]);
                }
                if self.low[element] == self.order[element] {
                    let start = components
                        .iter()
                        .rposition(|&member| member == element)
                        .expect("component root is on the stack");
                    let self_loop = self.depends_on_itself(solution, element);
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
        element: usize,
        next_order: &mut usize,
        components: &mut Vec<usize>,
        frames: &mut Vec<(usize, Vec<usize>)>,
    ) {
        self.order[element] = *next_order;
        self.low[element] = *next_order;
        *next_order += 1;
        self.on_stack[element] = true;
        components.push(element);
        let mut outputs = Vec::new();
        if let Some(next) = solution.cascading_next(element) {
            outputs.push(next);
        }
        outputs
            .extend_from_slice(&self.dependents[self.offsets[element]..self.offsets[element + 1]]);
        outputs.retain(|&output| self.in_cone[output] && !self.done[output]);
        frames.push((element, outputs));
    }

    fn depends_on_itself<S: PlanningSolution>(&self, solution: &S, element: usize) -> bool {
        solution.cascading_next(element) == Some(element)
            || self.dependents[self.offsets[element]..self.offsets[element + 1]].contains(&element)
    }
}
