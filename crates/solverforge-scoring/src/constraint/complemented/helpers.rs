use std::hash::Hash;

use crate::stream::collector::{Accumulator, UniCollector};
use solverforge_core::score::Score;
use solverforge_core::ImpactType;

use super::ComplementedGroupConstraint;

impl<S, A, B, K, EA, EB, KA, KB, C, D, W, Sc>
    ComplementedGroupConstraint<S, A, B, K, EA, EB, KA, KB, C, D, W, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    K: Clone + Eq + Hash + Send + Sync,
    EA: crate::stream::collection_extract::CollectionExtract<S, Item = A>,
    EB: crate::stream::collection_extract::CollectionExtract<S, Item = B>,
    KA: Fn(&A) -> Option<K> + Send + Sync,
    KB: Fn(&B) -> K + Send + Sync,
    C: UniCollector<A> + Send + Sync,
    C::Accumulator: Send + Sync,
    C::Result: Clone + Send + Sync,
    C::Value: Send + Sync,
    D: Fn(&B) -> C::Result + Send + Sync,
    W: Fn(&K, &C::Result) -> Sc + Send + Sync,
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
        let impact = self.impact_type;

        // Check if there's a B entity for this key
        let b_idx = self.b_by_key.get(&key).copied();
        let Some(b_idx) = b_idx else {
            // No B entity for this key - A entity doesn't affect score
            // Still track it for retraction
            let acc = self
                .groups
                .entry(key.clone())
                .or_insert_with(|| self.collector.create_accumulator());
            acc.accumulate(&value);
            self.entity_groups.insert(entity_index, key);
            self.entity_values.insert(entity_index, value);
            return Sc::zero();
        };

        let b = &entities_b[b_idx];

        // Compute old score for this B entity
        let old_result = self
            .groups
            .get(&key)
            .map(|acc| acc.finish())
            .unwrap_or_else(|| (self.default_fn)(b));
        let old_base = (self.weight_fn)(&key, &old_result);
        let old = match impact {
            ImpactType::Penalty => -old_base,
            ImpactType::Reward => old_base,
        };

        // Get or create accumulator and add value
        let acc = self
            .groups
            .entry(key.clone())
            .or_insert_with(|| self.collector.create_accumulator());
        acc.accumulate(&value);

        // Compute new score
        let new_result = acc.finish();
        let new_base = (self.weight_fn)(&key, &new_result);
        let new_score = match impact {
            ImpactType::Penalty => -new_base,
            ImpactType::Reward => new_base,
        };

        // Track entity -> key mapping and cache value for correct retraction
        self.entity_groups.insert(entity_index, key);
        self.entity_values.insert(entity_index, value);

        // Return delta
        new_score - old
    }

    fn b_score_for_key(&self, entities_b: &[B], key: &K, b_idx: usize) -> Sc {
        if b_idx >= entities_b.len() {
            return Sc::zero();
        }
        let b = &entities_b[b_idx];
        let result = self
            .groups
            .get(key)
            .map(|acc| acc.finish())
            .unwrap_or_else(|| (self.default_fn)(b));
        self.compute_score(key, &result)
    }

    pub(super) fn insert_b(&mut self, entities_b: &[B], b_idx: usize) -> Sc {
        if b_idx >= entities_b.len() {
            return Sc::zero();
        }
        let key = (self.key_b)(&entities_b[b_idx]);
        self.b_by_key.insert(key.clone(), b_idx);
        self.b_index_to_key.insert(b_idx, key.clone());
        self.b_score_for_key(entities_b, &key, b_idx)
    }

    pub(super) fn retract_b(&mut self, entities_b: &[B], b_idx: usize) -> Sc {
        let Some(key) = self.b_index_to_key.remove(&b_idx) else {
            return Sc::zero();
        };
        self.b_by_key.remove(&key);
        -self.b_score_for_key(entities_b, &key, b_idx)
    }

    // Retract an A entity and return the score delta.
    pub(super) fn retract_entity(
        &mut self,
        _entities_a: &[A],
        _entities_b: &[B],
        entity_index: usize,
    ) -> Sc {
        // Find which group this entity belonged to
        let Some(key) = self.entity_groups.remove(&entity_index) else {
            return Sc::zero();
        };

        // Use cached value (entity may have been mutated since insert)
        let Some(value) = self.entity_values.remove(&entity_index) else {
            return Sc::zero();
        };
        let impact = self.impact_type;

        // Check if there's a B entity for this key
        let b_idx = self.b_by_key.get(&key).copied();
        if b_idx.is_none() {
            // No B entity for this key - just update accumulator, no score delta
            if let Some(acc) = self.groups.get_mut(&key) {
                acc.retract(&value);
            }
            return Sc::zero();
        }

        // Get accumulator
        let Some(acc) = self.groups.get_mut(&key) else {
            return Sc::zero();
        };

        // Compute old score
        let old_result = acc.finish();
        let old_base = (self.weight_fn)(&key, &old_result);
        let old = match impact {
            ImpactType::Penalty => -old_base,
            ImpactType::Reward => old_base,
        };

        // Retract value
        acc.retract(&value);

        // Compute new score
        let new_result = acc.finish();
        let new_base = (self.weight_fn)(&key, &new_result);
        let new_score = match impact {
            ImpactType::Penalty => -new_base,
            ImpactType::Reward => new_base,
        };

        // Return delta
        new_score - old
    }
}
