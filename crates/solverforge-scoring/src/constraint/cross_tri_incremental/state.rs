/* Incremental cross-tri-constraint for three-source join evaluation.

Zero-erasure: all closures are concrete generic types, fully monomorphized.
A retained (A, B, C) row requires key_a(a) == key_b(b) == key_c(c) plus the
tri filter. Retained rows, per-source key indexes, and bucket maintenance
live in the shared arity-generic `cross_incremental` engine; this type owns
the arity-specific extractors, filter, weight, and change localization.
*/

use std::collections::HashMap;
use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use crate::constraint::cross_incremental::CrossJoinEngine;
use crate::stream::collection_extract::{ChangeSource, CollectionExtract};

use super::weight::{CrossTriWeight, IndexWeight, TripleWeight};

/* Zero-erasure incremental cross-tri-constraint.

All function types are concrete generics - no trait objects, no Arc.
*/
pub struct Tri<S, A, B, C, K, EA, EB, EC, KA, KB, KC, F, W, Sc>
where
    Sc: Score,
{
    pub(super) constraint_ref: ConstraintRef,
    pub(super) impact_type: ImpactType,
    pub(super) extractor_a: EA,
    pub(super) extractor_b: EB,
    pub(super) extractor_c: EC,
    pub(super) key_a: KA,
    pub(super) key_b: KB,
    pub(super) key_c: KC,
    pub(super) filter: F,
    pub(super) weight: W,
    pub(super) is_hard: bool,
    pub(super) a_source: ChangeSource,
    pub(super) b_source: ChangeSource,
    pub(super) c_source: ChangeSource,
    pub(super) engine: CrossJoinEngine<3, K, Sc>,
    pub(super) _phantom: PhantomData<(fn() -> S, fn() -> A, fn() -> B, fn() -> C)>,
}

impl<S, A, B, C, K, EA, EB, EC, KA, KB, KC, F, W, Sc>
    Tri<S, A, B, C, K, EA, EB, EC, KA, KB, KC, F, IndexWeight<W>, Sc>
where
    S: 'static,
    A: Clone + 'static,
    B: Clone + 'static,
    C: Clone + 'static,
    K: Eq + Hash + Clone,
    EA: CollectionExtract<S, Item = A>,
    EB: CollectionExtract<S, Item = B>,
    EC: CollectionExtract<S, Item = C>,
    KA: Fn(&A) -> K,
    KB: Fn(&B) -> K,
    KC: Fn(&C) -> K,
    F: Fn(&S, &A, &B, &C, usize, usize, usize) -> bool,
    W: Fn(&S, usize, usize, usize) -> Sc + Send + Sync,
    Sc: Score,
{
    /* Creates a new cross-tri-constraint.

    # Arguments
    All 12 arguments are semantically distinct (3 extractors, 3 key functions,
    1 filter, 1 weight, 1 is_hard) and cannot be meaningfully grouped without
    losing higher-ranked lifetime inference for the closures.
    */
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        constraint_ref: ConstraintRef,
        impact_type: ImpactType,
        extractor_a: EA,
        extractor_b: EB,
        extractor_c: EC,
        key_a: KA,
        key_b: KB,
        key_c: KC,
        filter: F,
        weight: W,
        is_hard: bool,
    ) -> Self {
        Self::new_with_weight(
            constraint_ref,
            impact_type,
            extractor_a,
            extractor_b,
            extractor_c,
            key_a,
            key_b,
            key_c,
            filter,
            IndexWeight::new(weight),
            is_hard,
        )
    }
}

impl<S, A, B, C, K, EA, EB, EC, KA, KB, KC, F, W, Sc>
    Tri<S, A, B, C, K, EA, EB, EC, KA, KB, KC, F, TripleWeight<W>, Sc>
where
    S: 'static,
    A: Clone + 'static,
    B: Clone + 'static,
    C: Clone + 'static,
    K: Eq + Hash + Clone,
    EA: CollectionExtract<S, Item = A>,
    EB: CollectionExtract<S, Item = B>,
    EC: CollectionExtract<S, Item = C>,
    KA: Fn(&A) -> K,
    KB: Fn(&B) -> K,
    KC: Fn(&C) -> K,
    F: Fn(&S, &A, &B, &C, usize, usize, usize) -> bool,
    W: Fn(&A, &B, &C) -> Sc + Send + Sync,
    Sc: Score,
{
    #[allow(clippy::too_many_arguments)]
    pub fn new_triple_weight(
        constraint_ref: ConstraintRef,
        impact_type: ImpactType,
        extractor_a: EA,
        extractor_b: EB,
        extractor_c: EC,
        key_a: KA,
        key_b: KB,
        key_c: KC,
        filter: F,
        weight: W,
        is_hard: bool,
    ) -> Self {
        Self::new_with_weight(
            constraint_ref,
            impact_type,
            extractor_a,
            extractor_b,
            extractor_c,
            key_a,
            key_b,
            key_c,
            filter,
            TripleWeight::new(weight),
            is_hard,
        )
    }
}

impl<S, A, B, C, K, EA, EB, EC, KA, KB, KC, F, W, Sc>
    Tri<S, A, B, C, K, EA, EB, EC, KA, KB, KC, F, W, Sc>
where
    S: 'static,
    A: Clone + 'static,
    B: Clone + 'static,
    C: Clone + 'static,
    K: Eq + Hash + Clone,
    EA: CollectionExtract<S, Item = A>,
    EB: CollectionExtract<S, Item = B>,
    EC: CollectionExtract<S, Item = C>,
    KA: Fn(&A) -> K,
    KB: Fn(&B) -> K,
    KC: Fn(&C) -> K,
    F: Fn(&S, &A, &B, &C, usize, usize, usize) -> bool,
    W: CrossTriWeight<S, A, B, C, Sc>,
    Sc: Score,
{
    #[allow(clippy::too_many_arguments)]
    fn new_with_weight(
        constraint_ref: ConstraintRef,
        impact_type: ImpactType,
        extractor_a: EA,
        extractor_b: EB,
        extractor_c: EC,
        key_a: KA,
        key_b: KB,
        key_c: KC,
        filter: F,
        weight: W,
        is_hard: bool,
    ) -> Self {
        let a_source = extractor_a.change_source();
        let b_source = extractor_b.change_source();
        let c_source = extractor_c.change_source();
        Self {
            constraint_ref,
            impact_type,
            extractor_a,
            extractor_b,
            extractor_c,
            key_a,
            key_b,
            key_c,
            filter,
            weight,
            is_hard,
            a_source,
            b_source,
            c_source,
            engine: CrossJoinEngine::new(),
            _phantom: PhantomData,
        }
    }

    #[inline]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn compute_score(
        &self,
        solution: &S,
        entities_a: &[A],
        entities_b: &[B],
        entities_c: &[C],
        a_idx: usize,
        b_idx: usize,
        c_idx: usize,
    ) -> Sc {
        let base = self.weight.score(
            solution, entities_a, entities_b, entities_c, a_idx, b_idx, c_idx,
        );
        match self.impact_type {
            ImpactType::Penalty => -base,
            ImpactType::Reward => base,
        }
    }

    // Indexes sources b and c for full-rebuild evaluation paths, mirroring
    // the bi constraint's b_index_for.
    pub(super) fn build_side_indexes(
        &self,
        solution: &S,
        entities_b: &[B],
        entities_c: &[C],
    ) -> (HashMap<K, Vec<usize>>, HashMap<K, Vec<usize>>) {
        let mut b_by_key: HashMap<K, Vec<usize>> = HashMap::new();
        for (b_idx, b) in entities_b.iter().enumerate() {
            if !self.extractor_b.contains(solution, b) {
                continue;
            }
            let key = (self.key_b)(b);
            b_by_key.entry(key).or_default().push(b_idx);
        }
        let mut c_by_key: HashMap<K, Vec<usize>> = HashMap::new();
        for (c_idx, c) in entities_c.iter().enumerate() {
            if !self.extractor_c.contains(solution, c) {
                continue;
            }
            let key = (self.key_c)(c);
            c_by_key.entry(key).or_default().push(c_idx);
        }
        (b_by_key, c_by_key)
    }

    #[inline]
    pub(super) fn matching_c_indices_in<'a>(
        &self,
        c_by_key: &'a HashMap<K, Vec<usize>>,
        b: &B,
    ) -> &'a [usize] {
        let key = (self.key_b)(b);
        c_by_key.get(&key).map(|v| v.as_slice()).unwrap_or(&[])
    }

    pub(super) fn build_indexes(
        &mut self,
        solution: &S,
        entities_a: &[A],
        entities_b: &[B],
        entities_c: &[C],
    ) {
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
        for (c_idx, c) in entities_c.iter().enumerate() {
            if !self.extractor_c.contains(solution, c) {
                continue;
            }
            let key = (self.key_c)(c);
            self.engine.insert_source_index(2, c_idx, key);
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn add_match(
        &mut self,
        solution: &S,
        entities_a: &[A],
        entities_b: &[B],
        entities_c: &[C],
        a_idx: usize,
        b_idx: usize,
        c_idx: usize,
    ) -> Sc {
        let tuple = [a_idx, b_idx, c_idx];
        if self.engine.contains(&tuple) {
            return Sc::zero();
        }
        let a = &entities_a[a_idx];
        let b = &entities_b[b_idx];
        let c = &entities_c[c_idx];
        if !self.extractor_a.contains(solution, a)
            || !self.extractor_b.contains(solution, b)
            || !self.extractor_c.contains(solution, c)
        {
            return Sc::zero();
        }
        if !(self.filter)(solution, a, b, c, a_idx, b_idx, c_idx) {
            return Sc::zero();
        }
        let score = self.compute_score(
            solution, entities_a, entities_b, entities_c, a_idx, b_idx, c_idx,
        );
        self.engine.add_row(tuple, score)
    }

    pub(super) fn insert_a(
        &mut self,
        solution: &S,
        entities_a: &[A],
        entities_b: &[B],
        entities_c: &[C],
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
        // (a, b, c) rows: probe b by key, then c by key, both retained.
        for b_idx in self.engine.key_indexes_for(1, &key) {
            for c_idx in self.engine.key_indexes_for(2, &key) {
                total = total
                    + self.add_match(
                        solution, entities_a, entities_b, entities_c, a_idx, b_idx, c_idx,
                    );
            }
        }

        total
    }

    pub(super) fn retract_a(&mut self, a_idx: usize) -> Sc {
        self.engine.remove_source_index(0, a_idx);
        let mut total = Sc::zero();
        for handle in self.engine.row_indexes_for(0, a_idx) {
            total = total + self.engine.remove_row_at(handle);
        }
        total
    }

    pub(super) fn insert_b(
        &mut self,
        solution: &S,
        entities_a: &[A],
        entities_b: &[B],
        entities_c: &[C],
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
            for c_idx in self.engine.key_indexes_for(2, &key) {
                total = total
                    + self.add_match(
                        solution, entities_a, entities_b, entities_c, a_idx, b_idx, c_idx,
                    );
            }
        }
        total
    }

    pub(super) fn retract_b(&mut self, b_idx: usize) -> Sc {
        self.engine.remove_source_index(1, b_idx);
        let mut total = Sc::zero();
        for handle in self.engine.row_indexes_for(1, b_idx) {
            total = total + self.engine.remove_row_at(handle);
        }
        total
    }

    pub(super) fn insert_c(
        &mut self,
        solution: &S,
        entities_a: &[A],
        entities_b: &[B],
        entities_c: &[C],
        c_idx: usize,
    ) -> Sc {
        if c_idx >= entities_c.len() {
            return Sc::zero();
        }

        let c = &entities_c[c_idx];
        if !self.extractor_c.contains(solution, c) {
            return Sc::zero();
        }
        let key = (self.key_c)(c);
        self.engine.insert_source_index(2, c_idx, key.clone());

        let mut total = Sc::zero();
        for a_idx in self.engine.key_indexes_for(0, &key) {
            for b_idx in self.engine.key_indexes_for(1, &key) {
                total = total
                    + self.add_match(
                        solution, entities_a, entities_b, entities_c, a_idx, b_idx, c_idx,
                    );
            }
        }
        total
    }

    pub(super) fn retract_c(&mut self, c_idx: usize) -> Sc {
        self.engine.remove_source_index(2, c_idx);
        let mut total = Sc::zero();
        for handle in self.engine.row_indexes_for(2, c_idx) {
            total = total + self.engine.remove_row_at(handle);
        }
        total
    }
}
