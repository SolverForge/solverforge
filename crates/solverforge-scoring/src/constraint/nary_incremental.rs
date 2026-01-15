//! Macro-generated N-ary incremental constraints for self-join evaluation.
//!
//! This module provides the `impl_incremental_nary_constraint!` macro that generates
//! fully monomorphized incremental constraint implementations for bi/tri/quad/penta arities.
//!
//! Zero-erasure: all closures are concrete generic types, no trait objects, no Arc.

/// Generates an incremental N-ary constraint struct and implementations.
///
/// This macro produces:
/// - The constraint struct with all fields
/// - Constructor `new()`
/// - Private helper methods `compute_score()`, `insert_entity()`, `retract_entity()`
/// - Full `IncrementalConstraint<S, Sc>` trait implementation
/// - `Debug` implementation
///
/// # Usage
///
/// ```text
/// impl_incremental_nary_constraint!(bi, IncrementalBiConstraint, (usize, usize), 2, a b);
/// impl_incremental_nary_constraint!(tri, IncrementalTriConstraint, (usize, usize, usize), 3, a b c);
/// impl_incremental_nary_constraint!(quad, IncrementalQuadConstraint, (usize, usize, usize, usize), 4, a b c d);
/// impl_incremental_nary_constraint!(penta, IncrementalPentaConstraint, (usize, usize, usize, usize, usize), 5, a b c d e);
/// ```
#[macro_export]
macro_rules! impl_incremental_nary_constraint {
    // ==================== BI (2-arity) ====================
    (bi, $struct_name:ident) => {
        /// Zero-erasure incremental bi-constraint for self-joins.
        ///
        /// All function types are concrete generics - no trait objects, no Arc.
        /// Uses key-based indexing: entities are grouped by join key for O(k) lookups.
        pub struct $struct_name<S, A, K, E, KE, F, W, Sc>
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
            entity_to_matches: HashMap<usize, HashSet<(usize, usize)>>,
            matches: HashSet<(usize, usize)>,
            key_to_indices: HashMap<K, HashSet<usize>>,
            index_to_key: HashMap<usize, K>,
            _phantom: PhantomData<(S, A, Sc)>,
        }

        impl<S, A, K, E, KE, F, W, Sc> $struct_name<S, A, K, E, KE, F, W, Sc>
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

            fn insert_entity(&mut self, solution: &S, entities: &[A], index: usize) -> Sc {
                if index >= entities.len() {
                    return Sc::zero();
                }

                let entity = &entities[index];
                let key = (self.key_extractor)(entity);

                self.index_to_key.insert(index, key.clone());
                self.key_to_indices
                    .entry(key.clone())
                    .or_default()
                    .insert(index);

                let key_to_indices = &self.key_to_indices;
                let matches = &mut self.matches;
                let entity_to_matches = &mut self.entity_to_matches;
                let filter = &self.filter;
                let weight = &self.weight;
                let impact_type = self.impact_type;

                let mut total = Sc::zero();
                if let Some(others) = key_to_indices.get(&key) {
                    for &other_idx in others {
                        if other_idx == index {
                            continue;
                        }

                        let other = &entities[other_idx];
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

            fn retract_entity(&mut self, entities: &[A], index: usize) -> Sc {
                if let Some(key) = self.index_to_key.remove(&index) {
                    if let Some(indices) = self.key_to_indices.get_mut(&key) {
                        indices.remove(&index);
                        if indices.is_empty() {
                            self.key_to_indices.remove(&key);
                        }
                    }
                }

                let Some(pairs) = self.entity_to_matches.remove(&index) else {
                    return Sc::zero();
                };

                let mut total = Sc::zero();
                for pair in pairs {
                    self.matches.remove(&pair);

                    let other = if pair.0 == index { pair.1 } else { pair.0 };
                    if let Some(other_set) = self.entity_to_matches.get_mut(&other) {
                        other_set.remove(&pair);
                        if other_set.is_empty() {
                            self.entity_to_matches.remove(&other);
                        }
                    }

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
            for $struct_name<S, A, K, E, KE, F, W, Sc>
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

                let mut temp_index: HashMap<K, Vec<usize>> = HashMap::new();
                for (i, entity) in entities.iter().enumerate() {
                    let key = (self.key_extractor)(entity);
                    temp_index.entry(key).or_default().push(i);
                }

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

                let mut temp_index: HashMap<K, Vec<usize>> = HashMap::new();
                for (i, entity) in entities.iter().enumerate() {
                    let key = (self.key_extractor)(entity);
                    temp_index.entry(key).or_default().push(i);
                }

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
                $crate::impl_get_matches_nary!(bi: self, solution)
            }
        }

        impl<S, A, K, E, KE, F, W, Sc: Score> std::fmt::Debug
            for $struct_name<S, A, K, E, KE, F, W, Sc>
        {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_struct(stringify!($struct_name))
                    .field("name", &self.constraint_ref.name)
                    .field("impact_type", &self.impact_type)
                    .field("match_count", &self.matches.len())
                    .finish()
            }
        }
    };

    // ==================== TRI (3-arity) ====================
    (tri, $struct_name:ident) => {
        /// Zero-erasure incremental tri-constraint for self-joins.
        ///
        /// All function types are concrete generics - no Arc, no dyn, fully monomorphized.
        /// Uses key-based indexing: entities are grouped by join key for O(k) lookups.
        /// Triples are ordered as (i, j, k) where i < j < k to avoid duplicates.
        pub struct $struct_name<S, A, K, E, KE, F, W, Sc>
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
            entity_to_matches: HashMap<usize, HashSet<(usize, usize, usize)>>,
            matches: HashSet<(usize, usize, usize)>,
            key_to_indices: HashMap<K, HashSet<usize>>,
            index_to_key: HashMap<usize, K>,
            _phantom: PhantomData<(S, A, Sc)>,
        }

        impl<S, A, K, E, KE, F, W, Sc> $struct_name<S, A, K, E, KE, F, W, Sc>
        where
            S: 'static,
            A: Clone + 'static,
            K: Eq + Hash + Clone,
            E: Fn(&S) -> &[A],
            KE: Fn(&A) -> K,
            F: Fn(&S, &A, &A, &A) -> bool,
            W: Fn(&A, &A, &A) -> Sc,
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
            fn compute_score(&self, a: &A, b: &A, c: &A) -> Sc {
                let base = (self.weight)(a, b, c);
                match self.impact_type {
                    ImpactType::Penalty => -base,
                    ImpactType::Reward => base,
                }
            }

            fn insert_entity(&mut self, solution: &S, entities: &[A], index: usize) -> Sc {
                if index >= entities.len() {
                    return Sc::zero();
                }

                let entity = &entities[index];
                let key = (self.key_extractor)(entity);

                self.index_to_key.insert(index, key.clone());
                self.key_to_indices
                    .entry(key.clone())
                    .or_default()
                    .insert(index);

                let key_to_indices = &self.key_to_indices;
                let matches = &mut self.matches;
                let entity_to_matches = &mut self.entity_to_matches;
                let filter = &self.filter;
                let weight = &self.weight;
                let impact_type = self.impact_type;

                let mut total = Sc::zero();
                if let Some(others) = key_to_indices.get(&key) {
                    for &i in others {
                        if i == index {
                            continue;
                        }
                        for &j in others {
                            if j <= i || j == index {
                                continue;
                            }

                            let mut arr = [index, i, j];
                            arr.sort();
                            let [a_idx, b_idx, c_idx] = arr;
                            let triple = (a_idx, b_idx, c_idx);

                            if matches.contains(&triple) {
                                continue;
                            }

                            let a = &entities[a_idx];
                            let b = &entities[b_idx];
                            let c = &entities[c_idx];

                            if filter(solution, a, b, c) && matches.insert(triple) {
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

            fn retract_entity(&mut self, entities: &[A], index: usize) -> Sc {
                if let Some(key) = self.index_to_key.remove(&index) {
                    if let Some(indices) = self.key_to_indices.get_mut(&key) {
                        indices.remove(&index);
                        if indices.is_empty() {
                            self.key_to_indices.remove(&key);
                        }
                    }
                }

                let Some(triples) = self.entity_to_matches.remove(&index) else {
                    return Sc::zero();
                };

                let mut total = Sc::zero();
                for triple in triples {
                    self.matches.remove(&triple);

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

                    if i < entities.len() && j < entities.len() && k < entities.len() {
                        let score =
                            self.compute_score(&entities[i], &entities[j], &entities[k]);
                        total = total - score;
                    }
                }

                total
            }
        }

        impl<S, A, K, E, KE, F, W, Sc> IncrementalConstraint<S, Sc>
            for $struct_name<S, A, K, E, KE, F, W, Sc>
        where
            S: Send + Sync + 'static,
            A: Clone + Debug + Send + Sync + 'static,
            K: Eq + Hash + Clone + Send + Sync,
            E: Fn(&S) -> &[A] + Send + Sync,
            KE: Fn(&A) -> K + Send + Sync,
            F: Fn(&S, &A, &A, &A) -> bool + Send + Sync,
            W: Fn(&A, &A, &A) -> Sc + Send + Sync,
            Sc: Score,
        {
            fn evaluate(&self, solution: &S) -> Sc {
                let entities = (self.extractor)(solution);
                let mut total = Sc::zero();

                let mut temp_index: HashMap<K, Vec<usize>> = HashMap::new();
                for (i, entity) in entities.iter().enumerate() {
                    let key = (self.key_extractor)(entity);
                    temp_index.entry(key).or_default().push(i);
                }

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
                                if (self.filter)(solution, a, b, c) {
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

                let mut temp_index: HashMap<K, Vec<usize>> = HashMap::new();
                for (i, entity) in entities.iter().enumerate() {
                    let key = (self.key_extractor)(entity);
                    temp_index.entry(key).or_default().push(i);
                }

                for indices in temp_index.values() {
                    for pos_i in 0..indices.len() {
                        for pos_j in (pos_i + 1)..indices.len() {
                            for pos_k in (pos_j + 1)..indices.len() {
                                let i = indices[pos_i];
                                let j = indices[pos_j];
                                let k = indices[pos_k];
                                if (self.filter)(
                                    solution,
                                    &entities[i],
                                    &entities[j],
                                    &entities[k],
                                ) {
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
                $crate::impl_get_matches_nary!(tri: self, solution)
            }
        }

        impl<S, A, K, E, KE, F, W, Sc: Score> std::fmt::Debug
            for $struct_name<S, A, K, E, KE, F, W, Sc>
        {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_struct(stringify!($struct_name))
                    .field("name", &self.constraint_ref.name)
                    .field("impact_type", &self.impact_type)
                    .field("match_count", &self.matches.len())
                    .finish()
            }
        }
    };

    // ==================== QUAD (4-arity) ====================
    (quad, $struct_name:ident) => {
        /// Zero-erasure incremental quad-constraint for self-joins.
        ///
        /// All function types are concrete generics - no Arc, no dyn, fully monomorphized.
        /// Uses key-based indexing: entities are grouped by join key for O(k) lookups.
        /// Quadruples are ordered as (i, j, k, l) where i < j < k < l to avoid duplicates.
        pub struct $struct_name<S, A, K, E, KE, F, W, Sc>
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
            entity_to_matches: HashMap<usize, HashSet<(usize, usize, usize, usize)>>,
            matches: HashSet<(usize, usize, usize, usize)>,
            key_to_indices: HashMap<K, HashSet<usize>>,
            index_to_key: HashMap<usize, K>,
            _phantom: PhantomData<(S, A, Sc)>,
        }

        impl<S, A, K, E, KE, F, W, Sc> $struct_name<S, A, K, E, KE, F, W, Sc>
        where
            S: 'static,
            A: Clone + 'static,
            K: Eq + Hash + Clone,
            E: Fn(&S) -> &[A],
            KE: Fn(&A) -> K,
            F: Fn(&S, &A, &A, &A, &A) -> bool,
            W: Fn(&A, &A, &A, &A) -> Sc,
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
            fn compute_score(&self, a: &A, b: &A, c: &A, d: &A) -> Sc {
                let base = (self.weight)(a, b, c, d);
                match self.impact_type {
                    ImpactType::Penalty => -base,
                    ImpactType::Reward => base,
                }
            }

            fn insert_entity(&mut self, solution: &S, entities: &[A], index: usize) -> Sc {
                if index >= entities.len() {
                    return Sc::zero();
                }

                let entity = &entities[index];
                let key = (self.key_extractor)(entity);

                self.index_to_key.insert(index, key.clone());
                self.key_to_indices
                    .entry(key.clone())
                    .or_default()
                    .insert(index);

                let key_to_indices = &self.key_to_indices;
                let matches = &mut self.matches;
                let entity_to_matches = &mut self.entity_to_matches;
                let filter = &self.filter;
                let weight = &self.weight;
                let impact_type = self.impact_type;

                let mut total = Sc::zero();
                if let Some(others) = key_to_indices.get(&key) {
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

                                let mut arr = [index, i, j, k];
                                arr.sort();
                                let [a_idx, b_idx, c_idx, d_idx] = arr;
                                let quad = (a_idx, b_idx, c_idx, d_idx);

                                if matches.contains(&quad) {
                                    continue;
                                }

                                let a = &entities[a_idx];
                                let b = &entities[b_idx];
                                let c = &entities[c_idx];
                                let d = &entities[d_idx];

                                if filter(solution, a, b, c, d) && matches.insert(quad) {
                                    entity_to_matches.entry(a_idx).or_default().insert(quad);
                                    entity_to_matches.entry(b_idx).or_default().insert(quad);
                                    entity_to_matches.entry(c_idx).or_default().insert(quad);
                                    entity_to_matches.entry(d_idx).or_default().insert(quad);
                                    let base = weight(a, b, c, d);
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

                total
            }

            fn retract_entity(&mut self, entities: &[A], index: usize) -> Sc {
                if let Some(key) = self.index_to_key.remove(&index) {
                    if let Some(indices) = self.key_to_indices.get_mut(&key) {
                        indices.remove(&index);
                        if indices.is_empty() {
                            self.key_to_indices.remove(&key);
                        }
                    }
                }

                let Some(quads) = self.entity_to_matches.remove(&index) else {
                    return Sc::zero();
                };

                let mut total = Sc::zero();
                for quad in quads {
                    self.matches.remove(&quad);

                    let (i, j, k, l) = quad;
                    for &other in &[i, j, k, l] {
                        if other != index {
                            if let Some(other_set) = self.entity_to_matches.get_mut(&other) {
                                other_set.remove(&quad);
                                if other_set.is_empty() {
                                    self.entity_to_matches.remove(&other);
                                }
                            }
                        }
                    }

                    if i < entities.len()
                        && j < entities.len()
                        && k < entities.len()
                        && l < entities.len()
                    {
                        let score = self.compute_score(
                            &entities[i],
                            &entities[j],
                            &entities[k],
                            &entities[l],
                        );
                        total = total - score;
                    }
                }

                total
            }
        }

        impl<S, A, K, E, KE, F, W, Sc> IncrementalConstraint<S, Sc>
            for $struct_name<S, A, K, E, KE, F, W, Sc>
        where
            S: Send + Sync + 'static,
            A: Clone + Debug + Send + Sync + 'static,
            K: Eq + Hash + Clone + Send + Sync,
            E: Fn(&S) -> &[A] + Send + Sync,
            KE: Fn(&A) -> K + Send + Sync,
            F: Fn(&S, &A, &A, &A, &A) -> bool + Send + Sync,
            W: Fn(&A, &A, &A, &A) -> Sc + Send + Sync,
            Sc: Score,
        {
            fn evaluate(&self, solution: &S) -> Sc {
                let entities = (self.extractor)(solution);
                let mut total = Sc::zero();

                let mut temp_index: HashMap<K, Vec<usize>> = HashMap::new();
                for (i, entity) in entities.iter().enumerate() {
                    let key = (self.key_extractor)(entity);
                    temp_index.entry(key).or_default().push(i);
                }

                for indices in temp_index.values() {
                    for pos_i in 0..indices.len() {
                        for pos_j in (pos_i + 1)..indices.len() {
                            for pos_k in (pos_j + 1)..indices.len() {
                                for pos_l in (pos_k + 1)..indices.len() {
                                    let i = indices[pos_i];
                                    let j = indices[pos_j];
                                    let k = indices[pos_k];
                                    let l = indices[pos_l];
                                    let a = &entities[i];
                                    let b = &entities[j];
                                    let c = &entities[k];
                                    let d = &entities[l];
                                    if (self.filter)(solution, a, b, c, d) {
                                        total = total + self.compute_score(a, b, c, d);
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

                let mut temp_index: HashMap<K, Vec<usize>> = HashMap::new();
                for (i, entity) in entities.iter().enumerate() {
                    let key = (self.key_extractor)(entity);
                    temp_index.entry(key).or_default().push(i);
                }

                for indices in temp_index.values() {
                    for pos_i in 0..indices.len() {
                        for pos_j in (pos_i + 1)..indices.len() {
                            for pos_k in (pos_j + 1)..indices.len() {
                                for pos_l in (pos_k + 1)..indices.len() {
                                    let i = indices[pos_i];
                                    let j = indices[pos_j];
                                    let k = indices[pos_k];
                                    let l = indices[pos_l];
                                    if (self.filter)(
                                        solution,
                                        &entities[i],
                                        &entities[j],
                                        &entities[k],
                                        &entities[l],
                                    ) {
                                        count += 1;
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
                $crate::impl_get_matches_nary!(quad: self, solution)
            }
        }

        impl<S, A, K, E, KE, F, W, Sc: Score> std::fmt::Debug
            for $struct_name<S, A, K, E, KE, F, W, Sc>
        {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_struct(stringify!($struct_name))
                    .field("name", &self.constraint_ref.name)
                    .field("impact_type", &self.impact_type)
                    .field("match_count", &self.matches.len())
                    .finish()
            }
        }
    };

    // ==================== PENTA (5-arity) ====================
    (penta, $struct_name:ident) => {
        /// Zero-erasure incremental penta-constraint for self-joins.
        ///
        /// All function types are concrete generics - no Arc, no dyn, fully monomorphized.
        /// Uses key-based indexing: entities are grouped by join key for O(k) lookups.
        /// Quintuples are ordered as (i, j, k, l, m) where i < j < k < l < m to avoid duplicates.
        pub struct $struct_name<S, A, K, E, KE, F, W, Sc>
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
            entity_to_matches: HashMap<usize, HashSet<(usize, usize, usize, usize, usize)>>,
            matches: HashSet<(usize, usize, usize, usize, usize)>,
            key_to_indices: HashMap<K, HashSet<usize>>,
            index_to_key: HashMap<usize, K>,
            _phantom: PhantomData<(S, A, Sc)>,
        }

        impl<S, A, K, E, KE, F, W, Sc> $struct_name<S, A, K, E, KE, F, W, Sc>
        where
            S: 'static,
            A: Clone + 'static,
            K: Eq + Hash + Clone,
            E: Fn(&S) -> &[A],
            KE: Fn(&A) -> K,
            F: Fn(&S, &A, &A, &A, &A, &A) -> bool,
            W: Fn(&A, &A, &A, &A, &A) -> Sc,
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
            fn compute_score(&self, a: &A, b: &A, c: &A, d: &A, e: &A) -> Sc {
                let base = (self.weight)(a, b, c, d, e);
                match self.impact_type {
                    ImpactType::Penalty => -base,
                    ImpactType::Reward => base,
                }
            }

            fn insert_entity(&mut self, solution: &S, entities: &[A], index: usize) -> Sc {
                if index >= entities.len() {
                    return Sc::zero();
                }

                let entity = &entities[index];
                let key = (self.key_extractor)(entity);

                self.index_to_key.insert(index, key.clone());
                self.key_to_indices
                    .entry(key.clone())
                    .or_default()
                    .insert(index);

                let key_to_indices = &self.key_to_indices;
                let matches = &mut self.matches;
                let entity_to_matches = &mut self.entity_to_matches;
                let filter = &self.filter;
                let weight = &self.weight;
                let impact_type = self.impact_type;

                let mut total = Sc::zero();
                if let Some(others) = key_to_indices.get(&key) {
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

                                    let mut arr = [index, i, j, k, l];
                                    arr.sort();
                                    let [a_idx, b_idx, c_idx, d_idx, e_idx] = arr;
                                    let penta = (a_idx, b_idx, c_idx, d_idx, e_idx);

                                    if matches.contains(&penta) {
                                        continue;
                                    }

                                    let a = &entities[a_idx];
                                    let b = &entities[b_idx];
                                    let c = &entities[c_idx];
                                    let d = &entities[d_idx];
                                    let e = &entities[e_idx];

                                    if filter(solution, a, b, c, d, e) && matches.insert(penta) {
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

            fn retract_entity(&mut self, entities: &[A], index: usize) -> Sc {
                if let Some(key) = self.index_to_key.remove(&index) {
                    if let Some(indices) = self.key_to_indices.get_mut(&key) {
                        indices.remove(&index);
                        if indices.is_empty() {
                            self.key_to_indices.remove(&key);
                        }
                    }
                }

                let Some(pentas) = self.entity_to_matches.remove(&index) else {
                    return Sc::zero();
                };

                let mut total = Sc::zero();
                for penta in pentas {
                    self.matches.remove(&penta);

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
            for $struct_name<S, A, K, E, KE, F, W, Sc>
        where
            S: Send + Sync + 'static,
            A: Clone + Debug + Send + Sync + 'static,
            K: Eq + Hash + Clone + Send + Sync,
            E: Fn(&S) -> &[A] + Send + Sync,
            KE: Fn(&A) -> K + Send + Sync,
            F: Fn(&S, &A, &A, &A, &A, &A) -> bool + Send + Sync,
            W: Fn(&A, &A, &A, &A, &A) -> Sc + Send + Sync,
            Sc: Score,
        {
            fn evaluate(&self, solution: &S) -> Sc {
                let entities = (self.extractor)(solution);
                let mut total = Sc::zero();

                let mut temp_index: HashMap<K, Vec<usize>> = HashMap::new();
                for (i, entity) in entities.iter().enumerate() {
                    let key = (self.key_extractor)(entity);
                    temp_index.entry(key).or_default().push(i);
                }

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
                                        if (self.filter)(solution, a, b, c, d, e) {
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

                let mut temp_index: HashMap<K, Vec<usize>> = HashMap::new();
                for (i, entity) in entities.iter().enumerate() {
                    let key = (self.key_extractor)(entity);
                    temp_index.entry(key).or_default().push(i);
                }

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
                                            solution,
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
                $crate::impl_get_matches_nary!(penta: self, solution)
            }
        }

        impl<S, A, K, E, KE, F, W, Sc: Score> std::fmt::Debug
            for $struct_name<S, A, K, E, KE, F, W, Sc>
        {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_struct(stringify!($struct_name))
                    .field("name", &self.constraint_ref.name)
                    .field("impact_type", &self.impact_type)
                    .field("match_count", &self.matches.len())
                    .finish()
            }
        }
    };
}

pub use impl_incremental_nary_constraint;
