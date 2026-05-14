use std::fmt::Debug;
use std::hash::Hash;

use crate::api::analysis::{ConstraintJustification, DetailedConstraintMatch, EntityRef};
use crate::api::constraint_set::IncrementalConstraint;
use crate::stream::collection_extract::CollectionExtract;
use solverforge_core::score::Score;
use solverforge_core::ConstraintRef;

use super::{CrossBiWeight, IncrementalCrossBiConstraint};

impl<S, A, B, K, EA, EB, KA, KB, F, W, Sc> IncrementalConstraint<S, Sc>
    for IncrementalCrossBiConstraint<S, A, B, K, EA, EB, KA, KB, F, W, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Debug + Send + Sync + 'static,
    B: Clone + Debug + Send + Sync + 'static,
    K: Eq + Hash + Clone + Send + Sync,
    EA: CollectionExtract<S, Item = A> + Send + Sync,
    EB: CollectionExtract<S, Item = B> + Send + Sync,
    KA: Fn(&A) -> K + Send + Sync,
    KB: Fn(&B) -> K + Send + Sync,
    F: Fn(&S, &A, &B, usize, usize) -> bool + Send + Sync,
    W: CrossBiWeight<S, A, B, Sc>,
    Sc: Score,
{
    fn evaluate(&self, solution: &S) -> Sc {
        let entities_a = self.extractor_a.extract(solution);
        let entities_b = self.extractor_b.extract(solution);
        let b_by_key = self.b_index_for(entities_b);
        let mut total = Sc::zero();

        for (a_idx, a) in entities_a.iter().enumerate() {
            for &b_idx in self.matching_b_indices_in(&b_by_key, a) {
                let b = &entities_b[b_idx];
                if (self.filter)(solution, a, b, a_idx, b_idx) {
                    total =
                        total + self.compute_score(solution, entities_a, entities_b, a_idx, b_idx);
                }
            }
        }

        total
    }

    fn match_count(&self, solution: &S) -> usize {
        let entities_a = self.extractor_a.extract(solution);
        let entities_b = self.extractor_b.extract(solution);
        let b_by_key = self.b_index_for(entities_b);
        let mut count = 0;

        for (a_idx, a) in entities_a.iter().enumerate() {
            for &b_idx in self.matching_b_indices_in(&b_by_key, a) {
                let b = &entities_b[b_idx];
                if (self.filter)(solution, a, b, a_idx, b_idx) {
                    count += 1;
                }
            }
        }

        count
    }

    fn initialize(&mut self, solution: &S) -> Sc {
        self.reset();

        let entities_a = self.extractor_a.extract(solution);
        let entities_b = self.extractor_b.extract(solution);

        self.build_indexes(entities_a, entities_b);

        let mut total = Sc::zero();
        for a_idx in 0..entities_a.len() {
            let key = (self.key_a)(&entities_a[a_idx]);
            let b_indices = self.b_by_key.get(&key).cloned().unwrap_or_default();
            for b_idx in b_indices {
                total = total + self.add_match(solution, entities_a, entities_b, a_idx, b_idx);
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
        let mut total = Sc::zero();

        if !a_changed && !b_changed {
            return total;
        }

        let entities_a = self.extractor_a.extract(solution);
        let entities_b = self.extractor_b.extract(solution);
        if a_changed {
            total = total + self.insert_a(solution, entities_a, entities_b, entity_index);
        }
        if b_changed {
            total = total + self.insert_b(solution, entities_a, entities_b, entity_index);
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
        let mut total = Sc::zero();

        if !a_changed && !b_changed {
            return total;
        }

        if a_changed {
            total = total + self.retract_a(entity_index);
        }
        if b_changed {
            total = total + self.retract_b(entity_index);
        }
        total
    }

    fn reset(&mut self) {
        self.matches.clear();
        self.match_rows.clear();
        self.a_to_matches.clear();
        self.b_to_matches.clear();
        self.a_by_key.clear();
        self.b_by_key.clear();
        self.a_index_to_key.clear();
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

    fn get_matches<'a>(&'a self, solution: &S) -> Vec<DetailedConstraintMatch<'a, Sc>> {
        let entities_a = self.extractor_a.extract(solution);
        let entities_b = self.extractor_b.extract(solution);
        let b_by_key = self.b_index_for(entities_b);
        let cref = self.constraint_ref();

        let mut matches = Vec::new();

        for (a_idx, a) in entities_a.iter().enumerate() {
            for &b_idx in self.matching_b_indices_in(&b_by_key, a) {
                let b = &entities_b[b_idx];
                if (self.filter)(solution, a, b, a_idx, b_idx) {
                    let entity_a = EntityRef::new(a);
                    let entity_b = EntityRef::new(b);
                    let justification = ConstraintJustification::new(vec![entity_a, entity_b]);
                    let score = self.compute_score(solution, entities_a, entities_b, a_idx, b_idx);
                    matches.push(DetailedConstraintMatch::new(cref, score, justification));
                }
            }
        }

        matches
    }
}
