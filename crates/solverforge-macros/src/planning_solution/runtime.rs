use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::Ident;

use crate::attr_parse::has_attribute;
use crate::list_registry::lookup_list_entity_metadata;

use super::standard_runtime::generate_standard_runtime_support;
use super::type_helpers::extract_collection_inner_type;

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
    let solve_expr = if config_path.is_some() || solver_toml_path.is_some() {
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
        quote! {
            #config_expr
            ::solverforge::__internal::run_solver_with_config(
                self,
                #constraints_fn,
                Self::descriptor,
                Self::entity_count,
                runtime,
                config,
                Self::__solverforge_default_time_limit_secs(),
                Self::__solverforge_is_trivial,
                Self::__solverforge_log_scale,
                Self::__solverforge_build_phases,
            )
        }
    } else {
        quote! {
            ::solverforge::__internal::run_solver(
                self,
                #constraints_fn,
                Self::descriptor,
                Self::entity_count,
                runtime,
                Self::__solverforge_default_time_limit_secs(),
                Self::__solverforge_is_trivial,
                Self::__solverforge_log_scale,
                Self::__solverforge_build_phases,
            )
        }
    };
    quote! {
        fn solve_internal(
            self,
            runtime: ::solverforge::SolverRuntime<Self>,
        ) -> Self {
            ::solverforge::__internal::init_console();

            #solve_expr
        }
    }
}

pub(super) fn generate_runtime_phase_support(
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
    constraints_path: &Option<String>,
    solution_name: &Ident,
) -> TokenStream {
    if constraints_path.is_none() {
        return TokenStream::new();
    }

    let list_owners: Vec<_> = fields
        .iter()
        .filter(|f| has_attribute(&f.attrs, "planning_entity_collection"))
        .enumerate()
        .filter_map(|(idx, field)| {
            let field_type = extract_collection_inner_type(&field.ty)?;
            let syn::Type::Path(type_path) = field_type else {
                return None;
            };
            let type_name = type_path.path.segments.last()?.ident.to_string();
            lookup_list_entity_metadata(&type_name).map(|_| (idx, field_type))
        })
        .collect();
    let standard_support = generate_standard_runtime_support(fields, solution_name);
    let standard_setup = standard_support.setup.clone();

    if !list_owners.is_empty() {
        let cross_enum_ident = format_ident!("__{}CrossDistanceMeter", solution_name);
        let intra_enum_ident = format_ident!("__{}IntraDistanceMeter", solution_name);

        let cross_variants: Vec<_> = list_owners
            .iter()
            .map(|(idx, field_type)| {
                let variant = format_ident!("Entity{idx}");
                quote! {
                    #variant(
                        <#field_type as ::solverforge::__internal::ListVariableEntity<#solution_name>>::CrossDistanceMeter
                    )
                }
            })
            .collect();
        let intra_variants: Vec<_> = list_owners
            .iter()
            .map(|(idx, field_type)| {
                let variant = format_ident!("Entity{idx}");
                quote! {
                    #variant(
                        <#field_type as ::solverforge::__internal::ListVariableEntity<#solution_name>>::IntraDistanceMeter
                    )
                }
            })
            .collect();
        let cross_match_arms: Vec<_> = list_owners
            .iter()
            .map(|(idx, _)| {
                let variant = format_ident!("Entity{idx}");
                quote! {
                    Self::#variant(meter) => meter.distance(solution, src_entity, src_pos, dst_entity, dst_pos),
                }
            })
            .collect();
        let intra_match_arms: Vec<_> = list_owners
            .iter()
            .map(|(idx, _)| {
                let variant = format_ident!("Entity{idx}");
                quote! {
                    Self::#variant(meter) => meter.distance(solution, src_entity, src_pos, dst_entity, dst_pos),
                }
            })
            .collect();
        let list_runtime_branches: Vec<_> = list_owners
            .iter()
            .map(|(idx, field_type)| {
                let variant = format_ident!("Entity{idx}");
                let descriptor_index_lit =
                    syn::LitInt::new(&idx.to_string(), proc_macro2::Span::call_site());
                let list_trait = quote! {
                    <#field_type as ::solverforge::__internal::ListVariableEntity<#solution_name>>
                };
                quote! {
                    if #list_trait::HAS_LIST_VARIABLE {
                        let metadata = #list_trait::list_metadata();
                        __solverforge_variables.push(
                            ::solverforge::__internal::VariableContext::List(
                                ::solverforge::__internal::ListVariableContext::new(
                                    stringify!(#field_type),
                                    Self::list_len_static,
                                    Self::list_remove,
                                    Self::list_insert,
                                    Self::list_get,
                                    Self::list_set,
                                    Self::list_reverse,
                                    Self::sublist_remove,
                                    Self::sublist_insert,
                                    Self::ruin_remove,
                                    Self::ruin_insert,
                                    Self::n_entities,
                                    #cross_enum_ident::#variant(metadata.cross_distance_meter.clone()),
                                    #intra_enum_ident::#variant(metadata.intra_distance_meter.clone()),
                                    #list_trait::LIST_VARIABLE_NAME,
                                    #descriptor_index_lit,
                                )
                            )
                        );
                        let model = ::solverforge::__internal::ModelContext::new(__solverforge_variables);
                        let construction = ::solverforge::__internal::ConstructionArgs {
                            element_count: Self::element_count,
                            assigned_elements: Self::assigned_elements,
                            entity_count: Self::n_entities,
                            list_len: Self::list_len_static,
                            list_insert: Self::list_insert,
                            list_remove: Self::list_remove_for_construction,
                            index_to_element: Self::index_to_element_static,
                            descriptor_index: #descriptor_index_lit,
                            entity_type_name: stringify!(#field_type),
                            variable_name: #list_trait::LIST_VARIABLE_NAME,
                            depot_fn: metadata.cw_depot_fn,
                            distance_fn: metadata.cw_distance_fn,
                            element_load_fn: metadata.cw_element_load_fn,
                            capacity_fn: metadata.cw_capacity_fn,
                            assign_route_fn: metadata.cw_assign_route_fn,
                            merge_feasible_fn: metadata.merge_feasible_fn,
                            k_opt_get_route: metadata.k_opt_get_route,
                            k_opt_set_route: metadata.k_opt_set_route,
                            k_opt_depot_fn: metadata.k_opt_depot_fn,
                            k_opt_distance_fn: metadata.k_opt_distance_fn,
                            k_opt_feasible_fn: metadata.k_opt_feasible_fn,
                        };
                        return ::solverforge::__internal::build_phases(
                            config,
                            &descriptor,
                            &model,
                            ::core::option::Option::Some(construction),
                        );
                    }
                }
            })
            .collect();

        return quote! {
            #[derive(Clone, Debug)]
            enum #cross_enum_ident {
                #(#cross_variants),*
            }

            impl ::solverforge::CrossEntityDistanceMeter<#solution_name> for #cross_enum_ident {
                fn distance(
                    &self,
                    solution: &#solution_name,
                    src_entity: usize,
                    src_pos: usize,
                    dst_entity: usize,
                    dst_pos: usize,
                ) -> f64 {
                    match self {
                        #(#cross_match_arms)*
                    }
                }
            }

            #[derive(Clone, Debug)]
            enum #intra_enum_ident {
                #(#intra_variants),*
            }

            impl ::solverforge::CrossEntityDistanceMeter<#solution_name> for #intra_enum_ident {
                fn distance(
                    &self,
                    solution: &#solution_name,
                    src_entity: usize,
                    src_pos: usize,
                    dst_entity: usize,
                    dst_pos: usize,
                ) -> f64 {
                    match self {
                        #(#intra_match_arms)*
                    }
                }
            }

            impl #solution_name {
                const fn __solverforge_default_time_limit_secs() -> u64 {
                    60
                }

                fn __solverforge_is_trivial(solution: &Self) -> bool {
                    let descriptor = Self::descriptor();
                    let has_standard = ::solverforge::__internal::descriptor_has_bindings(&descriptor);
                    let has_list = Self::n_entities(solution) > 0 && Self::element_count(solution) > 0;
                    (!has_standard && !has_list)
                        || (Self::n_entities(solution) == 0)
                        || (has_list && Self::element_count(solution) == 0)
                }

                fn __solverforge_log_scale(solution: &Self) {
                    let descriptor = Self::descriptor();
                    let has_standard = ::solverforge::__internal::descriptor_has_bindings(&descriptor);
                    ::solverforge::__internal::log_solve_start(
                        Self::n_entities(solution),
                        ::core::option::Option::Some(Self::element_count(solution)),
                        ::core::option::Option::Some(has_standard),
                        ::core::option::Option::None,
                    );
                }

                fn __solverforge_build_phases(
                    config: &::solverforge::__internal::SolverConfig,
                ) -> ::solverforge::__internal::PhaseSequence<
                    ::solverforge::__internal::RuntimePhase<
                        ::solverforge::__internal::Construction<#solution_name, usize>,
                        ::solverforge::__internal::LocalSearch<
                            #solution_name,
                            usize,
                            #cross_enum_ident,
                            #intra_enum_ident
                        >,
                        ::solverforge::__internal::Vnd<
                            #solution_name,
                            usize,
                            #cross_enum_ident,
                            #intra_enum_ident
                        >
                    >
                > {
                    let descriptor = Self::descriptor();
                    #standard_setup
                    #(#list_runtime_branches)*
                    let model = ::solverforge::__internal::ModelContext::<
                        #solution_name,
                        usize,
                        #cross_enum_ident,
                        #intra_enum_ident
                    >::new(__solverforge_variables);
                    ::solverforge::__internal::build_phases(
                        config,
                        &descriptor,
                        &model,
                        ::core::option::Option::None,
                    )
                }
            }
        };
    }

    quote! {
        impl #solution_name {
            const fn __solverforge_default_time_limit_secs() -> u64 {
                30
            }

            fn __solverforge_is_trivial(solution: &Self) -> bool {
                let descriptor = Self::descriptor();
                !::solverforge::__internal::descriptor_has_bindings(&descriptor)
                    || descriptor
                        .total_entity_count(solution as &dyn ::std::any::Any)
                        .unwrap_or(0)
                        == 0
            }

            fn __solverforge_log_scale(solution: &Self) {
                let descriptor = Self::descriptor();
                ::solverforge::__internal::log_solve_start(
                    descriptor
                        .total_entity_count(solution as &dyn ::std::any::Any)
                        .unwrap_or(0),
                    ::core::option::Option::None,
                    ::core::option::Option::None,
                    ::core::option::Option::Some(
                        descriptor.genuine_variable_descriptors().len(),
                    ),
                );
            }

            fn __solverforge_build_phases(
                config: &::solverforge::__internal::SolverConfig,
            ) -> ::solverforge::__internal::PhaseSequence<
                ::solverforge::__internal::RuntimePhase<
                    ::solverforge::__internal::Construction<#solution_name, usize>,
                    ::solverforge::__internal::LocalSearch<
                        #solution_name,
                        usize,
                        ::solverforge::__internal::DefaultCrossEntityDistanceMeter,
                        ::solverforge::__internal::DefaultCrossEntityDistanceMeter
                    >,
                    ::solverforge::__internal::Vnd<
                        #solution_name,
                        usize,
                        ::solverforge::__internal::DefaultCrossEntityDistanceMeter,
                        ::solverforge::__internal::DefaultCrossEntityDistanceMeter
                    >
                >
            > {
                let descriptor = Self::descriptor();
                #standard_setup
                let model = ::solverforge::__internal::ModelContext::<
                    #solution_name,
                    usize,
                    ::solverforge::__internal::DefaultCrossEntityDistanceMeter,
                    ::solverforge::__internal::DefaultCrossEntityDistanceMeter
                >::new(__solverforge_variables);
                ::solverforge::__internal::build_phases(
                    config,
                    &descriptor,
                    &model,
                    ::core::option::Option::None,
                )
            }
        }
    }
}

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
                ) {
                    let _ = #solution_name::solve_internal(self, runtime);
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
