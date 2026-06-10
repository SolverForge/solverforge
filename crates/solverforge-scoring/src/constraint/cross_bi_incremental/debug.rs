use solverforge_core::score::Score;

use super::Bi;

impl<S, A, B, K, EA, EB, KA, KB, F, W, Sc: Score> std::fmt::Debug
    for Bi<S, A, B, K, EA, EB, KA, KB, F, W, Sc>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Bi")
            .field("name", &self.constraint_ref.name)
            .field("impact_type", &self.impact_type)
            .field("match_count", &self.matches.len())
            .finish()
    }
}
