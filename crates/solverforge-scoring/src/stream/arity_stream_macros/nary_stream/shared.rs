/* Shared implementation for self-join n-ary constraint stream macros.

This keeps the per-arity files thin and explicit while centralizing the
repeated stream/builder generation strategy in one place.
*/

macro_rules! repeat_tokens {
    ($_ignore:tt => $($tokens:tt)*) => {
        $($tokens)*
    };
}

macro_rules! impl_nary_arity_stream_common {
    (
        stream = $stream:ident,
        builder = $builder:ident,
        constraint = $constraint:ident,
        filter_trait = $filter_trait:ident,
        and_filter = $and_filter:ident,
        fn_filter = $fn_filter:ident,
        entities = [$($entity:ident),+],
        weight_indices = [$($weight_idx:ident),+],
        filter_indices = [$($filter_idx:ident),*]
    ) => {
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
            F: super::filter::$filter_trait<S, $(repeat_tokens!($entity => A)),+>,
            Sc: solverforge_core::score::Score + 'static,
        {
            fn into_weighted_builder<W>(
                self,
                impact_type: solverforge_core::ImpactType,
                weight: W,
                is_hard: bool,
            ) -> $builder<S, A, K, E, KE, F, W, Sc>
            where
                W: Fn($(repeat_tokens!($entity => &A)),+) -> Sc + Send + Sync,
            {
                $builder {
                    extractor: self.extractor,
                    key_extractor: self.key_extractor,
                    filter: self.filter,
                    impact_type,
                    weight,
                    is_hard,
                    _phantom: std::marker::PhantomData,
                }
            }

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
	                super::filter::$and_filter<
	                    F,
	                    super::filter::$fn_filter<
	                        impl Fn(
	                            &S,
	                            $(repeat_tokens!($entity => &A)),+
	                            $(, repeat_tokens!($filter_idx => usize))*
	                        ) -> bool + Send + Sync,
	                    >,
	                >,
	                Sc,
            >
            where
                P: Fn($(repeat_tokens!($entity => &A)),+) -> bool + Send + Sync + 'static,
            {
                $stream {
                    extractor: self.extractor,
                    key_extractor: self.key_extractor,
                    filter: super::filter::$and_filter::new(
	                        self.filter,
	                        super::filter::$fn_filter::new(
	                            move |_s: &S,
	                                  $($entity: &A),+
	                                  $(, $filter_idx: usize)*| {
	                                let _ = ($($filter_idx),*);
	                                predicate($($entity),+)
	                            },
	                        ),
	                    ),
	                    _phantom: std::marker::PhantomData,
                }
            }

            pub fn penalize<W>(
                self,
                weight: W,
            ) -> $builder<
                S,
                A,
                K,
                E,
                KE,
                F,
                impl Fn($(repeat_tokens!($entity => &A)),+) -> Sc + Send + Sync,
                Sc,
            >
            where
                W: for<'w> super::weighting_support::ConstraintWeight<
                        ($(repeat_tokens!($entity => &'w A)),+),
                        Sc,
                    > + Send + Sync,
            {
                let is_hard = weight.is_hard();
                self.into_weighted_builder(
                    solverforge_core::ImpactType::Penalty,
                    move |$($entity: &A),+| {
                        weight.score(($($entity),+))
                    },
                    is_hard,
                )
            }

            pub fn reward<W>(
                self,
                weight: W,
            ) -> $builder<
                S,
                A,
                K,
                E,
                KE,
                F,
                impl Fn($(repeat_tokens!($entity => &A)),+) -> Sc + Send + Sync,
                Sc,
            >
            where
                W: for<'w> super::weighting_support::ConstraintWeight<
                        ($(repeat_tokens!($entity => &'w A)),+),
                        Sc,
                    > + Send + Sync,
            {
                let is_hard = weight.is_hard();
                self.into_weighted_builder(
                    solverforge_core::ImpactType::Reward,
                    move |$($entity: &A),+| {
                        weight.score(($($entity),+))
                    },
                    is_hard,
                )
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
            pub(crate) _phantom:
                std::marker::PhantomData<(fn() -> S, fn() -> A, fn() -> K, fn() -> Sc)>,
        }

        impl<S, A, K, E, KE, F, W, Sc> $builder<S, A, K, E, KE, F, W, Sc>
        where
            S: Send + Sync + 'static,
            A: Clone + Send + Sync + 'static,
            K: Eq + std::hash::Hash + Clone + Send + Sync,
            E: super::collection_extract::CollectionExtract<S, Item = A>,
            KE: super::key_extract::KeyExtract<S, A, K>,
            F: super::filter::$filter_trait<S, $(repeat_tokens!($entity => A)),+>,
            W: Fn($(repeat_tokens!($entity => &A)),+) -> Sc + Send + Sync,
            Sc: solverforge_core::score::Score + 'static,
        {
            pub fn named(
                self,
                name: &str,
            ) -> $constraint<
                S,
                A,
                K,
                E,
                KE,
                impl Fn(
                    &S,
                    $(repeat_tokens!($entity => &A)),+
                    $(, repeat_tokens!($filter_idx => usize))*
                ) -> bool
                       + Send
                       + Sync,
                impl Fn(
                    &S,
                    &[A],
                    $(repeat_tokens!($weight_idx => usize)),+
                ) -> Sc + Send + Sync,
                Sc,
            > {
                let filter = self.filter;
                let combined_filter =
                    move |s: &S, $($entity: &A),+ $(, $filter_idx: usize)*| {
                        filter.test(s, $($entity),+ $(, $filter_idx)*)
                    };

                let user_weight = self.weight;
                let adapted_weight = move |_solution: &S, entities: &[A], $($weight_idx: usize),+| {
                    user_weight($(&entities[$weight_idx]),+)
                };

                $constraint::new(
                    solverforge_core::ConstraintRef::new("", name),
                    self.impact_type,
                    self.extractor,
                    self.key_extractor,
                    combined_filter,
                    adapted_weight,
                    self.is_hard,
                )
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
