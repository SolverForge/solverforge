use std::collections::{hash_map::Entry, HashMap};
use std::hash::Hash;

use crate::stream::collector::{Accumulator, Collector};
use solverforge_core::score::Score;

use super::ComplementedGroupConstraint;

impl<S, A, B, K, EA, EB, KA, KB, C, V, R, Acc, D, W, Sc>
    ComplementedGroupConstraint<S, A, B, K, EA, EB, KA, KB, C, V, R, Acc, D, W, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    K: Clone + Eq + Hash + Send + Sync,
    EA: crate::stream::collection_extract::CollectionExtract<S, Item = A>,
    EB: crate::stream::collection_extract::CollectionExtract<S, Item = B>,
    KA: Fn(&A) -> Option<K> + Send + Sync,
    KB: Fn(&B) -> K + Send + Sync,
    C: for<'i> Collector<&'i A, Value = V, Result = R, Accumulator = Acc> + Send + Sync,
    V: Send + Sync,
    R: Send + Sync,
    Acc: Accumulator<V, R> + Send + Sync,
    D: Fn(&B) -> R + Send + Sync,
    W: Fn(&K, &R) -> Sc + Send + Sync,
    Sc: Score,
{
    // Insert an A entity and return the score delta.
    pub(super) fn insert_entity(
        &mut self,
        entities_b: &[B],
        entity_index: usize,
        entity: &A,
    ) -> Sc {
        // Skip entities with no key (e.g., unassigned shifts)
        let Some(key) = (self.key_a)(entity) else {
            return Sc::zero();
        };
        let value = self.collector.extract(entity);
        let old = self.key_score(entities_b, &key);

        let retraction = match self.groups.entry(key.clone()) {
            Entry::Occupied(mut entry) => {
                let group = entry.get_mut();
                let retraction = group.accumulator.accumulate(value);
                group.count += 1;
                retraction
            }
            Entry::Vacant(entry) => {
                let group = entry.insert(super::state::GroupState {
                    accumulator: self.collector.create_accumulator(),
                    count: 0,
                });
                let retraction = group.accumulator.accumulate(value);
                group.count += 1;
                retraction
            }
        };

        // Track entity -> key mapping and cache value for correct retraction
        self.entity_groups.insert(entity_index, key.clone());
        self.entity_retractions.insert(entity_index, retraction);

        let new_score = self.key_score(entities_b, &key);
        new_score - old
    }

    fn b_score_for_index(&self, entities_b: &[B], key: &K, b_idx: usize) -> Sc {
        if b_idx >= entities_b.len() {
            return Sc::zero();
        }
        let b = &entities_b[b_idx];
        let result = self.groups.get(key).map(|group| {
            group
                .accumulator
                .with_result(|result| self.compute_score(key, result))
        });
        result.unwrap_or_else(|| {
            let default_result = (self.default_fn)(b);
            self.compute_score(key, &default_result)
        })
    }

    fn key_score(&self, entities_b: &[B], key: &K) -> Sc {
        let Some(indices) = self.b_by_key.get(key) else {
            return Sc::zero();
        };
        indices.iter().fold(Sc::zero(), |total, &b_idx| {
            total + self.b_score_for_index(entities_b, key, b_idx)
        })
    }

    fn remove_index_from_key_bucket(
        indexes_by_key: &mut HashMap<K, Vec<usize>>,
        key: &K,
        idx: usize,
    ) {
        let mut remove_bucket = false;
        if let Some(indices) = indexes_by_key.get_mut(key) {
            if let Some(pos) = indices.iter().position(|candidate| *candidate == idx) {
                indices.swap_remove(pos);
            }
            remove_bucket = indices.is_empty();
        }
        if remove_bucket {
            indexes_by_key.remove(key);
        }
    }

    fn index_b(&mut self, key: K, b_idx: usize) {
        if let Some(old_key) = self.b_index_to_key.insert(b_idx, key.clone()) {
            Self::remove_index_from_key_bucket(&mut self.b_by_key, &old_key, b_idx);
        }
        self.b_by_key.entry(key).or_default().push(b_idx);
    }

    pub(super) fn insert_b(&mut self, entities_b: &[B], b_idx: usize) -> Sc {
        if b_idx >= entities_b.len() {
            return Sc::zero();
        }
        let key = (self.key_b)(&entities_b[b_idx]);
        self.index_b(key.clone(), b_idx);
        self.b_score_for_index(entities_b, &key, b_idx)
    }

    pub(super) fn retract_b(&mut self, entities_b: &[B], b_idx: usize) -> Sc {
        let Some(key) = self.b_index_to_key.remove(&b_idx) else {
            return Sc::zero();
        };
        let delta = -self.b_score_for_index(entities_b, &key, b_idx);
        Self::remove_index_from_key_bucket(&mut self.b_by_key, &key, b_idx);
        delta
    }

    // Retract an A entity and return the score delta.
    pub(super) fn retract_entity(
        &mut self,
        _entities_a: &[A],
        entities_b: &[B],
        entity_index: usize,
    ) -> Sc {
        // Find which group this entity belonged to
        let Some(key) = self.entity_groups.remove(&entity_index) else {
            return Sc::zero();
        };

        // Use cached retraction token (entity may have been mutated since insert)
        let Some(retraction) = self.entity_retractions.remove(&entity_index) else {
            return Sc::zero();
        };
        let old = self.key_score(entities_b, &key);
        let Entry::Occupied(mut entry) = self.groups.entry(key.clone()) else {
            return Sc::zero();
        };

        let group = entry.get_mut();
        group.accumulator.retract(retraction);
        group.count = group.count.saturating_sub(1);
        if group.count == 0 {
            entry.remove();
        }

        let new_score = self.key_score(entities_b, &key);
        new_score - old
    }
}
