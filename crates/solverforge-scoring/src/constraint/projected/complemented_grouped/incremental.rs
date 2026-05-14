use std::collections::HashMap;
use std::hash::Hash;

use solverforge_core::score::Score;
use solverforge_core::ConstraintRef;

use crate::api::constraint_set::IncrementalConstraint;
use crate::stream::collection_extract::CollectionExtract;
use crate::stream::collector::{Accumulator, Collector};
use crate::stream::filter::UniFilter;
use crate::stream::ProjectedSource;

use super::ProjectedComplementedGroupedConstraint;

impl<S, Out, B, K, Src, EB, F, KA, KB, C, V, R, Acc, D, W, Sc> IncrementalConstraint<S, Sc>
    for ProjectedComplementedGroupedConstraint<
        S,
        Out,
        B,
        K,
        Src,
        EB,
        F,
        KA,
        KB,
        C,
        V,
        R,
        Acc,
        D,
        W,
        Sc,
    >
where
    S: Send + Sync + 'static,
    Out: Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    K: Clone + Eq + Hash + Send + Sync + 'static,
    Src: ProjectedSource<S, Out>,
    EB: CollectionExtract<S, Item = B>,
    F: UniFilter<S, Out>,
    KA: Fn(&Out) -> Option<K> + Send + Sync,
    KB: Fn(&B) -> K + Send + Sync,
    C: for<'i> Collector<&'i Out, Value = V, Result = R, Accumulator = Acc> + Send + Sync,
    V: Send + Sync,
    R: Send + Sync,
    Acc: Accumulator<V, R> + Send + Sync,
    D: Fn(&B) -> R + Send + Sync,
    W: Fn(&K, &R) -> Sc + Send + Sync,
    Sc: Score + 'static,
{
    fn evaluate(&self, solution: &S) -> Sc {
        let entities_b = self.extractor_b.extract(solution);
        let state = self.source.build_state(solution);
        let mut groups = HashMap::<K, Acc>::new();
        self.source.collect_all(solution, &state, |_, output| {
            if !self.filter.test(solution, &output) {
                return;
            }
            let Some(key) = (self.key_a)(&output) else {
                return;
            };
            let value = self.collector.extract(&output);
            groups
                .entry(key)
                .or_insert_with(|| self.collector.create_accumulator())
                .accumulate(value);
        });

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
        self.extractor_b.extract(solution).len()
    }

    fn initialize(&mut self, solution: &S) -> Sc {
        self.reset();
        let entities_b = self.extractor_b.extract(solution);
        let mut total = Sc::zero();
        for (idx, b) in entities_b.iter().enumerate() {
            let key = (self.key_b)(b);
            self.b_by_key.entry(key.clone()).or_default().push(idx);
            self.b_index_to_key.insert(idx, key.clone());
            let default_result = (self.default_fn)(b);
            total = total + self.compute_score(&key, &default_result);
        }

        let state = self.source.build_state(solution);
        let mut rows = Vec::new();
        self.source
            .collect_all(solution, &state, |coordinate, output| {
                rows.push((coordinate, output));
            });
        self.source_state = Some(state);
        for (coordinate, output) in rows {
            total = total + self.insert_row(solution, entities_b, coordinate, output);
        }
        total
    }

    fn on_insert(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc {
        let owners = self.localized_owners(descriptor_index, entity_index);
        let b_changed = self
            .b_source
            .assert_localizes(descriptor_index, &self.constraint_ref.name);
        let entities_b = self.extractor_b.extract(solution);
        let mut total = Sc::zero();

        if !owners.is_empty() {
            self.ensure_source_state(solution);
            {
                let state = self.source_state.as_mut().expect("projected source state");
                for owner in &owners {
                    self.source.insert_entity_state(
                        solution,
                        state,
                        owner.source_slot,
                        owner.entity_index,
                    );
                }
            }
            let mut rows = Vec::new();
            let state = self.source_state.as_ref().expect("projected source state");
            for owner in &owners {
                self.source.collect_entity(
                    solution,
                    state,
                    owner.source_slot,
                    owner.entity_index,
                    |coordinate, output| rows.push((coordinate, output)),
                );
            }
            for (coordinate, output) in rows {
                total = total + self.insert_row(solution, entities_b, coordinate, output);
            }
        }
        if b_changed {
            total = total + self.insert_b(entities_b, entity_index);
        }
        total
    }

    fn on_retract(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc {
        let owners = self.localized_owners(descriptor_index, entity_index);
        let b_changed = self
            .b_source
            .assert_localizes(descriptor_index, &self.constraint_ref.name);
        let entities_b = self.extractor_b.extract(solution);
        let mut total = Sc::zero();

        for coordinate in self.coordinates_for_owners(&owners) {
            total = total + self.retract_row(entities_b, coordinate);
        }
        if let Some(state) = self.source_state.as_mut() {
            for owner in &owners {
                self.source.retract_entity_state(
                    solution,
                    state,
                    owner.source_slot,
                    owner.entity_index,
                );
            }
        }
        if b_changed {
            total = total + self.retract_b(entities_b, entity_index);
        }
        total
    }

    fn reset(&mut self) {
        self.source_state = None;
        self.groups.clear();
        self.row_outputs.clear();
        self.row_keys.clear();
        self.row_retractions.clear();
        self.rows_by_owner.clear();
        self.b_by_key.clear();
        self.b_index_to_key.clear();
    }

    fn constraint_ref(&self) -> &ConstraintRef {
        &self.constraint_ref
    }

    fn is_hard(&self) -> bool {
        self.is_hard
    }
}
