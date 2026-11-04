pub(super) fn generate_runtime_solve_internal(
    providers: &[super::config::ConstraintProvider],
    config_path: &Option<syn::Path>,
    solver_toml_path: &Option<String>,
    has_constraint_weights: bool,
) -> TokenStream {
    if providers.is_empty() {
        return TokenStream::new();
    }
    /* The base configuration is the embedded `solver_toml` file or the
    defaults; nothing is read from the working directory. A file that does not
    parse fails the solve through the returned error. */
    let base_config_expr = if let Some(solver_toml_path) = solver_toml_path.as_ref() {
        quote! {{
            static CONFIG: ::std::sync::OnceLock<
                ::core::result::Result<::solverforge::SolverConfig, ::solverforge::RuntimeBuildError>,
            > = ::std::sync::OnceLock::new();
            CONFIG
                .get_or_init(|| {
                    ::solverforge::SolverConfig::from_toml_str(include_str!(#solver_toml_path))
                        .map_err(|error| ::solverforge::RuntimeBuildError::Configuration {
                            message: error.to_string(),
                        })
                })
                .clone()?
        }}
    } else {
        quote! { ::solverforge::SolverConfig::default() }
    };
    let config_expr = if let Some(config_fn) = config_path.as_ref() {
        quote! {
            let base_config = #base_config_expr;
            let config = #config_fn(&self, base_config);
        }
    } else {
        quote! {
            let config = #base_config_expr;
        }
    };
    /* Only a solution type with a `#[constraint_weights]` field names the
    weighted entrypoint, so every other model monomorphizes exactly one
    (unweighted) solver. */
    let run_solver = if has_constraint_weights {
        quote! { ::solverforge::__internal::try_run_weighted_solver_with_config_and_search }
    } else {
        quote! { ::solverforge::__internal::try_run_solver_with_config_and_search }
    };
    /* `score_director.constraint_provider` selects the constraint set; each
    provider is its own monomorphized solve, and binds its own analysis for
    the job's snapshots unless `constraint_match_enabled` is false. */
    let declared: Vec<String> = providers.iter().map(|provider| provider.name.clone()).collect();
    let dispatch = |provenance: TokenStream| {
        let arms = providers.iter().enumerate().map(|(index, provider)| {
            let constraints_fn = &provider.path;
            let name = &provider.name;
            let analyze_fn = constraint_provider_analyze_ident(index);
            let pattern = if index == 0 {
                quote! { ::core::option::Option::None | ::core::option::Option::Some(#name) }
            } else {
                quote! { ::core::option::Option::Some(#name) }
            };
            quote! {
                #pattern => {
                    runtime.bind_snapshot_analysis(
                        snapshot_analysis_enabled.then_some(
                            Self::#analyze_fn
                                as fn(&Self) -> ::solverforge::ScoreAnalysis<
                                    <Self as ::solverforge::__internal::PlanningSolution>::Score,
                                >,
                        ),
                    );
                    #run_solver(
                        self,
                        (|_: &Self| #constraints_fn()) as fn(&Self) -> _,
                        Self::descriptor(),
                        Self::entity_count,
                        runtime,
                        config,
                        Self::__solverforge_log_scale,
                        #provenance,
                        Self::__solverforge_search_declaration,
                    )
                }
            }
        });
        quote! {
            let snapshot_analysis_enabled = config.constraint_match_enabled();
            let requested_provider = config
                .constraint_provider()
                .map(::std::string::ToString::to_string);
            match requested_provider.as_deref() {
                #(#arms)*
                ::core::option::Option::Some(requested) => {
                    ::core::result::Result::Err(
                        ::solverforge::RuntimeBuildError::UnknownConstraintProvider {
                            requested: ::std::string::ToString::to_string(requested),
                            declared: ::std::vec![
                                #(::std::string::ToString::to_string(#declared)),*
                            ],
                        },
                    )
                }
            }
        }
    };
    let retained_run = dispatch(quote! { qualified_candidate_trace_provenance });
    let detached_run = dispatch(quote! { ::core::option::Option::None });
    quote! {
        fn solve_internal(
            self,
            runtime: ::solverforge::SolverRuntime<Self>,
            qualified_candidate_trace_provenance: ::core::option::Option<
                ::solverforge::QualifiedCandidateTraceRunProvenance,
            >,
        ) -> ::solverforge::RuntimeBuildResult<Self> {
            ::solverforge::__internal::init_console();
            #config_expr
            #retained_run
        }

        /// Solves this solution on the calling thread with exactly `config`
        /// and returns the best solution found.
        ///
        /// Neither the embedded `solver_toml` nor the solution's `config`
        /// callback is applied. Fails with the same errors as a retained
        /// solve; nothing is published to a [`SolverManager`](::solverforge::SolverManager).
        pub fn solve_with_config(
            self,
            config: ::solverforge::SolverConfig,
        ) -> ::solverforge::RuntimeBuildResult<Self> {
            ::solverforge::__internal::init_console();
            let runtime = ::solverforge::SolverRuntime::detached();
            #detached_run
        }
    }
}
