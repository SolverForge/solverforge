use std::collections::HashMap;
use std::hash::Hash;

use crate::api::constraint_set::IncrementalConstraint;
use solverforge_core::score::Score;
use solverforge_core::ConstraintRef;

use super::FlattenedBiConstraint;

impl<S, A, B, C, K, CK, EA, EB, KA, KB, Flatten, CKeyFn, ALookup, F, W, Sc>
    IncrementalConstraint<S, Sc>
    for FlattenedBiConstraint<S, A, B, C, K, CK, EA, EB, KA, KB, Flatten, CKeyFn, ALookup, F, W, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    C: Clone + Send + Sync + 'static,
    K: Eq + Hash + Clone + Send + Sync,
    CK: Eq + Hash + Clone + Send + Sync,
    EA: crate::stream::collection_extract::CollectionExtract<S, Item = A>,
    EB: crate::stream::collection_extract::CollectionExtract<S, Item = B>,
    KA: Fn(&A) -> K + Send + Sync,
    KB: Fn(&B) -> K + Send + Sync,
    Flatten: Fn(&B) -> &[C] + Send + Sync,
    CKeyFn: Fn(&C) -> CK + Send + Sync,
    ALookup: Fn(&A) -> CK + Send + Sync,
    F: Fn(&S, &A, &C, usize, usize) -> bool + Send + Sync,
    W: Fn(&A, &C) -> Sc + Send + Sync,
    Sc: Score,
{
    fn evaluate(&self, solution: &S) -> Sc {
        let entities_a = self.extractor_a.extract(solution);
        let entities_b = self.extractor_b.extract(solution);
        let mut total = Sc::zero();

        // Build temporary index for standalone evaluation
        let mut temp_index: HashMap<(K, CK), Vec<(usize, C)>> = HashMap::new();
        for (b_idx, b) in entities_b.iter().enumerate() {
            if !self.extractor_b.contains(solution, b) {
                continue;
            }
            let join_key = (self.key_b)(b);
            for c in (self.flatten)(b) {
                let c_key = (self.c_key_fn)(c);
                temp_index
                    .entry((join_key.clone(), c_key))
                    .or_default()
                    .push((b_idx, c.clone()));
            }
        }

        for (a_idx, a) in entities_a.iter().enumerate() {
            if !self.extractor_a.contains(solution, a) {
                continue;
            }
            let join_key = (self.key_a)(a);
            let lookup_key = (self.a_lookup_fn)(a);

            if let Some(matches) = temp_index.get(&(join_key, lookup_key)) {
                for (b_idx, c) in matches {
                    if (self.filter)(solution, a, c, a_idx, *b_idx) {
                        total = total + self.compute_score(a, c);
                    }
                }
            }
        }

        total
    }

    fn match_count(&self, solution: &S) -> usize {
        let entities_a = self.extractor_a.extract(solution);
        let entities_b = self.extractor_b.extract(solution);
        let mut count = 0;

        // Build temporary index
        let mut temp_index: HashMap<(K, CK), Vec<(usize, C)>> = HashMap::new();
        for (b_idx, b) in entities_b.iter().enumerate() {
            if !self.extractor_b.contains(solution, b) {
                continue;
            }
            let join_key = (self.key_b)(b);
            for c in (self.flatten)(b) {
                let c_key = (self.c_key_fn)(c);
                temp_index
                    .entry((join_key.clone(), c_key))
                    .or_default()
                    .push((b_idx, c.clone()));
            }
        }

        for (a_idx, a) in entities_a.iter().enumerate() {
            if !self.extractor_a.contains(solution, a) {
                continue;
            }
            let join_key = (self.key_a)(a);
            let lookup_key = (self.a_lookup_fn)(a);

            if let Some(matches) = temp_index.get(&(join_key, lookup_key)) {
                for (b_idx, c) in matches {
                    if (self.filter)(solution, a, c, a_idx, *b_idx) {
                        count += 1;
                    }
                }
            }
        }

        count
    }

    fn initialize(&mut self, solution: &S) -> Sc {
        self.reset();

        let entities_a = self.extractor_a.extract(solution);
        let entities_b = self.extractor_b.extract(solution);

        // Build C index once: O(B × C)
        self.build_c_index(solution, entities_b);

        // Insert all A entities: O(A) with O(1) lookups each
        let mut total = Sc::zero();
        for a_idx in 0..entities_a.len() {
            total = total + self.insert_a(solution, entities_a, a_idx);
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
        if a_changed {
            total = total + self.insert_a(solution, entities_a, entity_index);
        }
        if b_changed {
            total = total + self.insert_b(solution, entities_a, entities_b, entity_index);
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
        let mut total = Sc::zero();
        if a_changed {
            total = total + self.retract_a(entity_index);
        }
        if b_changed {
            total = total + self.retract_b(solution, entities_a, entity_index);
        }
        total
    }

    fn reset(&mut self) {
        self.bucket_by_key.clear();
        self.c_index.clear();
        self.a_scores.clear();
        self.a_index_to_bucket.clear();
        self.a_by_bucket.clear();
        self.b_entries.clear();
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
