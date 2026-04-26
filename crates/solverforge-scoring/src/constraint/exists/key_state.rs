use std::any::TypeId;
use std::collections::HashMap;
use std::hash::Hash;

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExistsStorageKind {
    Hashed,
    IndexedUsize,
}

pub(super) struct HashedExistsState<K> {
    a_indices_by_key: HashMap<K, Vec<usize>>,
    b_key_counts: HashMap<K, usize>,
}

impl<K> Default for HashedExistsState<K> {
    fn default() -> Self {
        Self {
            a_indices_by_key: HashMap::new(),
            b_key_counts: HashMap::new(),
        }
    }
}

#[derive(Default)]
pub(super) struct IndexedUsizeExistsState {
    a_indices_by_key: Vec<Vec<usize>>,
    b_key_counts: Vec<usize>,
}

pub(super) enum ExistsKeyState<K> {
    Hashed(HashedExistsState<K>),
    IndexedUsize(IndexedUsizeExistsState),
}

#[derive(Debug, Clone, Copy)]
pub(super) struct MovedAIndex {
    pub(super) idx: usize,
    pub(super) bucket_pos: usize,
}

impl<K> ExistsKeyState<K>
where
    K: Eq + Hash + Clone + 'static,
{
    pub(super) fn new() -> Self {
        if TypeId::of::<K>() == TypeId::of::<usize>() {
            Self::IndexedUsize(IndexedUsizeExistsState::default())
        } else {
            Self::Hashed(HashedExistsState::default())
        }
    }

    #[cfg(test)]
    pub(super) fn storage_kind(&self) -> ExistsStorageKind {
        match self {
            Self::Hashed(_) => ExistsStorageKind::Hashed,
            Self::IndexedUsize(_) => ExistsStorageKind::IndexedUsize,
        }
    }

    pub(super) fn clear_a_buckets(&mut self) {
        match self {
            Self::Hashed(state) => state.a_indices_by_key.clear(),
            Self::IndexedUsize(state) => state.a_indices_by_key.clear(),
        }
    }

    pub(super) fn clear_b_counts(&mut self) {
        match self {
            Self::Hashed(state) => state.b_key_counts.clear(),
            Self::IndexedUsize(state) => state.b_key_counts.clear(),
        }
    }

    pub(super) fn insert_a_index(&mut self, key: K, idx: usize) -> usize {
        match self {
            Self::Hashed(state) => {
                let bucket = state.a_indices_by_key.entry(key).or_default();
                let bucket_pos = bucket.len();
                bucket.push(idx);
                bucket_pos
            }
            Self::IndexedUsize(state) => {
                let key = usize_key(&key);
                if state.a_indices_by_key.len() <= key {
                    state.a_indices_by_key.resize_with(key + 1, Vec::new);
                }
                let bucket = &mut state.a_indices_by_key[key];
                let bucket_pos = bucket.len();
                bucket.push(idx);
                bucket_pos
            }
        }
    }

    pub(super) fn remove_a_index(
        &mut self,
        key: &K,
        idx: usize,
        bucket_pos: usize,
    ) -> Option<MovedAIndex> {
        match self {
            Self::Hashed(state) => {
                let mut remove_key = false;
                let mut moved = None;
                if let Some(bucket) = state.a_indices_by_key.get_mut(key) {
                    let removed = bucket.swap_remove(bucket_pos);
                    debug_assert_eq!(removed, idx);
                    if bucket_pos < bucket.len() {
                        moved = Some(MovedAIndex {
                            idx: bucket[bucket_pos],
                            bucket_pos,
                        });
                    }
                    remove_key = bucket.is_empty();
                }
                if remove_key {
                    state.a_indices_by_key.remove(key);
                }
                moved
            }
            Self::IndexedUsize(state) => {
                let key = usize_key(key);
                let bucket = state.a_indices_by_key.get_mut(key)?;
                let removed = bucket.swap_remove(bucket_pos);
                debug_assert_eq!(removed, idx);
                if bucket_pos < bucket.len() {
                    Some(MovedAIndex {
                        idx: bucket[bucket_pos],
                        bucket_pos,
                    })
                } else {
                    None
                }
            }
        }
    }

    pub(super) fn a_indices(&self, key: &K) -> Vec<usize> {
        match self {
            Self::Hashed(state) => state.a_indices_by_key.get(key).cloned().unwrap_or_default(),
            Self::IndexedUsize(state) => state
                .a_indices_by_key
                .get(usize_key(key))
                .cloned()
                .unwrap_or_default(),
        }
    }

    pub(super) fn increment_b_count(&mut self, key: &K, count: usize) {
        match self {
            Self::Hashed(state) => {
                *state.b_key_counts.entry(key.clone()).or_insert(0) += count;
            }
            Self::IndexedUsize(state) => {
                let key = usize_key(key);
                if state.b_key_counts.len() <= key {
                    state.b_key_counts.resize(key + 1, 0);
                }
                state.b_key_counts[key] += count;
            }
        }
    }

    pub(super) fn decrement_b_count(&mut self, key: &K, count: usize) {
        match self {
            Self::Hashed(state) => {
                let mut remove_key = false;
                if let Some(entry) = state.b_key_counts.get_mut(key) {
                    *entry = entry.saturating_sub(count);
                    remove_key = *entry == 0;
                }
                if remove_key {
                    state.b_key_counts.remove(key);
                }
            }
            Self::IndexedUsize(state) => {
                let key = usize_key(key);
                if let Some(entry) = state.b_key_counts.get_mut(key) {
                    *entry = entry.saturating_sub(count);
                }
            }
        }
    }

    pub(super) fn b_count(&self, key: &K) -> usize {
        match self {
            Self::Hashed(state) => state.b_key_counts.get(key).copied().unwrap_or(0),
            Self::IndexedUsize(state) => {
                state.b_key_counts.get(usize_key(key)).copied().unwrap_or(0)
            }
        }
    }
}

#[inline]
fn usize_key<K: 'static>(key: &K) -> usize {
    debug_assert_eq!(TypeId::of::<K>(), TypeId::of::<usize>());
    // SAFETY: `IndexedUsize` is only constructed by `ExistsKeyState::new()`
    // when `K` is exactly `usize`, so this cast preserves layout and alignment.
    unsafe { *(key as *const K).cast::<usize>() }
}
