use crate::stream::relational::{HandleMap, RowHandle};

/// Output links of each input row, with a pool of emptied buckets.
///
/// A row that leaves and re-enters a node on every move would otherwise drop
/// its bucket on retraction and allocate a new one on insertion; the pool
/// keeps those buckets, so steady-state notification allocates nothing.
pub(super) struct Links<T> {
    buckets: HandleMap<Vec<T>>,
    retired: Vec<Vec<T>>,
}

impl<T> Links<T> {
    pub(super) fn new() -> Self {
        Self {
            buckets: HandleMap::new(),
            retired: Vec::new(),
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

    /// The bucket of `input`, taken from the pool when it has none.
    #[inline]
    pub(super) fn bucket(&mut self, input: RowHandle) -> &mut Vec<T> {
        let Self { buckets, retired } = self;
        buckets.get_or_insert_with(input, || retired.pop().unwrap_or_default())
    }

    /// Detaches the bucket of `input`; hand it back with [`Links::retire`].
    #[inline]
    pub(super) fn take(&mut self, input: RowHandle) -> Option<Vec<T>> {
        self.buckets.remove(input)
    }

    /// Returns an emptied bucket to the pool.
    #[inline]
    pub(super) fn retire(&mut self, mut bucket: Vec<T>) {
        bucket.clear();
        self.retired.push(bucket);
    }
}

impl Links<RowHandle> {
    /// Removes `output` from the bucket of `input` at `position` by swap
    /// removal and returns the output moved into that position. An emptied
    /// bucket goes back to the pool.
    #[inline]
    pub(super) fn remove_link(
        &mut self,
        input: RowHandle,
        output: RowHandle,
        position: usize,
    ) -> Option<RowHandle> {
        let bucket = self.buckets.get_mut(input)?;
        debug_assert_eq!(bucket[position], output);
        bucket.swap_remove(position);
        let moved = bucket.get(position).copied();
        if bucket.is_empty() {
            if let Some(bucket) = self.buckets.remove(input) {
                self.retired.push(bucket);
            }
        }
        moved
    }
}
