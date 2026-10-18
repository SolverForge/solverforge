use solverforge_core::score::Score;

use crate::api::constraint_set::IncrementalConstraintSealed;
use crate::stream::projected::Source;

use super::{
    projected::{Bi as ProjectedBi, DirectedBi as ProjectedDirectedBi, Uni as ProjectedUni},
    BalanceConstraint, IncrementalUniConstraint,
};

impl<S, O, W, Sc> IncrementalConstraintSealed for super::relational::OperatorTerminal<S, O, W, Sc> {}

impl<S, A, E, F, W, Sc> IncrementalConstraintSealed for IncrementalUniConstraint<S, A, E, F, W, Sc> where
    Sc: Score
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
