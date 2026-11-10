//! Closures the stream builders hand to constraints and filter nodes.
//!
//! A closure written inside a generic method has a type that names every
//! generic parameter of that method, used or not, so a closure created by a
//! stream combinator would spell out the whole upstream stream type (its
//! extractor, its filter chain, ...) again. Debug info prints these types in
//! full, and nested combinators made single type names megabytes long. Each
//! function here is generic only over what its closure captures and takes, so
//! the closure type names exactly those parts.

use solverforge_core::score::Score;

use super::filter::{BiFilter, PentaFilter, QuadFilter, TriFilter, UniFilter};
use super::joiner::Joiner;
use super::relational::operator::{FlattenView, Pair};
use super::relational::Leaf;
use super::weighting_support::ConstraintWeight;

/// A uni filter that ignores the solution.
pub(crate) fn entity_filter<S, A, P>(predicate: P) -> impl Fn(&S, &A) -> bool + Send + Sync
where
    P: Fn(&A) -> bool + Send + Sync,
{
    move |_s: &S, a: &A| predicate(a)
}

/// A pair filter that ignores the solution and the row indexes.
pub(crate) fn pair_filter<S, A, B, P>(
    predicate: P,
) -> impl Fn(&S, &A, &B, usize, usize) -> bool + Send + Sync
where
    P: Fn(&A, &B) -> bool + Send + Sync,
{
    move |_s: &S, a: &A, b: &B, _a_idx: usize, _b_idx: usize| predicate(a, b)
}

/// A triple filter that ignores the solution and the row indexes.
pub(crate) fn triple_filter<S, A, B, C, P>(
    predicate: P,
) -> impl Fn(&S, &A, &B, &C, usize, usize, usize) -> bool + Send + Sync
where
    P: Fn(&A, &B, &C) -> bool + Send + Sync,
{
    move |_s: &S, a: &A, b: &B, c: &C, _a_idx: usize, _b_idx: usize, _c_idx: usize| {
        predicate(a, b, c)
    }
}

/// A quadruple filter that ignores the solution and the row indexes.
#[allow(clippy::type_complexity)]
pub(crate) fn quad_filter<S, A, B, C, D, P>(
    predicate: P,
) -> impl Fn(&S, &A, &B, &C, &D, usize, usize, usize, usize) -> bool + Send + Sync
where
    P: Fn(&A, &B, &C, &D) -> bool + Send + Sync,
{
    move |_s: &S, a: &A, b: &B, c: &C, d: &D, _ai: usize, _bi: usize, _ci: usize, _di: usize| {
        predicate(a, b, c, d)
    }
}

/// A quintuple filter that ignores the solution and the row indexes.
#[allow(clippy::type_complexity)]
pub(crate) fn penta_filter<S, A, B, C, D, E, P>(
    predicate: P,
) -> impl Fn(&S, &A, &B, &C, &D, &E, usize, usize, usize, usize, usize) -> bool + Send + Sync
where
    P: Fn(&A, &B, &C, &D, &E) -> bool + Send + Sync,
{
    move |_s: &S,
          a: &A,
          b: &B,
          c: &C,
          d: &D,
          e: &E,
          _ai: usize,
          _bi: usize,
          _ci: usize,
          _di: usize,
          _ei: usize| predicate(a, b, c, d, e)
}

/// A uni filter as the closure an `IncrementalUniConstraint` tests.
pub(crate) fn uni_filter_fn<S, A, F>(filter: F) -> impl Fn(&S, &A) -> bool + Send + Sync
where
    F: UniFilter<S, A>,
{
    move |s: &S, a: &A| filter.test(s, a)
}

/// `key` wrapped so every row has a key.
pub(crate) fn some_key<A, K, F>(key: F) -> impl Fn(&A) -> Option<K> + Send + Sync
where
    F: Fn(&A) -> K + Send + Sync,
{
    move |a: &A| Some(key(a))
}

/// `weight` scored on a leaf row's entity.
pub(crate) fn leaf_weight<S, A, Sc, W>(weight: W) -> impl Fn(&S, &Leaf<'_, A>) -> Sc + Send + Sync
where
    Sc: Score,
    W: for<'w> ConstraintWeight<(&'w A,), Sc> + Send + Sync,
{
    move |_s: &S, row: &Leaf<'_, A>| weight.score((row.entity,))
}

/// `default` applied to a leaf row's entity.
pub(crate) fn leaf_default<S, B, R, F>(default: F) -> impl Fn(&S, &Leaf<'_, B>) -> R + Send + Sync
where
    F: Fn(&B) -> R + Send + Sync,
{
    move |_s: &S, row: &Leaf<'_, B>| default(row.entity)
}

/// `weight` applied to the two entities of a joined pair row.
pub(crate) fn pair_row_weight<S, A, B, Sc, W>(
    weight: W,
) -> impl Fn(&S, &Pair<Leaf<'_, A>, Leaf<'_, B>>) -> Sc + Send + Sync
where
    W: Fn(&A, &B) -> Sc + Send + Sync,
{
    move |_s: &S, row: &Pair<Leaf<'_, A>, Leaf<'_, B>>| weight(row.left.entity, row.right.entity)
}

/// `key` applied to the two entities of a joined pair row.
pub(crate) fn pair_row_key<A, B, K, G>(
    key: G,
) -> impl Fn(&Pair<Leaf<'_, A>, Leaf<'_, B>>) -> K + Send + Sync
where
    G: Fn(&A, &B) -> K + Send + Sync,
{
    move |row: &Pair<Leaf<'_, A>, Leaf<'_, B>>| key(row.left.entity, row.right.entity)
}

/// `weight` applied to the three entities of a joined triple row.
#[allow(clippy::type_complexity)]
pub(crate) fn triple_row_weight<S, A, B, C, Sc, W>(
    weight: W,
) -> impl Fn(&S, &Pair<Pair<Leaf<'_, A>, Leaf<'_, B>>, Leaf<'_, C>>) -> Sc + Send + Sync
where
    W: Fn(&A, &B, &C) -> Sc + Send + Sync,
{
    move |_s: &S, row: &Pair<Pair<Leaf<'_, A>, Leaf<'_, B>>, Leaf<'_, C>>| {
        weight(
            row.left.left.entity,
            row.left.right.entity,
            row.right.entity,
        )
    }
}

/// `weight` applied to the four entities of a joined quadruple row.
#[allow(clippy::type_complexity)]
pub(crate) fn quad_row_weight<S, A, B, C, D, Sc, W>(
    weight: W,
) -> impl Fn(&S, &Pair<Pair<Pair<Leaf<'_, A>, Leaf<'_, B>>, Leaf<'_, C>>, Leaf<'_, D>>) -> Sc + Send + Sync
where
    W: Fn(&A, &B, &C, &D) -> Sc + Send + Sync,
{
    move |_s: &S, row: &Pair<Pair<Pair<Leaf<'_, A>, Leaf<'_, B>>, Leaf<'_, C>>, Leaf<'_, D>>| {
        weight(
            row.left.left.left.entity,
            row.left.left.right.entity,
            row.left.right.entity,
            row.right.entity,
        )
    }
}

/// `weight` applied to the five entities of a joined quintuple row.
#[allow(clippy::type_complexity)]
pub(crate) fn penta_row_weight<S, A, B, C, D, E, Sc, W>(
    weight: W,
) -> impl Fn(
    &S,
    &Pair<Pair<Pair<Pair<Leaf<'_, A>, Leaf<'_, B>>, Leaf<'_, C>>, Leaf<'_, D>>, Leaf<'_, E>>,
) -> Sc
       + Send
       + Sync
where
    W: Fn(&A, &B, &C, &D, &E) -> Sc + Send + Sync,
{
    move |_s: &S,
          row: &Pair<
        Pair<Pair<Pair<Leaf<'_, A>, Leaf<'_, B>>, Leaf<'_, C>>, Leaf<'_, D>>,
        Leaf<'_, E>,
    >| {
        weight(
            row.left.left.left.left.entity,
            row.left.left.left.right.entity,
            row.left.left.right.entity,
            row.left.right.entity,
            row.right.entity,
        )
    }
}

/// `weight` applied to the parent entity and flattened item of a flattened
/// pair row.
pub(crate) fn flattened_row_weight<S, A, B, C, Sc, W>(
    weight: W,
) -> impl Fn(&S, &Pair<Leaf<'_, A>, FlattenView<'_, Leaf<'_, B>, C>>) -> Sc + Send + Sync
where
    W: Fn(&A, &C) -> Sc + Send + Sync,
{
    move |_s: &S, row: &Pair<Leaf<'_, A>, FlattenView<'_, Leaf<'_, B>, C>>| {
        weight(row.left.entity, row.right.value)
    }
}

/// A bi filter extended by a self-join joiner into a triple filter.
pub(crate) fn self_join_triple_filter<S, A, F, J>(
    filter: F,
    joiner: J,
) -> impl Fn(&S, &A, &A, &A, usize, usize, usize) -> bool + Send + Sync
where
    F: BiFilter<S, A, A>,
    J: Joiner<A, A>,
{
    move |s: &S, a: &A, b: &A, c: &A, a_idx: usize, b_idx: usize, _c_idx: usize| {
        filter.test(s, a, b, a_idx, b_idx) && joiner.matches(a, c)
    }
}

/// A tri filter extended by a self-join joiner into a quadruple filter.
#[allow(clippy::type_complexity)]
pub(crate) fn self_join_quad_filter<S, A, F, J>(
    filter: F,
    joiner: J,
) -> impl Fn(&S, &A, &A, &A, &A, usize, usize, usize, usize) -> bool + Send + Sync
where
    F: TriFilter<S, A, A, A>,
    J: Joiner<A, A>,
{
    move |s: &S,
          a: &A,
          b: &A,
          c: &A,
          d: &A,
          a_idx: usize,
          b_idx: usize,
          c_idx: usize,
          _d_idx: usize| {
        filter.test(s, a, b, c, a_idx, b_idx, c_idx) && joiner.matches(a, d)
    }
}

/// A quad filter extended by a self-join joiner into a quintuple filter.
#[allow(clippy::type_complexity)]
pub(crate) fn self_join_penta_filter<S, A, F, J>(
    filter: F,
    joiner: J,
) -> impl Fn(&S, &A, &A, &A, &A, &A, usize, usize, usize, usize, usize) -> bool + Send + Sync
where
    F: QuadFilter<S, A, A, A, A>,
    J: Joiner<A, A>,
{
    move |s: &S,
          a: &A,
          b: &A,
          c: &A,
          d: &A,
          e: &A,
          a_idx: usize,
          b_idx: usize,
          c_idx: usize,
          d_idx: usize,
          _e_idx: usize| {
        filter.test(s, a, b, c, d, a_idx, b_idx, c_idx, d_idx) && joiner.matches(a, e)
    }
}

/// A tri filter tested on a joined triple row.
#[allow(clippy::type_complexity)]
pub(crate) fn triple_row_filter<S, A, B, C, F>(
    filter: F,
) -> impl Fn(&S, &Pair<Pair<Leaf<'_, A>, Leaf<'_, B>>, Leaf<'_, C>>) -> bool + Send + Sync
where
    F: TriFilter<S, A, B, C>,
{
    move |s: &S, row: &Pair<Pair<Leaf<'_, A>, Leaf<'_, B>>, Leaf<'_, C>>| {
        filter.test(
            s,
            row.left.left.entity,
            row.left.right.entity,
            row.right.entity,
            row.left.left.index,
            row.left.right.index,
            row.right.index,
        )
    }
}

/// `key` applied to the three entities of a joined triple row.
#[allow(clippy::type_complexity)]
pub(crate) fn triple_row_key<A, B, C, K, G>(
    key: G,
) -> impl Fn(&Pair<Pair<Leaf<'_, A>, Leaf<'_, B>>, Leaf<'_, C>>) -> K + Send + Sync
where
    G: Fn(&A, &B, &C) -> K + Send + Sync,
{
    move |row: &Pair<Pair<Leaf<'_, A>, Leaf<'_, B>>, Leaf<'_, C>>| {
        key(
            row.left.left.entity,
            row.left.right.entity,
            row.right.entity,
        )
    }
}

/// A quad filter tested on a joined quadruple row.
#[allow(clippy::type_complexity)]
pub(crate) fn quad_row_filter<S, A, B, C, D, F>(
    filter: F,
) -> impl Fn(&S, &Pair<Pair<Pair<Leaf<'_, A>, Leaf<'_, B>>, Leaf<'_, C>>, Leaf<'_, D>>) -> bool
       + Send
       + Sync
where
    F: QuadFilter<S, A, B, C, D>,
{
    move |s: &S, row: &Pair<Pair<Pair<Leaf<'_, A>, Leaf<'_, B>>, Leaf<'_, C>>, Leaf<'_, D>>| {
        filter.test(
            s,
            row.left.left.left.entity,
            row.left.left.right.entity,
            row.left.right.entity,
            row.right.entity,
            row.left.left.left.index,
            row.left.left.right.index,
            row.left.right.index,
            row.right.index,
        )
    }
}

/// `key` applied to the four entities of a joined quadruple row.
#[allow(clippy::type_complexity)]
pub(crate) fn quad_row_key<A, B, C, D, K, G>(
    key: G,
) -> impl Fn(&Pair<Pair<Pair<Leaf<'_, A>, Leaf<'_, B>>, Leaf<'_, C>>, Leaf<'_, D>>) -> K + Send + Sync
where
    G: Fn(&A, &B, &C, &D) -> K + Send + Sync,
{
    move |row: &Pair<Pair<Pair<Leaf<'_, A>, Leaf<'_, B>>, Leaf<'_, C>>, Leaf<'_, D>>| {
        key(
            row.left.left.left.entity,
            row.left.left.right.entity,
            row.left.right.entity,
            row.right.entity,
        )
    }
}

/// A penta filter tested on a joined quintuple row.
#[allow(clippy::type_complexity)]
pub(crate) fn penta_row_filter<S, A, B, C, D, E, F>(
    filter: F,
) -> impl Fn(
    &S,
    &Pair<Pair<Pair<Pair<Leaf<'_, A>, Leaf<'_, B>>, Leaf<'_, C>>, Leaf<'_, D>>, Leaf<'_, E>>,
) -> bool
       + Send
       + Sync
where
    F: PentaFilter<S, A, B, C, D, E>,
{
    move |s: &S,
          row: &Pair<
        Pair<Pair<Pair<Leaf<'_, A>, Leaf<'_, B>>, Leaf<'_, C>>, Leaf<'_, D>>,
        Leaf<'_, E>,
    >| {
        filter.test(
            s,
            row.left.left.left.left.entity,
            row.left.left.left.right.entity,
            row.left.left.right.entity,
            row.left.right.entity,
            row.right.entity,
            row.left.left.left.left.index,
            row.left.left.left.right.index,
            row.left.left.right.index,
            row.left.right.index,
            row.right.index,
        )
    }
}

/// `key` applied to the five entities of a joined quintuple row.
#[allow(clippy::type_complexity)]
pub(crate) fn penta_row_key<A, B, C, D, E, K, G>(
    key: G,
) -> impl Fn(
    &Pair<Pair<Pair<Pair<Leaf<'_, A>, Leaf<'_, B>>, Leaf<'_, C>>, Leaf<'_, D>>, Leaf<'_, E>>,
) -> K
       + Send
       + Sync
where
    G: Fn(&A, &B, &C, &D, &E) -> K + Send + Sync,
{
    move |row: &Pair<
        Pair<Pair<Pair<Leaf<'_, A>, Leaf<'_, B>>, Leaf<'_, C>>, Leaf<'_, D>>,
        Leaf<'_, E>,
    >| {
        key(
            row.left.left.left.left.entity,
            row.left.left.left.right.entity,
            row.left.left.right.entity,
            row.left.right.entity,
            row.right.entity,
        )
    }
}

/// `weight` as a closure over 1 matched row.
pub(crate) fn weight1<A, Sc, W>(weight: W) -> impl Fn(&A) -> Sc + Send + Sync
where
    Sc: Score,
    W: for<'w> ConstraintWeight<(&'w A,), Sc> + Send + Sync,
{
    move |a: &A| weight.score((a,))
}

/// `weight` as a closure over 2 matched rows.
pub(crate) fn weight2<A, B, Sc, W>(weight: W) -> impl Fn(&A, &B) -> Sc + Send + Sync
where
    Sc: Score,
    W: for<'w> ConstraintWeight<(&'w A, &'w B), Sc> + Send + Sync,
{
    move |a: &A, b: &B| weight.score((a, b))
}

/// `weight` as a closure over 3 matched rows.
pub(crate) fn weight3<A, B, C, Sc, W>(weight: W) -> impl Fn(&A, &B, &C) -> Sc + Send + Sync
where
    Sc: Score,
    W: for<'w> ConstraintWeight<(&'w A, &'w B, &'w C), Sc> + Send + Sync,
{
    move |a: &A, b: &B, c: &C| weight.score((a, b, c))
}

/// `weight` as a closure over 4 matched rows.
pub(crate) fn weight4<A, B, C, D, Sc, W>(weight: W) -> impl Fn(&A, &B, &C, &D) -> Sc + Send + Sync
where
    Sc: Score,
    W: for<'w> ConstraintWeight<(&'w A, &'w B, &'w C, &'w D), Sc> + Send + Sync,
{
    move |a: &A, b: &B, c: &C, d: &D| weight.score((a, b, c, d))
}

/// `weight` as a closure over 5 matched rows.
pub(crate) fn weight5<A, B, C, D, E, Sc, W>(
    weight: W,
) -> impl Fn(&A, &B, &C, &D, &E) -> Sc + Send + Sync
where
    Sc: Score,
    W: for<'w> ConstraintWeight<(&'w A, &'w B, &'w C, &'w D, &'w E), Sc> + Send + Sync,
{
    move |a: &A, b: &B, c: &C, d: &D, e: &E| weight.score((a, b, c, d, e))
}
