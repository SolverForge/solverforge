use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;

use super::collection_extract::{CollectionExtract, FlattenExtract};
use super::existence_stream::{DirectExistenceStream, ExistenceMode, ExistsConstraintStream};
use super::filter::UniFilter;
use super::joiner::EqualJoiner;
use super::uni_stream::UniConstraintStream;
use crate::constraint::exists::SelfFlatten;

pub struct FlattenedCollectionTarget<S, P, B, EP, FP, Flatten, Sc>
where
    Sc: Score,
{
    pub(crate) right_stream: UniConstraintStream<S, P, EP, FP, Sc>,
    pub(crate) flatten: Flatten,
    pub(crate) _phantom: PhantomData<(fn() -> B, fn() -> Sc)>,
}

pub trait ExistenceTarget<S, A, EA, FA, Sc: Score>
where
    EA: CollectionExtract<S, Item = A>,
    FA: UniFilter<S, A>,
{
    type Output;

    fn apply(self, mode: ExistenceMode, extractor_a: EA, filter_a: FA) -> Self::Output;
}

impl<S, A, B, EA, FA, EP, FP, K, KA, KB, Sc> ExistenceTarget<S, A, EA, FA, Sc>
    for (
        UniConstraintStream<S, B, EP, FP, Sc>,
        EqualJoiner<KA, KB, K>,
    )
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    EA: CollectionExtract<S, Item = A>,
    FA: UniFilter<S, A>,
    EP: CollectionExtract<S, Item = B>,
    FP: UniFilter<S, B>,
    K: Eq + Hash + Clone + Send + Sync + 'static,
    KA: Fn(&A) -> K + Send + Sync,
    KB: Fn(&B) -> K + Send + Sync,
    Sc: Score + 'static,
{
    type Output = DirectExistenceStream<S, A, B, K, EA, EP, KA, KB, FA, FP, Sc>;

    fn apply(self, mode: ExistenceMode, extractor_a: EA, filter_a: FA) -> Self::Output {
        let (right_stream, joiner) = self;
        let (extractor_parent, filter_parent) = right_stream.into_parts();
        let (key_a, key_b) = joiner.into_keys();
        ExistsConstraintStream::new(
            mode,
            extractor_a,
            extractor_parent,
            (key_a, key_b),
            filter_a,
            filter_parent,
            SelfFlatten,
        )
    }
}

impl<S, A, P, B, EA, FA, EP, FP, K, KA, KB, Flatten, Sc> ExistenceTarget<S, A, EA, FA, Sc>
    for (
        FlattenedCollectionTarget<S, P, B, EP, FP, Flatten, Sc>,
        EqualJoiner<KA, KB, K>,
    )
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    P: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    EA: CollectionExtract<S, Item = A>,
    FA: UniFilter<S, A>,
    EP: CollectionExtract<S, Item = P>,
    FP: UniFilter<S, P>,
    K: Eq + Hash + Clone + Send + Sync + 'static,
    KA: Fn(&A) -> K + Send + Sync,
    KB: Fn(&B) -> K + Send + Sync,
    Flatten: FlattenExtract<P, Item = B> + Send + Sync,
    Sc: Score + 'static,
{
    type Output = ExistsConstraintStream<S, A, P, B, K, EA, EP, KA, KB, FA, FP, Flatten, Sc>;

    fn apply(self, mode: ExistenceMode, extractor_a: EA, filter_a: FA) -> Self::Output {
        let (target, joiner) = self;
        let FlattenedCollectionTarget {
            right_stream,
            flatten,
            ..
        } = target;
        let (extractor_parent, filter_parent) = right_stream.into_parts();
        let (key_a, key_b) = joiner.into_keys();
        ExistsConstraintStream::new(
            mode,
            extractor_a,
            extractor_parent,
            (key_a, key_b),
            filter_a,
            filter_parent,
            flatten,
        )
    }
}
