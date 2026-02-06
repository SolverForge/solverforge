// Penta-arity (5-tuple) incremental constraint macro.

// Generates a penta-arity incremental constraint struct.
#[macro_export]
macro_rules! impl_incremental_penta_constraint {
    ($struct_name:ident) => {
        // Zero-erasure incremental penta-constraint for self-joins.
        //
        // All function types are concrete generics - no Arc, no dyn, fully monomorphized.
        // Uses key-based indexing: entities are grouped by join key for O(k) lookups.
        // Quintuples are ordered as (i, j, k, l, m) where i < j < k < l < m to avoid duplicates.
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
            KE: Fn(&S, &A, usize) -> K,
            F: Fn(&S, &A, &A, &A, &A, &A) -> bool,
            W: Fn(&S, usize, usize, usize, usize, usize) -> Sc,
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
            fn compute_score(
                &self,
                solution: &S,
                a_idx: usize,
                b_idx: usize,
                c_idx: usize,
                d_idx: usize,
                e_idx: usize,
            ) -> Sc {
                let base = (self.weight)(solution, a_idx, b_idx, c_idx, d_idx, e_idx);
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
                let key = (self.key_extractor)(solution, entity, index);

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
                                        let base =
                                            weight(solution, a_idx, b_idx, c_idx, d_idx, e_idx);
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

            fn retract_entity(&mut self, solution: &S, entities: &[A], index: usize) -> Sc {
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
                        let score = self.compute_score(solution, i, j, k, l, m);
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
            KE: Fn(&S, &A, usize) -> K + Send + Sync,
            F: Fn(&S, &A, &A, &A, &A, &A) -> bool + Send + Sync,
            W: Fn(&S, usize, usize, usize, usize, usize) -> Sc + Send + Sync,
            Sc: Score,
        {
            fn evaluate(&self, solution: &S) -> Sc {
                let entities = (self.extractor)(solution);
                let mut total = Sc::zero();

                let mut temp_index: HashMap<K, Vec<usize>> = HashMap::new();
                for (i, entity) in entities.iter().enumerate() {
                    let key = (self.key_extractor)(solution, entity, i);
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
                                            total =
                                                total + self.compute_score(solution, i, j, k, l, m);
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
                    let key = (self.key_extractor)(solution, entity, i);
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

            fn on_insert(
                &mut self,
                solution: &S,
                entity_index: usize,
                _descriptor_index: usize,
            ) -> Sc {
                let entities = (self.extractor)(solution);
                self.insert_entity(solution, entities, entity_index)
            }

            fn on_retract(
                &mut self,
                solution: &S,
                entity_index: usize,
                _descriptor_index: usize,
            ) -> Sc {
                let entities = (self.extractor)(solution);
                self.retract_entity(solution, entities, entity_index)
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

pub use impl_incremental_penta_constraint;
