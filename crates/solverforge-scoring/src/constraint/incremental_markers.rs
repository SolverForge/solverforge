use solverforge_core::score::Score;

use crate::api::constraint_set::IncrementalConstraintSealed;
use crate::stream::collector::Accumulator;
use crate::stream::projected::Source;

use super::{
    complemented::Grouped as ComplementedGrouped,
    cross_bi_incremental::Bi as CrossBi,
    cross_complemented_grouped::ComplementedGrouped as CrossComplementedGrouped,
    cross_grouped::Grouped as CrossGrouped,
    grouped::Uni as GroupedUni,
    projected::{
        Bi as ProjectedBi, ComplementedGrouped as ProjectedComplementedGrouped,
        DirectedBi as ProjectedDirectedBi, Grouped as ProjectedGrouped, Uni as ProjectedUni,
    },
    BalanceConstraint, FlattenedBiConstraint, IncrementalBiConstraint, IncrementalExistsConstraint,
    IncrementalPentaConstraint, IncrementalQuadConstraint, IncrementalTriConstraint,
    IncrementalUniConstraint,
};

impl<S, A, E, F, W, Sc> IncrementalConstraintSealed for IncrementalUniConstraint<S, A, E, F, W, Sc> where
    Sc: Score
{
}

impl<S, A, K, E, KE, F, W, Sc> IncrementalConstraintSealed
    for IncrementalBiConstraint<S, A, K, E, KE, F, W, Sc>
where
    Sc: Score,
{
}

impl<S, A, K, E, KE, F, W, Sc> IncrementalConstraintSealed
    for IncrementalTriConstraint<S, A, K, E, KE, F, W, Sc>
where
    Sc: Score,
{
}

impl<S, A, K, E, KE, F, W, Sc> IncrementalConstraintSealed
    for IncrementalQuadConstraint<S, A, K, E, KE, F, W, Sc>
where
    Sc: Score,
{
}

impl<S, A, K, E, KE, F, W, Sc> IncrementalConstraintSealed
    for IncrementalPentaConstraint<S, A, K, E, KE, F, W, Sc>
where
    Sc: Score,
{
}

impl<S, A, B, K, EA, EB, KA, KB, F, W, Sc> IncrementalConstraintSealed
    for CrossBi<S, A, B, K, EA, EB, KA, KB, F, W, Sc>
where
    Sc: Score,
{
}

impl<S, A, B, C, K, CK, EA, EB, KA, KB, Flatten, CKeyFn, ALookup, F, W, Sc>
    IncrementalConstraintSealed
    for FlattenedBiConstraint<S, A, B, C, K, CK, EA, EB, KA, KB, Flatten, CKeyFn, ALookup, F, W, Sc>
where
    Sc: Score,
{
}

impl<S, A, K, E, Fi, KF, C, V, R, Acc, W, Sc> IncrementalConstraintSealed
    for GroupedUni<S, A, K, E, Fi, KF, C, V, R, Acc, W, Sc>
where
    Acc: Accumulator<V, R>,
    Sc: Score,
{
}

impl<S, A, B, JK, GK, EA, EB, KA, KB, F, GF, C, V, R, Acc, W, Sc> IncrementalConstraintSealed
    for CrossGrouped<S, A, B, JK, GK, EA, EB, KA, KB, F, GF, C, V, R, Acc, W, Sc>
where
    Acc: Accumulator<V, R>,
    Sc: Score,
{
}

impl<S, A, B, K, EA, EB, KA, KB, C, V, R, Acc, D, W, Sc> IncrementalConstraintSealed
    for ComplementedGrouped<S, A, B, K, EA, EB, KA, KB, C, V, R, Acc, D, W, Sc>
where
    Acc: Accumulator<V, R>,
    Sc: Score,
{
}

impl<S, A, B, T, JK, GK, EA, EB, ET, KA, KB, F, GF, KT, C, V, R, Acc, D, W, Sc>
    IncrementalConstraintSealed
    for CrossComplementedGrouped<
        S,
        A,
        B,
        T,
        JK,
        GK,
        EA,
        EB,
        ET,
        KA,
        KB,
        F,
        GF,
        KT,
        C,
        V,
        R,
        Acc,
        D,
        W,
        Sc,
    >
where
    Acc: Accumulator<V, R>,
    Sc: Score,
{
}

impl<S, A, P, B, K, EA, EP, KA, KB, FA, FP, Flatten, W, Sc> IncrementalConstraintSealed
    for IncrementalExistsConstraint<S, A, P, B, K, EA, EP, KA, KB, FA, FP, Flatten, W, Sc>
where
    Sc: Score,
{
}

impl<S, A, K, E, F, KF, Sc> IncrementalConstraintSealed for BalanceConstraint<S, A, K, E, F, KF, Sc> where
    Sc: Score
{
}

impl<S, Out, Src, F, W, Sc> IncrementalConstraintSealed for ProjectedUni<S, Out, Src, F, W, Sc>
where
    Src: Source<S, Out>,
    Sc: Score,
{
}

impl<S, Out, K, Src, F, KF, PF, W, Sc> IncrementalConstraintSealed
    for ProjectedBi<S, Out, K, Src, F, KF, PF, W, Sc>
where
    Src: Source<S, Out>,
    Sc: Score,
{
}

impl<S, Out, K, Src, F, KL, KR, PF, W, Sc> IncrementalConstraintSealed
    for ProjectedDirectedBi<S, Out, K, Src, F, KL, KR, PF, W, Sc>
where
    Src: Source<S, Out>,
    Sc: Score,
{
}

impl<S, Out, K, Src, F, KF, C, V, R, Acc, W, Sc> IncrementalConstraintSealed
    for ProjectedGrouped<S, Out, K, Src, F, KF, C, V, R, Acc, W, Sc>
where
    Acc: Accumulator<V, R>,
    Src: Source<S, Out>,
    Sc: Score,
{
}

impl<S, Out, B, K, Src, EB, F, KA, KB, C, V, R, Acc, D, W, Sc> IncrementalConstraintSealed
    for ProjectedComplementedGrouped<S, Out, B, K, Src, EB, F, KA, KB, C, V, R, Acc, D, W, Sc>
where
    Acc: Accumulator<V, R>,
    Src: Source<S, Out>,
    Sc: Score,
{
}
