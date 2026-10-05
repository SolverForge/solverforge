/* Arity-generic retained join state shared by cross-entity constraints.

Owns the retained match-row storage, per-source key indexes, and per-source
inverted row buckets once, so cross arity families (bi, tri, ...) delegate
their incremental bookkeeping to this engine instead of hand-copying the
swap_remove row maintenance per arity.

Row storage is the shared relational `DenseRowStore` behind stable
generational `RowHandle`s. Reverse links name handles; a dense
slot-indexed position record keeps bucket unlinking O(1) under high
fan-out at the exact cost profile of the positional scheme it replaces —
array indexing with a generation check, no hashing on the row path. Key
indexes stay per-source `HashMap`s; per-join typed indexes arrive with
the relational operators.
*/

use std::collections::HashMap;
use std::hash::Hash;

use solverforge_core::score::Score;

use crate::stream::relational::{DenseRowStore, RowHandle};

#[derive(Clone)]
struct RetainedRow<const N: usize, Sc>
where
    Sc: Score,
{
    tuple: [usize; N],
    score: Sc,
}

pub(crate) struct CrossJoinEngine<const N: usize, K, Sc>
where
    Sc: Score,
{
    matches: HashMap<[usize; N], RowHandle>,
    match_rows: DenseRowStore<RetainedRow<N, Sc>>,
    to_matches: [HashMap<usize, Vec<RowHandle>>; N],
    // Dense slot-indexed bucket positions with generation guards. Slot
    // reuse across generations must not alias a live record: every read
    // checks the generation first, exactly like handle resolution.
    row_pos: Vec<([usize; N], u32)>,
    by_key: [HashMap<K, Vec<usize>>; N],
    index_to_key: [HashMap<usize, K>; N],
}

impl<const N: usize, K, Sc> CrossJoinEngine<N, K, Sc>
where
    Sc: Score,
{
    pub(crate) fn new() -> Self {
        Self {
            matches: HashMap::new(),
            match_rows: DenseRowStore::new(),
            to_matches: std::array::from_fn(|_| HashMap::new()),
            row_pos: Vec::new(),
            by_key: std::array::from_fn(|_| HashMap::new()),
            index_to_key: std::array::from_fn(|_| HashMap::new()),
        }
    }

    pub(crate) fn clear(&mut self) {
        self.matches.clear();
        self.match_rows.clear();
        self.row_pos.clear();
        for buckets in self.to_matches.iter_mut() {
            buckets.clear();
        }
        for indexes in self.by_key.iter_mut() {
            indexes.clear();
        }
        for keys in self.index_to_key.iter_mut() {
            keys.clear();
        }
    }

    pub(crate) fn match_count(&self) -> usize {
        self.match_rows.len()
    }

    /* Retained tuples for structural debug output, in store order. */
    pub(crate) fn retained_tuples(&self) -> Vec<[usize; N]> {
        self.match_rows.iter().map(|(_, row)| row.tuple).collect()
    }

    /* Debug-only cross-check: the tuple map and the row store agree.

    Runs at the end of every mutation in debug builds, so bookkeeping
    drift from future operator work fails fast at the site of the drift
    instead of surfacing as a wrong score later. The `cfg!` gate keeps
    release builds at zero cost: without it the traversal itself
    (not just the assertions) would run on every mutation.
    */
    fn debug_check(&self) {
        if cfg!(debug_assertions) {
            debug_assert_eq!(self.matches.len(), self.match_rows.len());
            if self.matches.is_empty() {
                debug_assert!(self.match_rows.is_empty());
            }
            for (tuple, handle) in &self.matches {
                let row = self.match_rows.get(*handle).expect("reverse link resolves");
                debug_assert_eq!(&row.tuple, tuple);
            }
        }
    }
}

impl<const N: usize, K, Sc> CrossJoinEngine<N, K, Sc>
where
    K: Eq + Hash + Clone,
    Sc: Score,
{
    pub(crate) fn contains(&self, tuple: &[usize; N]) -> bool {
        self.matches.contains_key(tuple)
    }

    // -- key index maintenance -------------------------------------------

    pub(crate) fn insert_source_index(&mut self, source: usize, idx: usize, key: K) {
        self.by_key[source]
            .entry(key.clone())
            .or_default()
            .push(idx);
        self.index_to_key[source].insert(idx, key);
    }

    pub(crate) fn remove_source_index(&mut self, source: usize, idx: usize) {
        let mut remove_bucket = false;
        if let Some(key) = self.index_to_key[source].remove(&idx) {
            if let Some(indices) = self.by_key[source].get_mut(&key) {
                if let Some(pos) = indices.iter().position(|candidate| *candidate == idx) {
                    indices.swap_remove(pos);
                }
                remove_bucket = indices.is_empty();
            }
            if remove_bucket {
                self.by_key[source].remove(&key);
            }
        }
    }

    // -- retained row maintenance ----------------------------------------

    // Records this row's bucket positions under its slot, growing the
    // dense record on first use of a slot. The generation guard travels
    // with the positions so slot reuse can never alias a live record.
    fn record_pos(&mut self, handle: RowHandle, pos: [usize; N]) {
        let slot = handle.slot() as usize;
        if slot >= self.row_pos.len() {
            self.row_pos.resize(slot + 1, ([0usize; N], u32::MAX));
        }
        self.row_pos[slot] = (pos, handle.generation());
    }

    // Reads this row's bucket positions, rejecting stale generations.
    // A missing or generation-mismatched record is an internal
    // bookkeeping bug and panics; handles here always come from
    // `row_indexes_for` on live rows.
    fn take_pos(&mut self, handle: RowHandle) -> [usize; N] {
        let slot = handle.slot() as usize;
        let record = self.row_pos.get_mut(slot).expect("row position missing");
        debug_assert_eq!(record.1, handle.generation(), "stale row position");
        // Tombstone the generation so a double removal fails loudly in
        // debug builds instead of unlinking a reused slot's buckets.
        record.1 = record.1.wrapping_add(1);
        record.0
    }

    // Adds a retained match row for `tuple` with `score`, recording reverse
    // links from every source index to the row's stable handle. Caller has
    // already decided the tuple matches.
    pub(crate) fn add_row(&mut self, tuple: [usize; N], score: Sc) -> Sc {
        let handle = self.match_rows.insert(RetainedRow { tuple, score });
        let mut pos = [0usize; N];
        for (source, bucket) in self.to_matches.iter_mut().enumerate() {
            let entry = bucket.entry(tuple[source]).or_default();
            pos[source] = entry.len();
            entry.push(handle);
        }
        self.record_pos(handle, pos);
        self.matches.insert(tuple, handle);
        self.debug_check();
        score
    }

    // Removes the retained row named by `handle`, unlinking it from every
    // reverse bucket in O(1) via the recorded positions, and returns the
    // negated row score. A stale handle is an internal bookkeeping bug and
    // panics; callers only pass handles obtained from `row_indexes_for`.
    pub(crate) fn remove_row_at(&mut self, handle: RowHandle) -> Sc {
        let removed = self
            .match_rows
            .retract(handle)
            .expect("stale row handle in join engine");
        self.matches.remove(&removed.tuple);
        let pos = self.take_pos(handle);

        for (source, bucket) in self.to_matches.iter_mut().enumerate() {
            let idx = removed.tuple[source];
            let mut remove_bucket = false;
            if let Some(handles) = bucket.get_mut(&idx) {
                debug_assert_eq!(handles[pos[source]], handle);
                handles.swap_remove(pos[source]);
                if pos[source] < handles.len() {
                    let moved = handles[pos[source]];
                    let moved_slot = moved.slot() as usize;
                    if let Some(moved_record) = self.row_pos.get_mut(moved_slot) {
                        debug_assert_eq!(moved_record.1, moved.generation());
                        moved_record.0[source] = pos[source];
                    }
                }
                remove_bucket = handles.is_empty();
            }
            if remove_bucket {
                bucket.remove(&idx);
            }
        }

        self.debug_check();
        -removed.score
    }

    pub(crate) fn row_indexes_for(&self, source: usize, idx: usize) -> Vec<RowHandle> {
        self.to_matches[source]
            .get(&idx)
            .cloned()
            .unwrap_or_default()
    }

    pub(crate) fn key_indexes_for(&self, source: usize, key: &K) -> Vec<usize> {
        self.by_key[source].get(key).cloned().unwrap_or_default()
    }
}
