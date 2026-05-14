use std::collections::{hash_map::Entry, HashMap};
use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use crate::stream::collection_extract::{ChangeSource, CollectionExtract};
use crate::stream::collector::{Accumulator, Collector};

type CollectorRetraction<Acc, V, R> = <Acc as Accumulator<V, R>>::Retraction;

struct GroupState<Acc> {
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

pub struct CrossGroupedConstraint<S, A, B, JK, GK, EA, EB, KA, KB, F, GF, C, V, R, Acc, W, Sc>
where
    Acc: Accumulator<V, R>,
    Sc: Score,
{
    pub(super) constraint_ref: ConstraintRef,
    pub(super) impact_type: ImpactType,
    pub(super) extractor_a: EA,
    pub(super) extractor_b: EB,
    pub(super) key_a: KA,
    pub(super) key_b: KB,
    pub(super) filter: F,
    pub(super) group_key_fn: GF,
    pub(super) collector: C,
    pub(super) weight_fn: W,
    pub(super) is_hard: bool,
    pub(super) a_source: ChangeSource,
    pub(super) b_source: ChangeSource,
    pub(super) matches: HashMap<(usize, usize), usize>,
    pub(super) match_rows: Vec<MatchRow<GK, CollectorRetraction<Acc, V, R>>>,
    pub(super) a_to_matches: HashMap<usize, Vec<usize>>,
    pub(super) b_to_matches: HashMap<usize, Vec<usize>>,
    pub(super) a_by_key: HashMap<JK, Vec<usize>>,
    pub(super) b_by_key: HashMap<JK, Vec<usize>>,
    pub(super) a_index_to_key: HashMap<usize, JK>,
    pub(super) b_index_to_key: HashMap<usize, JK>,
    groups: HashMap<GK, GroupState<Acc>>,
    pub(super) _phantom: PhantomData<(
        fn() -> S,
        fn() -> A,
        fn() -> B,
        fn() -> V,
        fn() -> R,
        fn() -> Acc,
        fn() -> Sc,
    )>,
}

impl<S, A, B, JK, GK, EA, EB, KA, KB, F, GF, C, V, R, Acc, W, Sc>
    CrossGroupedConstraint<S, A, B, JK, GK, EA, EB, KA, KB, F, GF, C, V, R, Acc, W, Sc>
where
    S: 'static,
    A: Clone + 'static,
    B: Clone + 'static,
    JK: Clone + Eq + Hash,
    GK: Clone + Eq + Hash,
    EA: CollectionExtract<S, Item = A>,
    EB: CollectionExtract<S, Item = B>,
    KA: Fn(&A) -> JK,
    KB: Fn(&B) -> JK,
    F: Fn(&S, &A, &B) -> bool,
    GF: Fn(&A, &B) -> GK,
    C: for<'i> Collector<(&'i A, &'i B), Value = V, Result = R, Accumulator = Acc>,
    Acc: Accumulator<V, R>,
    W: Fn(&GK, &R) -> Sc,
    Sc: Score,
{
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        constraint_ref: ConstraintRef,
        impact_type: ImpactType,
        extractor_a: EA,
        extractor_b: EB,
        key_a: KA,
        key_b: KB,
        filter: F,
        group_key_fn: GF,
        collector: C,
        weight_fn: W,
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
            group_key_fn,
            collector,
            weight_fn,
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

    pub(super) fn build_indexes(&mut self, entities_a: &[A], entities_b: &[B]) {
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

    pub(super) fn insert_value(
        &mut self,
        key: GK,
        value: V,
    ) -> (Sc, CollectorRetraction<Acc, V, R>) {
        let impact = self.impact_type;
        let weight_fn = &self.weight_fn;
        match self.groups.entry(key) {
            Entry::Occupied(mut entry) => {
                let old_base = entry
                    .get()
                    .accumulator
                    .with_result(|result| weight_fn(entry.key(), result));
                let old = match impact {
                    ImpactType::Penalty => -old_base,
                    ImpactType::Reward => old_base,
                };
                let group = entry.get_mut();
                let retraction = group.accumulator.accumulate(value);
                group.count += 1;
                let new_base = entry
                    .get()
                    .accumulator
                    .with_result(|result| weight_fn(entry.key(), result));
                let new_score = match impact {
                    ImpactType::Penalty => -new_base,
                    ImpactType::Reward => new_base,
                };
                (new_score - old, retraction)
            }
            Entry::Vacant(entry) => {
                let mut entry = entry.insert_entry(GroupState {
                    accumulator: self.collector.create_accumulator(),
                    count: 0,
                });
                let group = entry.get_mut();
                let retraction = group.accumulator.accumulate(value);
                group.count += 1;
                let new_base = entry
                    .get()
                    .accumulator
                    .with_result(|result| weight_fn(entry.key(), result));
                let score = match impact {
                    ImpactType::Penalty => -new_base,
                    ImpactType::Reward => new_base,
                };
                (score, retraction)
            }
        }
    }

    pub(super) fn retract_value(
        &mut self,
        key: GK,
        retraction: CollectorRetraction<Acc, V, R>,
    ) -> Sc {
        let impact = self.impact_type;
        let weight_fn = &self.weight_fn;
        let Entry::Occupied(mut entry) = self.groups.entry(key) else {
            return Sc::zero();
        };
        let old_base = entry
            .get()
            .accumulator
            .with_result(|result| weight_fn(entry.key(), result));
        let old = match impact {
            ImpactType::Penalty => -old_base,
            ImpactType::Reward => old_base,
        };
        let group = entry.get_mut();
        group.accumulator.retract(retraction);
        group.count = group.count.saturating_sub(1);
        let new_score = if group.count == 0 {
            entry.remove();
            Sc::zero()
        } else {
            let new_base = entry
                .get()
                .accumulator
                .with_result(|result| weight_fn(entry.key(), result));
            match impact {
                ImpactType::Penalty => -new_base,
                ImpactType::Reward => new_base,
            }
        };
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
        self.groups.clear();
    }
}
