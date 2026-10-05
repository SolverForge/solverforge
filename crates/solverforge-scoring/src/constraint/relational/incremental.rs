use std::fmt::Debug;
use std::hash::Hash;

use crate::api::analysis::{ConstraintJustification, DetailedConstraintMatch, EntityRef};
use crate::api::constraint_set::IncrementalConstraint;
use crate::stream::collection_extract::CollectionExtract;
use crate::stream::relational::DeltaKind;
use solverforge_core::score::Score;
use solverforge_core::ConstraintRef;

use super::state::Terminal;
use super::weight::RelationalWeight;

impl<S, A, B, K, EA, EB, KA, KB, F, W, Sc> IncrementalConstraint<S, Sc>
    for Terminal<S, A, B, K, EA, EB, KA, KB, F, W, Sc>
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
    W: RelationalWeight<S, A, B, Sc>,
    Sc: Score,
{
    /* Full evaluation without retained state: meaningful uninitialized. */
    fn evaluate(&self, solution: &S) -> Sc {
        let entities_a = self.operator.left_extractor().extract(solution);
        let entities_b = self.operator.right_extractor().extract(solution);
        let filter = self.operator.filter();
        let mut total = Sc::zero();

        // Independent nested loops over accepted entities: the oracle
        // shape, not production indexes, so evaluate stays a check on the
        // retained paths rather than a copy of them.
        for (a_idx, a) in entities_a.iter().enumerate() {
            if !self.operator.left_extractor().contains(solution, a) {
                continue;
            }
            for (b_idx, b) in entities_b.iter().enumerate() {
                if !self.operator.right_extractor().contains(solution, b) {
                    continue;
                }
                // Key equality is the operator's relationship; evaluate
                // re-derives it directly instead of reusing indexes.
                if !(filter)(solution, a, b, a_idx, b_idx) {
                    continue;
                }
                if self.keys_equal(a, b) {
                    let base = self
                        .weight
                        .score(solution, entities_a, entities_b, a_idx, b_idx);
                    total = total + self.signed(base);
                }
            }
        }

        total
    }

    fn match_count(&self, solution: &S) -> usize {
        let entities_a = self.operator.left_extractor().extract(solution);
        let entities_b = self.operator.right_extractor().extract(solution);
        let filter = self.operator.filter();
        let mut count = 0;

        for (a_idx, a) in entities_a.iter().enumerate() {
            if !self.operator.left_extractor().contains(solution, a) {
                continue;
            }
            for (b_idx, b) in entities_b.iter().enumerate() {
                if !self.operator.right_extractor().contains(solution, b) {
                    continue;
                }
                if self.keys_equal(a, b) && (filter)(solution, a, b, a_idx, b_idx) {
                    count += 1;
                }
            }
        }

        count
    }

    fn initialize(&mut self, solution: &S) -> Sc {
        self.reset();
        self.operator.refresh(solution);
        self.initialized = true;

        let entities_a = self.operator.left_extractor().extract(solution);
        let entities_b = self.operator.right_extractor().extract(solution);
        let mut total = Sc::zero();
        for (a_idx, b_idx) in self.operator.output_rows() {
            let base = self
                .weight
                .score(solution, entities_a, entities_b, a_idx, b_idx);
            let signed = self.signed(base);
            self.pair_scores.insert((a_idx, b_idx), signed);
            total = total + signed;
        }
        total
    }

    fn on_insert(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc {
        let mut total = Sc::zero();
        for delta in self
            .operator
            .on_insert_source(solution, descriptor_index, entity_index)
        {
            if delta.kind != DeltaKind::Insert {
                continue;
            }
            let pair = (delta.left_idx, delta.right_idx);
            let entities_a = self.operator.left_extractor().extract(solution);
            let entities_b = self.operator.right_extractor().extract(solution);
            let base = self
                .weight
                .score(solution, entities_a, entities_b, pair.0, pair.1);
            let signed = self.signed(base);
            self.pair_scores.insert(pair, signed);
            total = total + signed;
        }
        total
    }

    fn on_retract(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc {
        let _ = solution;
        let mut total = Sc::zero();
        for delta in self
            .operator
            .on_retract_source(descriptor_index, entity_index)
        {
            if delta.kind != DeltaKind::Retract {
                continue;
            }
            let pair = (delta.left_idx, delta.right_idx);
            // Retract the PREVIOUSLY computed score: a changed key or value
            // must not recompute from new state here.
            if let Some(score) = self.pair_scores.remove(&pair) {
                total = total + (-score);
            }
        }
        total
    }

    fn reset(&mut self) {
        self.pair_scores.clear();
        self.operator.clear_outputs();
        self.initialized = false;
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
        let entities_a = self.operator.left_extractor().extract(solution);
        let entities_b = self.operator.right_extractor().extract(solution);
        let cref = self.constraint_ref();
        let mut matches = Vec::new();

        // Initialized explanations walk retained operator provenance in
        // authored binding order: the binding sequence IS the tuple
        // orientation, and retained pairs are exactly the scored rows.
        // Uninitialized explanations recompute with identical orientation,
        // matching the checked-in contract that matches work before init.
        if !self.initialized {
            return self.uninitialized_matches(solution);
        }
        for (a_idx, b_idx) in self.operator.output_rows() {
            let Some(provenance) = self.operator.provenance_of(a_idx, b_idx) else {
                continue;
            };
            let (Some(a), Some(b)) = (entities_a.get(a_idx), entities_b.get(b_idx)) else {
                continue;
            };
            let mut entities = Vec::with_capacity(provenance.bindings().len());
            for part in provenance.bindings() {
                // Binding order is authored tuple orientation: 0 is left,
                // 1 is right. Descriptors ride along for future derived
                // rows whose entities are not slice-addressable.
                let entity = match part.binding.0 {
                    0 => EntityRef::new(a),
                    _ => EntityRef::new(b),
                };
                entities.push(entity);
            }
            let justification = ConstraintJustification::new(entities);
            let base = self
                .weight
                .score(solution, entities_a, entities_b, a_idx, b_idx);
            matches.push(DetailedConstraintMatch::new(
                cref,
                self.signed(base),
                justification,
            ));
        }

        matches
    }
}

impl<S, A, B, K, EA, EB, KA, KB, F, W, Sc> Terminal<S, A, B, K, EA, EB, KA, KB, F, W, Sc>
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
    W: RelationalWeight<S, A, B, Sc>,
    Sc: Score,
{
    /* Key equality re-derived for stateless paths. */
    fn keys_equal(&self, a: &A, b: &B) -> bool {
        (self.operator.left_key())(a) == (self.operator.right_key())(b)
    }

    /* Stateless match enumeration with binary binding orientation. */
    fn uninitialized_matches(&self, solution: &S) -> Vec<DetailedConstraintMatch<'_, Sc>> {
        let entities_a = self.operator.left_extractor().extract(solution);
        let entities_b = self.operator.right_extractor().extract(solution);
        let filter = self.operator.filter();
        let cref = self.constraint_ref();
        let mut matches = Vec::new();

        for (a_idx, a) in entities_a.iter().enumerate() {
            if !self.operator.left_extractor().contains(solution, a) {
                continue;
            }
            for (b_idx, b) in entities_b.iter().enumerate() {
                if !self.operator.right_extractor().contains(solution, b) {
                    continue;
                }
                if !(filter)(solution, a, b, a_idx, b_idx) {
                    continue;
                }
                if !self.keys_equal(a, b) {
                    continue;
                }
                let justification =
                    ConstraintJustification::new(vec![EntityRef::new(a), EntityRef::new(b)]);
                let base = self
                    .weight
                    .score(solution, entities_a, entities_b, a_idx, b_idx);
                matches.push(DetailedConstraintMatch::new(
                    cref,
                    self.signed(base),
                    justification,
                ));
            }
        }

        matches
    }
}
