pub(super) fn generate_runtime_solve_internal(
    constraints_path: &Option<syn::Path>,
    config_path: &Option<syn::Path>,
    solver_toml_path: &Option<String>,
    has_constraint_weights: bool,
) -> TokenStream {
    let Some(constraints_fn) = constraints_path.as_ref() else {
        return TokenStream::new();
    };
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
    let run = |runtime: TokenStream, provenance: TokenStream| {
        quote! {
            #run_solver(
                self,
                (|_: &Self| #constraints_fn()) as fn(&Self) -> _,
                Self::descriptor(),
                Self::entity_count,
                #runtime,
                config,
                Self::__solverforge_default_time_limit_secs(),
                Self::__solverforge_log_scale,
                #provenance,
                Self::__solverforge_search_declaration,
            )
        }
    };
    let retained_run = run(quote! { runtime }, quote! { qualified_candidate_trace_provenance });
    let detached_run = run(
        quote! { ::solverforge::SolverRuntime::detached() },
        quote! { ::core::option::Option::None },
    );
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
            #detached_run
        }
    }
}
