use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;

use crate::stream::collector::UniCollector;
use crate::stream::filter::UniFilter;
use crate::stream::weighting_support::ConstraintWeight;

use super::source::ProjectedSource;

pub struct ProjectedGroupedConstraintStream<S, Out, K, Src, F, KF, C, Sc>
where
    Sc: Score,
{
    pub(crate) source: Src,
    pub(crate) filter: F,
    pub(crate) key_fn: KF,
    pub(crate) collector: C,
    pub(crate) _phantom: PhantomData<(fn() -> S, fn() -> Out, fn() -> K, fn() -> Sc)>,
}

impl<S, Out, K, Src, F, KF, C, Sc> ProjectedGroupedConstraintStream<S, Out, K, Src, F, KF, C, Sc>
where
    S: Send + Sync + 'static,
    Out: Send + Sync + 'static,
    K: Eq + Hash + Send + Sync + 'static,
    Src: ProjectedSource<S, Out>,
    F: UniFilter<S, Out>,
    KF: Fn(&Out) -> K + Send + Sync,
    C: UniCollector<Out> + Send + Sync + 'static,
    C::Accumulator: Send + Sync,
    C::Value: Send + Sync,
    C::Result: Send + Sync,
    Sc: Score + 'static,
{
    fn into_weighted_builder<W>(
        self,
        impact_type: solverforge_core::ImpactType,
        weight_fn: W,
        is_hard: bool,
    ) -> ProjectedGroupedConstraintBuilder<S, Out, K, Src, F, KF, C, W, Sc>
    where
        W: Fn(&K, &C::Result) -> Sc + Send + Sync,
    {
        ProjectedGroupedConstraintBuilder {
            source: self.source,
            filter: self.filter,
            key_fn: self.key_fn,
            collector: self.collector,
            impact_type,
            weight_fn,
            is_hard,
            _phantom: PhantomData,
        }
    }

    pub fn penalize<W>(
        self,
        weight_fn: W,
    ) -> ProjectedGroupedConstraintBuilder<
        S,
        Out,
        K,
        Src,
        F,
        KF,
        C,
        impl Fn(&K, &C::Result) -> Sc + Send + Sync,
        Sc,
    >
    where
        W: for<'w> ConstraintWeight<(&'w K, &'w C::Result), Sc> + Send + Sync,
    {
        let is_hard = weight_fn.is_hard();
        self.into_weighted_builder(
            solverforge_core::ImpactType::Penalty,
            move |key: &K, result: &C::Result| weight_fn.score((key, result)),
            is_hard,
        )
    }
}

pub struct ProjectedGroupedConstraintBuilder<S, Out, K, Src, F, KF, C, W, Sc>
where
    Sc: Score,
{
    pub(crate) source: Src,
    pub(crate) filter: F,
    pub(crate) key_fn: KF,
    pub(crate) collector: C,
    pub(crate) impact_type: solverforge_core::ImpactType,
    pub(crate) weight_fn: W,
    pub(crate) is_hard: bool,
    pub(crate) _phantom: PhantomData<(fn() -> S, fn() -> Out, fn() -> K, fn() -> Sc)>,
}

impl<S, Out, K, Src, F, KF, C, W, Sc>
    ProjectedGroupedConstraintBuilder<S, Out, K, Src, F, KF, C, W, Sc>
where
    S: Send + Sync + 'static,
    Out: Send + Sync + 'static,
    K: Eq + Hash + Send + Sync + 'static,
    Src: ProjectedSource<S, Out>,
    F: UniFilter<S, Out>,
    KF: Fn(&Out) -> K + Send + Sync,
    C: UniCollector<Out> + Send + Sync + 'static,
    C::Accumulator: Send + Sync,
    C::Value: Send + Sync,
    C::Result: Send + Sync,
    W: Fn(&K, &C::Result) -> Sc + Send + Sync,
    Sc: Score + 'static,
{
    pub fn named(
        self,
        name: &str,
    ) -> crate::constraint::projected::ProjectedGroupedConstraint<S, Out, K, Src, F, KF, C, W, Sc>
    {
        crate::constraint::projected::ProjectedGroupedConstraint::new(
            solverforge_core::ConstraintRef::new("", name),
            self.impact_type,
            self.source,
            self.filter,
            self.key_fn,
            self.collector,
            self.weight_fn,
            self.is_hard,
        )
    }
}
