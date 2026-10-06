/* Existence targets: turn a left filtered stream plus a right stream (direct
or flattened) into an operator-backed existence constraint.

The relationship is a named equality plan over the two operators' views.
There is no separate key/row engine: the left input is a `CollectionNode` over
the left stream (whose membership is the stream's own filter), and the right
input is a `CollectionNode` over the right stream, or a `FlattenNode` over it
when the right stream is flattened.
*/

use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;

use super::collection_extract::{CollectionExtract, FlattenExtract};
use super::existence_stream::{
    ChildKey, EntityKey, ExistenceMode, ExistsConstraintStream, ExistsEqualPlan,
};
use super::filter::UniFilter;
use super::joiner::EqualJoiner;
use super::relational::operator::{CollectionNode, FlattenNode, ParentFlatten};
use super::uni_stream::UniConstraintStream;

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

/* Direct existence: `.if_exists((other_stream, equal_bi(ka, kb)))`.

Both inputs are collections over their streams, so each input's membership is
that stream's accumulated filter, and the relationship keys off the leaf views.
*/
impl<S, A, B, EA, FA, EB, FB, K, KA, KB, Mode, Sc> ExistenceTarget<S, A, EA, FA, Sc>
    for (
        UniConstraintStream<S, B, EB, FB, Sc>,
        EqualJoiner<KA, KB, K, Mode>,
    )
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    EA: CollectionExtract<S, Item = A> + Send + Sync + 'static,
    FA: UniFilter<S, A> + Send + Sync + 'static,
    EB: CollectionExtract<S, Item = B> + Send + Sync + 'static,
    FB: UniFilter<S, B> + Send + Sync + 'static,
    K: Eq + Hash + Clone + Send + Sync + 'static,
    KA: Fn(&A) -> K + Send + Sync + 'static,
    KB: Fn(&B) -> K + Send + Sync + 'static,
    Sc: Score + 'static,
{
    type Output = ExistsConstraintStream<
        S,
        A,
        CollectionNode<S, UniConstraintStream<S, A, EA, FA, Sc>>,
        CollectionNode<S, UniConstraintStream<S, B, EB, FB, Sc>>,
        ExistsEqualPlan<K, EntityKey<KA>, EntityKey<KB>>,
        Sc,
    >;

    fn apply(self, mode: ExistenceMode, extractor_a: EA, filter_a: FA) -> Self::Output {
        let (right_stream, joiner) = self;
        let (key_a, key_b) = joiner.into_keys();
        let left_stream = UniConstraintStream::from_parts(extractor_a, filter_a);
        let left = CollectionNode::new(left_stream, 0);
        let right = CollectionNode::new(right_stream, 1);
        let condition = ExistsEqualPlan::new(EntityKey::new(key_a), EntityKey::new(key_b));
        ExistsConstraintStream::new(mode, left, right, condition)
    }
}

/* Flattened existence: `.if_not_exists((stream.flattened(|p| ..), equal_bi(..)))`.

The right input is a `FlattenNode` over the parent collection; the
relationship's right key unwraps the flattened child row.
*/
impl<S, A, P, B, EA, FA, EP, FP, K, KA, KB, Mode, Flatten, Sc> ExistenceTarget<S, A, EA, FA, Sc>
    for (
        FlattenedCollectionTarget<S, P, B, EP, FP, Flatten, Sc>,
        EqualJoiner<KA, KB, K, Mode>,
    )
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    P: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    EA: CollectionExtract<S, Item = A> + Send + Sync + 'static,
    FA: UniFilter<S, A> + Send + Sync + 'static,
    EP: CollectionExtract<S, Item = P> + Send + Sync + 'static,
    FP: UniFilter<S, P> + Send + Sync + 'static,
    K: Eq + Hash + Clone + Send + Sync + 'static,
    KA: Fn(&A) -> K + Send + Sync + 'static,
    KB: Fn(&B) -> K + Send + Sync + 'static,
    Flatten: FlattenExtract<P, Item = B> + Send + Sync + 'static,
    Sc: Score + 'static,
{
    type Output = ExistsConstraintStream<
        S,
        A,
        CollectionNode<S, UniConstraintStream<S, A, EA, FA, Sc>>,
        FlattenNode<
            CollectionNode<S, UniConstraintStream<S, P, EP, FP, Sc>>,
            ParentFlatten<Flatten>,
        >,
        ExistsEqualPlan<K, EntityKey<KA>, ChildKey<KB>>,
        Sc,
    >;

    fn apply(self, mode: ExistenceMode, extractor_a: EA, filter_a: FA) -> Self::Output {
        let (target, joiner) = self;
        let FlattenedCollectionTarget {
            right_stream,
            flatten,
            ..
        } = target;
        let (key_a, key_b) = joiner.into_keys();
        let left_stream = UniConstraintStream::from_parts(extractor_a, filter_a);
        let left = CollectionNode::new(left_stream, 0);
        let parents = CollectionNode::new(right_stream, 1);
        let right = FlattenNode::new(parents, ParentFlatten::new(flatten));
        let condition = ExistsEqualPlan::new(EntityKey::new(key_a), ChildKey::new(key_b));
        ExistsConstraintStream::new(mode, left, right, condition)
    }
}
