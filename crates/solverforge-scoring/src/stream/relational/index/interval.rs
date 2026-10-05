// Narrow on starts, then check retained ends. No validity assumption: the
// existing overlap predicate also defines behavior for reversed intervals.
use super::super::RowHandle;
use super::OrderedIndex;
use std::collections::HashMap;

#[derive(Debug)]
pub struct IntervalIndex<K> {
    starts: OrderedIndex<K>,
    ends: HashMap<RowHandle, K>,
}

impl<K: Ord + Clone> IntervalIndex<K> {
    pub fn new() -> Self {
        Self {
            starts: OrderedIndex::new(),
            ends: HashMap::new(),
        }
    }

    pub fn insert(&mut self, handle: RowHandle, start: K, end: K) {
        self.starts.insert(handle, start);
        self.ends.insert(handle, end);
    }

    pub fn remove(&mut self, handle: RowHandle) {
        self.starts.remove(handle);
        self.ends.remove(&handle);
    }

    pub fn overlapping(&self, start: K, end: K) -> Vec<RowHandle> {
        self.starts
            .less_than(&end, false)
            .into_iter()
            .filter(|h| self.ends.get(h).is_some_and(|e| start < *e))
            .collect()
    }

    pub fn clear(&mut self) {
        self.starts.clear();
        self.ends.clear();
    }
}
