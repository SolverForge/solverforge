use std::hash::Hash;

use solverforge_core::score::Score;

use crate::stream::filter::{TrueFilter, UniFilter};
use crate::stream::joiner::{Directed, EqualJoiner, Symmetric};

use super::bi::Bi;
use super::directed_bi::DirectedBi;
use super::source::Source;

pub trait ProjectedJoinTarget<S, Out, Src, F, Sc: Score>
where
    Src: Source<S, Out>,
    F: UniFilter<S, Out>,
{
    type Output;

    fn apply(self, source: Src, filter: F) -> Self::Output;
}

impl<S, Out, Src, F, K, KF, Sc> ProjectedJoinTarget<S, Out, Src, F, Sc>
    for EqualJoiner<KF, KF, K, Symmetric>
where
    S: Send + Sync + 'static,
    Out: Send + Sync + 'static,
    Src: Source<S, Out>,
    F: UniFilter<S, Out>,
    K: Eq + Hash + Send + Sync + 'static,
    KF: Fn(&Out) -> K + Send + Sync,
    Sc: Score + 'static,
{
    type Output = Bi<S, Out, K, Src, F, KF, TrueFilter, Sc>;

    fn apply(self, source: Src, filter: F) -> Self::Output {
        let (key_fn, _) = self.into_keys();
        Bi {
            source,
            filter,
            key_fn,
            pair_filter: TrueFilter,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<S, Out, Src, F, K, KL, KR, Sc> ProjectedJoinTarget<S, Out, Src, F, Sc>
    for EqualJoiner<KL, KR, K, Directed>
where
    S: Send + Sync + 'static,
    Out: Send + Sync + 'static,
    Src: Source<S, Out>,
    F: UniFilter<S, Out>,
    K: Eq + Hash + Send + Sync + 'static,
    KL: Fn(&Out) -> K + Send + Sync,
    KR: Fn(&Out) -> K + Send + Sync,
    Sc: Score + 'static,
{
    type Output = DirectedBi<S, Out, K, Src, F, KL, KR, TrueFilter, Sc>;

    fn apply(self, source: Src, filter: F) -> Self::Output {
        let (left_key_fn, right_key_fn) = self.into_keys();
        DirectedBi {
            source,
            filter,
            left_key_fn,
            right_key_fn,
            pair_filter: TrueFilter,
            _phantom: std::marker::PhantomData,
        }
    }
}
