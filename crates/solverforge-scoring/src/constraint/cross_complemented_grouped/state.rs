use std::collections::{hash_map::Entry, HashMap};
use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use crate::stream::collection_extract::{ChangeSource, CollectionExtract};
use crate::stream::collector::{Accumulator, Collector};

type CollectorRetraction<Acc, V, R> = <Acc as Accumulator<V, R>>::Retraction;

pub(super) struct GroupState<Acc> {
    accumulator: Acc,
    count: usize,
}

pub(super) struct MatchRow<GK, Retraction> {
    pub(super) pair: (usize, usize),
    pub(super) group_key: GK,
    pub(super) retraction: Retraction,
    pub(super) a_pos: usize,
    pub(super) b_pos: usize,
}

pub struct CrossComplementedGroupedConstraint<
    S,
    A,
    B,
    T,
    JK,
    GK,
    EA,
    EB,
    ET,
    KA,
    KB,
    F,
    GF,
    KT,
    C,
    V,
    R,
    Acc,
    D,
    W,
    Sc,
> where
    Acc: Accumulator<V, R>,
    Sc: Score,
{
    pub(super) constraint_ref: ConstraintRef,
    pub(super) impact_type: ImpactType,
    pub(super) extractor_a: EA,
    pub(super) extractor_b: EB,
    pub(super) extractor_t: ET,
    pub(super) key_a: KA,
    pub(super) key_b: KB,
    pub(super) filter: F,
    pub(super) group_key_fn: GF,
    pub(super) key_t: KT,
    pub(super) collector: C,
    pub(super) default_fn: D,
    pub(super) weight_fn: W,
    pub(super) is_hard: bool,
    pub(super) a_source: ChangeSource,
    pub(super) b_source: ChangeSource,
    pub(super) t_source: ChangeSource,
    pub(super) matches: HashMap<(usize, usize), usize>,
    pub(super) match_rows: Vec<MatchRow<GK, CollectorRetraction<Acc, V, R>>>,
    pub(super) a_to_matches: HashMap<usize, Vec<usize>>,
    pub(super) b_to_matches: HashMap<usize, Vec<usize>>,
    pub(super) a_by_key: HashMap<JK, Vec<usize>>,
    pub(super) b_by_key: HashMap<JK, Vec<usize>>,
    pub(super) a_index_to_key: HashMap<usize, JK>,
    pub(super) b_index_to_key: HashMap<usize, JK>,
    pub(super) t_by_key: HashMap<GK, Vec<usize>>,
    pub(super) t_index_to_key: HashMap<usize, GK>,
    groups: HashMap<GK, GroupState<Acc>>,
    pub(super) _phantom: PhantomData<(
        fn() -> S,
        fn() -> A,
        fn() -> B,
        fn() -> T,
        fn() -> V,
        fn() -> R,
        fn() -> Acc,
        fn() -> Sc,
    )>,
}

impl<S, A, B, T, JK, GK, EA, EB, ET, KA, KB, F, GF, KT, C, V, R, Acc, D, W, Sc>
    CrossComplementedGroupedConstraint<
        S,
        A,
        B,
        T,
        JK,
        GK,
        EA,
        EB,
        ET,
        KA,
        KB,
        F,
        GF,
        KT,
        C,
        V,
        R,
        Acc,
        D,
        W,
        Sc,
    >
where
    S: 'static,
    A: Clone + 'static,
    B: Clone + 'static,
    T: Clone + 'static,
    JK: Clone + Eq + Hash,
    GK: Clone + Eq + Hash,
    EA: CollectionExtract<S, Item = A>,
    EB: CollectionExtract<S, Item = B>,
    ET: CollectionExtract<S, Item = T>,
    KA: Fn(&A) -> JK,
    KB: Fn(&B) -> JK,
    F: Fn(&S, &A, &B, usize, usize) -> bool,
    GF: Fn(&A, &B) -> GK,
    KT: Fn(&T) -> GK,
    C: for<'i> Collector<(&'i A, &'i B), Value = V, Result = R, Accumulator = Acc>,
    Acc: Accumulator<V, R>,
    D: Fn(&T) -> R,
    W: Fn(&GK, &R) -> Sc,
    Sc: Score,
{
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        constraint_ref: ConstraintRef,
        impact_type: ImpactType,
        extractor_a: EA,
        extractor_b: EB,
        extractor_t: ET,
        key_a: KA,
        key_b: KB,
        filter: F,
        group_key_fn: GF,
        key_t: KT,
        collector: C,
        default_fn: D,
        weight_fn: W,
        is_hard: bool,
    ) -> Self {
        let a_source = extractor_a.change_source();
        let b_source = extractor_b.change_source();
        let t_source = extractor_t.change_source();
        Self {
            constraint_ref,
            impact_type,
            extractor_a,
            extractor_b,
            extractor_t,
            key_a,
            key_b,
            filter,
            group_key_fn,
            key_t,
            collector,
            default_fn,
            weight_fn,
            is_hard,
            a_source,
            b_source,
            t_source,
            matches: HashMap::new(),
            match_rows: Vec::new(),
            a_to_matches: HashMap::new(),
            b_to_matches: HashMap::new(),
            a_by_key: HashMap::new(),
            b_by_key: HashMap::new(),
            a_index_to_key: HashMap::new(),
            b_index_to_key: HashMap::new(),
            t_by_key: HashMap::new(),
            t_index_to_key: HashMap::new(),
            groups: HashMap::new(),
            _phantom: PhantomData,
        }
    }

    #[inline]
    pub(super) fn compute_score(&self, key: &GK, result: &R) -> Sc {
        let base = (self.weight_fn)(key, result);
        match self.impact_type {
            ImpactType::Penalty => -base,
            ImpactType::Reward => base,
        }
    }

    pub(super) fn b_index_for(&self, entities_b: &[B]) -> HashMap<JK, Vec<usize>> {
        let mut b_by_key = HashMap::<JK, Vec<usize>>::new();
        for (b_idx, b) in entities_b.iter().enumerate() {
            let key = (self.key_b)(b);
            b_by_key.entry(key).or_default().push(b_idx);
        }
        b_by_key
    }

    pub(super) fn build_join_indexes(&mut self, entities_a: &[A], entities_b: &[B]) {
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
    pub(super) fn matching_b_indices_in<'a>(
        &self,
        b_by_key: &'a HashMap<JK, Vec<usize>>,
        a: &A,
    ) -> &'a [usize] {
        let key = (self.key_a)(a);
        b_by_key.get(&key).map_or(&[], Vec::as_slice)
    }

    pub(super) fn complement_score_for_index(
        &self,
        entities_t: &[T],
        key: &GK,
        t_idx: usize,
    ) -> Sc {
        if t_idx >= entities_t.len() {
            return Sc::zero();
        }
        if let Some(group) = self.groups.get(key) {
            return group
                .accumulator
                .with_result(|result| self.compute_score(key, result));
        }
        let default_result = (self.default_fn)(&entities_t[t_idx]);
        self.compute_score(key, &default_result)
    }

    pub(super) fn key_score(&self, entities_t: &[T], key: &GK) -> Sc {
        let Some(indices) = self.t_by_key.get(key) else {
            return Sc::zero();
        };
        indices.iter().fold(Sc::zero(), |total, &t_idx| {
            total + self.complement_score_for_index(entities_t, key, t_idx)
        })
    }

    fn remove_index_from_key_bucket(
        indexes_by_key: &mut HashMap<GK, Vec<usize>>,
        key: &GK,
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

    pub(super) fn index_complement(&mut self, key: GK, t_idx: usize) {
        if let Some(old_key) = self.t_index_to_key.insert(t_idx, key.clone()) {
            Self::remove_index_from_key_bucket(&mut self.t_by_key, &old_key, t_idx);
        }
        self.t_by_key.entry(key).or_default().push(t_idx);
    }

    pub(super) fn insert_value(
        &mut self,
        entities_t: &[T],
        key: GK,
        value: V,
    ) -> (Sc, CollectorRetraction<Acc, V, R>) {
        let old = self.key_score(entities_t, &key);
        let retraction = match self.groups.entry(key.clone()) {
            Entry::Occupied(mut entry) => {
                let group = entry.get_mut();
                let retraction = group.accumulator.accumulate(value);
                group.count += 1;
                retraction
            }
            Entry::Vacant(entry) => {
                let group = entry.insert(GroupState {
                    accumulator: self.collector.create_accumulator(),
                    count: 0,
                });
                let retraction = group.accumulator.accumulate(value);
                group.count += 1;
                retraction
            }
        };
        let new_score = self.key_score(entities_t, &key);
        (new_score - old, retraction)
    }

    pub(super) fn retract_value(
        &mut self,
        entities_t: &[T],
        key: GK,
        retraction: CollectorRetraction<Acc, V, R>,
    ) -> Sc {
        let old = self.key_score(entities_t, &key);
        let Entry::Occupied(mut entry) = self.groups.entry(key.clone()) else {
            return Sc::zero();
        };
        let group = entry.get_mut();
        group.accumulator.retract(retraction);
        group.count = group.count.saturating_sub(1);
        if group.count == 0 {
            entry.remove();
        }
        let new_score = self.key_score(entities_t, &key);
        new_score - old
    }

    pub(super) fn clear_state(&mut self) {
        self.matches.clear();
        self.match_rows.clear();
        self.a_to_matches.clear();
        self.b_to_matches.clear();
        self.a_by_key.clear();
        self.b_by_key.clear();
        self.a_index_to_key.clear();
        self.b_index_to_key.clear();
        self.t_by_key.clear();
        self.t_index_to_key.clear();
        self.groups.clear();
    }
}
