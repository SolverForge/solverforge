use solverforge_core::score::Score;

use crate::api::constraint_set::IncrementalConstraintSealed;
use crate::stream::collector::Accumulator;
use crate::stream::ProjectedSource;

use super::{
    projected::ProjectedBiConstraint, BalanceConstraint, ComplementedGroupConstraint,
    CrossComplementedGroupedConstraint, CrossGroupedConstraint, FlattenedBiConstraint,
    GroupedUniConstraint, IncrementalBiConstraint, IncrementalCrossBiConstraint,
    IncrementalExistsConstraint, IncrementalPentaConstraint, IncrementalQuadConstraint,
    IncrementalTriConstraint, IncrementalUniConstraint, ProjectedComplementedGroupedConstraint,
    ProjectedGroupedConstraint, ProjectedUniConstraint,
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
    for IncrementalCrossBiConstraint<S, A, B, K, EA, EB, KA, KB, F, W, Sc>
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
    for GroupedUniConstraint<S, A, K, E, Fi, KF, C, V, R, Acc, W, Sc>
where
    Acc: Accumulator<V, R>,
    Sc: Score,
{
}

impl<S, A, B, JK, GK, EA, EB, KA, KB, F, GF, C, V, R, Acc, W, Sc> IncrementalConstraintSealed
    for CrossGroupedConstraint<S, A, B, JK, GK, EA, EB, KA, KB, F, GF, C, V, R, Acc, W, Sc>
where
    Acc: Accumulator<V, R>,
    Sc: Score,
{
}

impl<S, A, B, K, EA, EB, KA, KB, C, V, R, Acc, D, W, Sc> IncrementalConstraintSealed
    for ComplementedGroupConstraint<S, A, B, K, EA, EB, KA, KB, C, V, R, Acc, D, W, Sc>
where
    Acc: Accumulator<V, R>,
    Sc: Score,
{
}

impl<S, A, B, T, JK, GK, EA, EB, ET, KA, KB, F, GF, KT, C, V, R, Acc, D, W, Sc>
    IncrementalConstraintSealed
    for CrossComplementedGroupedConstraint<
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

impl<S, Out, Src, F, W, Sc> IncrementalConstraintSealed
    for ProjectedUniConstraint<S, Out, Src, F, W, Sc>
where
    Src: ProjectedSource<S, Out>,
    Sc: Score,
{
}

impl<S, Out, K, Src, F, KF, PF, W, Sc> IncrementalConstraintSealed
    for ProjectedBiConstraint<S, Out, K, Src, F, KF, PF, W, Sc>
where
    Src: ProjectedSource<S, Out>,
    Sc: Score,
{
}

impl<S, Out, K, Src, F, KF, C, V, R, Acc, W, Sc> IncrementalConstraintSealed
    for ProjectedGroupedConstraint<S, Out, K, Src, F, KF, C, V, R, Acc, W, Sc>
where
    Acc: Accumulator<V, R>,
    Src: ProjectedSource<S, Out>,
    Sc: Score,
{
}

impl<S, Out, B, K, Src, EB, F, KA, KB, C, V, R, Acc, D, W, Sc> IncrementalConstraintSealed
    for ProjectedComplementedGroupedConstraint<
        S,
        Out,
        B,
        K,
        Src,
        EB,
        F,
        KA,
        KB,
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
    Src: ProjectedSource<S, Out>,
    Sc: Score,
{
}
