/* Typed hash index for one join side's equality relationship.

Each join owns its own key type `K`: indexes are never shared across
joins, so heterogeneous successive keys stay in separate domains. The
index maps keys to stable input handles and retains the reverse
handle-to-old-key link, so retraction removes the exact bucket entry
without re-deriving keys from mutated values.
*/

use std::collections::HashMap;
use std::hash::Hash;

use super::super::identity::RowHandle;

#[derive(Clone, Debug, Default)]
pub(crate) struct HashIndex<K> {
    by_key: HashMap<K, Vec<RowHandle>>,
    key_of: HashMap<RowHandle, K>,
}

impl<K> HashIndex<K>
where
    K: Eq + Hash + Clone,
{
    pub(crate) fn new() -> HashIndex<K> {
        HashIndex {
            by_key: HashMap::new(),
            key_of: HashMap::new(),
        }
    }

    pub(crate) fn insert(&mut self, handle: RowHandle, key: K) {
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

    pub(crate) fn remove(&mut self, handle: RowHandle) {
        if let Some(key) = self.key_of.remove(&handle) {
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

    pub(crate) fn clear(&mut self) {
        self.by_key.clear();
        self.key_of.clear();
    }

    pub(crate) fn lookup(&self, key: &K) -> &[RowHandle] {
        self.by_key.get(key).map(Vec::as_slice).unwrap_or(&[])
    }
}
