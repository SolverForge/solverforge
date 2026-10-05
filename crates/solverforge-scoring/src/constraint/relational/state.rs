/* Generic binary relational terminal: operator state + retained scores.

Owns one `EquiJoin` plus the signed score per terminal pair, keyed by
semantic (left, right) indexes. The operator owns the only extractor and
key-closure copies; full evaluation reads them back through operator
accessors, so nothing is cloned and nothing drifts. Changing a key or
value retracts the previously computed score from retention — never a
recomputed new one — so signed deltas stay exact across mutations.
*/

use std::collections::HashMap;
use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use crate::stream::collection_extract::CollectionExtract;
use crate::stream::relational::EquiJoin;

use super::weight::RelationalWeight;

/* Zero-erasure generic terminal over one binary equi-join.

All function types are concrete generics - no trait objects, no Arc.
`K` is this join's own key type; successive joins keep their own.
*/
pub struct Terminal<S, A, B, K, EA, EB, KA, KB, F, W, Sc>
where
    Sc: Score,
{
    pub(super) constraint_ref: ConstraintRef,
    pub(super) impact_type: ImpactType,
    pub(super) weight: W,
    pub(super) is_hard: bool,
    pub(super) operator: EquiJoin<S, A, B, EA, EB, K, KA, KB, F>,
    pub(super) pair_scores: HashMap<(usize, usize), Sc>,
    pub(super) initialized: bool,
    pub(super) _phantom: PhantomData<(fn() -> S, fn() -> A, fn() -> B)>,
}

impl<S, A, B, K, EA, EB, KA, KB, F, W, Sc> Terminal<S, A, B, K, EA, EB, KA, KB, F, W, Sc>
where
    S: 'static,
    A: Clone + 'static,
    B: Clone + 'static,
    K: Eq + Hash + Clone,
    EA: CollectionExtract<S, Item = A>,
    EB: CollectionExtract<S, Item = B>,
    KA: Fn(&A) -> K,
    KB: Fn(&B) -> K,
    F: Fn(&S, &A, &B, usize, usize) -> bool,
    W: RelationalWeight<S, A, B, Sc>,
    Sc: Score,
{
    /* Creates a terminal over one binary equi-join with its own key type. */
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        constraint_ref: ConstraintRef,
        impact_type: ImpactType,
        extractor_a: EA,
        extractor_b: EB,
        key_a: KA,
        key_b: KB,
        filter: F,
        weight: W,
        is_hard: bool,
        left_descriptor: usize,
        right_descriptor: usize,
        name: &str,
    ) -> Self {
        Terminal {
            constraint_ref,
            impact_type,
            weight,
            is_hard,
            operator: EquiJoin::new(
                extractor_a,
                extractor_b,
                key_a,
                key_b,
                filter,
                left_descriptor,
                right_descriptor,
                name,
            ),
            pair_scores: HashMap::new(),
            initialized: false,
            _phantom: PhantomData,
        }
    }

    pub(super) fn signed(&self, base: Sc) -> Sc {
        match self.impact_type {
            ImpactType::Penalty => -base,
            ImpactType::Reward => base,
        }
    }
}
