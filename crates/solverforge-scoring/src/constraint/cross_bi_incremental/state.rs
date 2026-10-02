/* Incremental cross-bi-constraint for cross-entity join evaluation.

Zero-erasure: all closures are concrete generic types, fully monomorphized.
Retained match rows, per-source key indexes, and bucket maintenance live in
the shared arity-generic `cross_incremental` engine; this type owns only the
arity-specific extractors, filter, weight, and change localization.
*/

use std::collections::HashMap;
use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use crate::constraint::cross_incremental::CrossJoinEngine;
use crate::stream::collection_extract::{ChangeSource, CollectionExtract};

use super::weight::{CrossBiWeight, IndexWeight, PairWeight};

/* Zero-erasure incremental cross-bi-constraint.

All function types are concrete generics - no trait objects, no Arc.
*/
pub struct Bi<S, A, B, K, EA, EB, KA, KB, F, W, Sc>
where
    Sc: Score,
{
    pub(super) constraint_ref: ConstraintRef,
    pub(super) impact_type: ImpactType,
    pub(super) extractor_a: EA,
    pub(super) extractor_b: EB,
    pub(super) key_a: KA,
    pub(super) key_b: KB,
    pub(super) filter: F,
    pub(super) weight: W,
    pub(super) is_hard: bool,
    pub(super) a_source: ChangeSource,
    pub(super) b_source: ChangeSource,
    pub(super) engine: CrossJoinEngine<2, K, Sc>,
    pub(super) _phantom: PhantomData<(fn() -> S, fn() -> A, fn() -> B)>,
}

impl<S, A, B, K, EA, EB, KA, KB, F, W, Sc> Bi<S, A, B, K, EA, EB, KA, KB, F, IndexWeight<W>, Sc>
where
    S: 'static,
    A: Clone + 'static,
    B: Clone + 'static,
    K: Eq + Hash + Clone,
    EA: CollectionExtract<S, Item = A>,
    EB: CollectionExtract<S, Item = B>,
    KA: Fn(&A) -> K,
    KB: Fn(&B) -> K,
    F: Fn(&S, &A, &B, usize, usize) -> bool,
    W: Fn(&S, usize, usize) -> Sc + Send + Sync,
    Sc: Score,
{
    /* Creates a new cross-bi-constraint.

    # Arguments
    All 9 arguments are semantically distinct (2 extractors, 2 key functions,
    1 filter, 1 weight, 1 is_hard) and cannot be meaningfully grouped without losing
    higher-ranked lifetime inference for the closures.
    */
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        constraint_ref: ConstraintRef,
        impact_type: ImpactType,
        extractor_a: EA,
        extractor_b: EB,
        key_a: KA,
        key_b: KB,
        filter: F,
        weight: W,
        is_hard: bool,
    ) -> Self {
        Self::new_with_weight(
            constraint_ref,
            impact_type,
            extractor_a,
            extractor_b,
            key_a,
            key_b,
            filter,
            IndexWeight::new(weight),
            is_hard,
        )
    }
}

impl<S, A, B, K, EA, EB, KA, KB, F, W, Sc> Bi<S, A, B, K, EA, EB, KA, KB, F, PairWeight<W>, Sc>
where
    S: 'static,
    A: Clone + 'static,
    B: Clone + 'static,
    K: Eq + Hash + Clone,
    EA: CollectionExtract<S, Item = A>,
    EB: CollectionExtract<S, Item = B>,
    KA: Fn(&A) -> K,
    KB: Fn(&B) -> K,
    F: Fn(&S, &A, &B, usize, usize) -> bool,
    W: Fn(&A, &B) -> Sc + Send + Sync,
    Sc: Score,
{
    #[allow(clippy::too_many_arguments)]
    pub fn new_pair_weight(
        constraint_ref: ConstraintRef,
        impact_type: ImpactType,
        extractor_a: EA,
        extractor_b: EB,
        key_a: KA,
        key_b: KB,
        filter: F,
        weight: W,
        is_hard: bool,
    ) -> Self {
        Self::new_with_weight(
            constraint_ref,
            impact_type,
            extractor_a,
            extractor_b,
            key_a,
            key_b,
            filter,
            PairWeight::new(weight),
            is_hard,
        )
    }
}

impl<S, A, B, K, EA, EB, KA, KB, F, W, Sc> Bi<S, A, B, K, EA, EB, KA, KB, F, W, Sc>
where
    S: 'static,
    A: Clone + 'static,
    B: Clone + 'static,
    K: Eq + Hash + Clone,
    EA: CollectionExtract<S, Item = A>,
    EB: CollectionExtract<S, Item = B>,
    KA: Fn(&A) -> K,
    KB: Fn(&B) -> K,
    F: Fn(&S, &A, &B, usize, usize) -> bool,
    W: CrossBiWeight<S, A, B, Sc>,
    Sc: Score,
{
    #[allow(clippy::too_many_arguments)]
    fn new_with_weight(
        constraint_ref: ConstraintRef,
        impact_type: ImpactType,
        extractor_a: EA,
        extractor_b: EB,
        key_a: KA,
        key_b: KB,
        filter: F,
        weight: W,
        is_hard: bool,
    ) -> Self {
        let a_source = extractor_a.change_source();
        let b_source = extractor_b.change_source();
        Self {
            constraint_ref,
            impact_type,
            extractor_a,
            extractor_b,
            key_a,
            key_b,
            filter,
            weight,
            is_hard,
            a_source,
            b_source,
            engine: CrossJoinEngine::new(),
            _phantom: PhantomData,
        }
    }

    #[inline]
    pub(super) fn compute_score(
        &self,
        solution: &S,
        entities_a: &[A],
        entities_b: &[B],
        a_idx: usize,
        b_idx: usize,
    ) -> Sc {
        let base = self
            .weight
            .score(solution, entities_a, entities_b, a_idx, b_idx);
        match self.impact_type {
            ImpactType::Penalty => -base,
            ImpactType::Reward => base,
        }
    }

    pub(super) fn b_index_for(&self, solution: &S, entities_b: &[B]) -> HashMap<K, Vec<usize>> {
        let mut b_by_key: HashMap<K, Vec<usize>> = HashMap::new();
        for (b_idx, b) in entities_b.iter().enumerate() {
            if !self.extractor_b.contains(solution, b) {
                continue;
            }
            let key = (self.key_b)(b);
            b_by_key.entry(key).or_default().push(b_idx);
        }
        b_by_key
    }

    #[inline]
    pub(super) fn matching_b_indices_in<'a>(
        &self,
        b_by_key: &'a HashMap<K, Vec<usize>>,
        a: &A,
    ) -> &'a [usize] {
        let key = (self.key_a)(a);
        b_by_key.get(&key).map(|v| v.as_slice()).unwrap_or(&[])
    }

    pub(super) fn build_indexes(&mut self, solution: &S, entities_a: &[A], entities_b: &[B]) {
        self.engine.clear();
        for (a_idx, a) in entities_a.iter().enumerate() {
            if !self.extractor_a.contains(solution, a) {
                continue;
            }
            let key = (self.key_a)(a);
            self.engine.insert_source_index(0, a_idx, key);
        }
        for (b_idx, b) in entities_b.iter().enumerate() {
            if !self.extractor_b.contains(solution, b) {
                continue;
            }
            let key = (self.key_b)(b);
            self.engine.insert_source_index(1, b_idx, key);
        }
    }

    pub(super) fn add_match(
        &mut self,
        solution: &S,
        entities_a: &[A],
        entities_b: &[B],
        a_idx: usize,
        b_idx: usize,
    ) -> Sc {
        let tuple = [a_idx, b_idx];
        if self.engine.contains(&tuple) {
            return Sc::zero();
        }
        let a = &entities_a[a_idx];
        let b = &entities_b[b_idx];
        if !self.extractor_a.contains(solution, a) || !self.extractor_b.contains(solution, b) {
            return Sc::zero();
        }
        if !(self.filter)(solution, a, b, a_idx, b_idx) {
            return Sc::zero();
        }
        let score = self.compute_score(solution, entities_a, entities_b, a_idx, b_idx);
        self.engine.add_row(tuple, score)
    }

    pub(super) fn insert_a(
        &mut self,
        solution: &S,
        entities_a: &[A],
        entities_b: &[B],
        a_idx: usize,
    ) -> Sc {
        if a_idx >= entities_a.len() {
            return Sc::zero();
        }

        let a = &entities_a[a_idx];
        if !self.extractor_a.contains(solution, a) {
            return Sc::zero();
        }
        let key = (self.key_a)(a);
        self.engine.insert_source_index(0, a_idx, key.clone());

        let mut total = Sc::zero();
        for b_idx in self.engine.key_indexes_for(1, &key) {
            total = total + self.add_match(solution, entities_a, entities_b, a_idx, b_idx);
        }

        total
    }

    pub(super) fn retract_a(&mut self, a_idx: usize) -> Sc {
        self.engine.remove_source_index(0, a_idx);
        let mut total = Sc::zero();
        for row_idx in self.engine.row_indexes_for(0, a_idx) {
            total = total + self.engine.remove_row_at(row_idx);
        }
        total
    }

    pub(super) fn insert_b(
        &mut self,
        solution: &S,
        entities_a: &[A],
        entities_b: &[B],
        b_idx: usize,
    ) -> Sc {
        if b_idx >= entities_b.len() {
            return Sc::zero();
        }

        let b = &entities_b[b_idx];
        if !self.extractor_b.contains(solution, b) {
            return Sc::zero();
        }
        let key = (self.key_b)(b);
        self.engine.insert_source_index(1, b_idx, key.clone());

        let mut total = Sc::zero();
        for a_idx in self.engine.key_indexes_for(0, &key) {
            total = total + self.add_match(solution, entities_a, entities_b, a_idx, b_idx);
        }
        total
    }

    pub(super) fn retract_b(&mut self, b_idx: usize) -> Sc {
        self.engine.remove_source_index(1, b_idx);
        let mut total = Sc::zero();
        for row_idx in self.engine.row_indexes_for(1, b_idx) {
            total = total + self.engine.remove_row_at(row_idx);
        }

        total
    }
}
