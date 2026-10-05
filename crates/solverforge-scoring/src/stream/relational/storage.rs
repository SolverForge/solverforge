/* Dense row store with stable generational handles.

Owns payloads in a dense `Vec`, resolving `RowHandle`s through a
slot-to-position indirection table. Retraction uses `swap_remove` with
repair of the moved entry's slot link, so live handles survive arbitrary
removal order. No score/key assumptions: the store is payload-agnostic and
shared by operator state of every kind.
*/

use super::identity::RowHandle;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SlotLink {
    position: u32,
    generation: u32,
}

#[derive(Clone, Debug)]
struct Entry<T> {
    payload: T,
    slot: u32,
}

/* Dense owned storage for operator rows behind stable handles.

Insert returns a fresh handle; retract returns the owned payload so
callers can reuse retained keys/scores/tokens without re-reading them.
Point lookup, full traversal, and length back the retained-state
invariant checks the operator tests run after every event.
*/
#[derive(Clone, Debug, Default)]
pub(crate) struct DenseRowStore<T> {
    entries: Vec<Entry<T>>,
    slots: Vec<SlotLink>,
    free: Vec<u32>,
}

impl<T> DenseRowStore<T> {
    pub(crate) fn new() -> DenseRowStore<T> {
        DenseRowStore {
            entries: Vec::new(),
            slots: Vec::new(),
            free: Vec::new(),
        }
    }

    /* Live row count for retained-state invariant assertions. */
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }

    #[cfg(test)]
    pub(crate) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub(crate) fn clear(&mut self) {
        self.entries.clear();
        self.slots.clear();
        self.free.clear();
    }

    fn resolve(&self, handle: RowHandle) -> Option<usize> {
        let link = self.slots.get(handle.slot() as usize)?;
        if link.generation != handle.generation() {
            return None;
        }
        let position = link.position as usize;
        if position >= self.entries.len() {
            return None;
        }
        if self.entries[position].slot != handle.slot() {
            return None;
        }
        Some(position)
    }

    pub(crate) fn insert(&mut self, payload: T) -> RowHandle {
        let position = self.entries.len() as u32;
        let slot = match self.free.pop() {
            Some(slot) => {
                let link = &mut self.slots[slot as usize];
                link.generation = link.generation.wrapping_add(1);
                link.position = position;
                slot
            }
            None => {
                let slot = self.slots.len() as u32;
                debug_assert_ne!(slot, u32::MAX, "slot space exhausted");
                self.slots.push(SlotLink {
                    position,
                    generation: 0,
                });
                slot
            }
        };
        self.entries.push(Entry { payload, slot });
        RowHandle::new(slot, self.slots[slot as usize].generation)
    }

    pub(crate) fn get(&self, handle: RowHandle) -> Option<&T> {
        self.resolve(handle)
            .map(|position| &self.entries[position].payload)
    }

    pub(crate) fn get_mut(&mut self, handle: RowHandle) -> Option<&mut T> {
        let position = self.resolve(handle)?;
        Some(&mut self.entries[position].payload)
    }

    /* Removes a row, returning its owned payload.

    Unknown or stale handles resolve to `None` and change nothing, so
    double retraction is a safe no-op rather than a panic.
    */
    pub(crate) fn retract(&mut self, handle: RowHandle) -> Option<T> {
        let position = self.resolve(handle)?;
        let last = self.entries.len() - 1;
        let removed = self.entries.swap_remove(position);
        if position != last {
            let moved_slot = self.entries[position].slot;
            self.slots[moved_slot as usize].position = position as u32;
        }
        let slot = handle.slot() as usize;
        self.slots[slot].generation = self.slots[slot].generation.wrapping_add(1);
        self.free.push(handle.slot());
        Some(removed.payload)
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = (RowHandle, &T)> + '_ {
        self.entries.iter().map(|entry| {
            let slot = entry.slot as usize;
            (
                RowHandle::new(entry.slot, self.slots[slot].generation),
                &entry.payload,
            )
        })
    }
}
