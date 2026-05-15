/* JoinTarget trait for single `.join()` dispatch on UniConstraintStream.

Four impls cover all join patterns:
1. `EqualJoiner<KA, KA, K>` — self-join, returns `BiConstraintStream`
2. `(EB, EqualJoiner<KA, KB, K>)` — keyed cross-join, returns `CrossBiConstraintStream`
3. `(UniConstraintStream<...>, EqualJoiner<KA, KB, K>)` — keyed cross-join with a stream target
4. `(UniConstraintStream<...>, P)` — predicate cross-join, returns `CrossBiConstraintStream`
*/

use std::hash::Hash;

use solverforge_core::score::Score;

use super::bi_stream::BiConstraintStream;
use super::collection_extract::CollectionExtract;
use super::cross_bi_stream::CrossBiConstraintStream;
use super::filter::{UniFilter, UniLeftBiFilter, UniPairPredBiFilter};
use super::joiner::EqualJoiner;
use super::key_extract::EntityKeyAdapter;
use super::UniConstraintStream;

/* Trait for single `.join()` dispatch.

`E` is the extractor type of the left stream.
`F` is the filter type of the left stream.
Implementors consume `self` and receive the left stream's extractor and filter,
producing the appropriate cross-stream type.
*/
pub trait JoinTarget<S, A, E, F, Sc: Score> {
    // The resulting constraint stream type.
    type Output;

    // Applies the join, consuming both the target and the left stream's components.
    fn apply(self, extractor_a: E, filter_a: F) -> Self::Output;
}

// Self-join: `.join(equal(|a: &A| a.key))` — pairs same-collection entities.
impl<S, A, E, F, K, KA, Sc> JoinTarget<S, A, E, F, Sc> for EqualJoiner<KA, KA, K>
where
    S: Send + Sync + 'static,
    A: Clone + Hash + PartialEq + Send + Sync + 'static,
    E: CollectionExtract<S, Item = A>,
    F: UniFilter<S, A>,
    K: Eq + Hash + Clone + Send + Sync,
    KA: Fn(&A) -> K + Send + Sync,
    Sc: Score + 'static,
{
    type Output = BiConstraintStream<S, A, K, E, EntityKeyAdapter<KA>, UniLeftBiFilter<F, A>, Sc>;

    fn apply(self, extractor_a: E, filter_a: F) -> Self::Output {
        let (key_fn, _) = self.into_keys();
        let key_extractor = EntityKeyAdapter::new(key_fn);
        let bi_filter = UniLeftBiFilter::new(filter_a);
        BiConstraintStream::new_self_join_with_filter(extractor_a, key_extractor, bi_filter)
    }
}

// Keyed cross-join: `.join((extractor_b, equal_bi(ka, kb)))` — pairs two collections by key.
impl<S, A, B, E, F, EB, K, KA, KB, Sc> JoinTarget<S, A, E, F, Sc> for (EB, EqualJoiner<KA, KB, K>)
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    E: CollectionExtract<S, Item = A>,
    F: UniFilter<S, A>,
    EB: CollectionExtract<S, Item = B>,
    K: Eq + Hash + Clone + Send + Sync,
    KA: Fn(&A) -> K + Send + Sync,
    KB: Fn(&B) -> K + Send + Sync,
    Sc: Score + 'static,
{
    type Output = CrossBiConstraintStream<S, A, B, K, E, EB, KA, KB, UniLeftBiFilter<F, B>, Sc>;

    fn apply(self, extractor_a: E, filter_a: F) -> Self::Output {
        let (extractor_b, joiner) = self;
        let (key_a, key_b) = joiner.into_keys();
        let bi_filter = UniLeftBiFilter::new(filter_a);
        CrossBiConstraintStream::new_with_filter(extractor_a, extractor_b, key_a, key_b, bi_filter)
    }
}

// Predicate cross-join: `.join((other_stream, |a, b| predicate))` — O(n*m) nested loop.
impl<S, A, B, E, F, EB, FB, P, Sc> JoinTarget<S, A, E, F, Sc>
    for (UniConstraintStream<S, B, EB, FB, Sc>, P)
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    E: CollectionExtract<S, Item = A>,
    F: UniFilter<S, A>,
    EB: CollectionExtract<S, Item = B>,
    FB: UniFilter<S, B>,
    P: Fn(&A, &B) -> bool + Send + Sync + 'static,
    Sc: Score + 'static,
{
    type Output = CrossBiConstraintStream<
        S,
        A,
        B,
        u8,
        E,
        EB,
        fn(&A) -> u8,
        fn(&B) -> u8,
        UniPairPredBiFilter<F, FB, P>,
        Sc,
    >;

    fn apply(self, extractor_a: E, filter_a: F) -> Self::Output {
        let (other_stream, predicate) = self;
        let (extractor_b, filter_b) = other_stream.into_parts();
        let combined_filter = UniPairPredBiFilter::new(filter_a, filter_b, predicate);
        CrossBiConstraintStream::new_with_filter(
            extractor_a,
            extractor_b,
            (|_: &A| 0u8) as fn(&A) -> u8,
            (|_: &B| 0u8) as fn(&B) -> u8,
            combined_filter,
        )
    }
}
