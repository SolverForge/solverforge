//! Zero-erasure incremental penta-constraint for self-join quintuple evaluation.
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

/// Ordered quintuple indices (i, j, k, l, m) where i < j < k < l < m.
type Quintuple = (usize, usize, usize, usize, usize);

/// Zero-erasure incremental penta-constraint for self-joins.
///
/// All function types are concrete generics - no Arc, no dyn, fully monomorphized.
/// Uses key-based indexing: entities are grouped by join key for O(k) lookups
/// where k is the number of entities sharing the same key.
///
/// Quintuples are ordered as (i, j, k, l, m) where i < j < k < l < m to avoid duplicates.
///
/// # Type Parameters
///
/// - `S` - Solution type
/// - `A` - Entity type
/// - `K` - Key type for grouping (entities with same key form quintuples)
/// - `E` - Extractor function `Fn(&S) -> &[A]`
/// - `KE` - Key extractor function `Fn(&A) -> K`
/// - `F` - Filter function `Fn(&A, &A, &A, &A, &A) -> bool`
/// - `W` - Weight function `Fn(&A, &A, &A, &A, &A) -> Sc`
/// - `Sc` - Score type
///
/// # Example
///
/// ```
/// use solverforge_scoring::constraint::penta_incremental::IncrementalPentaConstraint;
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
/// // Penalize when five tasks are on the same team
/// let constraint = IncrementalPentaConstraint::new(
///     ConstraintRef::new("", "Team clustering"),
///     ImpactType::Penalty,
///     |s: &Solution| s.tasks.as_slice(),
///     |t: &Task| t.team,  // Group by team
///     |_a: &Task, _b: &Task, _c: &Task, _d: &Task, _e: &Task| true,
///     |_a: &Task, _b: &Task, _c: &Task, _d: &Task, _e: &Task| SimpleScore::of(1),
///     false,
/// );
///
/// let solution = Solution {
///     tasks: vec![
///         Task { team: 1 },
///         Task { team: 1 },
///         Task { team: 1 },
///         Task { team: 1 },
///         Task { team: 1 },
///         Task { team: 2 },
///     ],
/// };
///
/// // One quintuple: (0, 1, 2, 3, 4) all on team 1 = -1 penalty
/// assert_eq!(constraint.evaluate(&solution), SimpleScore::of(-1));
/// ```
pub struct IncrementalPentaConstraint<S, A, K, E, KE, F, W, Sc>
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
    /// entity_index -> set of quintuples involving this entity
    entity_to_matches: HashMap<usize, HashSet<Quintuple>>,
    /// All matched quintuples (i, j, k, l, m) where i < j < k < l < m
    matches: HashSet<Quintuple>,
    /// Key -> set of entity indices with that key (for O(k) lookup)
    key_to_indices: HashMap<K, HashSet<usize>>,
    /// entity_index -> key (for cleanup on retract)
    index_to_key: HashMap<usize, K>,
    _phantom: PhantomData<(S, A, Sc)>,
}

impl<S, A, K, E, KE, F, W, Sc> IncrementalPentaConstraint<S, A, K, E, KE, F, W, Sc>
where
    S: 'static,
    A: Clone + 'static,
    K: Eq + Hash + Clone,
    E: Fn(&S) -> &[A],
    KE: Fn(&A) -> K,
    F: Fn(&A, &A, &A, &A, &A) -> bool,
    W: Fn(&A, &A, &A, &A, &A) -> Sc,
    Sc: Score,
{
    /// Creates a new zero-erasure incremental penta-constraint.
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
    fn compute_score(&self, a: &A, b: &A, c: &A, d: &A, e: &A) -> Sc {
        let base = (self.weight)(a, b, c, d, e);
        match self.impact_type {
            ImpactType::Penalty => -base,
            ImpactType::Reward => base,
        }
    }

    /// Insert entity and find matches with other entity quads sharing the same key.
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

        // Find matches with all quads of other entities having the same key (zero allocation)
        let mut total = Sc::zero();
        if let Some(others) = key_to_indices.get(&key) {
            // Iterate over all quads (i, j, k, l) where i < j < k < l, excluding current index
            for &i in others {
                if i == index {
                    continue;
                }
                for &j in others {
                    if j <= i || j == index {
                        continue;
                    }
                    for &k in others {
                        if k <= j || k == index {
                            continue;
                        }
                        for &l in others {
                            if l <= k || l == index {
                                continue;
                            }

                            // Determine canonical ordering for this quintuple
                            let mut indices = [index, i, j, k, l];
                            indices.sort();
                            let [a_idx, b_idx, c_idx, d_idx, e_idx] = indices;

                            let penta = (a_idx, b_idx, c_idx, d_idx, e_idx);

                            // Skip if already matched
                            if matches.contains(&penta) {
                                continue;
                            }

                            let a = &entities[a_idx];
                            let b = &entities[b_idx];
                            let c = &entities[c_idx];
                            let d = &entities[d_idx];
                            let e = &entities[e_idx];

                            if filter(a, b, c, d, e) && matches.insert(penta) {
                                entity_to_matches.entry(a_idx).or_default().insert(penta);
                                entity_to_matches.entry(b_idx).or_default().insert(penta);
                                entity_to_matches.entry(c_idx).or_default().insert(penta);
                                entity_to_matches.entry(d_idx).or_default().insert(penta);
                                entity_to_matches.entry(e_idx).or_default().insert(penta);
                                let base = weight(a, b, c, d, e);
                                let score = match impact_type {
                                    ImpactType::Penalty => -base,
                                    ImpactType::Reward => base,
                                };
                                total = total + score;
                            }
                        }
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
        let Some(pentas) = self.entity_to_matches.remove(&index) else {
            return Sc::zero();
        };

        let mut total = Sc::zero();
        for penta in pentas {
            self.matches.remove(&penta);

            // Remove from other entities' match sets
            let (i, j, k, l, m) = penta;
            for &other in &[i, j, k, l, m] {
                if other != index {
                    if let Some(other_set) = self.entity_to_matches.get_mut(&other) {
                        other_set.remove(&penta);
                        if other_set.is_empty() {
                            self.entity_to_matches.remove(&other);
                        }
                    }
                }
            }

            // Compute reverse delta
            if i < entities.len()
                && j < entities.len()
                && k < entities.len()
                && l < entities.len()
                && m < entities.len()
            {
                let score = self.compute_score(
                    &entities[i],
                    &entities[j],
                    &entities[k],
                    &entities[l],
                    &entities[m],
                );
                total = total - score;
            }
        }

        total
    }
}

impl<S, A, K, E, KE, F, W, Sc> IncrementalConstraint<S, Sc>
    for IncrementalPentaConstraint<S, A, K, E, KE, F, W, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Debug + Send + Sync + 'static,
    K: Eq + Hash + Clone + Send + Sync,
    E: Fn(&S) -> &[A] + Send + Sync,
    KE: Fn(&A) -> K + Send + Sync,
    F: Fn(&A, &A, &A, &A, &A) -> bool + Send + Sync,
    W: Fn(&A, &A, &A, &A, &A) -> Sc + Send + Sync,
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

        // Evaluate quintuples within each key group
        for indices in temp_index.values() {
            for pos_i in 0..indices.len() {
                for pos_j in (pos_i + 1)..indices.len() {
                    for pos_k in (pos_j + 1)..indices.len() {
                        for pos_l in (pos_k + 1)..indices.len() {
                            for pos_m in (pos_l + 1)..indices.len() {
                                let i = indices[pos_i];
                                let j = indices[pos_j];
                                let k = indices[pos_k];
                                let l = indices[pos_l];
                                let m = indices[pos_m];
                                let a = &entities[i];
                                let b = &entities[j];
                                let c = &entities[k];
                                let d = &entities[l];
                                let e = &entities[m];
                                if (self.filter)(a, b, c, d, e) {
                                    total = total + self.compute_score(a, b, c, d, e);
                                }
                            }
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
                        for pos_l in (pos_k + 1)..indices.len() {
                            for pos_m in (pos_l + 1)..indices.len() {
                                let i = indices[pos_i];
                                let j = indices[pos_j];
                                let k = indices[pos_k];
                                let l = indices[pos_l];
                                let m = indices[pos_m];
                                if (self.filter)(
                                    &entities[i],
                                    &entities[j],
                                    &entities[k],
                                    &entities[l],
                                    &entities[m],
                                ) {
                                    count += 1;
                                }
                            }
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
        impl_get_matches_nary!(penta: self, solution)
    }
}

impl<S, A, K, E, KE, F, W, Sc: Score> std::fmt::Debug
    for IncrementalPentaConstraint<S, A, K, E, KE, F, W, Sc>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IncrementalPentaConstraint")
            .field("name", &self.constraint_ref.name)
            .field("impact_type", &self.impact_type)
            .field("match_count", &self.matches.len())
            .finish()
    }
}
