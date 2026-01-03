//! Zero-erasure incremental tri-constraint for self-join triple evaluation.
//!
//! All function types are concrete generics - no trait objects, no Arc.
//! Uses key-based indexing: entities are grouped by join key for O(k) lookups.

use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use crate::api::analysis::DetailedConstraintMatch;
use crate::api::constraint_set::IncrementalConstraint;

/// Zero-erasure incremental tri-constraint for self-joins.
///
/// All function types are concrete generics - no Arc, no dyn, fully monomorphized.
/// Uses key-based indexing: entities are grouped by join key for O(k) lookups
/// where k is the number of entities sharing the same key.
///
/// Triples are ordered as (i, j, k) where i < j < k to avoid duplicates.
///
/// # Type Parameters
///
/// - `S` - Solution type
/// - `A` - Entity type
/// - `K` - Key type for grouping (entities with same key form triples)
/// - `E` - Extractor function `Fn(&S) -> &[A]`
/// - `KE` - Key extractor function `Fn(&A) -> K`
/// - `F` - Filter function `Fn(&A, &A, &A) -> bool`
/// - `W` - Weight function `Fn(&A, &A, &A) -> Sc`
/// - `Sc` - Score type
///
/// # Example
///
/// ```
/// use solverforge_scoring::constraint::tri_incremental::IncrementalTriConstraint;
/// use solverforge_scoring::api::constraint_set::IncrementalConstraint;
/// use solverforge_core::{ConstraintRef, ImpactType};
/// use solverforge_core::score::SimpleScore;
///
/// #[derive(Clone, Debug, Hash, PartialEq, Eq)]
/// struct Task { team: u32 }
///
/// #[derive(Clone)]
/// struct Solution { tasks: Vec<Task> }
///
/// // Penalize when three tasks are on the same team
/// let constraint = IncrementalTriConstraint::new(
///     ConstraintRef::new("", "Team clustering"),
///     ImpactType::Penalty,
///     |s: &Solution| s.tasks.as_slice(),
///     |t: &Task| t.team,  // Group by team
///     |_a: &Task, _b: &Task, _c: &Task| true,  // All triples in same group match
///     |_a: &Task, _b: &Task, _c: &Task| SimpleScore::of(1),
///     false,
/// );
///
/// let solution = Solution {
///     tasks: vec![
///         Task { team: 1 },
///         Task { team: 1 },
///         Task { team: 1 },
///         Task { team: 2 },
///     ],
/// };
///
/// // One triple: (0, 1, 2) all on team 1 = -1 penalty
/// assert_eq!(constraint.evaluate(&solution), SimpleScore::of(-1));
/// ```
pub struct IncrementalTriConstraint<S, A, K, E, KE, F, W, Sc>
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
    /// entity_index -> set of (i, j, k) triples involving this entity
    entity_to_matches: HashMap<usize, HashSet<(usize, usize, usize)>>,
    /// All matched triples (i, j, k) where i < j < k
    matches: HashSet<(usize, usize, usize)>,
    /// Key -> set of entity indices with that key (for O(k) lookup)
    key_to_indices: HashMap<K, HashSet<usize>>,
    /// entity_index -> key (for cleanup on retract)
    index_to_key: HashMap<usize, K>,
    _phantom: PhantomData<(S, A, Sc)>,
}

impl<S, A, K, E, KE, F, W, Sc> IncrementalTriConstraint<S, A, K, E, KE, F, W, Sc>
where
    S: 'static,
    A: Clone + 'static,
    K: Eq + Hash + Clone,
    E: Fn(&S) -> &[A],
    KE: Fn(&A) -> K,
    F: Fn(&A, &A, &A) -> bool,
    W: Fn(&A, &A, &A) -> Sc,
    Sc: Score,
{
    /// Creates a new zero-erasure incremental tri-constraint.
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
    fn compute_score(&self, a: &A, b: &A, c: &A) -> Sc {
        let base = (self.weight)(a, b, c);
        match self.impact_type {
            ImpactType::Penalty => -base,
            ImpactType::Reward => base,
        }
    }

    /// Insert entity and find matches with other entity pairs sharing the same key.
    fn insert_entity(&mut self, entities: &[A], index: usize) -> Sc {
        if index >= entities.len() {
            return Sc::zero();
        }

        let entity = &entities[index];
        let key = (self.key_extractor)(entity);

        // Track this entity's key
        self.index_to_key.insert(index, key.clone());

        // Add this entity to the key index FIRST
        self.key_to_indices.entry(key.clone()).or_default().insert(index);

        // Split borrows to allow simultaneous read of key_to_indices and mutation of matches
        let key_to_indices = &self.key_to_indices;
        let matches = &mut self.matches;
        let entity_to_matches = &mut self.entity_to_matches;
        let filter = &self.filter;
        let weight = &self.weight;
        let impact_type = self.impact_type;

        // Find matches with all pairs of other entities having the same key (zero allocation)
        let mut total = Sc::zero();
        if let Some(others) = key_to_indices.get(&key) {
            // Iterate over all pairs (i, j) where i < j, excluding current index
            for &i in others {
                if i == index {
                    continue;
                }
                for &j in others {
                    // Ensure i < j and j is not current index
                    if j <= i || j == index {
                        continue;
                    }

                    // Determine canonical ordering for this triple
                    let mut indices = [index, i, j];
                    indices.sort();
                    let [a_idx, b_idx, c_idx] = indices;

                    let triple = (a_idx, b_idx, c_idx);

                    // Skip if already matched
                    if matches.contains(&triple) {
                        continue;
                    }

                    let a = &entities[a_idx];
                    let b = &entities[b_idx];
                    let c = &entities[c_idx];

                    if filter(a, b, c) && matches.insert(triple) {
                        entity_to_matches.entry(a_idx).or_default().insert(triple);
                        entity_to_matches.entry(b_idx).or_default().insert(triple);
                        entity_to_matches.entry(c_idx).or_default().insert(triple);
                        let base = weight(a, b, c);
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
        let Some(triples) = self.entity_to_matches.remove(&index) else {
            return Sc::zero();
        };

        let mut total = Sc::zero();
        for triple in triples {
            self.matches.remove(&triple);

            // Remove from other entities' match sets
            let (i, j, k) = triple;
            for &other in &[i, j, k] {
                if other != index {
                    if let Some(other_set) = self.entity_to_matches.get_mut(&other) {
                        other_set.remove(&triple);
                        if other_set.is_empty() {
                            self.entity_to_matches.remove(&other);
                        }
                    }
                }
            }

            // Compute reverse delta
            if i < entities.len() && j < entities.len() && k < entities.len() {
                let score = self.compute_score(&entities[i], &entities[j], &entities[k]);
                total = total - score;
            }
        }

        total
    }
}

impl<S, A, K, E, KE, F, W, Sc> IncrementalConstraint<S, Sc>
    for IncrementalTriConstraint<S, A, K, E, KE, F, W, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Debug + Send + Sync + 'static,
    K: Eq + Hash + Clone + Send + Sync,
    E: Fn(&S) -> &[A] + Send + Sync,
    KE: Fn(&A) -> K + Send + Sync,
    F: Fn(&A, &A, &A) -> bool + Send + Sync,
    W: Fn(&A, &A, &A) -> Sc + Send + Sync,
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

        // Evaluate triples within each key group
        for indices in temp_index.values() {
            for pos_i in 0..indices.len() {
                for pos_j in (pos_i + 1)..indices.len() {
                    for pos_k in (pos_j + 1)..indices.len() {
                        let i = indices[pos_i];
                        let j = indices[pos_j];
                        let k = indices[pos_k];
                        let a = &entities[i];
                        let b = &entities[j];
                        let c = &entities[k];
                        if (self.filter)(a, b, c) {
                            total = total + self.compute_score(a, b, c);
                        }
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
            for pos_i in 0..indices.len() {
                for pos_j in (pos_i + 1)..indices.len() {
                    for pos_k in (pos_j + 1)..indices.len() {
                        let i = indices[pos_i];
                        let j = indices[pos_j];
                        let k = indices[pos_k];
                        if (self.filter)(&entities[i], &entities[j], &entities[k]) {
                            count += 1;
                        }
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
            total = total + self.insert_entity(entities, i);
        }
        total
    }

    fn on_insert(&mut self, solution: &S, entity_index: usize) -> Sc {
        let entities = (self.extractor)(solution);
        self.insert_entity(entities, entity_index)
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
        impl_get_matches_nary!(tri: self, solution)
    }
}

impl<S, A, K, E, KE, F, W, Sc: Score> std::fmt::Debug
    for IncrementalTriConstraint<S, A, K, E, KE, F, W, Sc>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IncrementalTriConstraint")
            .field("name", &self.constraint_ref.name)
            .field("impact_type", &self.impact_type)
            .field("match_count", &self.matches.len())
            .finish()
    }
}
