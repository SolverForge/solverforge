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

/// Stream of projected rows `Out` from source `Src`, filtered by `F`; built by
/// [`UniConstraintStream::project`](crate::stream::UniConstraintStream::project).
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

    /// Keeps only rows for which `predicate` returns `true`, after the existing filter.
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

    /// Merges with another projected stream of the same row type: the result emits
    /// the rows of both, each filtered by its own stream's filter.
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

    /// Groups rows by `key_fn`, aggregating each group with `collector`.
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

    /// Self-joins the rows; see
    /// [`ProjectedJoinTarget`](crate::stream::projected::ProjectedJoinTarget).
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

    /// Penalizes each match by `weight`.
    ///
    /// `weight` is a fixed score, or a closure over the matched row returning one.
    /// A fixed weight with a non-zero hard level marks the constraint hard; closures
    /// are soft unless wrapped in [`hard_weight`](crate::stream::hard_weight).
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

    /// Rewards each match by `weight`.
    ///
    /// `weight` is a fixed score, or a closure over the matched row returning one.
    /// A fixed weight with a non-zero hard level marks the constraint hard; closures
    /// are soft unless wrapped in [`hard_weight`](crate::stream::hard_weight).
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
