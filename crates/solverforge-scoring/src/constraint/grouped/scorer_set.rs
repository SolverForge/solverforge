use std::hash::Hash;

use solverforge_core::score::Score;
use solverforge_core::ConstraintRef;

use crate::api::analysis::ConstraintAnalysis;
use crate::api::constraint_set::{ConstraintMetadata, ConstraintResult};

use super::scorer::GroupedTerminalScorer;
use super::state::GroupedStateView;

pub trait GroupedScorerSet<K, R, Sc: Score>: Send + Sync {
    fn evaluate<State>(&self, state: &State) -> Sc
    where
        State: GroupedStateView<K, R>;

    fn initialize<State>(&mut self, state: &State) -> Sc
    where
        State: GroupedStateView<K, R>;

    fn refresh_changed_keys<State>(&mut self, state: &State, changed_keys: &[K]) -> Sc
    where
        State: GroupedStateView<K, R>;

    fn constraint_count(&self) -> usize;

    fn constraint_metadata(&self) -> Vec<ConstraintMetadata<'_>>;

    fn evaluate_each<'a, State>(&'a self, state: &State) -> Vec<ConstraintResult<'a, Sc>>
    where
        State: GroupedStateView<K, R>;

    fn evaluate_detailed<'a, State>(&'a self, state: &State) -> Vec<ConstraintAnalysis<'a, Sc>>
    where
        State: GroupedStateView<K, R>;

    fn reset(&mut self);
}

impl<K, R, W, Sc> GroupedScorerSet<K, R, Sc> for GroupedTerminalScorer<K, R, W, Sc>
where
    K: Clone + Eq + Hash + Send + Sync + 'static,
    R: Send + Sync + 'static,
    W: Fn(&K, &R) -> Sc + Send + Sync,
    Sc: Score + 'static,
{
    fn evaluate<State>(&self, state: &State) -> Sc
    where
        State: GroupedStateView<K, R>,
    {
        self.evaluate(state)
    }

    fn initialize<State>(&mut self, state: &State) -> Sc
    where
        State: GroupedStateView<K, R>,
    {
        self.initialize(state)
    }

    fn refresh_changed_keys<State>(&mut self, state: &State, changed_keys: &[K]) -> Sc
    where
        State: GroupedStateView<K, R>,
    {
        self.refresh_changed_keys(state, changed_keys)
    }

    fn constraint_count(&self) -> usize {
        1
    }

    fn constraint_metadata(&self) -> Vec<ConstraintMetadata<'_>> {
        vec![ConstraintMetadata::new(
            self.constraint_ref(),
            self.is_hard(),
        )]
    }

    fn evaluate_each<'a, State>(&'a self, state: &State) -> Vec<ConstraintResult<'a, Sc>>
    where
        State: GroupedStateView<K, R>,
    {
        vec![ConstraintResult {
            name: self.name(),
            score: self.evaluate(state),
            match_count: state.group_count(),
            is_hard: self.is_hard(),
        }]
    }

    fn evaluate_detailed<'a, State>(&'a self, state: &State) -> Vec<ConstraintAnalysis<'a, Sc>>
    where
        State: GroupedStateView<K, R>,
    {
        vec![ConstraintAnalysis::new(
            self.constraint_ref(),
            Sc::zero(),
            self.evaluate(state),
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

macro_rules! impl_grouped_scorer_set_for_tuple {
    ($($idx:tt: $T:ident),+) => {
        impl<K, R, Sc, $($T),+> GroupedScorerSet<K, R, Sc> for ($($T,)+)
        where
            K: Clone + Eq + Hash + Send + Sync + 'static,
            R: Send + Sync + 'static,
            Sc: Score + 'static,
            $($T: GroupedScorerSet<K, R, Sc>,)+
        {
            fn evaluate<State>(&self, state: &State) -> Sc
            where
                State: GroupedStateView<K, R>,
            {
                let mut total = Sc::zero();
                $(total = total + self.$idx.evaluate(state);)+
                total
            }

            fn initialize<State>(&mut self, state: &State) -> Sc
            where
                State: GroupedStateView<K, R>,
            {
                let mut total = Sc::zero();
                $(total = total + self.$idx.initialize(state);)+
                total
            }

            fn refresh_changed_keys<State>(&mut self, state: &State, changed_keys: &[K]) -> Sc
            where
                State: GroupedStateView<K, R>,
            {
                let mut total = Sc::zero();
                $(total = total + self.$idx.refresh_changed_keys(state, changed_keys);)+
                total
            }

            fn constraint_count(&self) -> usize {
                let mut count = 0;
                $(let _ = &self.$idx; count += self.$idx.constraint_count();)+
                count
            }

            fn constraint_metadata(&self) -> Vec<ConstraintMetadata<'_>> {
                let mut metadata = Vec::new();
                $(
                    for item in self.$idx.constraint_metadata() {
                        push_constraint_metadata(&mut metadata, item.constraint_ref, item.is_hard);
                    }
                )+
                metadata
            }

            fn evaluate_each<'a, State>(&'a self, state: &State) -> Vec<ConstraintResult<'a, Sc>>
            where
                State: GroupedStateView<K, R>,
            {
                let mut results = Vec::new();
                $(results.extend(self.$idx.evaluate_each(state));)+
                results
            }

            fn evaluate_detailed<'a, State>(&'a self, state: &State) -> Vec<ConstraintAnalysis<'a, Sc>>
            where
                State: GroupedStateView<K, R>,
            {
                let mut analyses = Vec::new();
                $(analyses.extend(self.$idx.evaluate_detailed(state));)+
                analyses
            }

            fn reset(&mut self) {
                $(self.$idx.reset();)+
            }
        }
    };
}

impl_grouped_scorer_set_for_tuple!(0: C0);
impl_grouped_scorer_set_for_tuple!(0: C0, 1: C1);
impl_grouped_scorer_set_for_tuple!(0: C0, 1: C1, 2: C2);
impl_grouped_scorer_set_for_tuple!(0: C0, 1: C1, 2: C2, 3: C3);
impl_grouped_scorer_set_for_tuple!(0: C0, 1: C1, 2: C2, 3: C3, 4: C4);
impl_grouped_scorer_set_for_tuple!(0: C0, 1: C1, 2: C2, 3: C3, 4: C4, 5: C5);
impl_grouped_scorer_set_for_tuple!(0: C0, 1: C1, 2: C2, 3: C3, 4: C4, 5: C5, 6: C6);
impl_grouped_scorer_set_for_tuple!(0: C0, 1: C1, 2: C2, 3: C3, 4: C4, 5: C5, 6: C6, 7: C7);
impl_grouped_scorer_set_for_tuple!(0: C0, 1: C1, 2: C2, 3: C3, 4: C4, 5: C5, 6: C6, 7: C7, 8: C8);
impl_grouped_scorer_set_for_tuple!(0: C0, 1: C1, 2: C2, 3: C3, 4: C4, 5: C5, 6: C6, 7: C7, 8: C8, 9: C9);
impl_grouped_scorer_set_for_tuple!(0: C0, 1: C1, 2: C2, 3: C3, 4: C4, 5: C5, 6: C6, 7: C7, 8: C8, 9: C9, 10: C10);
impl_grouped_scorer_set_for_tuple!(0: C0, 1: C1, 2: C2, 3: C3, 4: C4, 5: C5, 6: C6, 7: C7, 8: C8, 9: C9, 10: C10, 11: C11);
