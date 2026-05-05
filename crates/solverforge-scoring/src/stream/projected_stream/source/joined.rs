use std::collections::HashMap;
use std::hash::Hash;
use std::marker::PhantomData;

use crate::stream::collection_extract::{ChangeSource, CollectionExtract};
use crate::stream::filter::BiFilter;

use super::{ProjectedRowCoordinate, ProjectedSource};

pub struct JoinedProjectedSource<S, A, B, K, EA, EB, KA, KB, F, P, Out> {
    extractor_a: EA,
    extractor_b: EB,
    key_a: KA,
    key_b: KB,
    filter: F,
    project: P,
    _phantom: PhantomData<(fn() -> S, fn() -> A, fn() -> B, fn() -> K, fn() -> Out)>,
}

impl<S, A, B, K, EA, EB, KA, KB, F, P, Out>
    JoinedProjectedSource<S, A, B, K, EA, EB, KA, KB, F, P, Out>
{
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

pub struct JoinedProjectedState<K> {
    a_by_key: HashMap<K, Vec<usize>>,
    b_by_key: HashMap<K, Vec<usize>>,
}

impl<K> Default for JoinedProjectedState<K> {
    fn default() -> Self {
        Self {
            a_by_key: HashMap::new(),
            b_by_key: HashMap::new(),
        }
    }
}

impl<K> JoinedProjectedState<K>
where
    K: Eq + Hash,
{
    fn insert_left(&mut self, entity_index: usize, key: K) {
        self.a_by_key.entry(key).or_default().push(entity_index);
    }

    fn insert_right(&mut self, entity_index: usize, key: K) {
        self.b_by_key.entry(key).or_default().push(entity_index);
    }

    fn retract_left(&mut self, entity_index: usize, key: &K) {
        Self::remove_index_from_key_bucket(&mut self.a_by_key, key, entity_index);
    }

    fn retract_right(&mut self, entity_index: usize, key: &K) {
        Self::remove_index_from_key_bucket(&mut self.b_by_key, key, entity_index);
    }

    fn remove_index_from_key_bucket(
        indexes_by_key: &mut HashMap<K, Vec<usize>>,
        key: &K,
        entity_index: usize,
    ) {
        let mut remove_bucket = false;
        if let Some(indices) = indexes_by_key.get_mut(key) {
            if let Some(pos) = indices
                .iter()
                .position(|candidate| *candidate == entity_index)
            {
                indices.swap_remove(pos);
            }
            remove_bucket = indices.is_empty();
        }
        if remove_bucket {
            indexes_by_key.remove(key);
        }
    }
}

impl<S, A, B, K, EA, EB, KA, KB, F, P, Out> ProjectedSource<S, Out>
    for JoinedProjectedSource<S, A, B, K, EA, EB, KA, KB, F, P, Out>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    K: Eq + Hash + Send + Sync + 'static,
    EA: CollectionExtract<S, Item = A>,
    EB: CollectionExtract<S, Item = B>,
    KA: Fn(&A) -> K + Send + Sync,
    KB: Fn(&B) -> K + Send + Sync,
    F: BiFilter<S, A, B>,
    P: Fn(&A, &B) -> Out + Send + Sync,
    Out: Send + Sync + 'static,
{
    type State = JoinedProjectedState<K>;

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
        let mut state = JoinedProjectedState::default();
        for (idx, entity) in self.extractor_a.extract(solution).iter().enumerate() {
            state.insert_left(idx, (self.key_a)(entity));
        }
        for (idx, entity) in self.extractor_b.extract(solution).iter().enumerate() {
            state.insert_right(idx, (self.key_b)(entity));
        }
        state
    }

    fn collect_all<V>(&self, solution: &S, state: &Self::State, mut visit: V)
    where
        V: FnMut(ProjectedRowCoordinate, Out),
    {
        let entities_a = self.extractor_a.extract(solution);
        let entities_b = self.extractor_b.extract(solution);
        for (a_idx, entity) in entities_a.iter().enumerate() {
            let key = (self.key_a)(entity);
            let Some(b_indices) = state.b_by_key.get(&key) else {
                continue;
            };
            for &b_idx in b_indices {
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
        V: FnMut(ProjectedRowCoordinate, Out),
    {
        let entities_a = self.extractor_a.extract(solution);
        let entities_b = self.extractor_b.extract(solution);
        match slot {
            0 => {
                let Some(entity) = entities_a.get(entity_index) else {
                    return;
                };
                let key = (self.key_a)(entity);
                let Some(b_indices) = state.b_by_key.get(&key) else {
                    return;
                };
                for &b_idx in b_indices {
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
                let key = (self.key_b)(entity);
                let Some(a_indices) = state.a_by_key.get(&key) else {
                    return;
                };
                for &a_idx in a_indices {
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
                    state.insert_left(entity_index, (self.key_a)(entity));
                }
            }
            1 => {
                if let Some(entity) = self.extractor_b.extract(solution).get(entity_index) {
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

impl<S, A, B, K, EA, EB, KA, KB, F, P, Out>
    JoinedProjectedSource<S, A, B, K, EA, EB, KA, KB, F, P, Out>
where
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
        V: FnMut(ProjectedRowCoordinate, Out),
    {
        let Some(a) = entities_a.get(a_idx) else {
            return;
        };
        let Some(b) = entities_b.get(b_idx) else {
            return;
        };
        if !self.filter.test(solution, a, b, a_idx, b_idx) {
            return;
        }
        let coordinate = ProjectedRowCoordinate::pair(0, a_idx, 1, b_idx, 0);
        visit(coordinate, (self.project)(a, b));
    }
}
