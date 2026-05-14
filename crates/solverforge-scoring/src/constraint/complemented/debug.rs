use crate::stream::collector::{Accumulator, Collector};
use solverforge_core::score::Score;

use super::ComplementedGroupConstraint;

impl<S, A, B, K, EA, EB, KA, KB, C, V, R, Acc, D, W, Sc> std::fmt::Debug
    for ComplementedGroupConstraint<S, A, B, K, EA, EB, KA, KB, C, V, R, Acc, D, W, Sc>
where
    C: for<'i> Collector<&'i A, Value = V, Result = R, Accumulator = Acc>,
    Acc: Accumulator<V, R>,
    Sc: Score,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ComplementedGroupConstraint")
            .field("name", &self.constraint_ref.name)
            .field("impact_type", &self.impact_type)
            .field("groups", &self.groups.len())
            .finish()
    }
}
