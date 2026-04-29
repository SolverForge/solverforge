use crate::stream::collector::UniCollector;
use solverforge_core::score::Score;

use super::ComplementedGroupConstraint;

impl<S, A, B, K, EA, EB, KA, KB, C, D, W, Sc> std::fmt::Debug
    for ComplementedGroupConstraint<S, A, B, K, EA, EB, KA, KB, C, D, W, Sc>
where
    C: UniCollector<A>,
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
