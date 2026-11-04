pub(super) fn generate_solvable_solution(
    solution_name: &Ident,
    providers: &[super::config::ConstraintProvider],
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

    let solvable_impl = providers.first().map(|default_provider| {
        let constraints_fn = &default_provider.path;
        // Analysis of a weighted solution type always reports bound weights.
        let director_ctor = if has_constraint_weights {
            quote! { ScoreDirector::with_descriptor_weighted }
        } else {
            quote! { ScoreDirector::with_descriptor }
        };
        let analyze_fns = providers.iter().enumerate().map(|(index, provider)| {
            let analyze_fn = constraint_provider_analyze_ident(index);
            let provider_fn = &provider.path;
            quote! {
                #[doc(hidden)]
                #[allow(dead_code)]
                pub fn #analyze_fn(
                    solution: &Self,
                ) -> ::solverforge::ScoreAnalysis<<Self as ::solverforge::__internal::PlanningSolution>::Score> {
                    use ::solverforge::__internal::{
                        Director, ScoreDirector,
                    };

                    let constraints = #provider_fn();
                    let mut director = #director_ctor(
                        solution.clone(),
                        constraints,
                        Self::descriptor(),
                        Self::entity_count,
                    );

                    if let ::core::option::Option::Some(weights) =
                        ::solverforge::__internal::PlanningSolution::constraint_weights(solution)
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
        });
        let default_analyze = constraint_provider_analyze_ident(0);

        quote! {
            impl ::solverforge::Solvable for #solution_name {
                fn solve(
                    self,
                    runtime: ::solverforge::SolverRuntime<Self>,
                    qualified_candidate_trace_provenance: ::core::option::Option<
                        ::solverforge::QualifiedCandidateTraceRunProvenance,
                    >,
                ) -> ::solverforge::RuntimeBuildResult<()> {
                    #solution_name::solve_internal(
                        self,
                        runtime,
                        qualified_candidate_trace_provenance,
                    )
                    .map(|_| ())
                }
            }

            impl #solution_name {
                /// Checks constraint weights against the constraints of this
                /// model's `constraints` function without solving.
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

                #(#analyze_fns)*
            }

            impl ::solverforge::Analyzable for #solution_name {
                fn analyze(&self) -> ::solverforge::ScoreAnalysis<<Self as ::solverforge::__internal::PlanningSolution>::Score> {
                    Self::#default_analyze(self)
                }
            }
        }
    });

    quote! {
        #solvable_solution_impl
        #solvable_impl
    }
}

/// The generated analysis function of the solution's `index`-th constraint
/// provider (`0` is the `constraints` function).
fn constraint_provider_analyze_ident(index: usize) -> Ident {
    format_ident!("__solverforge_analyze_constraint_provider_{}", index)
}
