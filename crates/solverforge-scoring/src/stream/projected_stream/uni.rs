use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;

use crate::stream::collector::{Accumulator, Collector};
use crate::stream::filter::{AndUniFilter, FnUniFilter, TrueFilter, UniFilter};
use crate::stream::weighting_support::ConstraintWeight;

use super::bi::Builder;
use super::grouped::Grouped;
use super::join_target::ProjectedJoinTarget;
use super::source::{FilteredSource, MergedSource, Source};

pub struct Stream<S, Out, Src, F, Sc>
where
    Sc: Score,
{
    pub(crate) source: Src,
    pub(crate) filter: F,
    pub(crate) _phantom: PhantomData<(fn() -> S, fn() -> Out, fn() -> Sc)>,
}

impl<S, Out, Src, F, Sc> Stream<S, Out, Src, F, Sc>
where
    S: Send + Sync + 'static,
    Out: Send + Sync + 'static,
    Src: Source<S, Out>,
    F: UniFilter<S, Out>,
    Sc: Score + 'static,
{
    pub(crate) fn new(source: Src) -> Stream<S, Out, Src, TrueFilter, Sc> {
        Stream {
            source,
            filter: TrueFilter,
            _phantom: PhantomData,
        }
    }

    pub fn filter<P>(
        self,
        predicate: P,
    ) -> Stream<
        S,
        Out,
        Src,
        AndUniFilter<F, FnUniFilter<impl Fn(&S, &Out) -> bool + Send + Sync>>,
        Sc,
    >
    where
        P: Fn(&Out) -> bool + Send + Sync + 'static,
    {
        Stream {
            source: self.source,
            filter: AndUniFilter::new(
                self.filter,
                FnUniFilter::new(move |_s: &S, output: &Out| predicate(output)),
            ),
            _phantom: PhantomData,
        }
    }

    pub fn merge<OtherSrc, OtherF>(
        self,
        other: Stream<S, Out, OtherSrc, OtherF, Sc>,
    ) -> Stream<
        S,
        Out,
        MergedSource<FilteredSource<S, Out, Src, F>, FilteredSource<S, Out, OtherSrc, OtherF>>,
        TrueFilter,
        Sc,
    >
    where
        OtherSrc: Source<S, Out>,
        OtherF: UniFilter<S, Out>,
    {
        let left = FilteredSource::new(self.source, self.filter);
        let right = FilteredSource::new(other.source, other.filter);
        Stream {
            source: MergedSource::new(left, right),
            filter: TrueFilter,
            _phantom: PhantomData,
        }
    }

    pub fn group_by<K, KF, C, V, R, Acc>(
        self,
        key_fn: KF,
        collector: C,
    ) -> Grouped<S, Out, K, Src, F, KF, C, V, R, Acc, Sc>
    where
        K: Eq + Hash + Send + Sync + 'static,
        KF: Fn(&Out) -> K + Send + Sync,
        C: for<'i> Collector<&'i Out, Value = V, Result = R, Accumulator = Acc>
            + Send
            + Sync
            + 'static,
        V: Send + Sync + 'static,
        R: Send + Sync + 'static,
        Acc: Accumulator<V, R> + Send + Sync + 'static,
    {
        Grouped {
            source: self.source,
            filter: self.filter,
            key_fn,
            collector,
            _phantom: PhantomData,
        }
    }

    pub fn join<J>(self, joiner: J) -> J::Output
    where
        J: ProjectedJoinTarget<S, Out, Src, F, Sc>,
    {
        joiner.apply(self.source, self.filter)
    }

    fn into_weighted_builder<W>(
        self,
        impact_type: solverforge_core::ImpactType,
        weight: W,
        is_hard: bool,
    ) -> Builder<S, Out, Src, F, W, Sc>
    where
        W: Fn(&Out) -> Sc + Send + Sync,
    {
        Builder {
            source: self.source,
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
    ) -> Builder<S, Out, Src, F, impl Fn(&Out) -> Sc + Send + Sync, Sc>
    where
        W: for<'w> ConstraintWeight<(&'w Out,), Sc> + Send + Sync,
    {
        let is_hard = weight.is_hard();
        self.into_weighted_builder(
            solverforge_core::ImpactType::Penalty,
            move |output: &Out| weight.score((output,)),
            is_hard,
        )
    }

    pub fn reward<W>(
        self,
        weight: W,
    ) -> Builder<S, Out, Src, F, impl Fn(&Out) -> Sc + Send + Sync, Sc>
    where
        W: for<'w> ConstraintWeight<(&'w Out,), Sc> + Send + Sync,
    {
        let is_hard = weight.is_hard();
        self.into_weighted_builder(
            solverforge_core::ImpactType::Reward,
            move |output: &Out| weight.score((output,)),
            is_hard,
        )
    }
}
