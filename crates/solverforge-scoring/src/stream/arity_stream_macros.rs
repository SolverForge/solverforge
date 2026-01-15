//! Macros for generating arity-specific constraint streams.
//!
//! These macros reduce code duplication across Bi/Tri/Quad/Penta streams
//! which all follow the same pattern with different tuple sizes.

/// Generates the constraint stream struct, builder struct, and common methods.
///
/// Doctests and unique methods (like join_self) should be defined outside the macro
/// in the individual stream files.
macro_rules! impl_arity_stream {
    // Bi arity (2 params: a, b)
    (bi, $stream:ident, $builder:ident, $constraint:ident) => {
        pub struct $stream<S, A, K, E, KE, F, Sc>
        where
            Sc: solverforge_core::score::Score,
        {
            pub(crate) extractor: E,
            pub(crate) key_extractor: KE,
            pub(crate) filter: F,
            pub(crate) _phantom: std::marker::PhantomData<(S, A, K, Sc)>,
        }

        impl<S, A, K, E, KE, Sc> $stream<S, A, K, E, KE, super::filter::TrueFilter, Sc>
        where
            S: Send + Sync + 'static,
            A: Clone + std::hash::Hash + PartialEq + Send + Sync + 'static,
            K: Eq + std::hash::Hash + Clone + Send + Sync,
            E: Fn(&S) -> &[A] + Send + Sync,
            KE: Fn(&A) -> K + Send + Sync,
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
            E: Fn(&S) -> &[A] + Send + Sync,
            KE: Fn(&A) -> K + Send + Sync,
            F: super::filter::BiFilter<S, A, A>,
            Sc: solverforge_core::score::Score + 'static,
        {
            pub fn new_self_join_with_filter(extractor: E, key_extractor: KE, filter: F) -> Self {
                Self { extractor, key_extractor, filter, _phantom: std::marker::PhantomData }
            }

            /// Adds a filter predicate to the stream.
            pub fn filter<P>(
                self,
                predicate: P,
            ) -> $stream<S, A, K, E, KE, super::filter::AndBiFilter<F, super::filter::FnBiFilter<impl Fn(&S, &A, &A) -> bool + Send + Sync>>, Sc>
            where
                P: Fn(&A, &A) -> bool + Send + Sync + 'static,
            {
                $stream {
                    extractor: self.extractor,
                    key_extractor: self.key_extractor,
                    filter: super::filter::AndBiFilter::new(
                        self.filter,
                        super::filter::FnBiFilter::new(move |_s: &S, a: &A, b: &A| predicate(a, b)),
                    ),
                    _phantom: std::marker::PhantomData,
                }
            }

            pub fn penalize(self, weight: Sc) -> $builder<S, A, K, E, KE, F, impl Fn(&A, &A) -> Sc + Send + Sync, Sc>
            where Sc: Copy,
            {
                let is_hard = weight.to_level_numbers().first().map(|&h| h != 0).unwrap_or(false);
                $builder {
                    extractor: self.extractor, key_extractor: self.key_extractor, filter: self.filter,
                    impact_type: solverforge_core::ImpactType::Penalty,
                    weight: move |_: &A, _: &A| weight,
                    is_hard,
                    _phantom: std::marker::PhantomData,
                }
            }

            pub fn penalize_with<W>(self, weight_fn: W) -> $builder<S, A, K, E, KE, F, W, Sc>
            where W: Fn(&A, &A) -> Sc + Send + Sync,
            {
                $builder {
                    extractor: self.extractor, key_extractor: self.key_extractor, filter: self.filter,
                    impact_type: solverforge_core::ImpactType::Penalty, weight: weight_fn,
                    is_hard: false,
                    _phantom: std::marker::PhantomData,
                }
            }

            pub fn penalize_hard_with<W>(self, weight_fn: W) -> $builder<S, A, K, E, KE, F, W, Sc>
            where W: Fn(&A, &A) -> Sc + Send + Sync,
            {
                $builder {
                    extractor: self.extractor, key_extractor: self.key_extractor, filter: self.filter,
                    impact_type: solverforge_core::ImpactType::Penalty, weight: weight_fn,
                    is_hard: true,
                    _phantom: std::marker::PhantomData,
                }
            }

            pub fn reward(self, weight: Sc) -> $builder<S, A, K, E, KE, F, impl Fn(&A, &A) -> Sc + Send + Sync, Sc>
            where Sc: Copy,
            {
                let is_hard = weight.to_level_numbers().first().map(|&h| h != 0).unwrap_or(false);
                $builder {
                    extractor: self.extractor, key_extractor: self.key_extractor, filter: self.filter,
                    impact_type: solverforge_core::ImpactType::Reward,
                    weight: move |_: &A, _: &A| weight,
                    is_hard,
                    _phantom: std::marker::PhantomData,
                }
            }

            pub fn reward_with<W>(self, weight_fn: W) -> $builder<S, A, K, E, KE, F, W, Sc>
            where W: Fn(&A, &A) -> Sc + Send + Sync,
            {
                $builder {
                    extractor: self.extractor, key_extractor: self.key_extractor, filter: self.filter,
                    impact_type: solverforge_core::ImpactType::Reward, weight: weight_fn,
                    is_hard: false,
                    _phantom: std::marker::PhantomData,
                }
            }

            pub fn reward_hard_with<W>(self, weight_fn: W) -> $builder<S, A, K, E, KE, F, W, Sc>
            where W: Fn(&A, &A) -> Sc + Send + Sync,
            {
                $builder {
                    extractor: self.extractor, key_extractor: self.key_extractor, filter: self.filter,
                    impact_type: solverforge_core::ImpactType::Reward, weight: weight_fn,
                    is_hard: true,
                    _phantom: std::marker::PhantomData,
                }
            }
        }

        impl<S, A, K, E, KE, F, Sc: solverforge_core::score::Score> std::fmt::Debug for $stream<S, A, K, E, KE, F, Sc> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_struct("BiConstraintStream").finish()
            }
        }

        pub struct $builder<S, A, K, E, KE, F, W, Sc> where Sc: solverforge_core::score::Score {
            pub(crate) extractor: E,
            pub(crate) key_extractor: KE,
            pub(crate) filter: F,
            pub(crate) impact_type: solverforge_core::ImpactType,
            pub(crate) weight: W,
            pub(crate) is_hard: bool,
            pub(crate) _phantom: std::marker::PhantomData<(S, A, K, Sc)>,
        }

        impl<S, A, K, E, KE, F, W, Sc> $builder<S, A, K, E, KE, F, W, Sc>
        where
            S: Send + Sync + 'static,
            A: Clone + Send + Sync + 'static,
            K: Eq + std::hash::Hash + Clone + Send + Sync,
            E: Fn(&S) -> &[A] + Send + Sync,
            KE: Fn(&A) -> K + Send + Sync,
            F: super::filter::BiFilter<S, A, A>,
            W: Fn(&A, &A) -> Sc + Send + Sync,
            Sc: solverforge_core::score::Score + 'static,
        {
            pub fn as_constraint(self, name: &str) -> $constraint<S, A, K, E, KE, impl Fn(&S, &A, &A) -> bool + Send + Sync, W, Sc> {
                let filter = self.filter;
                let combined_filter = move |s: &S, a: &A, b: &A| filter.test(s, a, b);
                $constraint::new(
                    solverforge_core::ConstraintRef::new("", name),
                    self.impact_type, self.extractor, self.key_extractor, combined_filter, self.weight,
                    self.is_hard,
                )
            }
        }

        impl<S, A, K, E, KE, F, W, Sc: solverforge_core::score::Score> std::fmt::Debug for $builder<S, A, K, E, KE, F, W, Sc> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_struct("BiConstraintBuilder").field("impact_type", &self.impact_type).finish()
            }
        }
    };

    // Tri arity (3 params: a, b, c)
    (tri, $stream:ident, $builder:ident, $constraint:ident) => {
        pub struct $stream<S, A, K, E, KE, F, Sc>
        where
            Sc: solverforge_core::score::Score,
        {
            pub(crate) extractor: E,
            pub(crate) key_extractor: KE,
            pub(crate) filter: F,
            pub(crate) _phantom: std::marker::PhantomData<(S, A, K, Sc)>,
        }

        impl<S, A, K, E, KE, Sc> $stream<S, A, K, E, KE, super::filter::TrueFilter, Sc>
        where
            S: Send + Sync + 'static,
            A: Clone + std::hash::Hash + PartialEq + Send + Sync + 'static,
            K: Eq + std::hash::Hash + Clone + Send + Sync,
            E: Fn(&S) -> &[A] + Send + Sync,
            KE: Fn(&A) -> K + Send + Sync,
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
            E: Fn(&S) -> &[A] + Send + Sync,
            KE: Fn(&A) -> K + Send + Sync,
            F: super::filter::TriFilter<S, A, A, A>,
            Sc: solverforge_core::score::Score + 'static,
        {
            pub fn new_self_join_with_filter(extractor: E, key_extractor: KE, filter: F) -> Self {
                Self { extractor, key_extractor, filter, _phantom: std::marker::PhantomData }
            }

            /// Adds a filter predicate to the stream.
            pub fn filter<P>(
                self,
                predicate: P,
            ) -> $stream<S, A, K, E, KE, super::filter::AndTriFilter<F, super::filter::FnTriFilter<impl Fn(&S, &A, &A, &A) -> bool + Send + Sync>>, Sc>
            where
                P: Fn(&A, &A, &A) -> bool + Send + Sync + 'static,
            {
                $stream {
                    extractor: self.extractor,
                    key_extractor: self.key_extractor,
                    filter: super::filter::AndTriFilter::new(
                        self.filter,
                        super::filter::FnTriFilter::new(move |_s: &S, a: &A, b: &A, c: &A| predicate(a, b, c)),
                    ),
                    _phantom: std::marker::PhantomData,
                }
            }

            pub fn penalize(self, weight: Sc) -> $builder<S, A, K, E, KE, F, impl Fn(&A, &A, &A) -> Sc + Send + Sync, Sc>
            where Sc: Copy,
            {
                let is_hard = weight.to_level_numbers().first().map(|&h| h != 0).unwrap_or(false);
                $builder {
                    extractor: self.extractor, key_extractor: self.key_extractor, filter: self.filter,
                    impact_type: solverforge_core::ImpactType::Penalty,
                    weight: move |_: &A, _: &A, _: &A| weight,
                    is_hard,
                    _phantom: std::marker::PhantomData,
                }
            }

            pub fn penalize_with<W>(self, weight_fn: W) -> $builder<S, A, K, E, KE, F, W, Sc>
            where W: Fn(&A, &A, &A) -> Sc + Send + Sync,
            {
                $builder {
                    extractor: self.extractor, key_extractor: self.key_extractor, filter: self.filter,
                    impact_type: solverforge_core::ImpactType::Penalty, weight: weight_fn,
                    is_hard: false,
                    _phantom: std::marker::PhantomData,
                }
            }

            pub fn penalize_hard_with<W>(self, weight_fn: W) -> $builder<S, A, K, E, KE, F, W, Sc>
            where W: Fn(&A, &A, &A) -> Sc + Send + Sync,
            {
                $builder {
                    extractor: self.extractor, key_extractor: self.key_extractor, filter: self.filter,
                    impact_type: solverforge_core::ImpactType::Penalty, weight: weight_fn,
                    is_hard: true,
                    _phantom: std::marker::PhantomData,
                }
            }

            pub fn reward(self, weight: Sc) -> $builder<S, A, K, E, KE, F, impl Fn(&A, &A, &A) -> Sc + Send + Sync, Sc>
            where Sc: Copy,
            {
                let is_hard = weight.to_level_numbers().first().map(|&h| h != 0).unwrap_or(false);
                $builder {
                    extractor: self.extractor, key_extractor: self.key_extractor, filter: self.filter,
                    impact_type: solverforge_core::ImpactType::Reward,
                    weight: move |_: &A, _: &A, _: &A| weight,
                    is_hard,
                    _phantom: std::marker::PhantomData,
                }
            }

            pub fn reward_with<W>(self, weight_fn: W) -> $builder<S, A, K, E, KE, F, W, Sc>
            where W: Fn(&A, &A, &A) -> Sc + Send + Sync,
            {
                $builder {
                    extractor: self.extractor, key_extractor: self.key_extractor, filter: self.filter,
                    impact_type: solverforge_core::ImpactType::Reward, weight: weight_fn,
                    is_hard: false,
                    _phantom: std::marker::PhantomData,
                }
            }

            pub fn reward_hard_with<W>(self, weight_fn: W) -> $builder<S, A, K, E, KE, F, W, Sc>
            where W: Fn(&A, &A, &A) -> Sc + Send + Sync,
            {
                $builder {
                    extractor: self.extractor, key_extractor: self.key_extractor, filter: self.filter,
                    impact_type: solverforge_core::ImpactType::Reward, weight: weight_fn,
                    is_hard: true,
                    _phantom: std::marker::PhantomData,
                }
            }
        }

        impl<S, A, K, E, KE, F, Sc: solverforge_core::score::Score> std::fmt::Debug for $stream<S, A, K, E, KE, F, Sc> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_struct("TriConstraintStream").finish()
            }
        }

        pub struct $builder<S, A, K, E, KE, F, W, Sc> where Sc: solverforge_core::score::Score {
            pub(crate) extractor: E,
            pub(crate) key_extractor: KE,
            pub(crate) filter: F,
            pub(crate) impact_type: solverforge_core::ImpactType,
            pub(crate) weight: W,
            pub(crate) is_hard: bool,
            pub(crate) _phantom: std::marker::PhantomData<(S, A, K, Sc)>,
        }

        impl<S, A, K, E, KE, F, W, Sc> $builder<S, A, K, E, KE, F, W, Sc>
        where
            S: Send + Sync + 'static,
            A: Clone + Send + Sync + 'static,
            K: Eq + std::hash::Hash + Clone + Send + Sync,
            E: Fn(&S) -> &[A] + Send + Sync,
            KE: Fn(&A) -> K + Send + Sync,
            F: super::filter::TriFilter<S, A, A, A>,
            W: Fn(&A, &A, &A) -> Sc + Send + Sync,
            Sc: solverforge_core::score::Score + 'static,
        {
            pub fn as_constraint(self, name: &str) -> $constraint<S, A, K, E, KE, impl Fn(&S, &A, &A, &A) -> bool + Send + Sync, W, Sc> {
                let filter = self.filter;
                let combined_filter = move |s: &S, a: &A, b: &A, c: &A| filter.test(s, a, b, c);
                $constraint::new(
                    solverforge_core::ConstraintRef::new("", name),
                    self.impact_type, self.extractor, self.key_extractor, combined_filter, self.weight,
                    self.is_hard,
                )
            }
        }

        impl<S, A, K, E, KE, F, W, Sc: solverforge_core::score::Score> std::fmt::Debug for $builder<S, A, K, E, KE, F, W, Sc> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_struct("TriConstraintBuilder").field("impact_type", &self.impact_type).finish()
            }
        }
    };

    // Quad arity (4 params: a, b, c, d)
    (quad, $stream:ident, $builder:ident, $constraint:ident) => {
        pub struct $stream<S, A, K, E, KE, F, Sc>
        where
            Sc: solverforge_core::score::Score,
        {
            pub(crate) extractor: E,
            pub(crate) key_extractor: KE,
            pub(crate) filter: F,
            pub(crate) _phantom: std::marker::PhantomData<(S, A, K, Sc)>,
        }

        impl<S, A, K, E, KE, Sc> $stream<S, A, K, E, KE, super::filter::TrueFilter, Sc>
        where
            S: Send + Sync + 'static,
            A: Clone + std::hash::Hash + PartialEq + Send + Sync + 'static,
            K: Eq + std::hash::Hash + Clone + Send + Sync,
            E: Fn(&S) -> &[A] + Send + Sync,
            KE: Fn(&A) -> K + Send + Sync,
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
            E: Fn(&S) -> &[A] + Send + Sync,
            KE: Fn(&A) -> K + Send + Sync,
            F: super::filter::QuadFilter<S, A, A, A, A>,
            Sc: solverforge_core::score::Score + 'static,
        {
            pub fn new_self_join_with_filter(extractor: E, key_extractor: KE, filter: F) -> Self {
                Self { extractor, key_extractor, filter, _phantom: std::marker::PhantomData }
            }

            /// Adds a filter predicate to the stream.
            pub fn filter<P>(
                self,
                predicate: P,
            ) -> $stream<S, A, K, E, KE, super::filter::AndQuadFilter<F, super::filter::FnQuadFilter<impl Fn(&S, &A, &A, &A, &A) -> bool + Send + Sync>>, Sc>
            where
                P: Fn(&A, &A, &A, &A) -> bool + Send + Sync + 'static,
            {
                $stream {
                    extractor: self.extractor,
                    key_extractor: self.key_extractor,
                    filter: super::filter::AndQuadFilter::new(
                        self.filter,
                        super::filter::FnQuadFilter::new(move |_s: &S, a: &A, b: &A, c: &A, d: &A| predicate(a, b, c, d)),
                    ),
                    _phantom: std::marker::PhantomData,
                }
            }

            pub fn penalize(self, weight: Sc) -> $builder<S, A, K, E, KE, F, impl Fn(&A, &A, &A, &A) -> Sc + Send + Sync, Sc>
            where Sc: Copy,
            {
                let is_hard = weight.to_level_numbers().first().map(|&h| h != 0).unwrap_or(false);
                $builder {
                    extractor: self.extractor, key_extractor: self.key_extractor, filter: self.filter,
                    impact_type: solverforge_core::ImpactType::Penalty,
                    weight: move |_: &A, _: &A, _: &A, _: &A| weight,
                    is_hard,
                    _phantom: std::marker::PhantomData,
                }
            }

            pub fn penalize_with<W>(self, weight_fn: W) -> $builder<S, A, K, E, KE, F, W, Sc>
            where W: Fn(&A, &A, &A, &A) -> Sc + Send + Sync,
            {
                $builder {
                    extractor: self.extractor, key_extractor: self.key_extractor, filter: self.filter,
                    impact_type: solverforge_core::ImpactType::Penalty, weight: weight_fn,
                    is_hard: false,
                    _phantom: std::marker::PhantomData,
                }
            }

            pub fn penalize_hard_with<W>(self, weight_fn: W) -> $builder<S, A, K, E, KE, F, W, Sc>
            where W: Fn(&A, &A, &A, &A) -> Sc + Send + Sync,
            {
                $builder {
                    extractor: self.extractor, key_extractor: self.key_extractor, filter: self.filter,
                    impact_type: solverforge_core::ImpactType::Penalty, weight: weight_fn,
                    is_hard: true,
                    _phantom: std::marker::PhantomData,
                }
            }

            pub fn reward(self, weight: Sc) -> $builder<S, A, K, E, KE, F, impl Fn(&A, &A, &A, &A) -> Sc + Send + Sync, Sc>
            where Sc: Copy,
            {
                let is_hard = weight.to_level_numbers().first().map(|&h| h != 0).unwrap_or(false);
                $builder {
                    extractor: self.extractor, key_extractor: self.key_extractor, filter: self.filter,
                    impact_type: solverforge_core::ImpactType::Reward,
                    weight: move |_: &A, _: &A, _: &A, _: &A| weight,
                    is_hard,
                    _phantom: std::marker::PhantomData,
                }
            }

            pub fn reward_with<W>(self, weight_fn: W) -> $builder<S, A, K, E, KE, F, W, Sc>
            where W: Fn(&A, &A, &A, &A) -> Sc + Send + Sync,
            {
                $builder {
                    extractor: self.extractor, key_extractor: self.key_extractor, filter: self.filter,
                    impact_type: solverforge_core::ImpactType::Reward, weight: weight_fn,
                    is_hard: false,
                    _phantom: std::marker::PhantomData,
                }
            }

            pub fn reward_hard_with<W>(self, weight_fn: W) -> $builder<S, A, K, E, KE, F, W, Sc>
            where W: Fn(&A, &A, &A, &A) -> Sc + Send + Sync,
            {
                $builder {
                    extractor: self.extractor, key_extractor: self.key_extractor, filter: self.filter,
                    impact_type: solverforge_core::ImpactType::Reward, weight: weight_fn,
                    is_hard: true,
                    _phantom: std::marker::PhantomData,
                }
            }
        }

        impl<S, A, K, E, KE, F, Sc: solverforge_core::score::Score> std::fmt::Debug for $stream<S, A, K, E, KE, F, Sc> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_struct("QuadConstraintStream").finish()
            }
        }

        pub struct $builder<S, A, K, E, KE, F, W, Sc> where Sc: solverforge_core::score::Score {
            pub(crate) extractor: E,
            pub(crate) key_extractor: KE,
            pub(crate) filter: F,
            pub(crate) impact_type: solverforge_core::ImpactType,
            pub(crate) weight: W,
            pub(crate) is_hard: bool,
            pub(crate) _phantom: std::marker::PhantomData<(S, A, K, Sc)>,
        }

        impl<S, A, K, E, KE, F, W, Sc> $builder<S, A, K, E, KE, F, W, Sc>
        where
            S: Send + Sync + 'static,
            A: Clone + Send + Sync + 'static,
            K: Eq + std::hash::Hash + Clone + Send + Sync,
            E: Fn(&S) -> &[A] + Send + Sync,
            KE: Fn(&A) -> K + Send + Sync,
            F: super::filter::QuadFilter<S, A, A, A, A>,
            W: Fn(&A, &A, &A, &A) -> Sc + Send + Sync,
            Sc: solverforge_core::score::Score + 'static,
        {
            pub fn as_constraint(self, name: &str) -> $constraint<S, A, K, E, KE, impl Fn(&S, &A, &A, &A, &A) -> bool + Send + Sync, W, Sc> {
                let filter = self.filter;
                let combined_filter = move |s: &S, a: &A, b: &A, c: &A, d: &A| filter.test(s, a, b, c, d);
                $constraint::new(
                    solverforge_core::ConstraintRef::new("", name),
                    self.impact_type, self.extractor, self.key_extractor, combined_filter, self.weight,
                    self.is_hard,
                )
            }
        }

        impl<S, A, K, E, KE, F, W, Sc: solverforge_core::score::Score> std::fmt::Debug for $builder<S, A, K, E, KE, F, W, Sc> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_struct("QuadConstraintBuilder").field("impact_type", &self.impact_type).finish()
            }
        }
    };

    // Penta arity (5 params: a, b, c, d, e)
    (penta, $stream:ident, $builder:ident, $constraint:ident) => {
        pub struct $stream<S, A, K, E, KE, F, Sc>
        where
            Sc: solverforge_core::score::Score,
        {
            pub(crate) extractor: E,
            pub(crate) key_extractor: KE,
            pub(crate) filter: F,
            pub(crate) _phantom: std::marker::PhantomData<(S, A, K, Sc)>,
        }

        impl<S, A, K, E, KE, Sc> $stream<S, A, K, E, KE, super::filter::TrueFilter, Sc>
        where
            S: Send + Sync + 'static,
            A: Clone + std::hash::Hash + PartialEq + Send + Sync + 'static,
            K: Eq + std::hash::Hash + Clone + Send + Sync,
            E: Fn(&S) -> &[A] + Send + Sync,
            KE: Fn(&A) -> K + Send + Sync,
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
            E: Fn(&S) -> &[A] + Send + Sync,
            KE: Fn(&A) -> K + Send + Sync,
            F: super::filter::PentaFilter<S, A, A, A, A, A>,
            Sc: solverforge_core::score::Score + 'static,
        {
            pub fn new_self_join_with_filter(extractor: E, key_extractor: KE, filter: F) -> Self {
                Self { extractor, key_extractor, filter, _phantom: std::marker::PhantomData }
            }

            /// Adds a filter predicate to the stream.
            pub fn filter<P>(
                self,
                predicate: P,
            ) -> $stream<S, A, K, E, KE, super::filter::AndPentaFilter<F, super::filter::FnPentaFilter<impl Fn(&S, &A, &A, &A, &A, &A) -> bool + Send + Sync>>, Sc>
            where
                P: Fn(&A, &A, &A, &A, &A) -> bool + Send + Sync + 'static,
            {
                $stream {
                    extractor: self.extractor,
                    key_extractor: self.key_extractor,
                    filter: super::filter::AndPentaFilter::new(
                        self.filter,
                        super::filter::FnPentaFilter::new(move |_s: &S, a: &A, b: &A, c: &A, d: &A, e: &A| predicate(a, b, c, d, e)),
                    ),
                    _phantom: std::marker::PhantomData,
                }
            }

            pub fn penalize(self, weight: Sc) -> $builder<S, A, K, E, KE, F, impl Fn(&A, &A, &A, &A, &A) -> Sc + Send + Sync, Sc>
            where Sc: Copy,
            {
                let is_hard = weight.to_level_numbers().first().map(|&h| h != 0).unwrap_or(false);
                $builder {
                    extractor: self.extractor, key_extractor: self.key_extractor, filter: self.filter,
                    impact_type: solverforge_core::ImpactType::Penalty,
                    weight: move |_: &A, _: &A, _: &A, _: &A, _: &A| weight,
                    is_hard,
                    _phantom: std::marker::PhantomData,
                }
            }

            pub fn penalize_with<W>(self, weight_fn: W) -> $builder<S, A, K, E, KE, F, W, Sc>
            where W: Fn(&A, &A, &A, &A, &A) -> Sc + Send + Sync,
            {
                $builder {
                    extractor: self.extractor, key_extractor: self.key_extractor, filter: self.filter,
                    impact_type: solverforge_core::ImpactType::Penalty, weight: weight_fn,
                    is_hard: false,
                    _phantom: std::marker::PhantomData,
                }
            }

            pub fn penalize_hard_with<W>(self, weight_fn: W) -> $builder<S, A, K, E, KE, F, W, Sc>
            where W: Fn(&A, &A, &A, &A, &A) -> Sc + Send + Sync,
            {
                $builder {
                    extractor: self.extractor, key_extractor: self.key_extractor, filter: self.filter,
                    impact_type: solverforge_core::ImpactType::Penalty, weight: weight_fn,
                    is_hard: true,
                    _phantom: std::marker::PhantomData,
                }
            }

            pub fn reward(self, weight: Sc) -> $builder<S, A, K, E, KE, F, impl Fn(&A, &A, &A, &A, &A) -> Sc + Send + Sync, Sc>
            where Sc: Copy,
            {
                let is_hard = weight.to_level_numbers().first().map(|&h| h != 0).unwrap_or(false);
                $builder {
                    extractor: self.extractor, key_extractor: self.key_extractor, filter: self.filter,
                    impact_type: solverforge_core::ImpactType::Reward,
                    weight: move |_: &A, _: &A, _: &A, _: &A, _: &A| weight,
                    is_hard,
                    _phantom: std::marker::PhantomData,
                }
            }

            pub fn reward_with<W>(self, weight_fn: W) -> $builder<S, A, K, E, KE, F, W, Sc>
            where W: Fn(&A, &A, &A, &A, &A) -> Sc + Send + Sync,
            {
                $builder {
                    extractor: self.extractor, key_extractor: self.key_extractor, filter: self.filter,
                    impact_type: solverforge_core::ImpactType::Reward, weight: weight_fn,
                    is_hard: false,
                    _phantom: std::marker::PhantomData,
                }
            }

            pub fn reward_hard_with<W>(self, weight_fn: W) -> $builder<S, A, K, E, KE, F, W, Sc>
            where W: Fn(&A, &A, &A, &A, &A) -> Sc + Send + Sync,
            {
                $builder {
                    extractor: self.extractor, key_extractor: self.key_extractor, filter: self.filter,
                    impact_type: solverforge_core::ImpactType::Reward, weight: weight_fn,
                    is_hard: true,
                    _phantom: std::marker::PhantomData,
                }
            }
        }

        impl<S, A, K, E, KE, F, Sc: solverforge_core::score::Score> std::fmt::Debug for $stream<S, A, K, E, KE, F, Sc> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_struct("PentaConstraintStream").finish()
            }
        }

        pub struct $builder<S, A, K, E, KE, F, W, Sc> where Sc: solverforge_core::score::Score {
            pub(crate) extractor: E,
            pub(crate) key_extractor: KE,
            pub(crate) filter: F,
            pub(crate) impact_type: solverforge_core::ImpactType,
            pub(crate) weight: W,
            pub(crate) is_hard: bool,
            pub(crate) _phantom: std::marker::PhantomData<(S, A, K, Sc)>,
        }

        impl<S, A, K, E, KE, F, W, Sc> $builder<S, A, K, E, KE, F, W, Sc>
        where
            S: Send + Sync + 'static,
            A: Clone + Send + Sync + 'static,
            K: Eq + std::hash::Hash + Clone + Send + Sync,
            E: Fn(&S) -> &[A] + Send + Sync,
            KE: Fn(&A) -> K + Send + Sync,
            F: super::filter::PentaFilter<S, A, A, A, A, A>,
            W: Fn(&A, &A, &A, &A, &A) -> Sc + Send + Sync,
            Sc: solverforge_core::score::Score + 'static,
        {
            pub fn as_constraint(self, name: &str) -> $constraint<S, A, K, E, KE, impl Fn(&S, &A, &A, &A, &A, &A) -> bool + Send + Sync, W, Sc> {
                let filter = self.filter;
                let combined_filter = move |s: &S, a: &A, b: &A, c: &A, d: &A, e: &A| filter.test(s, a, b, c, d, e);
                $constraint::new(
                    solverforge_core::ConstraintRef::new("", name),
                    self.impact_type, self.extractor, self.key_extractor, combined_filter, self.weight,
                    self.is_hard,
                )
            }
        }

        impl<S, A, K, E, KE, F, W, Sc: solverforge_core::score::Score> std::fmt::Debug for $builder<S, A, K, E, KE, F, W, Sc> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_struct("PentaConstraintBuilder").field("impact_type", &self.impact_type).finish()
            }
        }
    };
}

pub(crate) use impl_arity_stream;
