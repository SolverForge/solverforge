pub(super) fn generate_solvable_solution(
    solution_name: &Ident,
    constraints_path: &Option<String>,
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

            impl ::solverforge::Analyzable for #solution_name {
                fn analyze(&self) -> ::solverforge::ScoreAnalysis<<Self as ::solverforge::__internal::PlanningSolution>::Score> {
                    use ::solverforge::__internal::{
                        Director, ScoreDirector,
                    };

                    let constraints = #constraints_fn();
                    let mut director = ScoreDirector::with_descriptor(
                        self.clone(),
                        constraints,
                        Self::descriptor(),
                        Self::entity_count,
                    );

                    let score = director.calculate_score();
                    let constraint_scores = director.constraint_match_totals();

                    let constraints = constraint_scores
                        .into_iter()
                        .map(|(name, weight, contribution, match_count)| {
                            ::solverforge::ConstraintAnalysis {
                                name,
                                weight,
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
