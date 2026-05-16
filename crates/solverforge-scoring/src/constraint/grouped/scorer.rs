use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use super::state::GroupedStateView;

pub struct GroupedTerminalScorer<K, R, W, Sc>
where
    Sc: Score,
{
    pub(super) constraint_ref: ConstraintRef,
    pub(super) impact_type: ImpactType,
    pub(super) weight_fn: W,
    pub(super) is_hard: bool,
    pub(super) match_count: usize,
    pub(super) refresh_count: usize,
    cached_scores: Vec<Sc>,
    _phantom: PhantomData<fn() -> (K, R, Sc)>,
}

impl<K, R, W, Sc> GroupedTerminalScorer<K, R, W, Sc>
where
    R: Send + Sync + 'static,
    W: Fn(&K, &R) -> Sc + Send + Sync,
    Sc: Score + 'static,
{
    pub fn new(
        constraint_ref: ConstraintRef,
        impact_type: ImpactType,
        weight_fn: W,
        is_hard: bool,
    ) -> Self {
        Self {
            constraint_ref,
            impact_type,
            weight_fn,
            is_hard,
            match_count: 0,
            refresh_count: 0,
            cached_scores: Vec::new(),
            _phantom: PhantomData,
        }
    }

    pub fn evaluate<State>(&self, state: &State) -> Sc
    where
        State: GroupedStateView<K, R>,
    {
        let mut total = Sc::zero();
        state.for_each_group_result(|key, result| {
            total = total + self.compute_score(key, result);
        });
        total
    }

    pub fn initialize<State>(&mut self, state: &State) -> Sc
    where
        State: GroupedStateView<K, R>,
    {
        self.match_count = state.group_count();
        self.refresh_count = 0;
        self.cached_scores.clear();
        let mut total = Sc::zero();
        state.for_each_group_slot_result(|group_id, entry| {
            let score = self.score_entry(entry);
            self.replace_cached_score(group_id, score);
            total = total + score;
        });
        total
    }

    pub fn refresh_all<State>(&mut self, state: &State) -> Sc
    where
        State: GroupedStateView<K, R>,
    {
        self.match_count = state.group_count();
        self.refresh_count += 1;
        self.cached_scores.clear();
        let mut total = Sc::zero();
        state.for_each_group_slot_result(|group_id, entry| {
            let score = self.score_entry(entry);
            self.replace_cached_score(group_id, score);
            total = total + score;
        });
        total
    }

    pub fn refresh_changed<State>(&mut self, state: &State) -> Sc
    where
        State: GroupedStateView<K, R>,
    {
        self.match_count = state.group_count();
        self.refresh_count += 1;
        let mut delta = Sc::zero();
        state.for_each_changed_group_slot_result(|group_id, entry| {
            let score = self.score_entry(entry);
            delta = delta + self.replace_cached_score(group_id, score);
        });
        delta
    }

    pub fn reset(&mut self) {
        self.match_count = 0;
        self.refresh_count = 0;
        self.cached_scores.clear();
    }

    pub fn constraint_ref(&self) -> &ConstraintRef {
        &self.constraint_ref
    }

    pub fn name(&self) -> &str {
        &self.constraint_ref.name
    }

    pub fn is_hard(&self) -> bool {
        self.is_hard
    }

    pub fn match_count(&self) -> usize {
        self.match_count
    }

    pub fn refresh_count(&self) -> usize {
        self.refresh_count
    }

    #[inline]
    pub(super) fn compute_score(&self, key: &K, result: &R) -> Sc {
        let base = (self.weight_fn)(key, result);
        match self.impact_type {
            ImpactType::Penalty => -base,
            ImpactType::Reward => base,
        }
    }

    pub(crate) fn score_entry(&self, entry: Option<(&K, &R)>) -> Sc {
        match entry {
            Some((key, result)) => self.compute_score(key, result),
            None => Sc::zero(),
        }
    }

    pub(crate) fn replace_cached_score(&mut self, slot: usize, score: Sc) -> Sc {
        while self.cached_scores.len() <= slot {
            self.cached_scores.push(Sc::zero());
        }
        let previous = self.cached_scores[slot];
        self.cached_scores[slot] = score;
        score - previous
    }

    pub(crate) fn reset_incremental_cache(&mut self, match_count: usize) {
        self.match_count = match_count;
        self.refresh_count = 0;
        self.cached_scores.clear();
    }

    pub(crate) fn mark_incremental_refresh(&mut self, match_count: usize) {
        self.match_count = match_count;
        self.refresh_count += 1;
    }

    pub(crate) fn clear_incremental_scores(&mut self) {
        self.cached_scores.clear();
    }
}

impl<K, R, W, Sc> std::fmt::Debug for GroupedTerminalScorer<K, R, W, Sc>
where
    Sc: Score,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GroupedTerminalScorer")
            .field("name", &self.constraint_ref.name)
            .field("impact_type", &self.impact_type)
            .field("match_count", &self.match_count)
            .field("refresh_count", &self.refresh_count)
            .finish()
    }
}
