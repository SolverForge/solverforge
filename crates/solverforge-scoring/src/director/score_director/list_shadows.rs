/* Stream rows of list-shadowed elements.

An owner's list shadow update rewrites fields of the elements it holds, so
those writes are mutations the constraint streams must see. Every element row
is retracted while it still holds the shadows it was inserted with, and
inserted again once its shadows are final:

- before an owner's list changes, the rows of the elements it holds are
  retracted and marked pending;
- after the change, the rows of elements that arrived from no list are
  retracted, the shadows are rewritten, and every element the owner now
  holds is inserted;
- an element that left this owner and is still pending is held by no list
  yet: its shadows are cleared and its row inserted. If another owner picks it
  up afterwards, that owner retracts the cleared row like any arrival.

Pending marks are reused across moves, so tracking allocates only while the
element count grows.
*/

use solverforge_core::domain::PlanningSolution;
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
}

impl ListShadowRows {
    pub(super) fn clear(&mut self) {
        self.pending.clear();
        self.is_pending.fill(false);
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

    /* Rewrites the owner's shadows between retracting its arrivals and
    inserting everything it holds, and inserts the owner's own row too, so
    the owner observes its refreshed aggregates.
    */
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
}
