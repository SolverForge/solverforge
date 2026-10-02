/* Arity-generic retained join state shared by cross-entity constraints.

Owns the retained match-row storage, per-source key indexes, and per-source
inverted row buckets once, so cross arity families (bi, tri, ...) delegate
their incremental bookkeeping to this engine instead of hand-copying the
swap_remove row maintenance per arity.
*/

use std::collections::HashMap;
use std::hash::Hash;

use solverforge_core::score::Score;

#[derive(Clone)]
pub(crate) struct MatchRow<const N: usize, Sc>
where
    Sc: Score,
{
    tuple: [usize; N],
    score: Sc,
    pos: [usize; N],
}

pub(crate) struct CrossJoinEngine<const N: usize, K, Sc>
where
    Sc: Score,
{
    matches: HashMap<[usize; N], usize>,
    match_rows: Vec<MatchRow<N, Sc>>,
    to_matches: [HashMap<usize, Vec<usize>>; N],
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
            match_rows: Vec::new(),
            to_matches: std::array::from_fn(|_| HashMap::new()),
            by_key: std::array::from_fn(|_| HashMap::new()),
            index_to_key: std::array::from_fn(|_| HashMap::new()),
        }
    }

    pub(crate) fn clear(&mut self) {
        self.matches.clear();
        self.match_rows.clear();
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

    // Adds a retained match row for `tuple` with `score`, recording per-source
    // bucket positions. Caller has already decided the tuple matches.
    pub(crate) fn add_row(&mut self, tuple: [usize; N], score: Sc) -> Sc {
        let row_idx = self.match_rows.len();
        let mut pos = [0usize; N];
        for (source, bucket) in self.to_matches.iter_mut().enumerate() {
            let entry = bucket.entry(tuple[source]).or_default();
            pos[source] = entry.len();
            entry.push(row_idx);
        }
        self.match_rows.push(MatchRow { tuple, score, pos });
        self.matches.insert(tuple, row_idx);
        score
    }

    // Removes the retained row at `row_idx`, repairing the swapped-in row's
    // stored positions, and returns the negated row score.
    pub(crate) fn remove_row_at(&mut self, row_idx: usize) -> Sc {
        let row = self.match_rows[row_idx].clone();
        self.matches.remove(&row.tuple);

        for (source, bucket) in self.to_matches.iter_mut().enumerate() {
            let idx = row.tuple[source];
            let pos = row.pos[source];
            let mut remove_bucket = false;
            if let Some(rows) = bucket.get_mut(&idx) {
                debug_assert_eq!(rows[pos], row_idx);
                rows.swap_remove(pos);
                if pos < rows.len() {
                    let moved_row_idx = rows[pos];
                    self.match_rows[moved_row_idx].pos[source] = pos;
                }
                remove_bucket = rows.is_empty();
            }
            if remove_bucket {
                bucket.remove(&idx);
            }
        }

        let last_idx = self.match_rows.len() - 1;
        self.match_rows.swap_remove(row_idx);
        if row_idx != last_idx {
            let moved = self.match_rows[row_idx].clone();
            self.matches.insert(moved.tuple, row_idx);
            for source in 0..N {
                if let Some(rows) = self.to_matches[source].get_mut(&moved.tuple[source]) {
                    rows[moved.pos[source]] = row_idx;
                }
            }
        }

        -row.score
    }

    pub(crate) fn row_indexes_for(&self, source: usize, idx: usize) -> Vec<usize> {
        self.to_matches[source]
            .get(&idx)
            .cloned()
            .unwrap_or_default()
    }

    pub(crate) fn key_indexes_for(&self, source: usize, key: &K) -> Vec<usize> {
        self.by_key[source].get(key).cloned().unwrap_or_default()
    }
}
