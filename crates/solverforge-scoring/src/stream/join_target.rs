/* JoinTarget trait for single `.join()` dispatch on UniConstraintStream.

Three impls cover all join patterns:
1. `EqualJoiner<KA, KA, K>` — self-join, returns `BiConstraintStream`
2. `(EB, EqualJoiner<KA, KB, K>)` — keyed cross-join, returns `Bi`
3. `(UniConstraintStream<...>, P)` — predicate cross-join, returns `Bi`
*/

use std::hash::Hash;

use solverforge_core::score::Score;

use super::bi_stream::BiConstraintStream;
use super::collection_extract::CollectionExtract;
use super::cross_bi_stream::{
    Bi, BiPredicatePlan, BiUnaryPlan, ViewComparisonPlan, ViewOverlapPlan,
};
use super::filter::{UniBiFilter, UniFilter, UniLeftBiFilter, UniPairFilter};
use super::joiner::plan::{CompileCondition, ExecutablePlan, IndexedPlan};
use super::joiner::{
    AndJoiner, EqualJoiner, GreaterThanJoiner, GreaterThanOrEqualJoiner, LessThanJoiner,
    LessThanOrEqualJoiner, OverlappingJoiner, Symmetric,
};
use super::key_extract::EntityKeyAdapter;
use super::relational::view_plan::{EntityKey, ViewEqualPlan};
use super::relational::Leaf;
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
impl<S, A, E, F, K, KA, Sc> JoinTarget<S, A, E, F, Sc> for EqualJoiner<KA, KA, K, Symmetric>
where
    S: Send + Sync + 'static,
    A: Clone + Hash + PartialEq + Send + Sync + 'static,
    E: CollectionExtract<S, Item = A>,
    F: UniFilter<S, A>,
    K: Eq + Hash + Clone + Send + Sync,
    KA: Fn(&A) -> K + Send + Sync,
    Sc: Score + 'static,
{
    type Output = BiConstraintStream<S, A, K, E, EntityKeyAdapter<KA>, UniBiFilter<F, A>, Sc>;

    fn apply(self, extractor_a: E, filter_a: F) -> Self::Output {
        let (key_fn, _) = self.into_keys();
        let key_extractor = EntityKeyAdapter::new(key_fn);
        let bi_filter = UniBiFilter::new(filter_a);
        BiConstraintStream::new_self_join_with_filter(extractor_a, key_extractor, bi_filter)
    }
}

// Keyed cross-join: `.join((extractor_b, equal_bi(ka, kb)))` — pairs two collections by key.
impl<S, A, B, E, F, EB, K, KA, KB, Mode, Sc> JoinTarget<S, A, E, F, Sc>
    for (EB, EqualJoiner<KA, KB, K, Mode>)
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
    type Output = Bi<
        S,
        A,
        B,
        BiUnaryPlan<K, KA, KB>,
        super::collection_extract::FilteredExtract<E, F>,
        EB,
        super::filter::TrueFilter,
        Sc,
    >;

    fn apply(self, extractor_a: E, filter_a: F) -> Self::Output {
        let (extractor_b, joiner) = self;
        let (key_a, key_b) = joiner.into_keys();
        Bi::from_condition(
            super::collection_extract::FilteredExtract::new(extractor_a, filter_a),
            extractor_b,
            BiUnaryPlan::new(key_a, key_b),
            super::filter::TrueFilter,
        )
    }
}

macro_rules! impl_comparison_join_target {
    ($joiner:ident, $less:expr, $inclusive:expr) => {
        impl<S, A, B, E, F, EB, K, KA, KB, Sc> JoinTarget<S, A, E, F, Sc>
            for (EB, $joiner<KA, KB, K>)
        where
            S: Send + Sync + 'static,
            A: Clone + Send + Sync + 'static,
            B: Clone + Send + Sync + 'static,
            E: CollectionExtract<S, Item = A>,
            F: UniFilter<S, A>,
            EB: CollectionExtract<S, Item = B>,
            K: Ord + Clone + Send + Sync + 'static,
            KA: Fn(&A) -> K + Send + Sync + 'static,
            KB: Fn(&B) -> K + Send + Sync + 'static,
            Sc: Score + 'static,
        {
            type Output = Bi<
                S,
                A,
                B,
                ViewComparisonPlan<K, KA, KB, $less, $inclusive>,
                E,
                EB,
                UniLeftBiFilter<F, B>,
                Sc,
            >;

            fn apply(self, extractor_a: E, filter_a: F) -> Self::Output {
                let (extractor_b, joiner) = self;
                let (key_a, key_b) = joiner.into_keys();
                let bi_filter = UniLeftBiFilter::new(filter_a);
                Bi::from_condition(
                    extractor_a,
                    extractor_b,
                    ViewComparisonPlan::<K, KA, KB, $less, $inclusive>::new(key_a, key_b),
                    bi_filter,
                )
            }
        }
    };
}

// Comparison conditions: all four directions, strict and inclusive. Authored
// over entities, executed over leaf views by `ViewComparisonPlan`.
impl_comparison_join_target!(LessThanJoiner, true, false);
impl_comparison_join_target!(LessThanOrEqualJoiner, true, true);
impl_comparison_join_target!(GreaterThanJoiner, false, false);
impl_comparison_join_target!(GreaterThanOrEqualJoiner, false, true);

// Interval overlap over entity-authored bounds, executed over leaf views.
impl<S, A, B, E, F, EB, K, SA, EA, SB, EBd, Sc> JoinTarget<S, A, E, F, Sc>
    for (EB, OverlappingJoiner<SA, EA, SB, EBd, K>)
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    E: CollectionExtract<S, Item = A>,
    F: UniFilter<S, A>,
    EB: CollectionExtract<S, Item = B>,
    K: Ord + Clone + Send + Sync + 'static,
    SA: Fn(&A) -> K + Send + Sync + 'static,
    EA: Fn(&A) -> K + Send + Sync + 'static,
    SB: Fn(&B) -> K + Send + Sync + 'static,
    EBd: Fn(&B) -> K + Send + Sync + 'static,
    Sc: Score + 'static,
{
    type Output =
        Bi<S, A, B, ViewOverlapPlan<K, SA, EA, SB, EBd>, E, EB, UniLeftBiFilter<F, B>, Sc>;

    fn apply(self, extractor_a: E, filter_a: F) -> Self::Output {
        let (extractor_b, joiner) = self;
        let (start_a, end_a, start_b, end_b) = joiner.into_bounds();
        let bi_filter = UniLeftBiFilter::new(filter_a);
        Bi::from_condition(
            extractor_a,
            extractor_b,
            ViewOverlapPlan::new(start_a, end_a, start_b, end_b),
            bi_filter,
        )
    }
}

// Composed conjunction: each operand converts to its view plan, then the two
// compose into one concrete `AndJoiner` plan. No dynamic condition list.
impl<S, A, B, E, F, EB, J1, J2, Sc> JoinTarget<S, A, E, F, Sc> for (EB, AndJoiner<J1, J2>)
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    E: CollectionExtract<S, Item = A>,
    F: UniFilter<S, A>,
    EB: CollectionExtract<S, Item = B>,
    J1: ToViewPlan<A, B> + 'static,
    J2: ToViewPlan<A, B> + 'static,
    AndJoiner<J1::ViewPlan, J2::ViewPlan>: CompileCondition<Plan: IndexedPlan> + 'static,
    for<'a> <AndJoiner<J1::ViewPlan, J2::ViewPlan> as CompileCondition>::Plan:
        ExecutablePlan<Leaf<'a, A>, Leaf<'a, B>>,
    Sc: Score + 'static,
{
    type Output = Bi<
        S,
        A,
        B,
        <AndJoiner<J1::ViewPlan, J2::ViewPlan> as CompileCondition>::Plan,
        E,
        EB,
        UniLeftBiFilter<F, B>,
        Sc,
    >;

    fn apply(self, extractor_a: E, filter_a: F) -> Self::Output {
        let (extractor_b, joiner) = self;
        let (first, second) = joiner.into_parts();
        let composed = AndJoiner::new(first.into_view_plan(), second.into_view_plan());
        let bi_filter = UniLeftBiFilter::new(filter_a);
        Bi::from_condition(extractor_a, extractor_b, composed.compile(), bi_filter)
    }
}

// Predicate cross-join: `.join((other_stream, |a, b| predicate))` — an
// explicit opposite-input scan under the exact predicate. The relationship
// compiles to `FilteringJoiner`, so no synthetic constant equality key is
// ever built for a non-indexable condition.
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
    type Output = Bi<S, A, B, BiPredicatePlan<P>, E, EB, UniPairFilter<F, FB>, Sc>;

    fn apply(self, extractor_a: E, filter_a: F) -> Self::Output {
        let (other_stream, predicate) = self;
        let (extractor_b, filter_b) = other_stream.into_parts();
        let combined_filter = UniPairFilter::new(filter_a, filter_b);
        Bi::from_condition(
            extractor_a,
            extractor_b,
            BiPredicatePlan::new(predicate),
            combined_filter,
        )
    }
}

/* Converts an entity-authored condition into the leaf-view plan the operator
tree executes. Used by the conjunction impl to compose operands; the
top-level equality, comparison, and overlap impls build their plans directly
so the equality path keeps producing `BiUnaryPlan`. */
pub trait ToViewPlan<A, B> {
    type ViewPlan;
    fn into_view_plan(self) -> Self::ViewPlan;
}

impl<A, B, K, KA, KB, Mode> ToViewPlan<A, B> for EqualJoiner<KA, KB, K, Mode>
where
    K: Eq + Hash + Clone,
    KA: Fn(&A) -> K + Send + Sync,
    KB: Fn(&B) -> K + Send + Sync,
{
    type ViewPlan = ViewEqualPlan<K, EntityKey<KA>, EntityKey<KB>>;
    fn into_view_plan(self) -> Self::ViewPlan {
        let (key_a, key_b) = self.into_keys();
        ViewEqualPlan::new(EntityKey::new(key_a), EntityKey::new(key_b))
    }
}

macro_rules! impl_to_view_plan_comparison {
    ($joiner:ident, $less:expr, $inclusive:expr) => {
        impl<A, B, K, KA, KB> ToViewPlan<A, B> for $joiner<KA, KB, K>
        where
            K: Ord + Clone,
            KA: Fn(&A) -> K + Send + Sync,
            KB: Fn(&B) -> K + Send + Sync,
        {
            type ViewPlan = ViewComparisonPlan<K, KA, KB, $less, $inclusive>;
            fn into_view_plan(self) -> Self::ViewPlan {
                let (key_a, key_b) = self.into_keys();
                ViewComparisonPlan::new(key_a, key_b)
            }
        }
    };
}
impl_to_view_plan_comparison!(LessThanJoiner, true, false);
impl_to_view_plan_comparison!(LessThanOrEqualJoiner, true, true);
impl_to_view_plan_comparison!(GreaterThanJoiner, false, false);
impl_to_view_plan_comparison!(GreaterThanOrEqualJoiner, false, true);

impl<A, B, K, SA, EA, SB, EBd> ToViewPlan<A, B> for OverlappingJoiner<SA, EA, SB, EBd, K>
where
    K: Ord + Clone,
    SA: Fn(&A) -> K + Send + Sync,
    EA: Fn(&A) -> K + Send + Sync,
    SB: Fn(&B) -> K + Send + Sync,
    EBd: Fn(&B) -> K + Send + Sync,
{
    type ViewPlan = ViewOverlapPlan<K, SA, EA, SB, EBd>;
    fn into_view_plan(self) -> Self::ViewPlan {
        let (start_a, end_a, start_b, end_b) = self.into_bounds();
        ViewOverlapPlan::new(start_a, end_a, start_b, end_b)
    }
}

impl<A, B, J1, J2> ToViewPlan<A, B> for AndJoiner<J1, J2>
where
    J1: ToViewPlan<A, B>,
    J2: ToViewPlan<A, B>,
{
    type ViewPlan = AndJoiner<J1::ViewPlan, J2::ViewPlan>;
    fn into_view_plan(self) -> Self::ViewPlan {
        let (first, second) = self.into_parts();
        AndJoiner::new(first.into_view_plan(), second.into_view_plan())
    }
}
