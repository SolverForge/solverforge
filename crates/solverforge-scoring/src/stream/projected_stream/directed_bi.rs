use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;

use crate::stream::filter::{AndBiFilter, BiFilter, FnBiFilter, UniFilter};
use crate::stream::weighting_support::ConstraintWeight;

use super::source::Source;

pub struct DirectedBi<S, Out, K, Src, F, KL, KR, PF, Sc>
where
    Sc: Score,
{
    pub(crate) source: Src,
    pub(crate) filter: F,
    pub(crate) left_key_fn: KL,
    pub(crate) right_key_fn: KR,
    pub(crate) pair_filter: PF,
    pub(crate) _phantom: PhantomData<(fn() -> S, fn() -> Out, fn() -> K, fn() -> Sc)>,
}

impl<S, Out, K, Src, F, KL, KR, PF, Sc> DirectedBi<S, Out, K, Src, F, KL, KR, PF, Sc>
where
    S: Send + Sync + 'static,
    Out: Send + Sync + 'static,
    K: Eq + Hash + Send + Sync + 'static,
    Src: Source<S, Out>,
    F: UniFilter<S, Out>,
    KL: Fn(&Out) -> K + Send + Sync,
    KR: Fn(&Out) -> K + Send + Sync,
    PF: BiFilter<S, Out, Out>,
    Sc: Score + 'static,
{
    pub fn filter<P>(
        self,
        predicate: P,
    ) -> DirectedBi<
        S,
        Out,
        K,
        Src,
        F,
        KL,
        KR,
        AndBiFilter<PF, FnBiFilter<impl Fn(&S, &Out, &Out, usize, usize) -> bool + Send + Sync>>,
        Sc,
    >
    where
        P: Fn(&Out, &Out) -> bool + Send + Sync + 'static,
    {
        DirectedBi {
            source: self.source,
            filter: self.filter,
            left_key_fn: self.left_key_fn,
            right_key_fn: self.right_key_fn,
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
    ) -> DirectedBiBuilder<S, Out, K, Src, F, KL, KR, PF, W, Sc>
    where
        W: Fn(&Out, &Out) -> Sc + Send + Sync,
    {
        DirectedBiBuilder {
            source: self.source,
            filter: self.filter,
            left_key_fn: self.left_key_fn,
            right_key_fn: self.right_key_fn,
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
    ) -> DirectedBiBuilder<S, Out, K, Src, F, KL, KR, PF, impl Fn(&Out, &Out) -> Sc + Send + Sync, Sc>
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

    pub fn reward<W>(
        self,
        weight: W,
    ) -> DirectedBiBuilder<S, Out, K, Src, F, KL, KR, PF, impl Fn(&Out, &Out) -> Sc + Send + Sync, Sc>
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

pub struct DirectedBiBuilder<S, Out, K, Src, F, KL, KR, PF, W, Sc>
where
    Sc: Score,
{
    pub(crate) source: Src,
    pub(crate) filter: F,
    pub(crate) left_key_fn: KL,
    pub(crate) right_key_fn: KR,
    pub(crate) pair_filter: PF,
    pub(crate) impact_type: solverforge_core::ImpactType,
    pub(crate) weight: W,
    pub(crate) is_hard: bool,
    pub(crate) _phantom: PhantomData<(fn() -> S, fn() -> Out, fn() -> K, fn() -> Sc)>,
}

impl<S, Out, K, Src, F, KL, KR, PF, W, Sc> DirectedBiBuilder<S, Out, K, Src, F, KL, KR, PF, W, Sc>
where
    S: Send + Sync + 'static,
    Out: Send + Sync + 'static,
    K: Eq + Hash + Send + Sync + 'static,
    Src: Source<S, Out>,
    F: UniFilter<S, Out>,
    KL: Fn(&Out) -> K + Send + Sync,
    KR: Fn(&Out) -> K + Send + Sync,
    PF: BiFilter<S, Out, Out>,
    W: Fn(&Out, &Out) -> Sc + Send + Sync,
    Sc: Score + 'static,
{
    pub fn named(
        self,
        name: &str,
    ) -> crate::constraint::projected::DirectedBi<S, Out, K, Src, F, KL, KR, PF, W, Sc> {
        crate::constraint::projected::DirectedBi::new(
            solverforge_core::ConstraintRef::new("", name),
            self.impact_type,
            self.source,
            self.filter,
            self.left_key_fn,
            self.right_key_fn,
            self.pair_filter,
            self.weight,
            self.is_hard,
        )
    }
}
