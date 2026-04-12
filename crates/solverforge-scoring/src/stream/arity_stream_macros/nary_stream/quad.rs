macro_rules! impl_quad_arity_stream {
    ($stream:ident, $builder:ident, $constraint:ident) => {
        pub struct $stream<S, A, K, E, KE, F, Sc>
        where
            Sc: solverforge_core::score::Score,
        {
            pub(crate) extractor: E,
            pub(crate) key_extractor: KE,
            pub(crate) filter: F,
            pub(crate) _phantom:
                std::marker::PhantomData<(fn() -> S, fn() -> A, fn() -> K, fn() -> Sc)>,
        }

        impl<S, A, K, E, KE, Sc> $stream<S, A, K, E, KE, super::filter::TrueFilter, Sc>
        where
            S: Send + Sync + 'static,
            A: Clone + std::hash::Hash + PartialEq + Send + Sync + 'static,
            K: Eq + std::hash::Hash + Clone + Send + Sync,
            E: super::collection_extract::CollectionExtract<S, Item = A>,
            KE: super::key_extract::KeyExtract<S, A, K>,
            Sc: solverforge_core::score::Score + 'static,
        {
            pub fn new_self_join(extractor: E, key_extractor: KE) -> Self {
                Self {
                    extractor,
                    key_extractor,
                    filter: super::filter::TrueFilter,
                    _phantom: std::marker::PhantomData,
                }
            }
        }

        impl<S, A, K, E, KE, F, Sc> $stream<S, A, K, E, KE, F, Sc>
        where
            S: Send + Sync + 'static,
            A: Clone + std::hash::Hash + PartialEq + Send + Sync + 'static,
            K: Eq + std::hash::Hash + Clone + Send + Sync,
            E: super::collection_extract::CollectionExtract<S, Item = A>,
            KE: super::key_extract::KeyExtract<S, A, K>,
            F: super::filter::QuadFilter<S, A, A, A, A>,
            Sc: solverforge_core::score::Score + 'static,
        {
            pub fn new_self_join_with_filter(extractor: E, key_extractor: KE, filter: F) -> Self {
                Self {
                    extractor,
                    key_extractor,
                    filter,
                    _phantom: std::marker::PhantomData,
                }
            }

            pub fn filter<P>(
                self,
                predicate: P,
            ) -> $stream<
                S,
                A,
                K,
                E,
                KE,
                super::filter::AndQuadFilter<
                    F,
                    super::filter::FnQuadFilter<impl Fn(&S, &A, &A, &A, &A) -> bool + Send + Sync>,
                >,
                Sc,
            >
            where
                P: Fn(&A, &A, &A, &A) -> bool + Send + Sync + 'static,
            {
                $stream {
                    extractor: self.extractor,
                    key_extractor: self.key_extractor,
                    filter: super::filter::AndQuadFilter::new(
                        self.filter,
                        super::filter::FnQuadFilter::new(
                            move |_s: &S, a: &A, b: &A, c: &A, d: &A| predicate(a, b, c, d),
                        ),
                    ),
                    _phantom: std::marker::PhantomData,
                }
            }

            pub fn penalize(
                self,
                weight: Sc,
            ) -> $builder<S, A, K, E, KE, F, impl Fn(&A, &A, &A, &A) -> Sc + Send + Sync, Sc>
            where
                Sc: Copy,
            {
                let is_hard = weight
                    .to_level_numbers()
                    .first()
                    .map(|&h| h != 0)
                    .unwrap_or(false);
                $builder {
                    extractor: self.extractor,
                    key_extractor: self.key_extractor,
                    filter: self.filter,
                    impact_type: solverforge_core::ImpactType::Penalty,
                    weight: move |_: &A, _: &A, _: &A, _: &A| weight,
                    is_hard,
                    expected_descriptor: None,
                    _phantom: std::marker::PhantomData,
                }
            }

            pub fn penalize_with<W>(self, weight_fn: W) -> $builder<S, A, K, E, KE, F, W, Sc>
            where
                W: Fn(&A, &A, &A, &A) -> Sc + Send + Sync,
            {
                $builder {
                    extractor: self.extractor,
                    key_extractor: self.key_extractor,
                    filter: self.filter,
                    impact_type: solverforge_core::ImpactType::Penalty,
                    weight: weight_fn,
                    is_hard: false,
                    expected_descriptor: None,
                    _phantom: std::marker::PhantomData,
                }
            }

            pub fn penalize_hard_with<W>(self, weight_fn: W) -> $builder<S, A, K, E, KE, F, W, Sc>
            where
                W: Fn(&A, &A, &A, &A) -> Sc + Send + Sync,
            {
                $builder {
                    extractor: self.extractor,
                    key_extractor: self.key_extractor,
                    filter: self.filter,
                    impact_type: solverforge_core::ImpactType::Penalty,
                    weight: weight_fn,
                    is_hard: true,
                    expected_descriptor: None,
                    _phantom: std::marker::PhantomData,
                }
            }

            pub fn reward(
                self,
                weight: Sc,
            ) -> $builder<S, A, K, E, KE, F, impl Fn(&A, &A, &A, &A) -> Sc + Send + Sync, Sc>
            where
                Sc: Copy,
            {
                let is_hard = weight
                    .to_level_numbers()
                    .first()
                    .map(|&h| h != 0)
                    .unwrap_or(false);
                $builder {
                    extractor: self.extractor,
                    key_extractor: self.key_extractor,
                    filter: self.filter,
                    impact_type: solverforge_core::ImpactType::Reward,
                    weight: move |_: &A, _: &A, _: &A, _: &A| weight,
                    is_hard,
                    expected_descriptor: None,
                    _phantom: std::marker::PhantomData,
                }
            }

            pub fn reward_with<W>(self, weight_fn: W) -> $builder<S, A, K, E, KE, F, W, Sc>
            where
                W: Fn(&A, &A, &A, &A) -> Sc + Send + Sync,
            {
                $builder {
                    extractor: self.extractor,
                    key_extractor: self.key_extractor,
                    filter: self.filter,
                    impact_type: solverforge_core::ImpactType::Reward,
                    weight: weight_fn,
                    is_hard: false,
                    expected_descriptor: None,
                    _phantom: std::marker::PhantomData,
                }
            }

            pub fn reward_hard_with<W>(self, weight_fn: W) -> $builder<S, A, K, E, KE, F, W, Sc>
            where
                W: Fn(&A, &A, &A, &A) -> Sc + Send + Sync,
            {
                $builder {
                    extractor: self.extractor,
                    key_extractor: self.key_extractor,
                    filter: self.filter,
                    impact_type: solverforge_core::ImpactType::Reward,
                    weight: weight_fn,
                    is_hard: true,
                    expected_descriptor: None,
                    _phantom: std::marker::PhantomData,
                }
            }

            pub fn penalize_hard(
                self,
            ) -> $builder<S, A, K, E, KE, F, impl Fn(&A, &A, &A, &A) -> Sc + Send + Sync, Sc>
            where
                Sc: Copy,
            {
                self.penalize(Sc::one_hard())
            }

            pub fn penalize_soft(
                self,
            ) -> $builder<S, A, K, E, KE, F, impl Fn(&A, &A, &A, &A) -> Sc + Send + Sync, Sc>
            where
                Sc: Copy,
            {
                self.penalize(Sc::one_soft())
            }

            pub fn reward_hard(
                self,
            ) -> $builder<S, A, K, E, KE, F, impl Fn(&A, &A, &A, &A) -> Sc + Send + Sync, Sc>
            where
                Sc: Copy,
            {
                self.reward(Sc::one_hard())
            }

            pub fn reward_soft(
                self,
            ) -> $builder<S, A, K, E, KE, F, impl Fn(&A, &A, &A, &A) -> Sc + Send + Sync, Sc>
            where
                Sc: Copy,
            {
                self.reward(Sc::one_soft())
            }
        }

        impl<S, A, K, E, KE, F, Sc: solverforge_core::score::Score> std::fmt::Debug
            for $stream<S, A, K, E, KE, F, Sc>
        {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_struct(stringify!($stream)).finish()
            }
        }

        pub struct $builder<S, A, K, E, KE, F, W, Sc>
        where
            Sc: solverforge_core::score::Score,
        {
            pub(crate) extractor: E,
            pub(crate) key_extractor: KE,
            pub(crate) filter: F,
            pub(crate) impact_type: solverforge_core::ImpactType,
            pub(crate) weight: W,
            pub(crate) is_hard: bool,
            pub(crate) expected_descriptor: Option<usize>,
            pub(crate) _phantom:
                std::marker::PhantomData<(fn() -> S, fn() -> A, fn() -> K, fn() -> Sc)>,
        }

        impl<S, A, K, E, KE, F, W, Sc> $builder<S, A, K, E, KE, F, W, Sc>
        where
            S: Send + Sync + 'static,
            A: Clone + Send + Sync + 'static,
            K: Eq + std::hash::Hash + Clone + Send + Sync,
            E: super::collection_extract::CollectionExtract<S, Item = A> + Clone,
            KE: super::key_extract::KeyExtract<S, A, K>,
            F: super::filter::QuadFilter<S, A, A, A, A>,
            W: Fn(&A, &A, &A, &A) -> Sc + Send + Sync,
            Sc: solverforge_core::score::Score + 'static,
        {
            pub fn for_descriptor(mut self, descriptor_index: usize) -> Self {
                self.expected_descriptor = Some(descriptor_index);
                self
            }

            pub fn named(
                self,
                name: &str,
            ) -> $constraint<
                S,
                A,
                K,
                E,
                KE,
                impl Fn(&S, &A, &A, &A, &A) -> bool + Send + Sync,
                impl Fn(&S, usize, usize, usize, usize) -> Sc + Send + Sync,
                Sc,
            > {
                let filter = self.filter;
                let combined_filter =
                    move |s: &S, a: &A, b: &A, c: &A, d: &A| filter.test(s, a, b, c, d);

                let extractor_for_weight = self.extractor.clone();
                let user_weight = self.weight;
                let adapted_weight =
                    move |solution: &S, a_idx: usize, b_idx: usize, c_idx: usize, d_idx: usize| {
                        let entities = super::collection_extract::CollectionExtract::extract(
                            &extractor_for_weight,
                            solution,
                        );
                        user_weight(
                            &entities[a_idx],
                            &entities[b_idx],
                            &entities[c_idx],
                            &entities[d_idx],
                        )
                    };

                let mut constraint = $constraint::new(
                    solverforge_core::ConstraintRef::new("", name),
                    self.impact_type,
                    self.extractor,
                    self.key_extractor,
                    combined_filter,
                    adapted_weight,
                    self.is_hard,
                );
                if let Some(d) = self.expected_descriptor {
                    constraint = constraint.with_descriptor(d);
                }
                constraint
            }
        }

        impl<S, A, K, E, KE, F, W, Sc: solverforge_core::score::Score> std::fmt::Debug
            for $builder<S, A, K, E, KE, F, W, Sc>
        {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_struct(stringify!($builder))
                    .field("impact_type", &self.impact_type)
                    .finish()
            }
        }
    };
}
