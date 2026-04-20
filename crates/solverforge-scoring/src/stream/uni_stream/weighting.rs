use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use crate::constraint::incremental::IncrementalUniConstraint;

use super::super::collection_extract::CollectionExtract;
use super::super::filter::UniFilter;
use super::super::weighting_support::fixed_weight_is_hard;
use super::base::UniConstraintStream;

impl<S, A, E, F, Sc> UniConstraintStream<S, A, E, F, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    E: CollectionExtract<S, Item = A>,
    F: UniFilter<S, A>,
    Sc: Score + 'static,
{
    fn into_weighted_builder<W>(
        self,
        impact_type: ImpactType,
        weight: W,
        is_hard: bool,
    ) -> UniConstraintBuilder<S, A, E, F, W, Sc>
    where
        W: Fn(&A) -> Sc + Send + Sync,
    {
        UniConstraintBuilder {
            extractor: self.extractor,
            filter: self.filter,
            impact_type,
            weight,
            is_hard,
            expected_descriptor: None,
            _phantom: PhantomData,
        }
    }

    // Penalizes each matching entity with a fixed weight.
    pub fn penalize(
        self,
        weight: Sc,
    ) -> UniConstraintBuilder<S, A, E, F, impl Fn(&A) -> Sc + Send + Sync, Sc>
    where
        Sc: Copy,
    {
        self.into_weighted_builder(
            ImpactType::Penalty,
            move |_: &A| weight,
            fixed_weight_is_hard(weight),
        )
    }

    /* Penalizes each matching entity with a dynamic weight. */
    pub fn penalize_with<W>(self, weight_fn: W) -> UniConstraintBuilder<S, A, E, F, W, Sc>
    where
        W: Fn(&A) -> Sc + Send + Sync,
    {
        self.into_weighted_builder(ImpactType::Penalty, weight_fn, false)
    }

    // Penalizes each matching entity with a dynamic weight, explicitly marked as a hard constraint.
    pub fn penalize_hard_with<W>(self, weight_fn: W) -> UniConstraintBuilder<S, A, E, F, W, Sc>
    where
        W: Fn(&A) -> Sc + Send + Sync,
    {
        self.into_weighted_builder(ImpactType::Penalty, weight_fn, true)
    }

    // Rewards each matching entity with a fixed weight.
    pub fn reward(
        self,
        weight: Sc,
    ) -> UniConstraintBuilder<S, A, E, F, impl Fn(&A) -> Sc + Send + Sync, Sc>
    where
        Sc: Copy,
    {
        self.into_weighted_builder(
            ImpactType::Reward,
            move |_: &A| weight,
            fixed_weight_is_hard(weight),
        )
    }

    /* Rewards each matching entity with a dynamic weight. */
    pub fn reward_with<W>(self, weight_fn: W) -> UniConstraintBuilder<S, A, E, F, W, Sc>
    where
        W: Fn(&A) -> Sc + Send + Sync,
    {
        self.into_weighted_builder(ImpactType::Reward, weight_fn, false)
    }

    // Rewards each matching entity with a dynamic weight, explicitly marked as a hard constraint.
    pub fn reward_hard_with<W>(self, weight_fn: W) -> UniConstraintBuilder<S, A, E, F, W, Sc>
    where
        W: Fn(&A) -> Sc + Send + Sync,
    {
        self.into_weighted_builder(ImpactType::Reward, weight_fn, true)
    }

    // Penalizes each matching entity with one hard score unit.
    pub fn penalize_hard(
        self,
    ) -> UniConstraintBuilder<S, A, E, F, impl Fn(&A) -> Sc + Send + Sync, Sc>
    where
        Sc: Copy,
    {
        self.penalize(Sc::one_hard())
    }

    // Penalizes each matching entity with one soft score unit.
    pub fn penalize_soft(
        self,
    ) -> UniConstraintBuilder<S, A, E, F, impl Fn(&A) -> Sc + Send + Sync, Sc>
    where
        Sc: Copy,
    {
        self.penalize(Sc::one_soft())
    }

    // Rewards each matching entity with one hard score unit.
    pub fn reward_hard(
        self,
    ) -> UniConstraintBuilder<S, A, E, F, impl Fn(&A) -> Sc + Send + Sync, Sc>
    where
        Sc: Copy,
    {
        self.reward(Sc::one_hard())
    }

    // Rewards each matching entity with one soft score unit.
    pub fn reward_soft(
        self,
    ) -> UniConstraintBuilder<S, A, E, F, impl Fn(&A) -> Sc + Send + Sync, Sc>
    where
        Sc: Copy,
    {
        self.reward(Sc::one_soft())
    }
}

// Zero-erasure builder for finalizing a uni-constraint.
pub struct UniConstraintBuilder<S, A, E, F, W, Sc>
where
    Sc: Score,
{
    extractor: E,
    filter: F,
    impact_type: ImpactType,
    weight: W,
    is_hard: bool,
    expected_descriptor: Option<usize>,
    _phantom: PhantomData<(fn() -> S, fn() -> A, fn() -> Sc)>,
}

impl<S, A, E, F, W, Sc> UniConstraintBuilder<S, A, E, F, W, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    E: CollectionExtract<S, Item = A>,
    F: UniFilter<S, A>,
    W: Fn(&A) -> Sc + Send + Sync,
    Sc: Score + 'static,
{
    /* Restricts this constraint to only fire for the given descriptor index. */
    pub fn for_descriptor(mut self, descriptor_index: usize) -> Self {
        self.expected_descriptor = Some(descriptor_index);
        self
    }

    // Finalizes the builder into a zero-erasure `IncrementalUniConstraint`.
    pub fn named(
        self,
        name: &str,
    ) -> IncrementalUniConstraint<S, A, E, impl Fn(&S, &A) -> bool + Send + Sync, W, Sc> {
        let filter = self.filter;
        let combined_filter = move |s: &S, a: &A| filter.test(s, a);

        let mut constraint = IncrementalUniConstraint::new(
            ConstraintRef::new("", name),
            self.impact_type,
            self.extractor,
            combined_filter,
            self.weight,
            self.is_hard,
        );
        if let Some(d) = self.expected_descriptor {
            constraint = constraint.with_descriptor(d);
        }
        constraint
    }
}

impl<S, A, E, F, W, Sc: Score> std::fmt::Debug for UniConstraintBuilder<S, A, E, F, W, Sc> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UniConstraintBuilder")
            .field("impact_type", &self.impact_type)
            .finish()
    }
}
