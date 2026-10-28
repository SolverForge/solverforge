pub(super) fn generate_solvable_solution(
    solution_name: &Ident,
    constraints_path: &Option<String>,
    has_constraint_weights: bool,
) -> TokenStream {
    let solvable_solution_impl = quote! {
        impl ::solverforge::__internal::SolvableSolution for #solution_name {
            fn descriptor() -> ::solverforge::__internal::SolutionDescriptor {
                #solution_name::descriptor()
            }

            fn entity_count(solution: &Self, descriptor_index: usize) -> usize {
                #solution_name::entity_count(solution, descriptor_index)
            }
        }
    };

    let solvable_impl = constraints_path.as_ref().map(|path| {
        let constraints_fn: syn::Path =
            syn::parse_str(path).expect("constraints path must be a valid Rust path");
        // Analysis of a weighted solution type always reports bound weights.
        let director_ctor = if has_constraint_weights {
            quote! { ScoreDirector::with_descriptor_weighted }
        } else {
            quote! { ScoreDirector::with_descriptor }
        };

        quote! {
            impl ::solverforge::Solvable for #solution_name {
                fn solve(
                    self,
                    runtime: ::solverforge::SolverRuntime<Self>,
                    qualified_candidate_trace_provenance: ::core::option::Option<
                        ::solverforge::QualifiedCandidateTraceRunProvenance,
                    >,
                ) {
                    let _ = #solution_name::solve_internal(
                        self,
                        runtime,
                        qualified_candidate_trace_provenance,
                    );
                }
            }

            impl #solution_name {
                /// Checks constraint weights against this model's constraints without solving.
                ///
                /// Fails on a name that matches no constraint (with close-match
                /// suggestions) and on a multiplier above
                /// `MAX_CONSTRAINT_WEIGHT_MULTIPLIER`, the same checks a solve
                /// runs before it starts.
                #[allow(dead_code)]
                pub fn validate_constraint_weights(
                    weights: &::solverforge::ConstraintWeights,
                ) -> ::core::result::Result<(), ::solverforge::ConstraintWeightError> {
                    let constraints = #constraints_fn();
                    let metadata =
                        ::solverforge::__internal::ConstraintSet::constraint_metadata(&constraints);
                    weights.validate(metadata.iter().map(|entry| entry.constraint_ref))
                }
            }

            impl ::solverforge::Analyzable for #solution_name {
                fn analyze(&self) -> ::solverforge::ScoreAnalysis<<Self as ::solverforge::__internal::PlanningSolution>::Score> {
                    use ::solverforge::__internal::{
                        Director, ScoreDirector,
                    };

                    let constraints = #constraints_fn();
                    let mut director = #director_ctor(
                        self.clone(),
                        constraints,
                        Self::descriptor(),
                        Self::entity_count,
                    );

                    if let ::core::option::Option::Some(weights) =
                        ::solverforge::__internal::PlanningSolution::constraint_weights(self)
                    {
                        let metadata = director.constraint_metadata();
                        if let ::core::result::Result::Err(error) =
                            weights.validate(metadata.iter().map(|entry| entry.constraint_ref))
                        {
                            panic!("{error}");
                        }
                    }

                    let score = director.calculate_score();
                    let constraint_scores = director.constraint_match_totals();

                    let constraints = constraint_scores
                        .into_iter()
                        .map(|(name, weight_multiplier, contribution, match_count)| {
                            ::solverforge::ConstraintAnalysis {
                                name,
                                weight_multiplier,
                                score: contribution,
                                match_count,
                            }
                        })
                        .collect();

                    ::solverforge::ScoreAnalysis { score, constraints }
                }
            }
        }
    });

    quote! {
        #solvable_solution_impl
        #solvable_impl
    }
}
