use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use crate::constraint::incremental::IncrementalUniConstraint;

use super::super::collection_extract::CollectionExtract;
use super::super::filter::UniFilter;
use super::super::weighting_support::ConstraintWeight;
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
            _phantom: PhantomData,
        }
    }

    pub fn penalize<W>(
        self,
        weight: W,
    ) -> UniConstraintBuilder<S, A, E, F, impl Fn(&A) -> Sc + Send + Sync, Sc>
    where
        W: for<'w> ConstraintWeight<(&'w A,), Sc> + Send + Sync,
    {
        let is_hard = weight.is_hard();
        self.into_weighted_builder(
            ImpactType::Penalty,
            move |a: &A| weight.score((a,)),
            is_hard,
        )
    }

    pub fn reward<W>(
        self,
        weight: W,
    ) -> UniConstraintBuilder<S, A, E, F, impl Fn(&A) -> Sc + Send + Sync, Sc>
    where
        W: for<'w> ConstraintWeight<(&'w A,), Sc> + Send + Sync,
    {
        let is_hard = weight.is_hard();
        self.into_weighted_builder(ImpactType::Reward, move |a: &A| weight.score((a,)), is_hard)
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
    // Finalizes the builder into a zero-erasure `IncrementalUniConstraint`.
    pub fn named(
        self,
        name: &str,
    ) -> IncrementalUniConstraint<S, A, E, impl Fn(&S, &A) -> bool + Send + Sync, W, Sc> {
        let filter = self.filter;
        let combined_filter = move |s: &S, a: &A| filter.test(s, a);

        IncrementalUniConstraint::new(
            ConstraintRef::new("", name),
            self.impact_type,
            self.extractor,
            combined_filter,
            self.weight,
            self.is_hard,
        )
    }
}

impl<S, A, E, F, W, Sc: Score> std::fmt::Debug for UniConstraintBuilder<S, A, E, F, W, Sc> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UniConstraintBuilder")
            .field("impact_type", &self.impact_type)
            .finish()
    }
}
