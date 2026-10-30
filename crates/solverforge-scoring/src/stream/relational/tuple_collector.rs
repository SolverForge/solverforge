use super::{operator::Pair, Leaf};
use crate::stream::collector::{Accumulator, Collector};
use std::marker::PhantomData;

/// Adapts a tuple collector to borrowed operator rows without copying entities.
pub struct TupleCollector<C, V, R, Acc> {
    inner: C,
    marker: PhantomData<fn() -> (V, R, Acc)>,
}
impl<C, V, R, Acc> TupleCollector<C, V, R, Acc> {
    /// Wraps `inner`, which collects tuples of entity references.
    pub fn new(inner: C) -> Self {
        Self {
            inner,
            marker: PhantomData,
        }
    }
}

macro_rules! impl_tuple_collector {
    ([$($ty:ident),+], $row:ty, |$input:ident| $tuple:expr) => {
        impl<'a, $($ty: 'a),+, C, V, R, Acc> Collector<$row> for TupleCollector<C, V, R, Acc>
        where
            C: Collector<($(&'a $ty),+), Value = V, Result = R, Accumulator = Acc>,
            Acc: Accumulator<V, R>,
            R: Send + Sync,
        {
            type Value = V;
            type Result = R;
            type Accumulator = Acc;
            fn extract(&self, $input: $row) -> V { self.inner.extract($tuple) }
            fn create_accumulator(&self) -> Acc { self.inner.create_accumulator() }
        }
    };
}
impl_tuple_collector!([A, B], Pair<Leaf<'a, A>, Leaf<'a, B>>, |row| (
    row.left.entity,
    row.right.entity
));
impl_tuple_collector!(
    [A, B, D],
    Pair<Pair<Leaf<'a, A>, Leaf<'a, B>>, Leaf<'a, D>>,
    |row| (
        row.left.left.entity,
        row.left.right.entity,
        row.right.entity
    )
);
impl_tuple_collector!(
    [A, B, D, E],
    Pair<Pair<Pair<Leaf<'a, A>, Leaf<'a, B>>, Leaf<'a, D>>, Leaf<'a, E>>,
    |row| (
        row.left.left.left.entity,
        row.left.left.right.entity,
        row.left.right.entity,
        row.right.entity
    )
);
impl_tuple_collector!(
    [A, B, D, E, F],
    Pair<Pair<Pair<Pair<Leaf<'a, A>, Leaf<'a, B>>, Leaf<'a, D>>, Leaf<'a, E>>, Leaf<'a, F>>,
    |row| (
        row.left.left.left.left.entity,
        row.left.left.left.right.entity,
        row.left.left.right.entity,
        row.left.right.entity,
        row.right.entity
    )
);

macro_rules! impl_array_collector {
    ($n:literal, [$($index:literal),+]) => {
        impl<'a, A: 'a, C, V, R, Acc> Collector<[Leaf<'a, A>; $n]> for TupleCollector<C, V, R, Acc>
        where
            C: Collector<($(impl_array_collector!(@ty $index, &'a A)),+), Value = V, Result = R, Accumulator = Acc>,
            Acc: Accumulator<V, R>,
            R: Send + Sync,
        {
            type Value = V;
            type Result = R;
            type Accumulator = Acc;
            fn extract(&self, row: [Leaf<'a, A>; $n]) -> V { self.inner.extract(($(row[$index].entity),+)) }
            fn create_accumulator(&self) -> Acc { self.inner.create_accumulator() }
        }
    };
    (@ty $index:literal, $ty:ty) => { $ty };
}
impl_array_collector!(3, [0, 1, 2]);
impl_array_collector!(4, [0, 1, 2, 3]);
impl_array_collector!(5, [0, 1, 2, 3, 4]);
