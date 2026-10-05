// Ordered candidate buckets with retained old keys for exact retraction.
use std::collections::{BTreeMap, HashMap};
use std::ops::Bound::{Excluded, Included, Unbounded};

use super::super::RowHandle;

#[derive(Debug)]
pub struct OrderedIndex<K> {
    buckets: BTreeMap<K, Vec<RowHandle>>,
    keys: HashMap<RowHandle, K>,
}

impl<K: Ord + Clone> OrderedIndex<K> {
    pub fn new() -> Self {
        Self {
            buckets: BTreeMap::new(),
            keys: HashMap::new(),
        }
    }

    pub fn insert(&mut self, handle: RowHandle, key: K) {
        if self.keys.get(&handle) == Some(&key) {
            return;
        }
        self.remove(handle);
        self.keys.insert(handle, key.clone());
        self.buckets.entry(key).or_default().push(handle);
    }

    pub fn remove(&mut self, handle: RowHandle) {
        if let Some(key) = self.keys.remove(&handle) {
            if let Some(bucket) = self.buckets.get_mut(&key) {
                bucket.retain(|h| *h != handle);
                if bucket.is_empty() {
                    self.buckets.remove(&key);
                }
            }
        }
    }

    pub fn less_than(&self, key: &K, inclusive: bool) -> Vec<RowHandle> {
        let bound = if inclusive {
            Included(key)
        } else {
            Excluded(key)
        };
        self.buckets
            .range((Unbounded, bound))
            .flat_map(|(_, b)| b.iter().copied())
            .collect()
    }

    pub fn greater_than(&self, key: &K, inclusive: bool) -> Vec<RowHandle> {
        let bound = if inclusive {
            Included(key)
        } else {
            Excluded(key)
        };
        self.buckets
            .range((bound, Unbounded))
            .flat_map(|(_, b)| b.iter().copied())
            .collect()
    }

    pub fn clear(&mut self) {
        self.buckets.clear();
        self.keys.clear();
    }
}
