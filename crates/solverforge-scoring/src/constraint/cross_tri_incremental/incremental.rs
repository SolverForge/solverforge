use std::fmt::Debug;
use std::hash::Hash;

use crate::api::analysis::{ConstraintJustification, DetailedConstraintMatch, EntityRef};
use crate::api::constraint_set::IncrementalConstraint;
use crate::stream::collection_extract::CollectionExtract;
use solverforge_core::score::Score;
use solverforge_core::ConstraintRef;

use super::{CrossTriWeight, Tri};

impl<S, A, B, C, K, EA, EB, EC, KA, KB, KC, F, W, Sc> IncrementalConstraint<S, Sc>
    for Tri<S, A, B, C, K, EA, EB, EC, KA, KB, KC, F, W, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Debug + Send + Sync + 'static,
    B: Clone + Debug + Send + Sync + 'static,
    C: Clone + Debug + Send + Sync + 'static,
    K: Eq + Hash + Clone + Send + Sync,
    EA: CollectionExtract<S, Item = A> + Send + Sync,
    EB: CollectionExtract<S, Item = B> + Send + Sync,
    EC: CollectionExtract<S, Item = C> + Send + Sync,
    KA: Fn(&A) -> K + Send + Sync,
    KB: Fn(&B) -> K + Send + Sync,
    KC: Fn(&C) -> K + Send + Sync,
    F: Fn(&S, &A, &B, &C, usize, usize, usize) -> bool + Send + Sync,
    W: CrossTriWeight<S, A, B, C, Sc>,
    Sc: Score,
{
    fn evaluate(&self, solution: &S) -> Sc {
        let entities_a = self.extractor_a.extract(solution);
        let entities_b = self.extractor_b.extract(solution);
        let entities_c = self.extractor_c.extract(solution);
        let (b_by_key, c_by_key) = self.build_side_indexes(solution, entities_b, entities_c);
        let mut total = Sc::zero();

        for (a_idx, a) in entities_a.iter().enumerate() {
            if !self.extractor_a.contains(solution, a) {
                continue;
            }
            let key = (self.key_a)(a);
            let Some(b_indices) = b_by_key.get(&key) else {
                continue;
            };
            for &b_idx in b_indices {
                let b = &entities_b[b_idx];
                for &c_idx in self.matching_c_indices_in(&c_by_key, b) {
                    let c = &entities_c[c_idx];
                    if (self.filter)(solution, a, b, c, a_idx, b_idx, c_idx) {
                        total = total
                            + self.compute_score(
                                solution, entities_a, entities_b, entities_c, a_idx, b_idx, c_idx,
                            );
                    }
                }
            }
        }

        total
    }

    fn match_count(&self, solution: &S) -> usize {
        let entities_a = self.extractor_a.extract(solution);
        let entities_b = self.extractor_b.extract(solution);
        let entities_c = self.extractor_c.extract(solution);
        let (b_by_key, c_by_key) = self.build_side_indexes(solution, entities_b, entities_c);
        let mut count = 0;

        for (a_idx, a) in entities_a.iter().enumerate() {
            if !self.extractor_a.contains(solution, a) {
                continue;
            }
            let key = (self.key_a)(a);
            let Some(b_indices) = b_by_key.get(&key) else {
                continue;
            };
            for &b_idx in b_indices {
                let b = &entities_b[b_idx];
                for &c_idx in self.matching_c_indices_in(&c_by_key, b) {
                    let c = &entities_c[c_idx];
                    if (self.filter)(solution, a, b, c, a_idx, b_idx, c_idx) {
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
        let entities_c = self.extractor_c.extract(solution);

        self.build_indexes(solution, entities_a, entities_b, entities_c);

        let mut total = Sc::zero();
        for a_idx in 0..entities_a.len() {
            if !self.extractor_a.contains(solution, &entities_a[a_idx]) {
                continue;
            }
            let key = (self.key_a)(&entities_a[a_idx]);
            let b_indices = self.engine.key_indexes_for(1, &key);
            let c_indices = self.engine.key_indexes_for(2, &key);
            for b_idx in b_indices {
                for c_idx in &c_indices {
                    total = total
                        + self.add_match(
                            solution, entities_a, entities_b, entities_c, a_idx, b_idx, *c_idx,
                        );
                }
            }
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
        let c_changed = self
            .c_source
            .assert_localizes(descriptor_index, &self.constraint_ref.name);
        let mut total = Sc::zero();

        if !a_changed && !b_changed && !c_changed {
            return total;
        }

        let entities_a = self.extractor_a.extract(solution);
        let entities_b = self.extractor_b.extract(solution);
        let entities_c = self.extractor_c.extract(solution);
        if a_changed {
            total =
                total + self.insert_a(solution, entities_a, entities_b, entities_c, entity_index);
        }
        if b_changed {
            total =
                total + self.insert_b(solution, entities_a, entities_b, entities_c, entity_index);
        }
        if c_changed {
            total =
                total + self.insert_c(solution, entities_a, entities_b, entities_c, entity_index);
        }
        total
    }

    fn on_retract(&mut self, _solution: &S, entity_index: usize, descriptor_index: usize) -> Sc {
        let a_changed = self
            .a_source
            .assert_localizes(descriptor_index, &self.constraint_ref.name);
        let b_changed = self
            .b_source
            .assert_localizes(descriptor_index, &self.constraint_ref.name);
        let c_changed = self
            .c_source
            .assert_localizes(descriptor_index, &self.constraint_ref.name);
        let mut total = Sc::zero();

        if !a_changed && !b_changed && !c_changed {
            return total;
        }

        if a_changed {
            total = total + self.retract_a(entity_index);
        }
        if b_changed {
            total = total + self.retract_b(entity_index);
        }
        if c_changed {
            total = total + self.retract_c(entity_index);
        }
        total
    }

    fn reset(&mut self) {
        self.engine.clear();
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

    fn get_matches<'a>(&'a self, solution: &S) -> Vec<DetailedConstraintMatch<'a, Sc>> {
        let entities_a = self.extractor_a.extract(solution);
        let entities_b = self.extractor_b.extract(solution);
        let entities_c = self.extractor_c.extract(solution);
        let (b_by_key, c_by_key) = self.build_side_indexes(solution, entities_b, entities_c);
        let cref = self.constraint_ref();

        let mut matches = Vec::new();

        for (a_idx, a) in entities_a.iter().enumerate() {
            if !self.extractor_a.contains(solution, a) {
                continue;
            }
            let key = (self.key_a)(a);
            let Some(b_indices) = b_by_key.get(&key) else {
                continue;
            };
            for &b_idx in b_indices {
                let b = &entities_b[b_idx];
                for &c_idx in self.matching_c_indices_in(&c_by_key, b) {
                    let c = &entities_c[c_idx];
                    if (self.filter)(solution, a, b, c, a_idx, b_idx, c_idx) {
                        let entity_a = EntityRef::new(a);
                        let entity_b = EntityRef::new(b);
                        let entity_c = EntityRef::new(c);
                        let justification =
                            ConstraintJustification::new(vec![entity_a, entity_b, entity_c]);
                        let score = self.compute_score(
                            solution, entities_a, entities_b, entities_c, a_idx, b_idx, c_idx,
                        );
                        matches.push(DetailedConstraintMatch::new(cref, score, justification));
                    }
                }
            }
        }

        matches
    }
}
