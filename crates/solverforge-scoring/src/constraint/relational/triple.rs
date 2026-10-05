/* Generic triple relational terminal: chained operator state + retained scores.

Owns one `ChainedJoin` plus the signed score per terminal triple, keyed by
semantic (a, b, c) indexes. The chained operator owns the only extractor
and key-closure copies; full evaluation reads them back through operator
accessors, so nothing is cloned and nothing drifts. Changing any key or
value retracts the previously computed score from retention — never a
recomputed new one — so signed deltas stay exact across mutations.
*/

use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use crate::api::analysis::{ConstraintJustification, DetailedConstraintMatch, EntityRef};
use crate::api::constraint_set::IncrementalConstraint;
use crate::stream::collection_extract::CollectionExtract;
use crate::stream::relational::{ChainedJoin, Concat, DeltaKind, Leaf};

use super::weight::RelationalWeight3;

/* Zero-erasure generic terminal over one chained equi-join.

All function types are concrete generics - no trait objects, no Arc.
`K1` is the first join's key type, `K2` the second's; they never unify.
`LK` sees the whole left row, so the second relationship can inspect any
earlier binding.
*/
pub struct TripleTerminal<S, A, B, C, EA, EB, EC, K1, KA, KB, K2, LK, KC, F1, F2, W, Sc>
where
    Sc: Score,
{
    constraint_ref: ConstraintRef,
    impact_type: ImpactType,
    weight: W,
    is_hard: bool,
    operator: ChainedJoin<S, A, B, C, EA, EB, EC, K1, KA, KB, K2, LK, KC, F1, F2>,
    triple_scores: HashMap<(usize, usize, usize), Sc>,
    initialized: bool,
    _phantom: PhantomData<(fn() -> S, fn() -> A, fn() -> B, fn() -> C)>,
}

impl<S, A, B, C, EA, EB, EC, K1, KA, KB, K2, LK, KC, F1, F2, W, Sc>
    TripleTerminal<S, A, B, C, EA, EB, EC, K1, KA, KB, K2, LK, KC, F1, F2, W, Sc>
where
    S: 'static,
    A: Clone + 'static,
    B: Clone + 'static,
    C: Clone + 'static,
    K1: Eq + Hash + Clone,
    KA: Fn(&A) -> K1,
    KB: Fn(&B) -> K1,
    F1: Fn(&S, &A, &B, usize, usize) -> bool,
    K2: Eq + Hash + Clone,
    LK: for<'r> Fn(&Concat<Leaf<'r, A>, B>) -> K2,
    KC: Fn(&C) -> K2,
    F2: Fn(&S, &A, &B, &C, usize, usize, usize) -> bool,
    EC: CollectionExtract<S, Item = C>,
    W: RelationalWeight3<S, A, B, C, Sc>,
    Sc: Score,
{
    /* Creates a terminal over one chained equi-join with independent key types. */
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        constraint_ref: ConstraintRef,
        impact_type: ImpactType,
        first: crate::stream::relational::EquiJoin<S, A, B, EA, EB, K1, KA, KB, F1>,
        right_extractor: EC,
        left_key: LK,
        right_key: KC,
        filter: F2,
        weight: W,
        is_hard: bool,
        left_a_descriptor: usize,
        left_b_descriptor: usize,
        right_descriptor: usize,
        name: &str,
    ) -> Self
    where
        EA: CollectionExtract<S, Item = A>,
        EB: CollectionExtract<S, Item = B>,
    {
        TripleTerminal {
            constraint_ref,
            impact_type,
            weight,
            is_hard,
            operator: ChainedJoin::new(
                first,
                right_extractor,
                left_key,
                right_key,
                filter,
                left_a_descriptor,
                left_b_descriptor,
                right_descriptor,
                name,
            ),
            triple_scores: HashMap::new(),
            initialized: false,
            _phantom: PhantomData,
        }
    }

    fn signed(&self, base: Sc) -> Sc {
        match self.impact_type {
            ImpactType::Penalty => -base,
            ImpactType::Reward => base,
        }
    }
}

impl<S, A, B, C, EA, EB, EC, K1, KA, KB, K2, LK, KC, F1, F2, W, Sc> IncrementalConstraint<S, Sc>
    for TripleTerminal<S, A, B, C, EA, EB, EC, K1, KA, KB, K2, LK, KC, F1, F2, W, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Debug + Send + Sync + 'static,
    B: Clone + Debug + Send + Sync + 'static,
    C: Clone + Debug + Send + Sync + 'static,
    EA: CollectionExtract<S, Item = A> + Send + Sync,
    EB: CollectionExtract<S, Item = B> + Send + Sync,
    EC: CollectionExtract<S, Item = C> + Send + Sync,
    K1: Eq + Hash + Clone + Send + Sync,
    KA: Fn(&A) -> K1 + Send + Sync,
    KB: Fn(&B) -> K1 + Send + Sync,
    F1: Fn(&S, &A, &B, usize, usize) -> bool + Send + Sync,
    K2: Eq + Hash + Clone + Send + Sync,
    LK: for<'r> Fn(&Concat<Leaf<'r, A>, B>) -> K2 + Send + Sync,
    KC: Fn(&C) -> K2 + Send + Sync,
    F2: Fn(&S, &A, &B, &C, usize, usize, usize) -> bool + Send + Sync,
    W: RelationalWeight3<S, A, B, C, Sc>,
    Sc: Score,
{
    /* Full evaluation without retained state: meaningful uninitialized. */
    fn evaluate(&self, solution: &S) -> Sc {
        let first = self.operator.first();
        let entities_a = first.left_extractor().extract(solution);
        let entities_b = first.right_extractor().extract(solution);
        let filter = self.operator.filter();
        let mut total = Sc::zero();

        // Independent nested loops in authored binding order: the oracle
        // shape, not production indexes. The first join's filter applies
        // here exactly as the operator applies it during refresh — the
        // stateless paths must reject the same rows.
        let first_filter = first.filter();
        for (a_idx, a) in entities_a.iter().enumerate() {
            if !first.left_extractor().contains(solution, a) {
                continue;
            }
            for (b_idx, b) in entities_b.iter().enumerate() {
                if !first.right_extractor().contains(solution, b) {
                    continue;
                }
                if !self.first_keys_equal(a, b) {
                    continue;
                }
                if !(first_filter)(solution, a, b, a_idx, b_idx) {
                    continue;
                }
                let row = Concat::new(Leaf::new(a, a_idx), b, b_idx);
                let left_key = (self.operator.left_key())(&row);
                for (c_idx, c) in self.right_entities(solution).iter().enumerate() {
                    if !self.right_contains(solution, c) {
                        continue;
                    }
                    if !self.right_key_equals(&left_key, c) {
                        continue;
                    }
                    if !(filter)(solution, a, b, c, a_idx, b_idx, c_idx) {
                        continue;
                    }
                    let base = self.weight.score(
                        solution,
                        entities_a,
                        entities_b,
                        self.right_entities(solution),
                        a_idx,
                        b_idx,
                        c_idx,
                    );
                    total = total + self.signed(base);
                }
            }
        }

        total
    }

    fn match_count(&self, solution: &S) -> usize {
        let first = self.operator.first();
        let entities_a = first.left_extractor().extract(solution);
        let entities_b = first.right_extractor().extract(solution);
        let filter = self.operator.filter();
        let first_filter = first.filter();
        let mut count = 0;

        for (a_idx, a) in entities_a.iter().enumerate() {
            if !first.left_extractor().contains(solution, a) {
                continue;
            }
            for (b_idx, b) in entities_b.iter().enumerate() {
                if !first.right_extractor().contains(solution, b) {
                    continue;
                }
                if !self.first_keys_equal(a, b) {
                    continue;
                }
                if !(first_filter)(solution, a, b, a_idx, b_idx) {
                    continue;
                }
                let row = Concat::new(Leaf::new(a, a_idx), b, b_idx);
                let left_key = (self.operator.left_key())(&row);
                for (c_idx, c) in self.right_entities(solution).iter().enumerate() {
                    if !self.right_contains(solution, c) {
                        continue;
                    }
                    if !self.right_key_equals(&left_key, c) {
                        continue;
                    }
                    if (filter)(solution, a, b, c, a_idx, b_idx, c_idx) {
                        count += 1;
                    }
                }
            }
        }

        count
    }

    fn initialize(&mut self, solution: &S) -> Sc {
        self.reset();
        self.operator.refresh(solution);
        self.initialized = true;

        let first = self.operator.first();
        let entities_a = first.left_extractor().extract(solution);
        let entities_b = first.right_extractor().extract(solution);
        let mut total = Sc::zero();
        for (a_idx, b_idx, c_idx) in self.operator.output_rows() {
            let entities_c = self.right_entities(solution);
            let base = self.weight.score(
                solution, entities_a, entities_b, entities_c, a_idx, b_idx, c_idx,
            );
            let signed = self.signed(base);
            self.triple_scores.insert((a_idx, b_idx, c_idx), signed);
            total = total + signed;
        }
        total
    }

    fn on_insert(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc {
        let mut total = Sc::zero();
        for delta in self
            .operator
            .on_insert(solution, descriptor_index, entity_index)
        {
            if delta.kind != DeltaKind::Insert {
                continue;
            }
            let triple = (delta.a_idx, delta.b_idx, delta.c_idx);
            let first = self.operator.first();
            let entities_a = first.left_extractor().extract(solution);
            let entities_b = first.right_extractor().extract(solution);
            let base = self.weight.score(
                solution,
                entities_a,
                entities_b,
                self.right_entities(solution),
                triple.0,
                triple.1,
                triple.2,
            );
            let signed = self.signed(base);
            self.triple_scores.insert(triple, signed);
            total = total + signed;
        }
        total
    }

    fn on_retract(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc {
        let _ = solution;
        let mut total = Sc::zero();
        for delta in self.operator.on_retract(descriptor_index, entity_index) {
            if delta.kind != DeltaKind::Retract {
                continue;
            }
            let triple = (delta.a_idx, delta.b_idx, delta.c_idx);
            // Retract the PREVIOUSLY computed score: a changed key or value
            // must not recompute from new state here.
            if let Some(score) = self.triple_scores.remove(&triple) {
                total = total + (-score);
            }
        }
        total
    }

    fn reset(&mut self) {
        self.triple_scores.clear();
        self.operator.clear_outputs();
        self.initialized = false;
    }

    fn name(&self) -> &str {
        &self.constraint_ref.name
    }

    fn is_hard(&self) -> bool {
        self.is_hard
    }

    fn constraint_ref(&self) -> &ConstraintRef {
        &self.constraint_ref
    }

    fn get_matches<'a>(&'a self, solution: &S) -> Vec<DetailedConstraintMatch<'a, Sc>> {
        let first = self.operator.first();
        let entities_a = first.left_extractor().extract(solution);
        let entities_b = first.right_extractor().extract(solution);
        let entities_c = self.right_entities(solution);
        let cref = self.constraint_ref();
        let mut matches = Vec::new();

        // Initialized explanations walk retained chained provenance in
        // authored binding order; uninitialized explanations recompute
        // with identical orientation.
        if !self.initialized {
            return self.uninitialized_matches(solution);
        }
        for (a_idx, b_idx, c_idx) in self.operator.output_rows() {
            let Some(provenance) = self.operator.provenance_of(a_idx, b_idx, c_idx) else {
                continue;
            };
            let (Some(a), Some(b), Some(c)) = (
                entities_a.get(a_idx),
                entities_b.get(b_idx),
                entities_c.get(c_idx),
            ) else {
                continue;
            };
            let mut entities = Vec::with_capacity(provenance.bindings().len());
            for part in provenance.bindings() {
                let entity = match part.binding.0 {
                    0 => EntityRef::new(a),
                    1 => EntityRef::new(b),
                    _ => EntityRef::new(c),
                };
                entities.push(entity);
            }
            let justification = ConstraintJustification::new(entities);
            let base = self.weight.score(
                solution, entities_a, entities_b, entities_c, a_idx, b_idx, c_idx,
            );
            matches.push(DetailedConstraintMatch::new(
                cref,
                self.signed(base),
                justification,
            ));
        }

        matches
    }
}

impl<S, A, B, C, EA, EB, EC, K1, KA, KB, K2, LK, KC, F1, F2, W, Sc>
    TripleTerminal<S, A, B, C, EA, EB, EC, K1, KA, KB, K2, LK, KC, F1, F2, W, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Debug + Send + Sync + 'static,
    B: Clone + Debug + Send + Sync + 'static,
    C: Clone + Debug + Send + Sync + 'static,
    EA: CollectionExtract<S, Item = A> + Send + Sync,
    EB: CollectionExtract<S, Item = B> + Send + Sync,
    EC: CollectionExtract<S, Item = C> + Send + Sync,
    K1: Eq + Hash + Clone + Send + Sync,
    KA: Fn(&A) -> K1 + Send + Sync,
    KB: Fn(&B) -> K1 + Send + Sync,
    F1: Fn(&S, &A, &B, usize, usize) -> bool + Send + Sync,
    K2: Eq + Hash + Clone + Send + Sync,
    LK: for<'r> Fn(&Concat<Leaf<'r, A>, B>) -> K2 + Send + Sync,
    KC: Fn(&C) -> K2 + Send + Sync,
    F2: Fn(&S, &A, &B, &C, usize, usize, usize) -> bool + Send + Sync,
    W: RelationalWeight3<S, A, B, C, Sc>,
    Sc: Score,
{
    /* First-join key equality re-derived for stateless paths. */
    fn first_keys_equal(&self, a: &A, b: &B) -> bool {
        (self.operator.first().left_key())(a) == (self.operator.first().right_key())(b)
    }

    /* Right-side key equality against one derived left key. */
    fn right_key_equals(&self, left_key: &K2, c: &C) -> bool {
        *left_key == (self.operator.right_key())(c)
    }

    fn right_entities<'s>(&self, solution: &'s S) -> &'s [C] {
        self.operator.right_extractor().extract(solution)
    }

    fn right_contains(&self, solution: &S, c: &C) -> bool {
        self.operator.right_extractor().contains(solution, c)
    }

    /* Stateless match enumeration with triple binding orientation. */
    fn uninitialized_matches(&self, solution: &S) -> Vec<DetailedConstraintMatch<'_, Sc>>
    where
        EA: CollectionExtract<S, Item = A> + Send + Sync,
        EB: CollectionExtract<S, Item = B> + Send + Sync,
        F1: Fn(&S, &A, &B, usize, usize) -> bool + Send + Sync,
        LK: for<'r> Fn(&Concat<Leaf<'r, A>, B>) -> K2 + Send + Sync,
        F2: Fn(&S, &A, &B, &C, usize, usize, usize) -> bool + Send + Sync,
        W: RelationalWeight3<S, A, B, C, Sc>,
    {
        let first = self.operator.first();
        let entities_a = first.left_extractor().extract(solution);
        let entities_b = first.right_extractor().extract(solution);
        let entities_c = self.right_entities(solution);
        let filter = self.operator.filter();
        let first_filter = first.filter();
        let cref = self.constraint_ref();
        let mut matches = Vec::new();

        for (a_idx, a) in entities_a.iter().enumerate() {
            if !first.left_extractor().contains(solution, a) {
                continue;
            }
            for (b_idx, b) in entities_b.iter().enumerate() {
                if !first.right_extractor().contains(solution, b) {
                    continue;
                }
                if !self.first_keys_equal(a, b) {
                    continue;
                }
                if !(first_filter)(solution, a, b, a_idx, b_idx) {
                    continue;
                }
                let row = Concat::new(Leaf::new(a, a_idx), b, b_idx);
                let left_key = (self.operator.left_key())(&row);
                for (c_idx, c) in entities_c.iter().enumerate() {
                    if !self.right_contains(solution, c) {
                        continue;
                    }
                    if !self.right_key_equals(&left_key, c) {
                        continue;
                    }
                    if !(filter)(solution, a, b, c, a_idx, b_idx, c_idx) {
                        continue;
                    }
                    let justification = ConstraintJustification::new(vec![
                        EntityRef::new(a),
                        EntityRef::new(b),
                        EntityRef::new(c),
                    ]);
                    let base = self.weight.score(
                        solution, entities_a, entities_b, entities_c, a_idx, b_idx, c_idx,
                    );
                    matches.push(DetailedConstraintMatch::new(
                        cref,
                        self.signed(base),
                        justification,
                    ));
                }
            }
        }

        matches
    }
}

impl<S, A, B, C, EA, EB, EC, K1, KA, KB, K2, LK, KC, F1, F2, W, Sc> std::fmt::Debug
    for TripleTerminal<S, A, B, C, EA, EB, EC, K1, KA, KB, K2, LK, KC, F1, F2, W, Sc>
where
    Sc: Score,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TripleTerminal")
            .field("name", &self.constraint_ref.name)
            .field("match_count", &self.triple_scores.len())
            .finish()
    }
}
