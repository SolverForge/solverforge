/* Typed hash index for one join side's equality relationship.

Each join owns its own key type `K`: indexes are never shared across
joins, so heterogeneous successive keys stay in separate domains. The
index maps keys to stable input handles and retains the reverse
handle-to-old-key link, so retraction removes the exact bucket entry
without re-deriving keys from mutated values.
*/

use std::hash::Hash;

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::hash::{Hash, Hasher};

    #[derive(Clone)]
    struct Key<'a>(u32, &'a Cell<usize>);
    impl PartialEq for Key<'_> {
        fn eq(&self, other: &Self) -> bool {
            self.0 == other.0
        }
    }
    impl Eq for Key<'_> {}
    impl Hash for Key<'_> {
        fn hash<H: Hasher>(&self, hasher: &mut H) {
            self.1.set(self.1.get() + 1);
            self.0.hash(hasher);
        }
    }

    #[test]
    fn retract_from_surviving_bucket_probes_once() {
        let calls = Cell::new(0);
        let mut index = HashIndex::new();
        let first = RowHandle::new(0, 0);
        let second = RowHandle::new(1, 0);
        index.insert(first, Key(7, &calls));
        index.insert(second, Key(7, &calls));
        calls.set(0);
        index.remove(first);
        assert_eq!(calls.get(), 1, "a surviving bucket must stay in the table");
        assert_eq!(index.lookup(&Key(7, &calls)), &[second]);
        index.remove(second);
        assert!(index.lookup(&Key(7, &calls)).is_empty());
        index.insert(first, Key(9, &calls));
        index.insert(first, Key(10, &calls));
        assert!(index.lookup(&Key(9, &calls)).is_empty());
        assert_eq!(index.lookup(&Key(10, &calls)), &[first]);
    }
}

use super::super::{identity::RowHandle, FastMap, HandleMap};

#[derive(Clone, Debug)]
pub struct HashIndex<K> {
    by_key: FastMap<K, Vec<RowHandle>>,
    key_of: HandleMap<K>,
    retired: Vec<Vec<RowHandle>>,
}

impl<K> HashIndex<K>
where
    K: Eq + Hash + Clone,
{
    pub fn new() -> HashIndex<K> {
        HashIndex {
            by_key: FastMap::default(),
            key_of: HandleMap::new(),
            retired: Vec::new(),
        }
    }

    // Keep occupied buckets in place. Only empty buckets relinquish their
    // allocation to the pool; entry access hashes the owned key once.
    fn remove_from_bucket(&mut self, handle: RowHandle, key: K) {
        if let std::collections::hash_map::Entry::Occupied(mut entry) = self.by_key.entry(key) {
            let bucket = entry.get_mut();
            if let Some(pos) = bucket.iter().position(|h| *h == handle) {
                bucket.swap_remove(pos);
            }
            if bucket.is_empty() {
                self.retired.push(entry.remove());
            }
        }
    }

    pub fn insert(&mut self, handle: RowHandle, key: K) {
        // Refresh path reinserts live handles: drop the stale reverse link
        // first so a changed key leaves no ghost bucket entry.
        if let Some(old) = self.key_of.insert(handle, key.clone()) {
            if old != key {
                self.remove_from_bucket(handle, old);
            } else if self.by_key.get(&key).is_some_and(|b| b.contains(&handle)) {
                return;
            }
        }
        self.by_key
            .entry(key)
            .or_insert_with(|| self.retired.pop().unwrap_or_default())
            .push(handle);
    }

    /// Insert a fresh transient row without reverse retention.
    /// Only use for traversal indexes whose rows are never retracted.
    pub(crate) fn insert_transient(&mut self, handle: RowHandle, key: K) {
        self.by_key
            .entry(key)
            .or_insert_with(|| self.retired.pop().unwrap_or_default())
            .push(handle);
    }

    pub fn remove(&mut self, handle: RowHandle) {
        let Some(key) = self.key_of.remove(handle) else {
            return;
        };
        self.remove_from_bucket(handle, key);
    }

    pub fn clear(&mut self) {
        self.by_key.clear();
        self.key_of.clear();
        self.retired.clear();
    }

    #[inline]
    pub fn lookup(&self, key: &K) -> &[RowHandle] {
        self.by_key.get(key).map(Vec::as_slice).unwrap_or(&[])
    }
}
