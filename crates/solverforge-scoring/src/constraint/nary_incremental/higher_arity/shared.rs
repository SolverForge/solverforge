/* Shared higher-arity incremental constraint scaffolding for tri/quad/penta.

This keeps the exported per-arity macros explicit while centralizing the
repeated lifecycle, indexing, and delta-application structure.
*/

macro_rules! repeat_nary_constraint_tokens {
    ($_ignore:tt => $($tokens:tt)*) => {
        $($tokens)*
    };
}

macro_rules! for_each_indices_combination {
    (
        $indices:expr,
        positions = [$($pos:ident),+],
        values = [$($value:ident),+],
        $body:block
    ) => {
        for_each_indices_combination!(
            @loop
            $indices,
            0usize,
            [$($pos),+],
            [$($value),+],
            $body
        );
    };
    (@loop $indices:expr, $start:expr, [], [], $body:block) => {
        $body
    };
    (@loop $indices:expr, $start:expr, [$pos:ident $(, $rest_pos:ident)*], [$value:ident $(, $rest_value:ident)*], $body:block) => {
        for $pos in $start..$indices.len() {
            let $value = $indices[$pos];
            for_each_indices_combination!(
                @loop
                $indices,
                $pos + 1,
                [$($rest_pos),*],
                [$($rest_value),*],
                $body
            );
        }
    };
}

macro_rules! for_each_other_indices_combination {
    ($others:expr, $index:expr, values = [$first:ident $(, $rest:ident)*], $body:block) => {
        for &$first in $others {
            if $first == $index {
                continue;
            }
            for_each_other_indices_combination!(@rest $others, $index, $first, [$($rest),*], $body);
        }
    };
    (@rest $others:expr, $index:expr, $prev:ident, [], $body:block) => {
        $body
    };
    (@rest $others:expr, $index:expr, $prev:ident, [$current:ident $(, $rest:ident)*], $body:block) => {
        for &$current in $others {
            if $current <= $prev || $current == $index {
                continue;
            }
            for_each_other_indices_combination!(@rest $others, $index, $current, [$($rest),*], $body);
        }
    };
}

macro_rules! impl_incremental_higher_arity_constraint_common {
    (
        struct_name = $struct_name:ident,
        match_kind = $match_kind:ident,
        entities = [$($entity:ident),+],
        match_indices = [$($match_idx:ident),+],
        combo_positions = [$($combo_pos:ident),+],
        combo_values = [$($combo_value:ident),+],
        other_values = [$($other_value:ident),+]
    ) => {
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
            change_source: $crate::stream::collection_extract::ChangeSource,
            entity_to_matches:
                HashMap<usize, HashSet<($(repeat_nary_constraint_tokens!($match_idx => usize)),+)>>,
            matches: HashSet<($(repeat_nary_constraint_tokens!($match_idx => usize)),+)>,
            key_to_indices: HashMap<K, HashSet<usize>>,
            index_to_key: HashMap<usize, K>,
            _phantom: PhantomData<(fn() -> S, fn() -> A, fn() -> Sc)>,
        }

        impl<S, A, K, E, KE, F, W, Sc> $struct_name<S, A, K, E, KE, F, W, Sc>
        where
            S: 'static,
            A: Clone + 'static,
            K: Eq + Hash + Clone,
            E: $crate::stream::collection_extract::CollectionExtract<S, Item = A>,
            KE: $crate::stream::key_extract::KeyExtract<S, A, K>,
	            F: Fn(
	                &S,
	                $(repeat_nary_constraint_tokens!($entity => &A)),+,
	                $(repeat_nary_constraint_tokens!($match_idx => usize)),+
	            ) -> bool,
            W: Fn(&S, &[A], $(repeat_nary_constraint_tokens!($match_idx => usize)),+) -> Sc,
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
                let change_source = extractor.change_source();
                Self {
                    constraint_ref,
                    impact_type,
                    extractor,
                    key_extractor,
                    filter,
                    weight,
                    is_hard,
                    change_source,
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
                entities: &[A],
                match_indices: ($(repeat_nary_constraint_tokens!($match_idx => usize)),+),
            ) -> Sc {
                let ($($match_idx),+) = match_indices;
                let base = (self.weight)(solution, entities, $($match_idx),+);
                match self.impact_type {
                    ImpactType::Penalty => -base,
                    ImpactType::Reward => base,
                }
            }

            fn build_index_map(&self, solution: &S, entities: &[A]) -> HashMap<K, Vec<usize>> {
                let mut temp_index: HashMap<K, Vec<usize>> = HashMap::new();
                for (i, entity) in entities.iter().enumerate() {
                    let key = $crate::stream::key_extract::KeyExtract::extract(
                        &self.key_extractor,
                        solution,
                        entity,
                        i,
                    );
                    temp_index.entry(key).or_default().push(i);
                }
                temp_index
            }

            fn insert_entity(&mut self, solution: &S, entities: &[A], index: usize) -> Sc {
                if index >= entities.len() {
                    return Sc::zero();
                }

                let entity = &entities[index];
                let key = $crate::stream::key_extract::KeyExtract::extract(
                    &self.key_extractor,
                    solution,
                    entity,
                    index,
                );

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
                    for_each_other_indices_combination!(others, index, values = [$($other_value),+], {
                        let mut arr = [index, $($other_value),+];
                        arr.sort();
                        let [$($match_idx),+] = arr;
                        let match_tuple = ($($match_idx),+);

                        if matches.contains(&match_tuple) {
                            continue;
                        }

                        $(let $entity = &entities[$match_idx];)+

	                        if filter(solution, $($entity),+, $($match_idx),+) && matches.insert(match_tuple) {
                            $(entity_to_matches.entry($match_idx).or_default().insert(match_tuple);)+
                            let base = weight(solution, entities, $($match_idx),+);
                            let score = match impact_type {
                                ImpactType::Penalty => -base,
                                ImpactType::Reward => base,
                            };
                            total = total + score;
                        }
                    });
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

                let Some(match_tuples) = self.entity_to_matches.remove(&index) else {
                    return Sc::zero();
                };

                let mut total = Sc::zero();
                for match_tuple in match_tuples {
                    self.matches.remove(&match_tuple);

                    let ($($match_idx),+) = match_tuple;
                    for &other in &[$($match_idx),+] {
                        if other != index {
                            if let Some(other_set) = self.entity_to_matches.get_mut(&other) {
                                other_set.remove(&match_tuple);
                                if other_set.is_empty() {
                                    self.entity_to_matches.remove(&other);
                                }
                            }
                        }
                    }

                    if true $(&& $match_idx < entities.len())* {
                        let score = self.compute_score(solution, entities, match_tuple);
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
            E: $crate::stream::collection_extract::CollectionExtract<S, Item = A>,
            KE: $crate::stream::key_extract::KeyExtract<S, A, K>,
	            F: Fn(
	                &S,
	                $(repeat_nary_constraint_tokens!($entity => &A)),+,
	                $(repeat_nary_constraint_tokens!($match_idx => usize)),+
	            ) -> bool + Send + Sync,
            W: Fn(&S, &[A], $(repeat_nary_constraint_tokens!($match_idx => usize)),+) -> Sc + Send + Sync,
            Sc: Score,
        {
            fn evaluate(&self, solution: &S) -> Sc {
                let entities = $crate::stream::collection_extract::CollectionExtract::extract(
                    &self.extractor,
                    solution,
                );
                let mut total = Sc::zero();
                let temp_index = self.build_index_map(solution, entities);

                for indices in temp_index.values() {
                    for_each_indices_combination!(
                        indices,
                        positions = [$($combo_pos),+],
                        values = [$($combo_value),+],
                        {
                            $(let $entity = &entities[$combo_value];)+
	                            if (self.filter)(solution, $($entity),+, $($combo_value),+) {
                                total = total + self.compute_score(solution, entities, ($($combo_value),+));
                            }
                        }
                    );
                }

                total
            }

            fn match_count(&self, solution: &S) -> usize {
                let entities = $crate::stream::collection_extract::CollectionExtract::extract(
                    &self.extractor,
                    solution,
                );
                let mut count = 0;
                let temp_index = self.build_index_map(solution, entities);

                for indices in temp_index.values() {
                    for_each_indices_combination!(
                        indices,
                        positions = [$($combo_pos),+],
                        values = [$($combo_value),+],
                        {
	                            if (self.filter)(solution, $(&entities[$combo_value]),+, $($combo_value),+) {
                                count += 1;
                            }
                        }
                    );
                }

                count
            }

            fn initialize(&mut self, solution: &S) -> Sc {
                self.reset();
                let entities = $crate::stream::collection_extract::CollectionExtract::extract(
                    &self.extractor,
                    solution,
                );
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
                descriptor_index: usize,
            ) -> Sc {
                if !self
                    .change_source
                    .assert_localizes(descriptor_index, &self.constraint_ref.name)
                {
                    return Sc::zero();
                }
                let entities = $crate::stream::collection_extract::CollectionExtract::extract(
                    &self.extractor,
                    solution,
                );
                self.insert_entity(solution, entities, entity_index)
            }

            fn on_retract(
                &mut self,
                solution: &S,
                entity_index: usize,
                descriptor_index: usize,
            ) -> Sc {
                if !self
                    .change_source
                    .assert_localizes(descriptor_index, &self.constraint_ref.name)
                {
                    return Sc::zero();
                }
                let entities = $crate::stream::collection_extract::CollectionExtract::extract(
                    &self.extractor,
                    solution,
                );
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

            fn constraint_ref(&self) -> &ConstraintRef {
        &self.constraint_ref
    }

            fn get_matches<'a>(&'a self, solution: &S) -> Vec<DetailedConstraintMatch<'a, Sc>> {
                $crate::impl_get_matches_nary!($match_kind: self, solution)
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
