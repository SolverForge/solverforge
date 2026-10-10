/* Typed hash index for one join side's equality relationship.

Each join owns its own key type `K`: indexes are never shared across
joins, so heterogeneous successive keys stay in separate domains. The
index maps keys to stable input handles and retains the reverse
handle-to-old-key link, so retraction removes the exact bucket entry
without re-deriving keys from mutated values.
*/

use std::hash::Hash;

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

    /// Takes the bucket for `key` out of the table, or draws a retired one.
    ///
    /// Buckets are per-key and short lived: a variable change retracts and
    /// re-inserts the row, so its bucket is emptied and rebuilt on every event.
    /// Retired buckets are kept so the steady state reuses one allocation per
    /// key instead of one allocation per event.
    fn take_bucket(&mut self, key: &K) -> Vec<RowHandle> {
        match self.by_key.remove(key) {
            Some(bucket) => bucket,
            None => self.retired.pop().unwrap_or_default(),
        }
    }

    fn put_bucket(&mut self, key: K, mut bucket: Vec<RowHandle>) {
        if bucket.is_empty() {
            bucket.clear();
            self.retired.push(bucket);
        } else {
            self.by_key.insert(key, bucket);
        }
    }

    pub fn insert(&mut self, handle: RowHandle, key: K) {
        // Refresh path reinserts live handles: drop the stale reverse link
        // first so a changed key leaves no ghost bucket entry.
        if let Some(old) = self.key_of.insert(handle, key.clone()) {
            if old != key {
                let mut bucket = self.take_bucket(&old);
                if let Some(pos) = bucket.iter().position(|h| *h == handle) {
                    bucket.swap_remove(pos);
                }
                self.put_bucket(old, bucket);
            } else if self.by_key.get(&key).is_some_and(|b| b.contains(&handle)) {
                return;
            }
        }
        let mut bucket = self.take_bucket(&key);
        bucket.push(handle);
        self.put_bucket(key, bucket);
    }

    /// Insert a fresh transient row without reverse retention.
    /// Only use for traversal indexes whose rows are never retracted.
    pub(crate) fn insert_transient(&mut self, handle: RowHandle, key: K) {
        let mut bucket = self.take_bucket(&key);
        bucket.push(handle);
        self.put_bucket(key, bucket);
    }

    pub fn remove(&mut self, handle: RowHandle) {
        let Some(key) = self.key_of.remove(handle) else {
            return;
        };
        let mut bucket = self.take_bucket(&key);
        if let Some(pos) = bucket.iter().position(|h| *h == handle) {
            bucket.swap_remove(pos);
        }
        self.put_bucket(key, bucket);
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
