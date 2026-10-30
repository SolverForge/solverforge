use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;

use crate::stream::collection_extract::CollectionExtract;
use crate::stream::filter::{AndBiFilter, BiFilter, FnBiFilter, TrueFilter, UniFilter};
use crate::stream::uni_stream::UniConstraintStream;
use crate::stream::weighting_support::ConstraintWeight;

use super::source::{Projection, SingleSource, Source};
use super::uni::Stream;

/// Symmetric self-join over projected rows with equal keys `KF`; produced by
/// [`Stream::join`](crate::stream::projected::Stream::join) with `equal(..)`.
pub struct Bi<S, Out, K, Src, F, KF, PF, Sc>
where
    Sc: Score,
{
    pub(crate) source: Src,
    pub(crate) filter: F,
    pub(crate) key_fn: KF,
    pub(crate) pair_filter: PF,
    pub(crate) _phantom: PhantomData<(fn() -> S, fn() -> Out, fn() -> K, fn() -> Sc)>,
}

impl<S, Out, K, Src, F, KF, PF, Sc> Bi<S, Out, K, Src, F, KF, PF, Sc>
where
    S: Send + Sync + 'static,
    Out: Send + Sync + 'static,
    K: Eq + Hash + Send + Sync + 'static,
    Src: Source<S, Out>,
    F: UniFilter<S, Out>,
    KF: Fn(&Out) -> K + Send + Sync,
    PF: BiFilter<S, Out, Out>,
    Sc: Score + 'static,
{
    /// Keeps only pairs for which `predicate` returns `true`, after the existing filter.
    pub fn filter<P>(
        self,
        predicate: P,
    ) -> Bi<
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
        Bi {
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
    ) -> BiBuilder<S, Out, K, Src, F, KF, PF, W, Sc>
    where
        W: Fn(&Out, &Out) -> Sc + Send + Sync,
    {
        BiBuilder {
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

    /// Penalizes each match by `weight`.
    ///
    /// `weight` is a fixed score, or a closure over the matched `(&Out, &Out)` pair returning one.
    /// A fixed weight with a non-zero hard level marks the constraint hard; closures
    /// are soft unless wrapped in [`hard_weight`](crate::stream::hard_weight).
    pub fn penalize<W>(
        self,
        weight: W,
    ) -> BiBuilder<S, Out, K, Src, F, KF, PF, impl Fn(&Out, &Out) -> Sc + Send + Sync, Sc>
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

    /// Rewards each match by `weight`.
    ///
    /// `weight` is a fixed score, or a closure over the matched `(&Out, &Out)` pair returning one.
    /// A fixed weight with a non-zero hard level marks the constraint hard; closures
    /// are soft unless wrapped in [`hard_weight`](crate::stream::hard_weight).
    pub fn reward<W>(
        self,
        weight: W,
    ) -> BiBuilder<S, Out, K, Src, F, KF, PF, impl Fn(&Out, &Out) -> Sc + Send + Sync, Sc>
    where
        W: for<'w> ConstraintWeight<(&'w Out, &'w Out), Sc> + Send + Sync,
    {
        let is_hard = weight.is_hard();
        self.into_weighted_builder(
            solverforge_core::ImpactType::Reward,
            move |left: &Out, right: &Out| weight.score((left, right)),
            is_hard,
        )
    }
}

/// Builder holding a weighted projected self-join constraint until
/// [`named`](Self::named) finalizes it.
pub struct BiBuilder<S, Out, K, Src, F, KF, PF, W, Sc>
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

impl<S, Out, K, Src, F, KF, PF, W, Sc> BiBuilder<S, Out, K, Src, F, KF, PF, W, Sc>
where
    S: Send + Sync + 'static,
    Out: Send + Sync + 'static,
    K: Eq + Hash + Send + Sync + 'static,
    Src: Source<S, Out>,
    F: UniFilter<S, Out>,
    KF: Fn(&Out) -> K + Send + Sync,
    PF: BiFilter<S, Out, Out>,
    W: Fn(&Out, &Out) -> Sc + Send + Sync,
    Sc: Score + 'static,
{
    /// Names the constraint and builds the projected `Bi` constraint.
    pub fn named(
        self,
        name: &str,
    ) -> crate::constraint::projected::Bi<S, Out, K, Src, F, KF, PF, W, Sc> {
        crate::constraint::projected::Bi::new(
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

/// Builder holding a weighted projected-row constraint until [`named`](Self::named) finalizes it.
pub struct Builder<S, Out, Src, F, W, Sc>
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

impl<S, Out, Src, F, W, Sc> Builder<S, Out, Src, F, W, Sc>
where
    S: Send + Sync + 'static,
    Out: Send + Sync + 'static,
    Src: Source<S, Out>,
    F: UniFilter<S, Out>,
    W: Fn(&Out) -> Sc + Send + Sync,
    Sc: Score + 'static,
{
    /// Names the constraint and builds the projected `Uni` constraint.
    pub fn named(self, name: &str) -> crate::constraint::projected::Uni<S, Out, Src, F, W, Sc> {
        crate::constraint::projected::Uni::new(
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
    /// Projects each entity that passes the stream filter into zero or more `Out`
    /// rows emitted by `projection`.
    pub fn project<P>(
        self,
        projection: P,
    ) -> Stream<S, P::Out, SingleSource<S, A, E, F, P, P::Out>, TrueFilter, Sc>
    where
        P: Projection<A> + 'static,
    {
        let (extractor, filter) = self.into_parts();
        Stream::<S, P::Out, SingleSource<S, A, E, F, P, P::Out>, TrueFilter, Sc>::new(
            SingleSource::new(extractor, filter, projection),
        )
    }
}
