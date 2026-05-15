use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;

use super::super::balance_stream::BalanceConstraintStream;
use super::super::collection_extract::{ChangeSource, CollectionExtract, FlattenVecExtract};
use super::super::collector::{Accumulator, Collector};
use super::super::existence_stream::ExistenceMode;
use super::super::existence_target::ExistenceTarget;
use super::super::existence_target::FlattenedCollectionTarget;
use super::super::filter::{AndUniFilter, FnUniFilter, TrueFilter, UniFilter};
use super::super::grouped_stream::GroupedConstraintStream;

/* Zero-erasure constraint stream over a single entity type.

`UniConstraintStream` accumulates filters and can be finalized into
an `IncrementalUniConstraint` via `penalize()` or `reward()`.

All type parameters are concrete - no trait objects, no Arc allocations
in the hot path.

# Type Parameters

- `S` - Solution type
- `A` - Entity type
- `E` - Extractor function type
- `F` - Combined filter type
- `Sc` - Score type
*/
pub struct UniConstraintStream<S, A, E, F, Sc>
where
    Sc: Score,
{
    pub(super) extractor: E,
    pub(super) filter: F,
    pub(super) _phantom: PhantomData<(fn() -> S, fn() -> A, fn() -> Sc)>,
}

impl<S, A, E, Sc> UniConstraintStream<S, A, E, TrueFilter, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    E: CollectionExtract<S, Item = A>,
    Sc: Score + 'static,
{
    // Creates a new uni-constraint stream with the given extractor.
    pub fn new(extractor: E) -> Self {
        Self {
            extractor,
            filter: TrueFilter,
            _phantom: PhantomData,
        }
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
    pub fn flattened<B, Flat>(
        self,
        flatten: Flat,
    ) -> FlattenedCollectionTarget<S, A, B, E, F, FlattenVecExtract<Flat>, Sc>
    where
        E: CollectionExtract<S, Item = A>,
        B: Clone + Send + Sync + 'static,
        Flat: for<'a> Fn(&'a A) -> &'a Vec<B> + Send + Sync,
    {
        FlattenedCollectionTarget {
            right_stream: self,
            flatten: FlattenVecExtract(flatten),
            _phantom: PhantomData,
        }
    }

    /* Adds a filter predicate to the stream. */
    pub fn filter<P>(
        self,
        predicate: P,
    ) -> UniConstraintStream<
        S,
        A,
        E,
        AndUniFilter<F, FnUniFilter<impl Fn(&S, &A) -> bool + Send + Sync>>,
        Sc,
    >
    where
        P: Fn(&A) -> bool + Send + Sync + 'static,
    {
        UniConstraintStream {
            extractor: self.extractor,
            filter: AndUniFilter::new(
                self.filter,
                FnUniFilter::new(move |_s: &S, a: &A| predicate(a)),
            ),
            _phantom: PhantomData,
        }
    }

    /* Joins this stream using the provided join target. */
    pub fn join<J>(self, target: J) -> J::Output
    where
        J: super::super::join_target::JoinTarget<S, A, E, F, Sc>,
    {
        target.apply(self.extractor, self.filter)
    }

    /* Groups entities by key and aggregates with a collector. */
    pub fn group_by<K, KF, C, V, R, Acc>(
        self,
        key_fn: KF,
        collector: C,
    ) -> GroupedConstraintStream<S, A, K, E, F, KF, C, V, R, Acc, Sc>
    where
        K: Clone + Eq + Hash + Send + Sync + 'static,
        KF: Fn(&A) -> K + Send + Sync,
        C: for<'i> Collector<&'i A, Value = V, Result = R, Accumulator = Acc>
            + Send
            + Sync
            + 'static,
        V: Send + Sync + 'static,
        R: Send + Sync + 'static,
        Acc: Accumulator<V, R> + Send + Sync + 'static,
    {
        GroupedConstraintStream::new(self.extractor, self.filter, key_fn, collector)
    }

    /* Creates a balance constraint that penalizes uneven distribution across groups. */
    pub fn balance<K, KF>(self, key_fn: KF) -> BalanceConstraintStream<S, A, K, E, F, KF, Sc>
    where
        K: Clone + Eq + Hash + Send + Sync + 'static,
        KF: Fn(&A) -> Option<K> + Send + Sync,
    {
        BalanceConstraintStream::new(self.extractor, self.filter, key_fn)
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
    pub fn if_exists<T>(self, target: T) -> T::Output
    where
        T: ExistenceTarget<S, A, E, F, Sc>,
    {
        target.apply(ExistenceMode::Exists, self.extractor, self.filter)
    }

    pub fn if_not_exists<T>(self, target: T) -> T::Output
    where
        T: ExistenceTarget<S, A, E, F, Sc>,
    {
        target.apply(ExistenceMode::NotExists, self.extractor, self.filter)
    }
}

impl<S, A, E, F, Sc: Score> UniConstraintStream<S, A, E, F, Sc> {
    #[doc(hidden)]
    pub fn extractor(&self) -> &E {
        &self.extractor
    }

    #[doc(hidden)]
    pub fn into_parts(self) -> (E, F) {
        (self.extractor, self.filter)
    }

    #[doc(hidden)]
    pub fn from_parts(extractor: E, filter: F) -> Self {
        Self {
            extractor,
            filter,
            _phantom: PhantomData,
        }
    }
}

impl<S, A, E, F, Sc: Score> std::fmt::Debug for UniConstraintStream<S, A, E, F, Sc> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UniConstraintStream").finish()
    }
}

impl<S, A, E, F, Sc> CollectionExtract<S> for UniConstraintStream<S, A, E, F, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    E: CollectionExtract<S, Item = A>,
    F: UniFilter<S, A>,
    Sc: Score + 'static,
{
    type Item = A;

    #[inline]
    fn extract<'s>(&self, s: &'s S) -> &'s [A] {
        self.extractor.extract(s)
    }

    #[inline]
    fn contains(&self, s: &S, item: &A) -> bool {
        self.filter.test(s, item)
    }

    #[inline]
    fn change_source(&self) -> ChangeSource {
        self.extractor.change_source()
    }
}
