use solverforge_core::score::Score;

use crate::api::constraint_set::SingleConstraint;
use crate::stream::projected::Source;

use super::{
    projected::{Bi as ProjectedBi, DirectedBi as ProjectedDirectedBi, Uni as ProjectedUni},
    BalanceConstraint, IncrementalUniConstraint,
};

impl<S, O, W, Sc> SingleConstraint for super::relational::OperatorTerminal<S, O, W, Sc> {}

impl<S, A, E, F, W, Sc> SingleConstraint for IncrementalUniConstraint<S, A, E, F, W, Sc> where
    Sc: Score
{
}

impl<S, A, K, E, F, KF, Sc> SingleConstraint for BalanceConstraint<S, A, K, E, F, KF, Sc> where
    Sc: Score
{
}

impl<S, Out, Src, F, W, Sc> SingleConstraint for ProjectedUni<S, Out, Src, F, W, Sc>
where
    Src: Source<S, Out>,
    Sc: Score,
{
}

impl<S, Out, K, Src, F, KF, PF, W, Sc> SingleConstraint
    for ProjectedBi<S, Out, K, Src, F, KF, PF, W, Sc>
where
    Src: Source<S, Out>,
    Sc: Score,
{
}

impl<S, Out, K, Src, F, KL, KR, PF, W, Sc> SingleConstraint
    for ProjectedDirectedBi<S, Out, K, Src, F, KL, KR, PF, W, Sc>
where
    Src: Source<S, Out>,
    Sc: Score,
{
}
