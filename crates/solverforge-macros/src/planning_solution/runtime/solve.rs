pub(super) fn generate_runtime_solve_internal(
    constraints_path: &Option<String>,
    config_path: &Option<String>,
    solver_toml_path: &Option<String>,
) -> TokenStream {
    let Some(path) = constraints_path.as_ref() else {
        return TokenStream::new();
    };

    let constraints_fn: syn::Path =
        syn::parse_str(path).expect("constraints path must be a valid Rust path");
    let base_config_expr = if let Some(solver_toml_path) = solver_toml_path.as_ref() {
        quote! {{
            static CONFIG: ::std::sync::OnceLock<::solverforge::SolverConfig> =
                ::std::sync::OnceLock::new();
            CONFIG
                .get_or_init(|| {
                    ::solverforge::SolverConfig::from_toml_str(include_str!(#solver_toml_path))
                        .expect("embedded solver.toml must be valid")
                })
                .clone()
        }}
    } else {
        quote! { ::solverforge::__internal::load_solver_config() }
    };
    let config_expr = if let Some(config_path) = config_path.as_ref() {
        let config_fn: syn::Path =
            syn::parse_str(config_path).expect("config path must be a valid Rust path");
        quote! {
            let base_config = #base_config_expr;
            let config = #config_fn(&self, base_config);
        }
    } else {
        quote! {
            let config = #base_config_expr;
        }
    };
    let solve_expr = quote! {
        #config_expr
        ::solverforge::__internal::try_run_solver_with_config_and_search(
            self,
            #constraints_fn(),
            Self::descriptor(),
            Self::entity_count,
            runtime,
            config,
            Self::__solverforge_default_time_limit_secs(),
            Self::__solverforge_log_scale,
            qualified_candidate_trace_provenance,
            Self::__solverforge_search_declaration,
        )
    };
    quote! {
        fn solve_internal(
            self,
            runtime: ::solverforge::SolverRuntime<Self>,
            qualified_candidate_trace_provenance: ::core::option::Option<
                ::solverforge::QualifiedCandidateTraceRunProvenance,
            >,
        ) -> Self {
            ::solverforge::__internal::init_console();

            match { #solve_expr } {
                ::core::result::Result::Ok(solution) => solution,
                ::core::result::Result::Err(error) => panic!("{error}"),
            }
        }
    }
}
