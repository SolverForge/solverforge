/* Incremental cross-bi-constraint for cross-entity join evaluation.

Zero-erasure: all closures are concrete generic types, fully monomorphized.
*/

use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use crate::api::analysis::{ConstraintJustification, DetailedConstraintMatch, EntityRef};
use crate::api::constraint_set::IncrementalConstraint;
use crate::stream::collection_extract::{ChangeSource, CollectionExtract};

pub trait CrossBiWeight<S, A, B, Sc>: Send + Sync {
    fn score(
        &self,
        solution: &S,
        entities_a: &[A],
        entities_b: &[B],
        a_idx: usize,
        b_idx: usize,
    ) -> Sc;
}

pub struct IndexWeight<W>(W);

impl<W> IndexWeight<W> {
    #[inline]
    fn new(weight: W) -> Self {
        Self(weight)
    }
}

impl<S, A, B, W, Sc> CrossBiWeight<S, A, B, Sc> for IndexWeight<W>
where
    W: Fn(&S, usize, usize) -> Sc + Send + Sync,
{
    #[inline]
    fn score(
        &self,
        solution: &S,
        _entities_a: &[A],
        _entities_b: &[B],
        a_idx: usize,
        b_idx: usize,
    ) -> Sc {
        (self.0)(solution, a_idx, b_idx)
    }
}

pub struct PairWeight<W>(W);

impl<W> PairWeight<W> {
    #[inline]
    fn new(weight: W) -> Self {
        Self(weight)
    }
}

impl<S, A, B, W, Sc> CrossBiWeight<S, A, B, Sc> for PairWeight<W>
where
    W: Fn(&A, &B) -> Sc + Send + Sync,
{
    #[inline]
    fn score(
        &self,
        _solution: &S,
        entities_a: &[A],
        entities_b: &[B],
        a_idx: usize,
        b_idx: usize,
    ) -> Sc {
        (self.0)(&entities_a[a_idx], &entities_b[b_idx])
    }
}

#[derive(Clone)]
struct MatchRow<Sc>
where
    Sc: Score,
{
    pair: (usize, usize),
    score: Sc,
    a_pos: usize,
    b_pos: usize,
}

/* Zero-erasure incremental cross-bi-constraint.

All function types are concrete generics - no trait objects, no Arc.
*/
pub struct IncrementalCrossBiConstraint<S, A, B, K, EA, EB, KA, KB, F, W, Sc>
where
    Sc: Score,
{
    constraint_ref: ConstraintRef,
    impact_type: ImpactType,
    extractor_a: EA,
    extractor_b: EB,
    key_a: KA,
    key_b: KB,
    filter: F,
    weight: W,
    is_hard: bool,
    a_source: ChangeSource,
    b_source: ChangeSource,
    matches: HashMap<(usize, usize), usize>,
    match_rows: Vec<MatchRow<Sc>>,
    a_to_matches: HashMap<usize, Vec<usize>>,
    b_to_matches: HashMap<usize, Vec<usize>>,
    a_by_key: HashMap<K, Vec<usize>>,
    b_by_key: HashMap<K, Vec<usize>>,
    a_index_to_key: HashMap<usize, K>,
    b_index_to_key: HashMap<usize, K>,
    _phantom: PhantomData<(fn() -> S, fn() -> A, fn() -> B)>,
}

impl<S, A, B, K, EA, EB, KA, KB, F, W, Sc>
    IncrementalCrossBiConstraint<S, A, B, K, EA, EB, KA, KB, F, IndexWeight<W>, Sc>
where
    S: 'static,
    A: Clone + 'static,
    B: Clone + 'static,
    K: Eq + Hash + Clone,
    EA: CollectionExtract<S, Item = A>,
    EB: CollectionExtract<S, Item = B>,
    KA: Fn(&A) -> K,
    KB: Fn(&B) -> K,
    F: Fn(&S, &A, &B) -> bool,
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

impl<S, A, B, K, EA, EB, KA, KB, F, W, Sc>
    IncrementalCrossBiConstraint<S, A, B, K, EA, EB, KA, KB, F, PairWeight<W>, Sc>
where
    S: 'static,
    A: Clone + 'static,
    B: Clone + 'static,
    K: Eq + Hash + Clone,
    EA: CollectionExtract<S, Item = A>,
    EB: CollectionExtract<S, Item = B>,
    KA: Fn(&A) -> K,
    KB: Fn(&B) -> K,
    F: Fn(&S, &A, &B) -> bool,
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

impl<S, A, B, K, EA, EB, KA, KB, F, W, Sc>
    IncrementalCrossBiConstraint<S, A, B, K, EA, EB, KA, KB, F, W, Sc>
where
    S: 'static,
    A: Clone + 'static,
    B: Clone + 'static,
    K: Eq + Hash + Clone,
    EA: CollectionExtract<S, Item = A>,
    EB: CollectionExtract<S, Item = B>,
    KA: Fn(&A) -> K,
    KB: Fn(&B) -> K,
    F: Fn(&S, &A, &B) -> bool,
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
            matches: HashMap::new(),
            match_rows: Vec::new(),
            a_to_matches: HashMap::new(),
            b_to_matches: HashMap::new(),
            a_by_key: HashMap::new(),
            b_by_key: HashMap::new(),
            a_index_to_key: HashMap::new(),
            b_index_to_key: HashMap::new(),
            _phantom: PhantomData,
        }
    }

    #[inline]
    fn compute_score(
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

    fn b_index_for(&self, entities_b: &[B]) -> HashMap<K, Vec<usize>> {
        let mut b_by_key: HashMap<K, Vec<usize>> = HashMap::new();
        for (b_idx, b) in entities_b.iter().enumerate() {
            let key = (self.key_b)(b);
            b_by_key.entry(key).or_default().push(b_idx);
        }
        b_by_key
    }

    fn build_indexes(&mut self, entities_a: &[A], entities_b: &[B]) {
        self.a_by_key.clear();
        self.b_by_key.clear();
        self.a_index_to_key.clear();
        self.b_index_to_key.clear();
        for (a_idx, a) in entities_a.iter().enumerate() {
            let key = (self.key_a)(a);
            self.a_index_to_key.insert(a_idx, key.clone());
            self.a_by_key.entry(key).or_default().push(a_idx);
        }
        for (b_idx, b) in entities_b.iter().enumerate() {
            let key = (self.key_b)(b);
            self.b_index_to_key.insert(b_idx, key.clone());
            self.b_by_key.entry(key).or_default().push(b_idx);
        }
    }

    #[inline]
    fn matching_b_indices_in<'a>(
        &self,
        b_by_key: &'a HashMap<K, Vec<usize>>,
        a: &A,
    ) -> &'a [usize] {
        let key = (self.key_a)(a);
        b_by_key.get(&key).map(|v| v.as_slice()).unwrap_or(&[])
    }

    fn add_match(
        &mut self,
        solution: &S,
        entities_a: &[A],
        entities_b: &[B],
        a_idx: usize,
        b_idx: usize,
    ) -> Sc {
        let pair = (a_idx, b_idx);
        if self.matches.contains_key(&pair) {
            return Sc::zero();
        }
        let a = &entities_a[a_idx];
        let b = &entities_b[b_idx];
        if !(self.filter)(solution, a, b) {
            return Sc::zero();
        }
        let score = self.compute_score(solution, entities_a, entities_b, a_idx, b_idx);
        let row_idx = self.match_rows.len();
        let a_bucket = self.a_to_matches.entry(a_idx).or_default();
        let a_pos = a_bucket.len();
        a_bucket.push(row_idx);
        let b_bucket = self.b_to_matches.entry(b_idx).or_default();
        let b_pos = b_bucket.len();
        b_bucket.push(row_idx);
        self.match_rows.push(MatchRow {
            pair,
            score,
            a_pos,
            b_pos,
        });
        self.matches.insert(pair, row_idx);
        score
    }

    fn remove_match_at(&mut self, row_idx: usize) -> Sc {
        if row_idx >= self.match_rows.len() {
            return Sc::zero();
        }

        let row = self.match_rows[row_idx].clone();
        self.matches.remove(&row.pair);
        self.remove_from_a_bucket(row.pair.0, row_idx, row.a_pos);
        self.remove_from_b_bucket(row.pair.1, row_idx, row.b_pos);

        let last_idx = self.match_rows.len() - 1;
        self.match_rows.swap_remove(row_idx);
        if row_idx != last_idx {
            let moved = self.match_rows[row_idx].clone();
            self.matches.insert(moved.pair, row_idx);
            if let Some(a_matches) = self.a_to_matches.get_mut(&moved.pair.0) {
                a_matches[moved.a_pos] = row_idx;
            }
            if let Some(b_matches) = self.b_to_matches.get_mut(&moved.pair.1) {
                b_matches[moved.b_pos] = row_idx;
            }
        }

        -row.score
    }

    fn remove_from_a_bucket(&mut self, a_idx: usize, row_idx: usize, pos: usize) {
        let mut remove_bucket = false;
        if let Some(a_matches) = self.a_to_matches.get_mut(&a_idx) {
            debug_assert_eq!(a_matches[pos], row_idx);
            a_matches.swap_remove(pos);
            if pos < a_matches.len() {
                let moved_row_idx = a_matches[pos];
                self.match_rows[moved_row_idx].a_pos = pos;
            }
            remove_bucket = a_matches.is_empty();
        }
        if remove_bucket {
            self.a_to_matches.remove(&a_idx);
        }
    }

    fn remove_from_b_bucket(&mut self, b_idx: usize, row_idx: usize, pos: usize) {
        let mut remove_bucket = false;
        if let Some(b_matches) = self.b_to_matches.get_mut(&b_idx) {
            debug_assert_eq!(b_matches[pos], row_idx);
            b_matches.swap_remove(pos);
            if pos < b_matches.len() {
                let moved_row_idx = b_matches[pos];
                self.match_rows[moved_row_idx].b_pos = pos;
            }
            remove_bucket = b_matches.is_empty();
        }
        if remove_bucket {
            self.b_to_matches.remove(&b_idx);
        }
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

    fn insert_a(&mut self, solution: &S, entities_a: &[A], entities_b: &[B], a_idx: usize) -> Sc {
        if a_idx >= entities_a.len() {
            return Sc::zero();
        }

        let a = &entities_a[a_idx];
        let key = (self.key_a)(a);
        self.a_index_to_key.insert(a_idx, key.clone());
        self.a_by_key.entry(key.clone()).or_default().push(a_idx);

        let b_indices = self.b_by_key.get(&key).cloned().unwrap_or_default();

        let mut total = Sc::zero();
        for b_idx in b_indices {
            total = total + self.add_match(solution, entities_a, entities_b, a_idx, b_idx);
        }

        total
    }

    fn retract_a(&mut self, a_idx: usize) -> Sc {
        if let Some(key) = self.a_index_to_key.remove(&a_idx) {
            Self::remove_index_from_key_bucket(&mut self.a_by_key, &key, a_idx);
        }
        let mut total = Sc::zero();
        while let Some(row_idx) = self
            .a_to_matches
            .get(&a_idx)
            .and_then(|matches| matches.last())
            .copied()
        {
            total = total + self.remove_match_at(row_idx);
        }
        total
    }

    fn insert_b(&mut self, solution: &S, entities_a: &[A], entities_b: &[B], b_idx: usize) -> Sc {
        if b_idx >= entities_b.len() {
            return Sc::zero();
        }

        let b = &entities_b[b_idx];
        let key = (self.key_b)(b);
        self.b_index_to_key.insert(b_idx, key.clone());
        self.b_by_key.entry(key.clone()).or_default().push(b_idx);

        let a_indices = self.a_by_key.get(&key).cloned().unwrap_or_default();
        let mut total = Sc::zero();
        for a_idx in a_indices {
            total = total + self.add_match(solution, entities_a, entities_b, a_idx, b_idx);
        }
        total
    }

    fn retract_b(&mut self, b_idx: usize) -> Sc {
        if let Some(key) = self.b_index_to_key.remove(&b_idx) {
            Self::remove_index_from_key_bucket(&mut self.b_by_key, &key, b_idx);
        }
        let mut total = Sc::zero();
        while let Some(row_idx) = self
            .b_to_matches
            .get(&b_idx)
            .and_then(|matches| matches.last())
            .copied()
        {
            total = total + self.remove_match_at(row_idx);
        }

        total
    }
}

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
    F: Fn(&S, &A, &B) -> bool + Send + Sync,
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
                if (self.filter)(solution, a, b) {
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

        for a in entities_a {
            for &b_idx in self.matching_b_indices_in(&b_by_key, a) {
                let b = &entities_b[b_idx];
                if (self.filter)(solution, a, b) {
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

        if !a_changed && !b_changed {
            return Sc::zero();
        }

        let entities_a = self.extractor_a.extract(solution);
        let entities_b = self.extractor_b.extract(solution);
        let mut total = Sc::zero();
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

        if !a_changed && !b_changed {
            return Sc::zero();
        }

        let mut total = Sc::zero();
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

    fn constraint_ref(&self) -> ConstraintRef {
        self.constraint_ref.clone()
    }

    fn get_matches(&self, solution: &S) -> Vec<DetailedConstraintMatch<Sc>> {
        let entities_a = self.extractor_a.extract(solution);
        let entities_b = self.extractor_b.extract(solution);
        let b_by_key = self.b_index_for(entities_b);
        let cref = self.constraint_ref.clone();

        let mut matches = Vec::new();

        for (a_idx, a) in entities_a.iter().enumerate() {
            for &b_idx in self.matching_b_indices_in(&b_by_key, a) {
                let b = &entities_b[b_idx];
                if (self.filter)(solution, a, b) {
                    let entity_a = EntityRef::new(a);
                    let entity_b = EntityRef::new(b);
                    let justification = ConstraintJustification::new(vec![entity_a, entity_b]);
                    let score = self.compute_score(solution, entities_a, entities_b, a_idx, b_idx);
                    matches.push(DetailedConstraintMatch::new(
                        cref.clone(),
                        score,
                        justification,
                    ));
                }
            }
        }

        matches
    }
}

impl<S, A, B, K, EA, EB, KA, KB, F, W, Sc: Score> std::fmt::Debug
    for IncrementalCrossBiConstraint<S, A, B, K, EA, EB, KA, KB, F, W, Sc>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IncrementalCrossBiConstraint")
            .field("name", &self.constraint_ref.name)
            .field("impact_type", &self.impact_type)
            .field("match_count", &self.matches.len())
            .finish()
    }
}
