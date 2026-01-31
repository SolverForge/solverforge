//! Incremental cross-bi-constraint for cross-entity join evaluation.
//!
//! Zero-erasure: all closures are concrete generic types, fully monomorphized.

use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use crate::api::analysis::{ConstraintJustification, DetailedConstraintMatch, EntityRef};
use crate::api::constraint_set::IncrementalConstraint;

/// Zero-erasure incremental cross-bi-constraint.
///
/// All function types are concrete generics - no trait objects, no Arc.
pub struct IncrementalCrossBiConstraint<S, A, B, K, EA, EB, KA, KB, F, W, Sc>
where
    Sc: Score,
{
    constraint_ref: ConstraintRef,
    impact_type: ImpactType,
    extractor_a: EA,
    extractor_b: EB,
    key_a: KA,
    key_b: KB,
    filter: F,
    weight: W,
    is_hard: bool,
    matches: HashMap<(usize, usize), Sc>,
    a_to_matches: HashMap<usize, HashSet<(usize, usize)>>,
    b_by_key: HashMap<K, Vec<usize>>,
    _phantom: PhantomData<(S, A, B)>,
}

impl<S, A, B, K, EA, EB, KA, KB, F, W, Sc>
    IncrementalCrossBiConstraint<S, A, B, K, EA, EB, KA, KB, F, W, Sc>
where
    S: 'static,
    A: Clone + 'static,
    B: Clone + 'static,
    K: Eq + Hash + Clone,
    EA: Fn(&S) -> &[A],
    EB: Fn(&S) -> &[B],
    KA: Fn(&A) -> K,
    KB: Fn(&B) -> K,
    F: Fn(&S, &A, &B) -> bool,
    W: Fn(&A, &B) -> Sc,
    Sc: Score,
{
    /// Creates a new cross-bi-constraint.
    ///
    /// # Arguments
    /// All 9 arguments are semantically distinct (2 extractors, 2 key functions,
    /// 1 filter, 1 weight, 1 is_hard) and cannot be meaningfully grouped without losing
    /// higher-ranked lifetime inference for the closures.
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
    ) -> Self {
        Self {
            constraint_ref,
            impact_type,
            extractor_a,
            extractor_b,
            key_a,
            key_b,
            filter,
            weight,
            is_hard,
            matches: HashMap::new(),
            a_to_matches: HashMap::new(),
            b_by_key: HashMap::new(),
            _phantom: PhantomData,
        }
    }

    #[inline]
    fn compute_score(&self, a: &A, b: &B) -> Sc {
        let base = (self.weight)(a, b);
        match self.impact_type {
            ImpactType::Penalty => -base,
            ImpactType::Reward => base,
        }
    }

    fn build_b_index(&mut self, entities_b: &[B]) {
        self.b_by_key.clear();
        for (b_idx, b) in entities_b.iter().enumerate() {
            let key = (self.key_b)(b);
            self.b_by_key.entry(key).or_default().push(b_idx);
        }
    }

    #[inline]
    fn matching_b_indices(&self, a: &A) -> &[usize] {
        let key = (self.key_a)(a);
        self.b_by_key.get(&key).map(|v| v.as_slice()).unwrap_or(&[])
    }

    fn insert_a(&mut self, solution: &S, entities_a: &[A], entities_b: &[B], a_idx: usize) -> Sc {
        if a_idx >= entities_a.len() {
            return Sc::zero();
        }

        let a = &entities_a[a_idx];
        let key = (self.key_a)(a);

        // Split borrows to allow simultaneous read of b_by_key and mutation of matches
        let b_by_key = &self.b_by_key;
        let matches = &mut self.matches;
        let a_to_matches = &mut self.a_to_matches;
        let filter = &self.filter;
        let weight = &self.weight;
        let impact_type = self.impact_type;

        // Get slice reference instead of cloning (zero allocation)
        let b_indices = b_by_key.get(&key).map(|v| v.as_slice()).unwrap_or(&[]);

        let mut total = Sc::zero();
        for &b_idx in b_indices {
            let b = &entities_b[b_idx];
            if filter(solution, a, b) {
                let pair = (a_idx, b_idx);
                let base = weight(a, b);
                let score = match impact_type {
                    ImpactType::Penalty => -base,
                    ImpactType::Reward => base,
                };
                matches.insert(pair, score);
                a_to_matches.entry(a_idx).or_default().insert(pair);
                total = total + score;
            }
        }

        total
    }

    fn retract_a(&mut self, entities_a: &[A], entities_b: &[B], a_idx: usize) -> Sc {
        let Some(pairs) = self.a_to_matches.remove(&a_idx) else {
            return Sc::zero();
        };

        let mut total = Sc::zero();
        for pair in pairs {
            if let Some(score) = self.matches.remove(&pair) {
                let (a_i, b_i) = pair;
                if a_i < entities_a.len() && b_i < entities_b.len() {
                    total = total - score;
                }
            }
        }

        total
    }
}

impl<S, A, B, K, EA, EB, KA, KB, F, W, Sc> IncrementalConstraint<S, Sc>
    for IncrementalCrossBiConstraint<S, A, B, K, EA, EB, KA, KB, F, W, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Debug + Send + Sync + 'static,
    B: Clone + Debug + Send + Sync + 'static,
    K: Eq + Hash + Clone + Send + Sync,
    EA: Fn(&S) -> &[A] + Send + Sync,
    EB: Fn(&S) -> &[B] + Send + Sync,
    KA: Fn(&A) -> K + Send + Sync,
    KB: Fn(&B) -> K + Send + Sync,
    F: Fn(&S, &A, &B) -> bool + Send + Sync,
    W: Fn(&A, &B) -> Sc + Send + Sync,
    Sc: Score,
{
    fn evaluate(&self, solution: &S) -> Sc {
        let entities_a = (self.extractor_a)(solution);
        let entities_b = (self.extractor_b)(solution);
        let mut total = Sc::zero();

        for a in entities_a {
            for &b_idx in self.matching_b_indices(a) {
                let b = &entities_b[b_idx];
                if (self.filter)(solution, a, b) {
                    total = total + self.compute_score(a, b);
                }
            }
        }

        total
    }

    fn match_count(&self, solution: &S) -> usize {
        let entities_a = (self.extractor_a)(solution);
        let entities_b = (self.extractor_b)(solution);
        let mut count = 0;

        for a in entities_a {
            for &b_idx in self.matching_b_indices(a) {
                let b = &entities_b[b_idx];
                if (self.filter)(solution, a, b) {
                    count += 1;
                }
            }
        }

        count
    }

    fn initialize(&mut self, solution: &S) -> Sc {
        self.reset();

        let entities_a = (self.extractor_a)(solution);
        let entities_b = (self.extractor_b)(solution);

        self.build_b_index(entities_b);

        let mut total = Sc::zero();
        for a_idx in 0..entities_a.len() {
            total = total + self.insert_a(solution, entities_a, entities_b, a_idx);
        }

        total
    }

    fn on_insert(&mut self, solution: &S, entity_index: usize, _descriptor_index: usize) -> Sc {
        let entities_a = (self.extractor_a)(solution);
        let entities_b = (self.extractor_b)(solution);
        self.insert_a(solution, entities_a, entities_b, entity_index)
    }

    fn on_retract(&mut self, solution: &S, entity_index: usize, _descriptor_index: usize) -> Sc {
        let entities_a = (self.extractor_a)(solution);
        let entities_b = (self.extractor_b)(solution);
        self.retract_a(entities_a, entities_b, entity_index)
    }

    fn reset(&mut self) {
        self.matches.clear();
        self.a_to_matches.clear();
        self.b_by_key.clear();
    }

    fn name(&self) -> &str {
        &self.constraint_ref.name
    }

    fn is_hard(&self) -> bool {
        self.is_hard
    }

    fn constraint_ref(&self) -> ConstraintRef {
        self.constraint_ref.clone()
    }

    fn get_matches(&self, solution: &S) -> Vec<DetailedConstraintMatch<Sc>> {
        let entities_a = (self.extractor_a)(solution);
        let entities_b = (self.extractor_b)(solution);
        let cref = self.constraint_ref.clone();

        let mut matches = Vec::new();

        for a in entities_a {
            for &b_idx in self.matching_b_indices(a) {
                let b = &entities_b[b_idx];
                if (self.filter)(solution, a, b) {
                    let entity_a = EntityRef::new(a);
                    let entity_b = EntityRef::new(b);
                    let justification = ConstraintJustification::new(vec![entity_a, entity_b]);
                    let score = self.compute_score(a, b);
                    matches.push(DetailedConstraintMatch::new(
                        cref.clone(),
                        score,
                        justification,
                    ));
                }
            }
        }

        matches
    }
}

impl<S, A, B, K, EA, EB, KA, KB, F, W, Sc: Score> std::fmt::Debug
    for IncrementalCrossBiConstraint<S, A, B, K, EA, EB, KA, KB, F, W, Sc>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IncrementalCrossBiConstraint")
            .field("name", &self.constraint_ref.name)
            .field("impact_type", &self.impact_type)
            .field("match_count", &self.matches.len())
            .finish()
    }
}
