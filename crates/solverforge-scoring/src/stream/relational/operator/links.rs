use crate::stream::relational::{HandleMap, RowHandle};

/// Output links of each input row, kept in place across notifications.
///
/// A row that leaves and re-enters a node on every move would otherwise drop
/// its bucket on retraction and allocate a new one on insertion. Emptied
/// buckets stay in their slot instead, and a later handle of the same slot
/// takes the bucket over, so steady-state notification allocates nothing and
/// holds no memory beyond one bucket per slot.
pub(super) struct Links<T> {
    buckets: HandleMap<Vec<T>>,
}

impl<T> Links<T> {
    pub(super) fn new() -> Self {
        Self {
            buckets: HandleMap::new(),
        }
    }

    pub(super) fn clear(&mut self) {
        self.buckets.clear();
    }

    pub(super) fn get(&self, input: RowHandle) -> Option<&Vec<T>> {
        self.buckets.get(input)
    }

    pub(super) fn get_mut(&mut self, input: RowHandle) -> Option<&mut Vec<T>> {
        self.buckets.get_mut(input)
    }

    /// The bucket of `input`, taking over the bucket a stale handle of the
    /// same slot left behind.
    #[inline]
    pub(super) fn bucket(&mut self, input: RowHandle) -> &mut Vec<T> {
        self.buckets.get_or_reuse(input, Vec::new, Vec::clear)
    }

    /// Detaches the links of `input`, leaving its slot empty until
    /// [`Links::restore`] hands the bucket back.
    #[inline]
    pub(super) fn take(&mut self, input: RowHandle) -> Option<Vec<T>> {
        self.buckets.get_mut(input).map(std::mem::take)
    }

    /// Puts a detached bucket back into the slot of `input`, emptied.
    #[inline]
    pub(super) fn restore(&mut self, input: RowHandle, mut bucket: Vec<T>) {
        bucket.clear();
        if let Some(slot) = self.buckets.get_mut(input) {
            *slot = bucket;
        }
    }
}

impl Links<RowHandle> {
    /// Removes `output` from the bucket of `input` at `position` by swap
    /// removal and returns the output moved into that position. A detached
    /// bucket has nothing to remove.
    #[inline]
    pub(super) fn remove_link(
        &mut self,
        input: RowHandle,
        output: RowHandle,
        position: usize,
    ) -> Option<RowHandle> {
        let bucket = self.buckets.get_mut(input)?;
        if position >= bucket.len() {
            return None;
        }
        debug_assert_eq!(bucket[position], output);
        bucket.swap_remove(position);
        bucket.get(position).copied()
    }
}
