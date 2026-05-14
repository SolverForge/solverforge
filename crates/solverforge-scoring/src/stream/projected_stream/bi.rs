use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;

use crate::stream::collection_extract::CollectionExtract;
use crate::stream::filter::{AndBiFilter, BiFilter, FnBiFilter, TrueFilter, UniFilter};
use crate::stream::uni_stream::UniConstraintStream;
use crate::stream::weighting_support::ConstraintWeight;

use super::source::{ProjectedSource, Projection, SingleProjectedSource};
use super::uni::ProjectedConstraintStream;

pub struct ProjectedBiConstraintStream<S, Out, K, Src, F, KF, PF, Sc>
where
    Sc: Score,
{
    pub(crate) source: Src,
    pub(crate) filter: F,
    pub(crate) key_fn: KF,
    pub(crate) pair_filter: PF,
    pub(crate) _phantom: PhantomData<(fn() -> S, fn() -> Out, fn() -> K, fn() -> Sc)>,
}

impl<S, Out, K, Src, F, KF, PF, Sc> ProjectedBiConstraintStream<S, Out, K, Src, F, KF, PF, Sc>
where
    S: Send + Sync + 'static,
    Out: Send + Sync + 'static,
    K: Eq + Hash + Send + Sync + 'static,
    Src: ProjectedSource<S, Out>,
    F: UniFilter<S, Out>,
    KF: Fn(&Out) -> K + Send + Sync,
    PF: BiFilter<S, Out, Out>,
    Sc: Score + 'static,
{
    pub fn filter<P>(
        self,
        predicate: P,
    ) -> ProjectedBiConstraintStream<
        S,
        Out,
        K,
        Src,
        F,
        KF,
        AndBiFilter<PF, FnBiFilter<impl Fn(&S, &Out, &Out, usize, usize) -> bool + Send + Sync>>,
        Sc,
    >
    where
        P: Fn(&Out, &Out) -> bool + Send + Sync + 'static,
    {
        ProjectedBiConstraintStream {
            source: self.source,
            filter: self.filter,
            key_fn: self.key_fn,
            pair_filter: AndBiFilter::new(
                self.pair_filter,
                FnBiFilter::new(
                    move |_s: &S, left: &Out, right: &Out, _left_idx: usize, _right_idx: usize| {
                        predicate(left, right)
                    },
                ),
            ),
            _phantom: PhantomData,
        }
    }

    fn into_weighted_builder<W>(
        self,
        impact_type: solverforge_core::ImpactType,
        weight: W,
        is_hard: bool,
    ) -> ProjectedBiConstraintBuilder<S, Out, K, Src, F, KF, PF, W, Sc>
    where
        W: Fn(&Out, &Out) -> Sc + Send + Sync,
    {
        ProjectedBiConstraintBuilder {
            source: self.source,
            filter: self.filter,
            key_fn: self.key_fn,
            pair_filter: self.pair_filter,
            impact_type,
            weight,
            is_hard,
            _phantom: PhantomData,
        }
    }

    pub fn penalize<W>(
        self,
        weight: W,
    ) -> ProjectedBiConstraintBuilder<
        S,
        Out,
        K,
        Src,
        F,
        KF,
        PF,
        impl Fn(&Out, &Out) -> Sc + Send + Sync,
        Sc,
    >
    where
        W: for<'w> ConstraintWeight<(&'w Out, &'w Out), Sc> + Send + Sync,
    {
        let is_hard = weight.is_hard();
        self.into_weighted_builder(
            solverforge_core::ImpactType::Penalty,
            move |left: &Out, right: &Out| weight.score((left, right)),
            is_hard,
        )
    }
}

pub struct ProjectedBiConstraintBuilder<S, Out, K, Src, F, KF, PF, W, Sc>
where
    Sc: Score,
{
    pub(crate) source: Src,
    pub(crate) filter: F,
    pub(crate) key_fn: KF,
    pub(crate) pair_filter: PF,
    pub(crate) impact_type: solverforge_core::ImpactType,
    pub(crate) weight: W,
    pub(crate) is_hard: bool,
    pub(crate) _phantom: PhantomData<(fn() -> S, fn() -> Out, fn() -> K, fn() -> Sc)>,
}

impl<S, Out, K, Src, F, KF, PF, W, Sc>
    ProjectedBiConstraintBuilder<S, Out, K, Src, F, KF, PF, W, Sc>
where
    S: Send + Sync + 'static,
    Out: Send + Sync + 'static,
    K: Eq + Hash + Send + Sync + 'static,
    Src: ProjectedSource<S, Out>,
    F: UniFilter<S, Out>,
    KF: Fn(&Out) -> K + Send + Sync,
    PF: BiFilter<S, Out, Out>,
    W: Fn(&Out, &Out) -> Sc + Send + Sync,
    Sc: Score + 'static,
{
    pub fn named(
        self,
        name: &str,
    ) -> crate::constraint::projected::ProjectedBiConstraint<S, Out, K, Src, F, KF, PF, W, Sc> {
        crate::constraint::projected::ProjectedBiConstraint::new(
            solverforge_core::ConstraintRef::new("", name),
            self.impact_type,
            self.source,
            self.filter,
            self.key_fn,
            self.pair_filter,
            self.weight,
            self.is_hard,
        )
    }
}

pub struct ProjectedConstraintBuilder<S, Out, Src, F, W, Sc>
where
    Sc: Score,
{
    pub(super) source: Src,
    pub(super) filter: F,
    pub(super) impact_type: solverforge_core::ImpactType,
    pub(super) weight: W,
    pub(super) is_hard: bool,
    pub(super) _phantom: PhantomData<(fn() -> S, fn() -> Out, fn() -> Sc)>,
}

impl<S, Out, Src, F, W, Sc> ProjectedConstraintBuilder<S, Out, Src, F, W, Sc>
where
    S: Send + Sync + 'static,
    Out: Send + Sync + 'static,
    Src: ProjectedSource<S, Out>,
    F: UniFilter<S, Out>,
    W: Fn(&Out) -> Sc + Send + Sync,
    Sc: Score + 'static,
{
    pub fn named(
        self,
        name: &str,
    ) -> crate::constraint::projected::ProjectedUniConstraint<S, Out, Src, F, W, Sc> {
        crate::constraint::projected::ProjectedUniConstraint::new(
            solverforge_core::ConstraintRef::new("", name),
            self.impact_type,
            self.source,
            self.filter,
            self.weight,
            self.is_hard,
        )
    }
}

impl<S, A, E, F, Sc> UniConstraintStream<S, A, E, F, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    E: CollectionExtract<S, Item = A>,
    F: UniFilter<S, A>,
    Sc: Score + 'static,
{
    pub fn project<P>(
        self,
        projection: P,
    ) -> ProjectedConstraintStream<
        S,
        P::Out,
        SingleProjectedSource<S, A, E, F, P, P::Out>,
        TrueFilter,
        Sc,
    >
    where
        P: Projection<A> + 'static,
    {
        let (extractor, filter) = self.into_parts();
        ProjectedConstraintStream::<
            S,
            P::Out,
            SingleProjectedSource<S, A, E, F, P, P::Out>,
            TrueFilter,
            Sc,
        >::new(SingleProjectedSource::new(extractor, filter, projection))
    }
}
