/* Typed hash index for one join side's equality relationship.

Each join owns its own key type `K`: indexes are never shared across
joins, so heterogeneous successive keys stay in separate domains. The
index maps keys to stable input handles and retains the reverse
handle-to-old-key link, so retraction removes the exact bucket entry
without re-deriving keys from mutated values.
*/

use std::collections::HashMap;
use std::hash::Hash;

use super::super::{identity::RowHandle, HandleMap};

#[derive(Clone, Debug)]
pub struct HashIndex<K> {
    by_key: HashMap<K, Vec<RowHandle>>,
    key_of: HandleMap<K>,
}

impl<K> HashIndex<K>
where
    K: Eq + Hash + Clone,
{
    pub fn new() -> HashIndex<K> {
        HashIndex {
            by_key: HashMap::new(),
            key_of: HandleMap::new(),
        }
    }

    pub fn insert(&mut self, handle: RowHandle, key: K) {
        // Refresh path reinserts live handles: drop the stale reverse link
        // first so a changed key leaves no ghost bucket entry.
        if let Some(old) = self.key_of.insert(handle, key.clone()) {
            if old != key {
                if let Some(bucket) = self.by_key.get_mut(&old) {
                    if let Some(pos) = bucket.iter().position(|h| *h == handle) {
                        bucket.swap_remove(pos);
                    }
                    if bucket.is_empty() {
                        self.by_key.remove(&old);
                    }
                }
            } else if let Some(bucket) = self.by_key.get(&key) {
                if bucket.contains(&handle) {
                    return;
                }
            }
        }
        self.by_key.entry(key).or_default().push(handle);
    }

    /// Insert a fresh transient row without reverse retention.
    /// Only use for traversal indexes whose rows are never retracted.
    pub(crate) fn insert_transient(&mut self, handle: RowHandle, key: K) {
        self.by_key.entry(key).or_default().push(handle);
    }

    pub fn remove(&mut self, handle: RowHandle) {
        if let Some(key) = self.key_of.remove(handle) {
            let mut drop_bucket = false;
            if let Some(bucket) = self.by_key.get_mut(&key) {
                if let Some(pos) = bucket.iter().position(|h| *h == handle) {
                    bucket.swap_remove(pos);
                }
                drop_bucket = bucket.is_empty();
            }
            if drop_bucket {
                self.by_key.remove(&key);
            }
        }
    }

    pub fn clear(&mut self) {
        self.by_key.clear();
        self.key_of.clear();
    }

    #[inline]
    pub fn lookup(&self, key: &K) -> &[RowHandle] {
        self.by_key.get(key).map(Vec::as_slice).unwrap_or(&[])
    }
}
