use std::hash::Hash;
use std::marker::PhantomData;
use std::slice;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use crate::api::constraint_set::IncrementalConstraint;
use crate::stream::collection_extract::{ChangeSource, TrackedCollectionExtract};
use crate::stream::filter::UniFilter;
use crate::stream::{ExistenceMode, FlattenExtract};

mod key_state;

use key_state::ExistsKeyState;
#[cfg(test)]
pub(crate) use key_state::ExistsStorageKind;

#[derive(Debug, Clone)]
struct ASlot<K, Sc>
where
    Sc: Score,
{
    key: Option<K>,
    bucket_pos: usize,
    contribution: Sc,
}

impl<K, Sc> Default for ASlot<K, Sc>
where
    Sc: Score,
{
    fn default() -> Self {
        Self {
            key: None,
            bucket_pos: 0,
            contribution: Sc::zero(),
        }
    }
}

pub struct IncrementalExistsConstraint<S, A, P, B, K, EA, EP, KA, KB, FA, FP, Flatten, W, Sc>
where
    Sc: Score,
{
    constraint_ref: ConstraintRef,
    impact_type: ImpactType,
    mode: ExistenceMode,
    extractor_a: EA,
    extractor_parent: EP,
    key_a: KA,
    key_b: KB,
    filter_a: FA,
    filter_parent: FP,
    flatten: Flatten,
    weight: W,
    is_hard: bool,
    a_source: ChangeSource,
    parent_source: ChangeSource,
    a_slots: Vec<ASlot<K, Sc>>,
    key_state: ExistsKeyState<K>,
    _phantom: PhantomData<(fn() -> S, fn() -> A, fn() -> P, fn() -> B)>,
}

impl<S, A, P, B, K, EA, EP, KA, KB, FA, FP, Flatten, W, Sc>
    IncrementalExistsConstraint<S, A, P, B, K, EA, EP, KA, KB, FA, FP, Flatten, W, Sc>
where
    S: 'static,
    A: Clone + 'static,
    P: Clone + 'static,
    B: Clone + 'static,
    K: Eq + Hash + Clone + 'static,
    EA: TrackedCollectionExtract<S, Item = A>,
    EP: TrackedCollectionExtract<S, Item = P>,
    KA: Fn(&A) -> K,
    KB: Fn(&B) -> K,
    FA: UniFilter<S, A>,
    FP: UniFilter<S, P>,
    Flatten: FlattenExtract<P, Item = B>,
    W: Fn(&A) -> Sc,
    Sc: Score,
{
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        constraint_ref: ConstraintRef,
        impact_type: ImpactType,
        mode: ExistenceMode,
        extractor_a: EA,
        extractor_parent: EP,
        key_a: KA,
        key_b: KB,
        filter_a: FA,
        filter_parent: FP,
        flatten: Flatten,
        weight: W,
        is_hard: bool,
    ) -> Self {
        let a_source = extractor_a.change_source();
        let parent_source = extractor_parent.change_source();
        Self {
            constraint_ref,
            impact_type,
            mode,
            extractor_a,
            extractor_parent,
            key_a,
            key_b,
            filter_a,
            filter_parent,
            flatten,
            weight,
            is_hard,
            a_source,
            parent_source,
            a_slots: Vec::new(),
            key_state: ExistsKeyState::new(),
            _phantom: PhantomData,
        }
    }

    #[cfg(test)]
    pub(crate) fn storage_kind(&self) -> ExistsStorageKind {
        self.key_state.storage_kind()
    }

    #[inline]
    fn compute_score(&self, a: &A) -> Sc {
        let base = (self.weight)(a);
        match self.impact_type {
            ImpactType::Penalty => -base,
            ImpactType::Reward => base,
        }
    }

    #[inline]
    fn matches_existence(&self, key: &K) -> bool {
        self.matches_count(self.key_state.b_count(key))
    }

    #[inline]
    fn matches_count(&self, count: usize) -> bool {
        match self.mode {
            ExistenceMode::Exists => count > 0,
            ExistenceMode::NotExists => count == 0,
        }
    }

    fn rebuild_b_counts(&mut self, solution: &S) {
        self.key_state.clear_b_counts();
        for parent in self.extractor_parent.extract(solution) {
            if !self.filter_parent.test(solution, parent) {
                continue;
            }
            for item in self.flatten.extract(parent) {
                let key = (self.key_b)(item);
                self.key_state.increment_b_count(&key, 1);
            }
        }
    }

    fn remove_a_from_bucket(&mut self, idx: usize, key: &K, bucket_pos: usize) {
        if let Some(moved) = self.key_state.remove_a_index(key, idx, bucket_pos) {
            self.a_slots[moved.idx].bucket_pos = moved.bucket_pos;
        }
    }

    fn retract_a(&mut self, idx: usize) -> Sc {
        if idx >= self.a_slots.len() {
            return Sc::zero();
        }
        let slot = self.a_slots[idx].clone();
        let Some(key) = slot.key.clone() else {
            return Sc::zero();
        };
        self.remove_a_from_bucket(idx, &key, slot.bucket_pos);
        self.a_slots[idx] = ASlot::default();
        -slot.contribution
    }

    fn insert_a(&mut self, solution: &S, idx: usize) -> Sc {
        let entities_a = self.extractor_a.extract(solution);
        if idx >= entities_a.len() {
            return Sc::zero();
        }
        if self.a_slots.len() < entities_a.len() {
            self.a_slots.resize(entities_a.len(), ASlot::default());
        }

        let a = &entities_a[idx];
        if !self.filter_a.test(solution, a) {
            self.a_slots[idx] = ASlot::default();
            return Sc::zero();
        }

        let key = (self.key_a)(a);
        let bucket_pos = self.key_state.insert_a_index(key.clone(), idx);

        let contribution = if self.matches_existence(&key) {
            self.compute_score(a)
        } else {
            Sc::zero()
        };

        self.a_slots[idx] = ASlot {
            key: Some(key),
            bucket_pos,
            contribution,
        };
        contribution
    }

    fn reevaluate_key(&mut self, solution: &S, key: &K) -> Sc {
        let indices = self.key_state.a_indices(key);
        if indices.is_empty() {
            return Sc::zero();
        };
        let entities_a = self.extractor_a.extract(solution);
        let mut total = Sc::zero();
        let exists = self.matches_existence(key);

        for idx in indices {
            let a = &entities_a[idx];
            let new_contribution = if exists {
                self.compute_score(a)
            } else {
                Sc::zero()
            };
            let old_contribution = self.a_slots[idx].contribution;
            self.a_slots[idx].contribution = new_contribution;
            total = total + (new_contribution - old_contribution);
        }

        total
    }

    fn update_key_counts(&mut self, solution: &S, key_counts: &[(K, usize)], insert: bool) -> Sc {
        let mut total = Sc::zero();

        for (key, count) in key_counts {
            if insert {
                self.key_state.increment_b_count(key, *count);
            } else {
                self.key_state.decrement_b_count(key, *count);
            }
        }

        for (key, _) in key_counts {
            total = total + self.reevaluate_key(solution, key);
        }

        total
    }

    fn parent_key_counts(&self, solution: &S, idx: usize) -> Vec<(K, usize)> {
        let parents = self.extractor_parent.extract(solution);
        if idx >= parents.len() {
            return Vec::new();
        }
        let parent = &parents[idx];
        if !self.filter_parent.test(solution, parent) {
            return Vec::new();
        }

        let mut key_counts = Vec::<(K, usize)>::new();
        for item in self.flatten.extract(parent) {
            let key = (self.key_b)(item);
            if let Some((_, count)) = key_counts
                .iter_mut()
                .find(|(existing_key, _)| existing_key == &key)
            {
                *count += 1;
            } else {
                key_counts.push((key, 1));
            }
        }
        key_counts
    }

    fn initialize_a_state(&mut self, solution: &S) -> Sc {
        self.a_slots.clear();
        self.key_state.clear_a_buckets();

        let len = self.extractor_a.extract(solution).len();
        self.a_slots.resize(len, ASlot::default());

        let mut total = Sc::zero();
        for idx in 0..len {
            total = total + self.insert_a(solution, idx);
        }
        total
    }

    fn build_b_counts(&self, solution: &S) -> ExistsKeyState<K> {
        let mut key_state = ExistsKeyState::new();
        for parent in self.extractor_parent.extract(solution) {
            if !self.filter_parent.test(solution, parent) {
                continue;
            }
            for item in self.flatten.extract(parent) {
                let key = (self.key_b)(item);
                key_state.increment_b_count(&key, 1);
            }
        }
        key_state
    }

    fn full_match_count(&self, solution: &S) -> usize {
        let key_state = self.build_b_counts(solution);

        self.extractor_a
            .extract(solution)
            .iter()
            .filter(|a| {
                if !self.filter_a.test(solution, a) {
                    return false;
                }
                let key = (self.key_a)(a);
                self.matches_count(key_state.b_count(&key))
            })
            .count()
    }
}

impl<S, A, P, B, K, EA, EP, KA, KB, FA, FP, Flatten, W, Sc> IncrementalConstraint<S, Sc>
    for IncrementalExistsConstraint<S, A, P, B, K, EA, EP, KA, KB, FA, FP, Flatten, W, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    P: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    K: Eq + Hash + Clone + Send + Sync + 'static,
    EA: TrackedCollectionExtract<S, Item = A> + Send + Sync,
    EP: TrackedCollectionExtract<S, Item = P> + Send + Sync,
    KA: Fn(&A) -> K + Send + Sync,
    KB: Fn(&B) -> K + Send + Sync,
    FA: UniFilter<S, A> + Send + Sync,
    FP: UniFilter<S, P> + Send + Sync,
    Flatten: FlattenExtract<P, Item = B> + Send + Sync,
    W: Fn(&A) -> Sc + Send + Sync,
    Sc: Score,
{
    fn evaluate(&self, solution: &S) -> Sc {
        let key_state = self.build_b_counts(solution);

        let mut total = Sc::zero();
        for a in self.extractor_a.extract(solution) {
            if !self.filter_a.test(solution, a) {
                continue;
            }
            let key = (self.key_a)(a);
            if self.matches_count(key_state.b_count(&key)) {
                total = total + self.compute_score(a);
            }
        }
        total
    }

    fn match_count(&self, solution: &S) -> usize {
        self.full_match_count(solution)
    }

    fn initialize(&mut self, solution: &S) -> Sc {
        self.reset();
        self.rebuild_b_counts(solution);
        self.initialize_a_state(solution)
    }

    fn on_insert(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc {
        let a_changed =
            matches!(self.a_source, ChangeSource::Descriptor(idx) if idx == descriptor_index);
        let parent_changed =
            matches!(self.parent_source, ChangeSource::Descriptor(idx) if idx == descriptor_index);
        let same_source = self.a_source == self.parent_source && a_changed && parent_changed;

        let mut total = Sc::zero();
        if same_source {
            let keys = self.parent_key_counts(solution, entity_index);
            total = total + self.update_key_counts(solution, &keys, true);
            total = total + self.insert_a(solution, entity_index);
            return total;
        }

        if parent_changed {
            let keys = self.parent_key_counts(solution, entity_index);
            total = total + self.update_key_counts(solution, &keys, true);
        }
        if a_changed {
            total = total + self.insert_a(solution, entity_index);
        }
        total
    }

    fn on_retract(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc {
        let a_changed =
            matches!(self.a_source, ChangeSource::Descriptor(idx) if idx == descriptor_index);
        let parent_changed =
            matches!(self.parent_source, ChangeSource::Descriptor(idx) if idx == descriptor_index);
        let same_source = self.a_source == self.parent_source && a_changed && parent_changed;

        let mut total = Sc::zero();
        if same_source {
            let keys = self.parent_key_counts(solution, entity_index);
            total = total + self.retract_a(entity_index);
            total = total + self.update_key_counts(solution, &keys, false);
            return total;
        }

        if a_changed {
            total = total + self.retract_a(entity_index);
        }
        if parent_changed {
            let keys = self.parent_key_counts(solution, entity_index);
            total = total + self.update_key_counts(solution, &keys, false);
        }
        total
    }

    fn reset(&mut self) {
        self.a_slots.clear();
        self.key_state.clear_a_buckets();
        self.key_state.clear_b_counts();
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
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SelfFlatten;

impl<T> FlattenExtract<T> for SelfFlatten
where
    T: Send + Sync,
{
    type Item = T;

    fn extract<'a>(&self, parent: &'a T) -> &'a [T] {
        slice::from_ref(parent)
    }
}
