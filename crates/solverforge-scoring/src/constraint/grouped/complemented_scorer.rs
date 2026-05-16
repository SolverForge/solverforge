use solverforge_core::score::Score;
use solverforge_core::ConstraintRef;

use crate::api::analysis::ConstraintAnalysis;
use crate::api::constraint_set::{ConstraintMetadata, ConstraintResult};

use super::scorer::GroupedTerminalScorer;

pub trait ComplementedGroupedStateView<K, R> {
    fn for_each_complement_result<Visit>(&self, visit: Visit)
    where
        Visit: FnMut(&K, &R);

    fn for_each_complement_slot_result<Visit>(&self, visit: Visit)
    where
        Visit: FnMut(usize, Option<(&K, &R)>);

    fn for_each_changed_complement_slot_result<Visit>(&self, visit: Visit)
    where
        Visit: FnMut(usize, Option<(&K, &R)>);

    fn for_each_key_result<Visit>(&self, key: &K, visit: Visit)
    where
        Visit: FnMut(&R);

    fn complement_count(&self) -> usize;
}

pub trait ComplementedGroupedScorerSet<K, R, Sc: Score>: Send + Sync {
    fn evaluate<State>(&self, state: &State) -> Sc
    where
        State: ComplementedGroupedStateView<K, R>;

    fn initialize<State>(&mut self, state: &State) -> Sc
    where
        State: ComplementedGroupedStateView<K, R>;

    fn refresh_all<State>(&mut self, state: &State) -> Sc
    where
        State: ComplementedGroupedStateView<K, R>;

    fn refresh_changed<State>(&mut self, state: &State) -> Sc
    where
        State: ComplementedGroupedStateView<K, R>;

    fn constraint_count(&self) -> usize;

    fn primary_constraint_ref(&self) -> &ConstraintRef;

    fn constraint_metadata(&self) -> Vec<ConstraintMetadata<'_>>;

    fn evaluate_each<'a, State>(&'a self, state: &State) -> Vec<ConstraintResult<'a, Sc>>
    where
        State: ComplementedGroupedStateView<K, R>;

    fn evaluate_detailed<'a, State>(&'a self, state: &State) -> Vec<ConstraintAnalysis<'a, Sc>>
    where
        State: ComplementedGroupedStateView<K, R>;

    fn reset(&mut self);
}

impl<K, R, W, Sc> ComplementedGroupedScorerSet<K, R, Sc> for GroupedTerminalScorer<K, R, W, Sc>
where
    R: Send + Sync + 'static,
    W: Fn(&K, &R) -> Sc + Send + Sync,
    Sc: Score + 'static,
{
    fn evaluate<State>(&self, state: &State) -> Sc
    where
        State: ComplementedGroupedStateView<K, R>,
    {
        let mut total = Sc::zero();
        state.for_each_complement_result(|key, result| {
            total = total + self.compute_score(key, result);
        });
        total
    }

    fn initialize<State>(&mut self, state: &State) -> Sc
    where
        State: ComplementedGroupedStateView<K, R>,
    {
        self.reset_incremental_cache(state.complement_count());
        let mut total = Sc::zero();
        state.for_each_complement_slot_result(|slot, entry| {
            let score = self.score_entry(entry);
            self.replace_cached_score(slot, score);
            total = total + score;
        });
        total
    }

    fn refresh_all<State>(&mut self, state: &State) -> Sc
    where
        State: ComplementedGroupedStateView<K, R>,
    {
        self.mark_incremental_refresh(state.complement_count());
        self.clear_incremental_scores();
        let mut total = Sc::zero();
        state.for_each_complement_slot_result(|slot, entry| {
            let score = self.score_entry(entry);
            self.replace_cached_score(slot, score);
            total = total + score;
        });
        total
    }

    fn refresh_changed<State>(&mut self, state: &State) -> Sc
    where
        State: ComplementedGroupedStateView<K, R>,
    {
        self.mark_incremental_refresh(state.complement_count());
        let mut delta = Sc::zero();
        state.for_each_changed_complement_slot_result(|slot, entry| {
            let score = self.score_entry(entry);
            delta = delta + self.replace_cached_score(slot, score);
        });
        delta
    }

    fn constraint_count(&self) -> usize {
        1
    }

    fn primary_constraint_ref(&self) -> &ConstraintRef {
        self.constraint_ref()
    }

    fn constraint_metadata(&self) -> Vec<ConstraintMetadata<'_>> {
        vec![ConstraintMetadata::new(
            self.constraint_ref(),
            self.is_hard(),
        )]
    }

    fn evaluate_each<'a, State>(&'a self, state: &State) -> Vec<ConstraintResult<'a, Sc>>
    where
        State: ComplementedGroupedStateView<K, R>,
    {
        vec![ConstraintResult {
            name: self.name(),
            score: <Self as ComplementedGroupedScorerSet<K, R, Sc>>::evaluate(self, state),
            match_count: state.complement_count(),
            is_hard: self.is_hard(),
        }]
    }

    fn evaluate_detailed<'a, State>(&'a self, state: &State) -> Vec<ConstraintAnalysis<'a, Sc>>
    where
        State: ComplementedGroupedStateView<K, R>,
    {
        vec![ConstraintAnalysis::new(
            self.constraint_ref(),
            Sc::zero(),
            <Self as ComplementedGroupedScorerSet<K, R, Sc>>::evaluate(self, state),
            Vec::new(),
            self.is_hard(),
        )]
    }

    fn reset(&mut self) {
        self.reset();
    }
}

fn push_constraint_metadata<'a>(
    metadata: &mut Vec<ConstraintMetadata<'a>>,
    constraint_ref: &'a ConstraintRef,
    is_hard: bool,
) {
    if let Some(existing) = metadata
        .iter()
        .find(|item| item.constraint_ref == constraint_ref)
    {
        assert_eq!(
            existing.is_hard,
            is_hard,
            "constraint `{}` has conflicting hard/non-hard metadata",
            constraint_ref.full_name()
        );
        return;
    }
    metadata.push(ConstraintMetadata::new(constraint_ref, is_hard));
}

macro_rules! impl_complemented_grouped_scorer_set_for_tuple {
    ($($idx:tt: $T:ident),+) => {
        impl<K, R, Sc, $($T),+> ComplementedGroupedScorerSet<K, R, Sc> for ($($T,)+)
        where
            R: Send + Sync + 'static,
            Sc: Score + 'static,
            $($T: ComplementedGroupedScorerSet<K, R, Sc>,)+
        {
            fn evaluate<State>(&self, state: &State) -> Sc
            where
                State: ComplementedGroupedStateView<K, R>,
            {
                let mut total = Sc::zero();
                $(total = total + ComplementedGroupedScorerSet::evaluate(&self.$idx, state);)+
                total
            }

            fn initialize<State>(&mut self, state: &State) -> Sc
            where
                State: ComplementedGroupedStateView<K, R>,
            {
                let mut total = Sc::zero();
                $(total = total + ComplementedGroupedScorerSet::initialize(&mut self.$idx, state);)+
                total
            }

            fn refresh_all<State>(&mut self, state: &State) -> Sc
            where
                State: ComplementedGroupedStateView<K, R>,
            {
                let mut total = Sc::zero();
                $(total = total + ComplementedGroupedScorerSet::refresh_all(&mut self.$idx, state);)+
                total
            }

            fn refresh_changed<State>(&mut self, state: &State) -> Sc
            where
                State: ComplementedGroupedStateView<K, R>,
            {
                let mut total = Sc::zero();
                $(total = total + ComplementedGroupedScorerSet::refresh_changed(&mut self.$idx, state);)+
                total
            }

            fn constraint_count(&self) -> usize {
                let mut count = 0;
                $(let _ = &self.$idx; count += ComplementedGroupedScorerSet::constraint_count(&self.$idx);)+
                count
            }

            fn primary_constraint_ref(&self) -> &ConstraintRef {
                self.0.primary_constraint_ref()
            }

            fn constraint_metadata(&self) -> Vec<ConstraintMetadata<'_>> {
                let mut metadata = Vec::new();
                $(
                    for item in ComplementedGroupedScorerSet::constraint_metadata(&self.$idx) {
                        push_constraint_metadata(&mut metadata, item.constraint_ref, item.is_hard);
                    }
                )+
                metadata
            }

            fn evaluate_each<'a, State>(&'a self, state: &State) -> Vec<ConstraintResult<'a, Sc>>
            where
                State: ComplementedGroupedStateView<K, R>,
            {
                let mut results = Vec::new();
                $(results.extend(ComplementedGroupedScorerSet::evaluate_each(&self.$idx, state));)+
                results
            }

            fn evaluate_detailed<'a, State>(&'a self, state: &State) -> Vec<ConstraintAnalysis<'a, Sc>>
            where
                State: ComplementedGroupedStateView<K, R>,
            {
                let mut analyses = Vec::new();
                $(analyses.extend(ComplementedGroupedScorerSet::evaluate_detailed(&self.$idx, state));)+
                analyses
            }

            fn reset(&mut self) {
                $(ComplementedGroupedScorerSet::reset(&mut self.$idx);)+
            }
        }
    };
}

impl_complemented_grouped_scorer_set_for_tuple!(0: C0);
impl_complemented_grouped_scorer_set_for_tuple!(0: C0, 1: C1);
impl_complemented_grouped_scorer_set_for_tuple!(0: C0, 1: C1, 2: C2);
impl_complemented_grouped_scorer_set_for_tuple!(0: C0, 1: C1, 2: C2, 3: C3);
impl_complemented_grouped_scorer_set_for_tuple!(0: C0, 1: C1, 2: C2, 3: C3, 4: C4);
impl_complemented_grouped_scorer_set_for_tuple!(0: C0, 1: C1, 2: C2, 3: C3, 4: C4, 5: C5);
impl_complemented_grouped_scorer_set_for_tuple!(0: C0, 1: C1, 2: C2, 3: C3, 4: C4, 5: C5, 6: C6);
impl_complemented_grouped_scorer_set_for_tuple!(0: C0, 1: C1, 2: C2, 3: C3, 4: C4, 5: C5, 6: C6, 7: C7);
