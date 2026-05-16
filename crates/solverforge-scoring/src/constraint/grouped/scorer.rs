use std::collections::HashMap;
use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use super::state::GroupedStateView;

pub struct GroupedTerminalScorer<K, R, W, Sc>
where
    Sc: Score,
{
    constraint_ref: ConstraintRef,
    impact_type: ImpactType,
    weight_fn: W,
    is_hard: bool,
    prior_scores: HashMap<K, Sc>,
    match_count: usize,
    refresh_count: usize,
    _phantom: PhantomData<fn() -> R>,
}

impl<K, R, W, Sc> GroupedTerminalScorer<K, R, W, Sc>
where
    K: Clone + Eq + Hash + Send + Sync + 'static,
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
            prior_scores: HashMap::new(),
            match_count: 0,
            refresh_count: 0,
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
        self.prior_scores.clear();
        self.match_count = 0;
        let mut total = Sc::zero();
        state.for_each_group_result(|key, result| {
            let score = self.compute_score(key, result);
            self.prior_scores.insert(key.clone(), score);
            self.match_count += 1;
            total = total + score;
        });
        total
    }

    pub fn refresh_changed_keys<State>(&mut self, state: &State, changed_keys: &[K]) -> Sc
    where
        State: GroupedStateView<K, R>,
    {
        let mut delta = Sc::zero();
        for key in changed_keys {
            let old = self.prior_scores.remove(key).unwrap_or_else(Sc::zero);
            let maybe_new = state.with_group_result(
                key,
                |result| Some(self.compute_score(key, result)),
                || None,
            );
            let new_score = maybe_new.unwrap_or_else(Sc::zero);
            if let Some(score) = maybe_new {
                self.prior_scores.insert(key.clone(), score);
            }
            delta = delta + (new_score - old);
            self.refresh_count += 1;
        }
        self.match_count = self.prior_scores.len();
        delta
    }

    pub fn reset(&mut self) {
        self.prior_scores.clear();
        self.match_count = 0;
        self.refresh_count = 0;
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
    fn compute_score(&self, key: &K, result: &R) -> Sc {
        let base = (self.weight_fn)(key, result);
        match self.impact_type {
            ImpactType::Penalty => -base,
            ImpactType::Reward => base,
        }
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
