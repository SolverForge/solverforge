use solverforge_core::score::Score;

use super::Tri;

impl<S, A, B, C, K, EA, EB, EC, KA, KB, KC, F, W, Sc: Score> std::fmt::Debug
    for Tri<S, A, B, C, K, EA, EB, EC, KA, KB, KC, F, W, Sc>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tri")
            .field("name", &self.constraint_ref.name)
            .field("impact_type", &self.impact_type)
            .field("match_count", &self.engine.match_count())
            .field("retained_tuples", &self.engine.retained_tuples())
            .finish()
    }
}
