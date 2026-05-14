use std::hash::Hash;

use crate::api::constraint_set::IncrementalConstraint;
use crate::stream::collector::{Accumulator, Collector};
use solverforge_core::score::Score;
use solverforge_core::ConstraintRef;

use super::ComplementedGroupConstraint;

impl<S, A, B, K, EA, EB, KA, KB, C, V, R, Acc, D, W, Sc> IncrementalConstraint<S, Sc>
    for ComplementedGroupConstraint<S, A, B, K, EA, EB, KA, KB, C, V, R, Acc, D, W, Sc>
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
    fn evaluate(&self, solution: &S) -> Sc {
        let entities_a = self.extractor_a.extract(solution);
        let entities_b = self.extractor_b.extract(solution);

        let groups = self.build_groups(entities_a);

        let mut total = Sc::zero();
        for b in entities_b {
            let key = (self.key_b)(b);
            total = total
                + match groups.get(&key) {
                    Some(acc) => acc.with_result(|result| self.compute_score(&key, result)),
                    None => {
                        let default_result = (self.default_fn)(b);
                        self.compute_score(&key, &default_result)
                    }
                };
        }

        total
    }

    fn match_count(&self, solution: &S) -> usize {
        let entities_b = self.extractor_b.extract(solution);
        entities_b.len()
    }

    fn initialize(&mut self, solution: &S) -> Sc {
        self.reset();

        let entities_a = self.extractor_a.extract(solution);
        let entities_b = self.extractor_b.extract(solution);

        // Build B key -> index mapping
        for (idx, b) in entities_b.iter().enumerate() {
            let key = (self.key_b)(b);
            self.b_by_key.entry(key.clone()).or_default().push(idx);
            self.b_index_to_key.insert(idx, key);
        }

        // Initialize all B entities with default scores
        let mut total = Sc::zero();
        for b in entities_b {
            let key = (self.key_b)(b);
            let default_result = (self.default_fn)(b);
            total = total + self.compute_score(&key, &default_result);
        }

        // Now insert all A entities incrementally
        for (idx, a) in entities_a.iter().enumerate() {
            total = total + self.insert_entity(entities_b, idx, a);
        }

        total
    }

    fn on_insert(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc {
        let a_changed = self
            .a_source
            .assert_localizes(descriptor_index, &self.constraint_ref.name);
        let b_changed = self
            .b_source
            .assert_localizes(descriptor_index, &self.constraint_ref.name);
        let entities_a = self.extractor_a.extract(solution);
        let entities_b = self.extractor_b.extract(solution);

        let mut total = Sc::zero();
        if a_changed && entity_index < entities_a.len() {
            let entity = &entities_a[entity_index];
            total = total + self.insert_entity(entities_b, entity_index, entity);
        }
        if b_changed {
            total = total + self.insert_b(entities_b, entity_index);
        }
        total
    }

    fn on_retract(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc {
        let a_changed = self
            .a_source
            .assert_localizes(descriptor_index, &self.constraint_ref.name);
        let b_changed = self
            .b_source
            .assert_localizes(descriptor_index, &self.constraint_ref.name);
        let entities_a = self.extractor_a.extract(solution);
        let entities_b = self.extractor_b.extract(solution);

        let mut total = Sc::zero();
        if a_changed {
            total = total + self.retract_entity(entities_a, entities_b, entity_index);
        }
        if b_changed {
            total = total + self.retract_b(entities_b, entity_index);
        }
        total
    }

    fn reset(&mut self) {
        self.groups.clear();
        self.entity_groups.clear();
        self.entity_retractions.clear();
        self.b_by_key.clear();
        self.b_index_to_key.clear();
    }

    fn name(&self) -> &str {
        &self.constraint_ref.name
    }

    fn is_hard(&self) -> bool {
        self.is_hard
    }

    fn constraint_ref(&self) -> &ConstraintRef {
        &self.constraint_ref
    }
}
