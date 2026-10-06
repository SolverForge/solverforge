/* Adapters that let entity collectors drive the leaf-view `GroupNode`.

`GroupNode` groups operator rows, whose view is `Leaf<'a, A>`. The fluent
collector surface is written against `&A`. `LeafCollector` bridges the two
without cloning the collector or the entity. `V`, `R`, `Acc` are named so the
`for<'i> Collector<&'i A>` bound can expose its associated types.
*/

use std::marker::PhantomData;

use crate::stream::collector::{Accumulator, Collector};
use crate::stream::relational::Leaf;

pub struct LeafCollector<C, V, R, Acc> {
    inner: C,
    marker: PhantomData<fn() -> (V, R, Acc)>,
}

impl<C, V, R, Acc> LeafCollector<C, V, R, Acc> {
    pub fn new(inner: C) -> Self {
        Self {
            inner,
            marker: PhantomData,
        }
    }
}

impl<A, C, V, R, Acc> Collector<Leaf<'_, A>> for LeafCollector<C, V, R, Acc>
where
    C: for<'i> Collector<&'i A, Value = V, Result = R, Accumulator = Acc>,
    Acc: Accumulator<V, R>,
    R: Send + Sync,
{
    type Value = V;
    type Result = R;
    type Accumulator = Acc;

    #[inline]
    fn extract(&self, input: Leaf<'_, A>) -> Self::Value {
        self.inner.extract(input.entity)
    }

    #[inline]
    fn create_accumulator(&self) -> Self::Accumulator {
        self.inner.create_accumulator()
    }
}

/* Reads an entity key off a leaf view and wraps it as `Some`, so a target
domain joins against an `Option`-keyed group without a capturing closure in
the plan's type. */
pub struct OptionalEntityKey<F>(F);

impl<F> OptionalEntityKey<F> {
    pub fn new(inner: F) -> Self {
        Self(inner)
    }
}

impl<B, F, K> crate::stream::relational::view_plan::ViewKey<Leaf<'_, B>> for OptionalEntityKey<F>
where
    F: Fn(&B) -> K + Send + Sync,
{
    type Key = Option<K>;

    #[inline]
    fn key(&self, view: &Leaf<'_, B>) -> Option<K> {
        Some((self.0)(view.entity))
    }
}

/* A key read straight off a leaf view's entity. */
pub fn leaf_key<A, K, F>(f: F) -> impl Fn(&Leaf<'_, A>) -> K + Send + Sync
where
    F: Fn(&A) -> K + Send + Sync,
{
    move |leaf: &Leaf<'_, A>| f(leaf.entity)
}
