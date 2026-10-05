use std::hash::Hash;

use solverforge_core::score::Score;

use crate::stream::collection_extract::CollectionExtract;

use super::state::Terminal;

impl<S, A, B, K, EA, EB, KA, KB, F, W, Sc: Score> std::fmt::Debug
    for Terminal<S, A, B, K, EA, EB, KA, KB, F, W, Sc>
where
    K: Eq + Hash + Clone,
    EA: CollectionExtract<S, Item = A>,
    EB: CollectionExtract<S, Item = B>,
    KA: Fn(&A) -> K,
    KB: Fn(&B) -> K,
    F: Fn(&S, &A, &B, usize, usize) -> bool,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Terminal")
            .field("name", &self.constraint_ref.name)
            .field("impact_type", &self.impact_type)
            .field("match_count", &self.pair_scores.len())
            .field("retained_pairs", &self.operator.output_rows())
            .finish()
    }
}
