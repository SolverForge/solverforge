//! Stream rows of list-shadowed elements.
//!
//! An owner's list shadow update rewrites fields of the elements it holds, so
//! those writes are mutations the constraint streams must see. Every element row
//! is retracted while it still holds the shadows it was inserted with, and
//! inserted again once its shadows are final:
//!
//! - before an owner's list changes, the rows of the elements it holds are
//!   retracted and marked pending;
//! - after the change, the rows of elements that arrived from no list are
//!   retracted, the shadows are rewritten, and every element the owner now
//!   holds is inserted;
//! - an element that left this owner and is still pending is held by no list
//!   yet: its shadows are cleared and its row inserted. If another owner picks it
//!   up afterwards, that owner retracts the cleared row like any arrival.
//!
//! Pending marks are reused across moves, so tracking allocates only while the
//! element count grows.
//!
//! A solution with a cascading shadow (`PlanningSolution::HAS_CASCADING_SHADOWS`)
//! rescores only the elements whose shadows change, and defers their rows to
//! the end of the move. Its element rows read nothing but the list structure
//! (owner, previous, next and, when declared, index) and the cascading value,
//! so an element keeps its row while those stay equal:
//!
//! - before an owner's list changes, the elements it holds are recorded, and
//!   no row is retracted;
//! - after the change, an element whose structure shadows differ from its new
//!   place (`PlanningSolution::list_structure_unchanged`, read before the
//!   structure is rewritten), including an arrival from no list, is retracted
//!   while it still holds its old shadows, then the structure is rewritten;
//! - once every owner the move notified has been refreshed, an element that
//!   was held and is now in no list is retracted and cleared, the cascading
//!   values are propagated from the retracted elements, each element whose
//!   value changes is retracted before its write, and every retracted
//!   element is inserted exactly once.
//!
//! Solutions without a cascading shadow never reach that code: the branch is
//! on an associated constant.

use solverforge_core::domain::{CascadePropagation, PlanningSolution};
use solverforge_core::score::Score;

use super::weight_mode::RowDeltas;

#[derive(Clone, Copy)]
struct PendingElement {
    element: usize,
    source: usize,
    owner_descriptor: usize,
    owner_index: usize,
}

#[derive(Default)]
pub(super) struct ListShadowRows {
    pending: Vec<PendingElement>,
    is_pending: Vec<bool>,
    // Present only for solutions with a cascading shadow, from their first
    // notified change; boxed so other solutions pay one pointer.
    cascade: Option<Box<CascadeRows>>,
}

struct CascadeRows {
    propagation: CascadePropagation,
    // Owner notifications of the current move still waiting for their
    // `after` half.
    open_changes: usize,
    // The change source of element rows.
    source: usize,
    // Elements a refreshed owner holds after the move, and the list of them.
    held: Vec<bool>,
    held_elements: Vec<usize>,
    // Elements the notified owners held before the move.
    recorded: Vec<usize>,
    // The elements propagation starts from, reused across moves.
    seeds: Vec<usize>,
}

impl ListShadowRows {
    pub(super) fn clear(&mut self) {
        self.pending.clear();
        self.is_pending.fill(false);
        // The dependency index is rebuilt from the next solution.
        self.cascade = None;
    }

    fn mark(&mut self, element: usize, pending: bool) {
        if element >= self.is_pending.len() {
            self.is_pending.resize(element + 1, false);
        }
        self.is_pending[element] = pending;
    }

    fn pending(&self, element: usize) -> bool {
        self.is_pending.get(element).copied().unwrap_or(false)
    }

    pub(super) fn retract_held<S, C>(
        &mut self,
        solution: &S,
        constraints: &mut C,
        owner_descriptor: usize,
        owner_index: usize,
    ) -> S::Score
    where
        S: PlanningSolution,
        C: RowDeltas<S, S::Score>,
    {
        let mut delta = S::Score::zero();
        let Some((source, elements)) = solution.list_shadow_elements(owner_descriptor, owner_index)
        else {
            return delta;
        };
        for &element in elements {
            if self.pending(element) {
                continue;
            }
            delta = delta + constraints.retract_row(solution, element, source);
            self.mark(element, true);
            self.pending.push(PendingElement {
                element,
                source,
                owner_descriptor,
                owner_index,
            });
        }
        delta
    }

    /// Rewrites the owner's shadows between retracting its arrivals and
    /// inserting everything it holds, and inserts the owner's own row too, so
    /// the owner observes its refreshed aggregates.
    pub(super) fn refresh_owner<S, C>(
        &mut self,
        solution: &mut S,
        constraints: &mut C,
        owner_descriptor: usize,
        owner_index: usize,
    ) -> S::Score
    where
        S: PlanningSolution,
        C: RowDeltas<S, S::Score>,
    {
        let mut delta = S::Score::zero();
        if let Some((source, elements)) =
            solution.list_shadow_elements(owner_descriptor, owner_index)
        {
            for &element in elements {
                if !self.pending(element) {
                    delta = delta + constraints.retract_row(solution, element, source);
                }
            }
        }
        solution.update_entity_shadows(owner_descriptor, owner_index);
        delta = delta + constraints.insert_row(solution, owner_index, owner_descriptor);
        if let Some((source, elements)) =
            solution.list_shadow_elements(owner_descriptor, owner_index)
        {
            for &element in elements {
                delta = delta + constraints.insert_row(solution, element, source);
                self.mark(element, false);
            }
        }
        let mut position = 0;
        while position < self.pending.len() {
            let entry = self.pending[position];
            if entry.owner_descriptor != owner_descriptor || entry.owner_index != owner_index {
                position += 1;
                continue;
            }
            self.pending.swap_remove(position);
            if self.pending(entry.element) {
                solution.clear_list_shadows(entry.element);
                delta = delta + constraints.insert_row(solution, entry.element, entry.source);
                self.mark(entry.element, false);
            }
        }
        delta
    }

    /// Opens one owner notification of a cascading move and records the
    /// elements the owner holds before it changes. No row is retracted yet:
    /// only the elements whose structure the change rewrites are.
    pub(super) fn begin_cascading_change<S: PlanningSolution>(
        &mut self,
        solution: &S,
        owner_descriptor: usize,
        owner_index: usize,
    ) {
        let cascade = self.cascade.get_or_insert_with(|| {
            let count = solution.cascading_element_count();
            Box::new(CascadeRows {
                propagation: CascadePropagation::new(solution),
                open_changes: 0,
                source: 0,
                held: vec![false; count],
                held_elements: Vec::new(),
                recorded: Vec::new(),
                seeds: Vec::new(),
            })
        });
        cascade.open_changes += 1;
        let Some((source, elements)) = solution.list_shadow_elements(owner_descriptor, owner_index)
        else {
            return;
        };
        cascade.source = source;
        cascade.recorded.extend_from_slice(elements);
    }

    /// Retracts the rows of the owner's elements whose structure changes
    /// (moved, re-linked, or arrived from no list), refreshes the owner's
    /// list structure for a cascading move and inserts the owner's row;
    /// element rows stay retracted until the move's last owner is refreshed,
    /// when the cascading values are propagated and every pending element is
    /// inserted once.
    pub(super) fn refresh_cascading_owner<S, C>(
        &mut self,
        solution: &mut S,
        constraints: &mut C,
        owner_descriptor: usize,
        owner_index: usize,
    ) -> S::Score
    where
        S: PlanningSolution,
        C: RowDeltas<S, S::Score>,
    {
        let mut delta = S::Score::zero();
        if let Some((source, elements)) =
            solution.list_shadow_elements(owner_descriptor, owner_index)
        {
            let cascade = self
                .cascade
                .as_mut()
                .expect("a cascading change was opened");
            cascade.source = source;
            for (index, &element) in elements.iter().enumerate() {
                if !cascade.held[element] {
                    cascade.held[element] = true;
                    cascade.held_elements.push(element);
                }
                if self.is_pending.get(element).copied().unwrap_or(false)
                    || solution.list_structure_unchanged(
                        element,
                        owner_descriptor,
                        owner_index,
                        index,
                        index.checked_sub(1).map(|previous| elements[previous]),
                        elements.get(index + 1).copied(),
                    )
                {
                    continue;
                }
                // The structure is rewritten below: an element that is not
                // pending still holds the structure its row was inserted with.
                delta = delta + constraints.retract_row(solution, element, source);
                if element >= self.is_pending.len() {
                    self.is_pending.resize(element + 1, false);
                }
                self.is_pending[element] = true;
                self.pending.push(PendingElement {
                    element,
                    source,
                    owner_descriptor,
                    owner_index,
                });
            }
        }
        solution.update_list_structure_shadows(owner_descriptor, owner_index);
        delta = delta + constraints.insert_row(solution, owner_index, owner_descriptor);
        let cascade = self
            .cascade
            .as_mut()
            .expect("a cascading change was opened");
        cascade.open_changes -= 1;
        if cascade.open_changes == 0 {
            delta = delta + self.finish_cascading_move(solution, constraints);
        }
        delta
    }

    fn finish_cascading_move<S, C>(&mut self, solution: &mut S, constraints: &mut C) -> S::Score
    where
        S: PlanningSolution,
        C: RowDeltas<S, S::Score>,
    {
        let mut delta = S::Score::zero();
        let cascade = self
            .cascade
            .as_mut()
            .expect("a cascading change was opened");
        let CascadeRows {
            propagation,
            source,
            held,
            held_elements,
            recorded,
            seeds,
            ..
        } = &mut **cascade;
        let source = *source;
        // Elements that left their owner and joined no refreshed owner are in
        // no list: their rows still hold their old structure.
        for &element in recorded.iter() {
            if !held[element] && !self.is_pending.get(element).copied().unwrap_or(false) {
                delta = delta + constraints.retract_row(solution, element, source);
                if element >= self.is_pending.len() {
                    self.is_pending.resize(element + 1, false);
                }
                self.is_pending[element] = true;
                self.pending.push(PendingElement {
                    element,
                    source,
                    owner_descriptor: usize::MAX,
                    owner_index: usize::MAX,
                });
            }
        }
        recorded.clear();
        for entry in &self.pending {
            if !held[entry.element] {
                solution.clear_list_shadows(entry.element);
            }
        }
        seeds.clear();
        seeds.extend(self.pending.iter().map(|entry| entry.element));
        let pending = &mut self.pending;
        let is_pending = &mut self.is_pending;
        propagation.propagate(solution, seeds.iter().copied(), |solution, element| {
            if !is_pending.get(element).copied().unwrap_or(false) {
                delta = delta + constraints.retract_row(solution, element, source);
                if element >= is_pending.len() {
                    is_pending.resize(element + 1, false);
                }
                is_pending[element] = true;
                pending.push(PendingElement {
                    element,
                    source,
                    owner_descriptor: usize::MAX,
                    owner_index: usize::MAX,
                });
            }
        });
        for entry in self.pending.drain(..) {
            delta = delta + constraints.insert_row(solution, entry.element, entry.source);
            self.is_pending[entry.element] = false;
        }
        for &element in held_elements.iter() {
            held[element] = false;
        }
        held_elements.clear();
        delta
    }
}
