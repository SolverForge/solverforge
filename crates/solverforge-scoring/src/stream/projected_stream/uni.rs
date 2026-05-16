use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;

use crate::stream::collector::{Accumulator, Collector};
use crate::stream::filter::{AndUniFilter, FnUniFilter, TrueFilter, UniFilter};
use crate::stream::joiner::EqualJoiner;
use crate::stream::weighting_support::ConstraintWeight;

use super::bi::{ProjectedBiConstraintStream, ProjectedConstraintBuilder};
use super::grouped::ProjectedGroupedConstraintStream;
use super::source::{FilteredProjectedSource, MergedProjectedSource, ProjectedSource};

pub struct ProjectedConstraintStream<S, Out, Src, F, Sc>
where
    Sc: Score,
{
    pub(crate) source: Src,
    pub(crate) filter: F,
    pub(crate) _phantom: PhantomData<(fn() -> S, fn() -> Out, fn() -> Sc)>,
}

impl<S, Out, Src, F, Sc> ProjectedConstraintStream<S, Out, Src, F, Sc>
where
    S: Send + Sync + 'static,
    Out: Send + Sync + 'static,
    Src: ProjectedSource<S, Out>,
    F: UniFilter<S, Out>,
    Sc: Score + 'static,
{
    pub(crate) fn new(source: Src) -> ProjectedConstraintStream<S, Out, Src, TrueFilter, Sc> {
        ProjectedConstraintStream {
            source,
            filter: TrueFilter,
            _phantom: PhantomData,
        }
    }

    pub fn filter<P>(
        self,
        predicate: P,
    ) -> ProjectedConstraintStream<
        S,
        Out,
        Src,
        AndUniFilter<F, FnUniFilter<impl Fn(&S, &Out) -> bool + Send + Sync>>,
        Sc,
    >
    where
        P: Fn(&Out) -> bool + Send + Sync + 'static,
    {
        ProjectedConstraintStream {
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
        other: ProjectedConstraintStream<S, Out, OtherSrc, OtherF, Sc>,
    ) -> ProjectedConstraintStream<
        S,
        Out,
        MergedProjectedSource<
            FilteredProjectedSource<S, Out, Src, F>,
            FilteredProjectedSource<S, Out, OtherSrc, OtherF>,
        >,
        TrueFilter,
        Sc,
    >
    where
        OtherSrc: ProjectedSource<S, Out>,
        OtherF: UniFilter<S, Out>,
    {
        let left = FilteredProjectedSource::new(self.source, self.filter);
        let right = FilteredProjectedSource::new(other.source, other.filter);
        ProjectedConstraintStream {
            source: MergedProjectedSource::new(left, right),
            filter: TrueFilter,
            _phantom: PhantomData,
        }
    }

    pub fn group_by<K, KF, C, V, R, Acc>(
        self,
        key_fn: KF,
        collector: C,
    ) -> ProjectedGroupedConstraintStream<S, Out, K, Src, F, KF, C, V, R, Acc, Sc>
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
        ProjectedGroupedConstraintStream {
            source: self.source,
            filter: self.filter,
            key_fn,
            collector,
            _phantom: PhantomData,
        }
    }

    pub fn join<K, KF>(
        self,
        joiner: EqualJoiner<KF, KF, K>,
    ) -> ProjectedBiConstraintStream<S, Out, K, Src, F, KF, TrueFilter, Sc>
    where
        K: Eq + Hash + Send + Sync + 'static,
        KF: Fn(&Out) -> K + Send + Sync,
    {
        let (key_fn, _) = joiner.into_keys();
        ProjectedBiConstraintStream {
            source: self.source,
            filter: self.filter,
            key_fn,
            pair_filter: TrueFilter,
            _phantom: PhantomData,
        }
    }

    fn into_weighted_builder<W>(
        self,
        impact_type: solverforge_core::ImpactType,
        weight: W,
        is_hard: bool,
    ) -> ProjectedConstraintBuilder<S, Out, Src, F, W, Sc>
    where
        W: Fn(&Out) -> Sc + Send + Sync,
    {
        ProjectedConstraintBuilder {
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
    ) -> ProjectedConstraintBuilder<S, Out, Src, F, impl Fn(&Out) -> Sc + Send + Sync, Sc>
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
}
