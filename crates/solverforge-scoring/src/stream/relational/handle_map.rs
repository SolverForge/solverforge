use super::RowHandle;

/// Direct side-table for handles issued by one operator's slot allocator.
/// Keys are trusted allocator slots, not arbitrary user hash keys.
#[derive(Clone, Debug)]
pub(crate) struct HandleMap<T> {
    slots: Vec<Option<(u32, T)>>,
}
impl<T> HandleMap<T> {
    pub(crate) fn new() -> Self {
        Self { slots: Vec::new() }
    }
    pub(crate) fn clear(&mut self) {
        self.slots.clear();
    }
    pub(crate) fn get(&self, h: RowHandle) -> Option<&T> {
        self.slots
            .get(h.slot() as usize)?
            .as_ref()
            .and_then(|(g, v)| (*g == h.generation()).then_some(v))
    }
    pub(crate) fn get_mut(&mut self, h: RowHandle) -> Option<&mut T> {
        self.slots
            .get_mut(h.slot() as usize)?
            .as_mut()
            .and_then(|(g, v)| (*g == h.generation()).then_some(v))
    }
    pub(crate) fn insert(&mut self, h: RowHandle, value: T) -> Option<T> {
        let slot = h.slot() as usize;
        if slot >= self.slots.len() {
            self.slots.resize_with(slot + 1, || None);
        }
        self.slots[slot]
            .replace((h.generation(), value))
            .and_then(|(g, v)| (g == h.generation()).then_some(v))
    }
    pub(crate) fn remove(&mut self, h: RowHandle) -> Option<T> {
        let slot = self.slots.get_mut(h.slot() as usize)?;
        if slot.as_ref().is_some_and(|(g, _)| *g == h.generation()) {
            slot.take().map(|(_, v)| v)
        } else {
            None
        }
    }
    pub(crate) fn get_or_insert_with(&mut self, h: RowHandle, make: impl FnOnce() -> T) -> &mut T {
        if self.get(h).is_none() {
            self.insert(h, make());
        }
        self.get_mut(h).expect("inserted side-table entry")
    }
}
