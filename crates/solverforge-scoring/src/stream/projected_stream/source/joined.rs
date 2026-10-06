use std::hash::Hash;
use std::marker::PhantomData;

use crate::stream::collection_extract::{ChangeSource, CollectionExtract};
use crate::stream::filter::BiFilter;
use crate::stream::relational::{DenseRowStore, HashIndex};

use super::{RowCoordinate, Source};

pub struct JoinedSource<S, A, B, K, EA, EB, KA, KB, F, P, Out> {
    extractor_a: EA,
    extractor_b: EB,
    key_a: KA,
    key_b: KB,
    filter: F,
    project: P,
    _phantom: PhantomData<(fn() -> S, fn() -> A, fn() -> B, fn() -> K, fn() -> Out)>,
}

impl<S, A, B, K, EA, EB, KA, KB, F, P, Out> JoinedSource<S, A, B, K, EA, EB, KA, KB, F, P, Out> {
    pub(crate) fn new(
        extractor_a: EA,
        extractor_b: EB,
        key_a: KA,
        key_b: KB,
        filter: F,
        project: P,
    ) -> Self {
        Self {
            extractor_a,
            extractor_b,
            key_a,
            key_b,
            filter,
            project,
            _phantom: PhantomData,
        }
    }
}

/* Retained join index state.

Uses the shared `HashIndex` and `DenseRowStore` rather than a bespoke
`HashMap<K, Vec<usize>>` pair: each accepted entity gets a stable handle
with a reverse handle-to-old-key link, so retraction removes the exact
bucket entry without re-deriving a key from mutated data. The store
payload is the entity's semantic slice index, keeping reverse lookups
free.
*/
pub struct JoinedState<K> {
    left_by_key: HashIndex<K>,
    right_by_key: HashIndex<K>,
    left_rows: DenseRowStore<usize>,
    right_rows: DenseRowStore<usize>,
}

impl<K> Default for JoinedState<K>
where
    K: Eq + Hash + Clone,
{
    fn default() -> Self {
        Self {
            left_by_key: HashIndex::new(),
            right_by_key: HashIndex::new(),
            left_rows: DenseRowStore::new(),
            right_rows: DenseRowStore::new(),
        }
    }
}

impl<K> JoinedState<K>
where
    K: Eq + Hash + Clone,
{
    fn insert_left(&mut self, entity_index: usize, key: K) {
        let handle = self.left_rows.insert(entity_index);
        self.left_by_key.insert(handle, key);
    }

    fn insert_right(&mut self, entity_index: usize, key: K) {
        let handle = self.right_rows.insert(entity_index);
        self.right_by_key.insert(handle, key);
    }

    /* Drops one entity's handle, key, and bucket entry.

    The reverse handle-to-old-key link is what lets the shared index drop
    the exact bucket entry; callers may pass a stale key without harm.
    */
    fn retract_left(&mut self, entity_index: usize, _key: &K) {
        let handle = self
            .left_rows
            .iter()
            .find(|(_, idx)| **idx == entity_index)
            .map(|(h, _)| h);
        if let Some(handle) = handle {
            self.left_by_key.remove(handle);
            self.left_rows.retract(handle);
        }
    }

    fn retract_right(&mut self, entity_index: usize, _key: &K) {
        let handle = self
            .right_rows
            .iter()
            .find(|(_, idx)| **idx == entity_index)
            .map(|(h, _)| h);
        if let Some(handle) = handle {
            self.right_by_key.remove(handle);
            self.right_rows.retract(handle);
        }
    }

    /* Semantic slice indexes of the opposite side matching `key`. */
    fn left_indexes(&self, key: &K) -> Vec<usize> {
        self.left_by_key
            .lookup(key)
            .iter()
            .filter_map(|h| self.left_rows.get(*h).copied())
            .collect()
    }

    fn right_indexes(&self, key: &K) -> Vec<usize> {
        self.right_by_key
            .lookup(key)
            .iter()
            .filter_map(|h| self.right_rows.get(*h).copied())
            .collect()
    }
}

impl<S, A, B, K, EA, EB, KA, KB, F, P, Out> Source<S, Out>
    for JoinedSource<S, A, B, K, EA, EB, KA, KB, F, P, Out>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    K: Eq + Hash + Clone + Send + Sync + 'static,
    EA: CollectionExtract<S, Item = A>,
    EB: CollectionExtract<S, Item = B>,
    KA: Fn(&A) -> K + Send + Sync,
    KB: Fn(&B) -> K + Send + Sync,
    F: BiFilter<S, A, B>,
    P: Fn(&A, &B) -> Out + Send + Sync,
    Out: Send + Sync + 'static,
{
    type State = JoinedState<K>;

    const MAX_EMITS: usize = 1;

    fn source_count(&self) -> usize {
        2
    }

    fn change_source(&self, slot: usize) -> ChangeSource {
        match slot {
            0 => self.extractor_a.change_source(),
            1 => self.extractor_b.change_source(),
            _ => ChangeSource::Static,
        }
    }

    fn build_state(&self, solution: &S) -> Self::State {
        let mut state = JoinedState::default();
        for (idx, entity) in self.extractor_a.extract(solution).iter().enumerate() {
            if !self.extractor_a.contains(solution, entity) {
                continue;
            }
            state.insert_left(idx, (self.key_a)(entity));
        }
        for (idx, entity) in self.extractor_b.extract(solution).iter().enumerate() {
            if !self.extractor_b.contains(solution, entity) {
                continue;
            }
            state.insert_right(idx, (self.key_b)(entity));
        }
        state
    }

    fn collect_all<V>(&self, solution: &S, state: &Self::State, mut visit: V)
    where
        V: FnMut(RowCoordinate, Out),
    {
        let entities_a = self.extractor_a.extract(solution);
        let entities_b = self.extractor_b.extract(solution);
        for (a_idx, entity) in entities_a.iter().enumerate() {
            if !self.extractor_a.contains(solution, entity) {
                continue;
            }
            let key = (self.key_a)(entity);
            for b_idx in state.right_indexes(&key) {
                self.project_pair(solution, entities_a, entities_b, a_idx, b_idx, &mut visit);
            }
        }
    }

    fn collect_entity<V>(
        &self,
        solution: &S,
        state: &Self::State,
        slot: usize,
        entity_index: usize,
        mut visit: V,
    ) where
        V: FnMut(RowCoordinate, Out),
    {
        let entities_a = self.extractor_a.extract(solution);
        let entities_b = self.extractor_b.extract(solution);
        match slot {
            0 => {
                let Some(entity) = entities_a.get(entity_index) else {
                    return;
                };
                if !self.extractor_a.contains(solution, entity) {
                    return;
                }
                let key = (self.key_a)(entity);
                for b_idx in state.right_indexes(&key) {
                    self.project_pair(
                        solution,
                        entities_a,
                        entities_b,
                        entity_index,
                        b_idx,
                        &mut visit,
                    );
                }
            }
            1 => {
                let Some(entity) = entities_b.get(entity_index) else {
                    return;
                };
                if !self.extractor_b.contains(solution, entity) {
                    return;
                }
                let key = (self.key_b)(entity);
                for a_idx in state.left_indexes(&key) {
                    self.project_pair(
                        solution,
                        entities_a,
                        entities_b,
                        a_idx,
                        entity_index,
                        &mut visit,
                    );
                }
            }
            _ => {}
        }
    }

    fn insert_entity_state(
        &self,
        solution: &S,
        state: &mut Self::State,
        slot: usize,
        entity_index: usize,
    ) {
        match slot {
            0 => {
                if let Some(entity) = self.extractor_a.extract(solution).get(entity_index) {
                    if !self.extractor_a.contains(solution, entity) {
                        return;
                    }
                    state.insert_left(entity_index, (self.key_a)(entity));
                }
            }
            1 => {
                if let Some(entity) = self.extractor_b.extract(solution).get(entity_index) {
                    if !self.extractor_b.contains(solution, entity) {
                        return;
                    }
                    state.insert_right(entity_index, (self.key_b)(entity));
                }
            }
            _ => {}
        }
    }

    fn retract_entity_state(
        &self,
        solution: &S,
        state: &mut Self::State,
        slot: usize,
        entity_index: usize,
    ) {
        match slot {
            0 => {
                if let Some(entity) = self.extractor_a.extract(solution).get(entity_index) {
                    state.retract_left(entity_index, &(self.key_a)(entity));
                }
            }
            1 => {
                if let Some(entity) = self.extractor_b.extract(solution).get(entity_index) {
                    state.retract_right(entity_index, &(self.key_b)(entity));
                }
            }
            _ => {}
        }
    }
}

impl<S, A, B, K, EA, EB, KA, KB, F, P, Out> JoinedSource<S, A, B, K, EA, EB, KA, KB, F, P, Out>
where
    EA: CollectionExtract<S, Item = A>,
    EB: CollectionExtract<S, Item = B>,
    F: BiFilter<S, A, B>,
    P: Fn(&A, &B) -> Out + Send + Sync,
{
    fn project_pair<V>(
        &self,
        solution: &S,
        entities_a: &[A],
        entities_b: &[B],
        a_idx: usize,
        b_idx: usize,
        visit: &mut V,
    ) where
        V: FnMut(RowCoordinate, Out),
    {
        let Some(a) = entities_a.get(a_idx) else {
            return;
        };
        let Some(b) = entities_b.get(b_idx) else {
            return;
        };
        if !self.extractor_a.contains(solution, a) || !self.extractor_b.contains(solution, b) {
            return;
        }
        if !self.filter.test(solution, a, b, a_idx, b_idx) {
            return;
        }
        let coordinate = RowCoordinate::pair(0, a_idx, 1, b_idx, 0);
        visit(coordinate, (self.project)(a, b));
    }
}
