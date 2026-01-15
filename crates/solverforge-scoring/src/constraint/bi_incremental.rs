//! Incremental bi-constraint for self-join evaluation.
//!
//! Zero-erasure: all closures are concrete generic types, fully monomorphized.
//! Uses key-based indexing for O(k) lookups instead of O(n) iteration.

use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use crate::api::analysis::DetailedConstraintMatch;
use crate::api::constraint_set::IncrementalConstraint;

/// Zero-erasure incremental bi-constraint for self-joins.
///
/// All function types are concrete generics - no trait objects, no Arc.
/// Uses key-based indexing: entities are grouped by join key for O(k) lookups.
pub struct IncrementalBiConstraint<S, A, K, E, KE, F, W, Sc>
where
    Sc: Score,
{
    constraint_ref: ConstraintRef,
    impact_type: ImpactType,
    extractor: E,
    key_extractor: KE,
    filter: F,
    weight: W,
    is_hard: bool,
    /// entity_index -> set of (low_idx, high_idx) pairs involving this entity
    entity_to_matches: HashMap<usize, HashSet<(usize, usize)>>,
    /// All matched pairs (low_idx, high_idx) where low_idx < high_idx
    matches: HashSet<(usize, usize)>,
    /// Key -> set of entity indices with that key (for O(k) lookup)
    key_to_indices: HashMap<K, HashSet<usize>>,
    /// entity_index -> key (for cleanup on retract)
    index_to_key: HashMap<usize, K>,
    _phantom: PhantomData<(S, A, Sc)>,
}

impl<S, A, K, E, KE, F, W, Sc> IncrementalBiConstraint<S, A, K, E, KE, F, W, Sc>
where
    S: 'static,
    A: Clone + 'static,
    K: Eq + Hash + Clone,
    E: Fn(&S) -> &[A],
    KE: Fn(&A) -> K,
    F: Fn(&S, &A, &A) -> bool,
    W: Fn(&A, &A) -> Sc,
    Sc: Score,
{
    pub fn new(
        constraint_ref: ConstraintRef,
        impact_type: ImpactType,
        extractor: E,
        key_extractor: KE,
        filter: F,
        weight: W,
        is_hard: bool,
    ) -> Self {
        Self {
            constraint_ref,
            impact_type,
            extractor,
            key_extractor,
            filter,
            weight,
            is_hard,
            entity_to_matches: HashMap::new(),
            matches: HashSet::new(),
            key_to_indices: HashMap::new(),
            index_to_key: HashMap::new(),
            _phantom: PhantomData,
        }
    }

    #[inline]
    fn compute_score(&self, a: &A, b: &A) -> Sc {
        let base = (self.weight)(a, b);
        match self.impact_type {
            ImpactType::Penalty => -base,
            ImpactType::Reward => base,
        }
    }

    /// Insert entity and find matches with other entities sharing the same key.
    fn insert_entity(&mut self, solution: &S, entities: &[A], index: usize) -> Sc {
        if index >= entities.len() {
            return Sc::zero();
        }

        let entity = &entities[index];
        let key = (self.key_extractor)(entity);

        // Track this entity's key
        self.index_to_key.insert(index, key.clone());

        // Add this entity to the key index FIRST
        self.key_to_indices
            .entry(key.clone())
            .or_default()
            .insert(index);

        // Split borrows to allow simultaneous read of key_to_indices and mutation of matches
        let key_to_indices = &self.key_to_indices;
        let matches = &mut self.matches;
        let entity_to_matches = &mut self.entity_to_matches;
        let filter = &self.filter;
        let weight = &self.weight;
        let impact_type = self.impact_type;

        // Find matches with other entities having the same key (zero allocation)
        let mut total = Sc::zero();
        if let Some(others) = key_to_indices.get(&key) {
            for &other_idx in others {
                if other_idx == index {
                    continue;
                }

                let other = &entities[other_idx];

                // Canonical ordering: (low, high) where low < high
                let (low_idx, high_idx, low_entity, high_entity) = if index < other_idx {
                    (index, other_idx, entity, other)
                } else {
                    (other_idx, index, other, entity)
                };

                if filter(solution, low_entity, high_entity) {
                    let pair = (low_idx, high_idx);
                    if matches.insert(pair) {
                        entity_to_matches.entry(low_idx).or_default().insert(pair);
                        entity_to_matches.entry(high_idx).or_default().insert(pair);
                        let base = weight(low_entity, high_entity);
                        let score = match impact_type {
                            ImpactType::Penalty => -base,
                            ImpactType::Reward => base,
                        };
                        total = total + score;
                    }
                }
            }
        }

        total
    }

    /// Retract entity and remove all its matches.
    fn retract_entity(&mut self, entities: &[A], index: usize) -> Sc {
        // Remove from key index
        if let Some(key) = self.index_to_key.remove(&index) {
            if let Some(indices) = self.key_to_indices.get_mut(&key) {
                indices.remove(&index);
                if indices.is_empty() {
                    self.key_to_indices.remove(&key);
                }
            }
        }

        // Remove all matches involving this entity
        let Some(pairs) = self.entity_to_matches.remove(&index) else {
            return Sc::zero();
        };

        let mut total = Sc::zero();
        for pair in pairs {
            self.matches.remove(&pair);

            // Remove from other entity's match set
            let other = if pair.0 == index { pair.1 } else { pair.0 };
            if let Some(other_set) = self.entity_to_matches.get_mut(&other) {
                other_set.remove(&pair);
                if other_set.is_empty() {
                    self.entity_to_matches.remove(&other);
                }
            }

            // Compute reverse delta
            let (low_idx, high_idx) = pair;
            if low_idx < entities.len() && high_idx < entities.len() {
                let score = self.compute_score(&entities[low_idx], &entities[high_idx]);
                total = total - score;
            }
        }

        total
    }
}

impl<S, A, K, E, KE, F, W, Sc> IncrementalConstraint<S, Sc>
    for IncrementalBiConstraint<S, A, K, E, KE, F, W, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Debug + Send + Sync + 'static,
    K: Eq + Hash + Clone + Send + Sync,
    E: Fn(&S) -> &[A] + Send + Sync,
    KE: Fn(&A) -> K + Send + Sync,
    F: Fn(&S, &A, &A) -> bool + Send + Sync,
    W: Fn(&A, &A) -> Sc + Send + Sync,
    Sc: Score,
{
    fn evaluate(&self, solution: &S) -> Sc {
        let entities = (self.extractor)(solution);
        let mut total = Sc::zero();

        // Build temporary key index for evaluation
        let mut temp_index: HashMap<K, Vec<usize>> = HashMap::new();
        for (i, entity) in entities.iter().enumerate() {
            let key = (self.key_extractor)(entity);
            temp_index.entry(key).or_default().push(i);
        }

        // Evaluate pairs within each key group
        for indices in temp_index.values() {
            for i in 0..indices.len() {
                for j in (i + 1)..indices.len() {
                    let low = indices[i];
                    let high = indices[j];
                    let a = &entities[low];
                    let b = &entities[high];
                    if (self.filter)(solution, a, b) {
                        total = total + self.compute_score(a, b);
                    }
                }
            }
        }

        total
    }

    fn match_count(&self, solution: &S) -> usize {
        let entities = (self.extractor)(solution);
        let mut count = 0;

        // Build temporary key index
        let mut temp_index: HashMap<K, Vec<usize>> = HashMap::new();
        for (i, entity) in entities.iter().enumerate() {
            let key = (self.key_extractor)(entity);
            temp_index.entry(key).or_default().push(i);
        }

        // Count matches within each key group
        for indices in temp_index.values() {
            for i in 0..indices.len() {
                for j in (i + 1)..indices.len() {
                    let low = indices[i];
                    let high = indices[j];
                    if (self.filter)(solution, &entities[low], &entities[high]) {
                        count += 1;
                    }
                }
            }
        }

        count
    }

    fn initialize(&mut self, solution: &S) -> Sc {
        self.reset();

        let entities = (self.extractor)(solution);
        let mut total = Sc::zero();
        for i in 0..entities.len() {
            total = total + self.insert_entity(solution, entities, i);
        }
        total
    }

    fn on_insert(&mut self, solution: &S, entity_index: usize) -> Sc {
        let entities = (self.extractor)(solution);
        self.insert_entity(solution, entities, entity_index)
    }

    fn on_retract(&mut self, solution: &S, entity_index: usize) -> Sc {
        let entities = (self.extractor)(solution);
        self.retract_entity(entities, entity_index)
    }

    fn reset(&mut self) {
        self.entity_to_matches.clear();
        self.matches.clear();
        self.key_to_indices.clear();
        self.index_to_key.clear();
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
        impl_get_matches_nary!(bi: self, solution)
    }
}

impl<S, A, K, E, KE, F, W, Sc: Score> std::fmt::Debug
    for IncrementalBiConstraint<S, A, K, E, KE, F, W, Sc>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IncrementalBiConstraint")
            .field("name", &self.constraint_ref.name)
            .field("impact_type", &self.impact_type)
            .field("match_count", &self.matches.len())
            .finish()
    }
}
