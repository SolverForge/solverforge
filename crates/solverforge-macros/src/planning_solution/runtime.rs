use std::collections::BTreeSet;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::Ident;

use super::type_helpers::extract_collection_inner_type;
use crate::attr_parse::has_attribute;
use crate::standard_registry::lookup_standard_entity_metadata;

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

fn generate_scalar_runtime_setup(
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
    solution_name: &Ident,
) -> TokenStream {
    let entity_fields: Vec<_> = fields
        .iter()
        .filter(|f| has_attribute(&f.attrs, "planning_entity_collection"))
        .enumerate()
        .filter_map(|(idx, field)| {
            let field_name = field.ident.as_ref()?;
            let field_type = extract_collection_inner_type(&field.ty)?;
            let syn::Type::Path(type_path) = field_type else {
                return None;
            };
            let type_name = type_path.path.segments.last()?.ident.to_string();
            let metadata = lookup_standard_entity_metadata(&type_name)?;
            if metadata.variables.is_empty() {
                return None;
            }
            Some((idx, field_name, field_type, type_name, metadata))
        })
        .collect();

    let mut provider_fields = BTreeSet::new();
    for (_, _, _, _, metadata) in &entity_fields {
        for variable in &metadata.variables {
            if variable.provider_is_entity_field {
                continue;
            }
            if let Some(provider) = &variable.value_range_provider {
                provider_fields.insert(provider.clone());
            }
        }
    }

    let provider_count_helpers: Vec<_> = provider_fields
        .into_iter()
        .map(|provider_field_name| {
            let provider_ident = format_ident!("{provider_field_name}");
            let count_fn_ident =
                format_ident!("__solverforge_standard_count_{}", provider_field_name);
            quote! {
                fn #count_fn_ident(solution: &#solution_name) -> usize {
                    solution.#provider_ident.len()
                }
            }
        })
        .collect();

    let entity_count_helpers: Vec<_> = entity_fields
        .iter()
        .map(|(_, field_name, _, _, _)| {
            let count_fn_ident = format_ident!("__solverforge_standard_count_{}", field_name);
            quote! {
                fn #count_fn_ident(solution: &#solution_name) -> usize {
                    solution.#field_name.len()
                }
            }
        })
        .collect();

    let scalar_context_pushes: Vec<_> = entity_fields
        .iter()
        .flat_map(
            |(descriptor_index, field_name, field_type, type_name, metadata)| {
                metadata.variables.iter().map(move |variable| {
                let variable_name = &variable.field_name;
                let allows_unassigned = variable.allows_unassigned;
                let getter_ident = format_ident!(
                    "__solverforge_standard_get_{}_{}",
                    field_name,
                    variable.field_name
                );
                let setter_ident = format_ident!(
                    "__solverforge_standard_set_{}_{}",
                    field_name,
                    variable.field_name
                );
                let entity_count_fn_ident =
                    format_ident!("__solverforge_standard_count_{}", field_name);
                let typed_getter_ident =
                    format_ident!("__solverforge_get_{}_typed", variable.field_name);
                let typed_setter_ident =
                    format_ident!("__solverforge_set_{}_typed", variable.field_name);
                let maybe_slice_helper = if variable.provider_is_entity_field {
                    let slice_ident = format_ident!(
                        "__solverforge_standard_values_{}_{}",
                        field_name,
                        variable.field_name
                    );
                    let typed_slice_ident =
                        format_ident!("__solverforge_values_for_{}_typed", variable.field_name);
                    quote! {
                        fn #slice_ident(
                            solution: &#solution_name,
                            entity_index: usize,
                        ) -> &[usize] {
                            <#field_type>::#typed_slice_ident(&solution.#field_name[entity_index])
                        }
                    }
                } else {
                    TokenStream::new()
                };

                let value_source = if variable.provider_is_entity_field {
                    let slice_ident = format_ident!(
                        "__solverforge_standard_values_{}_{}",
                        field_name,
                        variable.field_name
                    );
                    quote! {
                        ::solverforge::__internal::ValueSource::EntitySlice {
                            values_for_entity: #slice_ident,
                        }
                    }
                } else if let Some((from, to)) = variable.countable_range {
                    let from_usize = usize::try_from(from).expect(
                        "countable_range start must be non-negative for canonical standard solving",
                    );
                    let to_usize = usize::try_from(to).expect(
                        "countable_range end must be non-negative for canonical standard solving",
                    );
                    quote! {
                        ::solverforge::__internal::ValueSource::CountableRange {
                            from: #from_usize,
                            to: #to_usize,
                        }
                    }
                } else if let Some(provider_field_name) = &variable.value_range_provider {
                    let count_fn_ident =
                        format_ident!("__solverforge_standard_count_{}", provider_field_name);
                    quote! {
                        ::solverforge::__internal::ValueSource::SolutionCount {
                            count_fn: #count_fn_ident,
                        }
                    }
                } else {
                    quote! { ::solverforge::__internal::ValueSource::Empty }
                };

                quote! {
                    fn #getter_ident(
                        solution: &#solution_name,
                        entity_index: usize,
                    ) -> ::core::option::Option<usize> {
                        <#field_type>::#typed_getter_ident(&solution.#field_name[entity_index])
                    }

                    fn #setter_ident(
                        solution: &mut #solution_name,
                        entity_index: usize,
                        value: ::core::option::Option<usize>,
                    ) {
                        <#field_type>::#typed_setter_ident(
                            &mut solution.#field_name[entity_index],
                            value,
                        );
                    }

                    #maybe_slice_helper

                    __solverforge_variables.push(
                        ::solverforge::__internal::VariableContext::Scalar(
                            ::solverforge::__internal::ScalarVariableContext::new(
                                #descriptor_index,
                                #type_name,
                                #entity_count_fn_ident,
                                #variable_name,
                                #getter_ident,
                                #setter_ident,
                                #value_source,
                                #allows_unassigned,
                            )
                        )
                    );
                }
            })
            },
        )
        .collect();

    quote! {
        let mut __solverforge_variables = ::std::vec::Vec::new();
        #(#provider_count_helpers)*
        #(#entity_count_helpers)*
        #(#scalar_context_pushes)*
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
            let field_ident = field.ident.as_ref()?;
            let field_type = extract_collection_inner_type(&field.ty)?;
            Some((idx, field_ident, field_type))
        })
        .collect();
    let standard_setup = generate_scalar_runtime_setup(fields, solution_name);

    if !list_owners.is_empty() {
        let cross_enum_ident = format_ident!("__{}CrossDistanceMeter", solution_name);
        let intra_enum_ident = format_ident!("__{}IntraDistanceMeter", solution_name);
        let has_list_variable_terms: Vec<_> = list_owners
            .iter()
            .map(|(_, _, field_type)| {
                let list_trait =
                    quote! { <#field_type as ::solverforge::__internal::ListVariableEntity<#solution_name>> };
                quote! { #list_trait::HAS_LIST_VARIABLE }
            })
            .collect();

        let cross_variants: Vec<_> = list_owners
            .iter()
            .map(|(idx, _, field_type)| {
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
            .map(|(idx, _, field_type)| {
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
            .map(|(idx, _, _)| {
                let variant = format_ident!("Entity{idx}");
                quote! {
                    Self::#variant(meter) => meter.distance(solution, src_entity, src_pos, dst_entity, dst_pos),
                }
            })
            .collect();
        let intra_match_arms: Vec<_> = list_owners
            .iter()
            .map(|(idx, _, _)| {
                let variant = format_ident!("Entity{idx}");
                quote! {
                    Self::#variant(meter) => meter.distance(solution, src_entity, src_pos, dst_entity, dst_pos),
                }
            })
            .collect();
        let list_runtime_setup: Vec<_> = list_owners
            .iter()
            .map(|(idx, field_ident, field_type)| {
                let field_name = field_ident.to_string();
                let variant = format_ident!("Entity{idx}");
                let descriptor_index_lit =
                    syn::LitInt::new(&idx.to_string(), proc_macro2::Span::call_site());
                let list_trait = quote! {
                    <#field_type as ::solverforge::__internal::ListVariableEntity<#solution_name>>
                };
                let list_len_ident = format_ident!("__solverforge_list_len_{}", field_name);
                let list_remove_ident = format_ident!("__solverforge_list_remove_{}", field_name);
                let list_insert_ident = format_ident!("__solverforge_list_insert_{}", field_name);
                let list_get_ident = format_ident!("__solverforge_list_get_{}", field_name);
                let list_set_ident = format_ident!("__solverforge_list_set_{}", field_name);
                let list_reverse_ident =
                    format_ident!("__solverforge_list_reverse_{}", field_name);
                let sublist_remove_ident =
                    format_ident!("__solverforge_sublist_remove_{}", field_name);
                let sublist_insert_ident =
                    format_ident!("__solverforge_sublist_insert_{}", field_name);
                let ruin_remove_ident = format_ident!("__solverforge_ruin_remove_{}", field_name);
                let ruin_insert_ident = format_ident!("__solverforge_ruin_insert_{}", field_name);
                let n_entities_ident = format_ident!("__solverforge_n_entities_{}", field_name);
                let element_count_ident =
                    format_ident!("__solverforge_element_count_{}", field_name);
                let assigned_elements_ident =
                    format_ident!("__solverforge_assigned_elements_{}", field_name);
                let list_remove_for_construction_ident = format_ident!(
                    "__solverforge_list_remove_for_construction_{}",
                    field_name
                );
                let index_to_element_ident =
                    format_ident!("__solverforge_index_to_element_{}", field_name);
                quote! {
                    if #list_trait::HAS_LIST_VARIABLE {
                        let metadata = #list_trait::list_metadata();
                        __solverforge_variables.push(
                            ::solverforge::__internal::VariableContext::List(
                                ::solverforge::__internal::ListVariableContext::new(
                                    stringify!(#field_type),
                                    Self::#list_len_ident,
                                    Self::#list_remove_ident,
                                    Self::#list_insert_ident,
                                    Self::#list_get_ident,
                                    Self::#list_set_ident,
                                    Self::#list_reverse_ident,
                                    Self::#sublist_remove_ident,
                                    Self::#sublist_insert_ident,
                                    Self::#ruin_remove_ident,
                                    Self::#ruin_insert_ident,
                                    Self::#n_entities_ident,
                                    #cross_enum_ident::#variant(metadata.cross_distance_meter.clone()),
                                    #intra_enum_ident::#variant(metadata.intra_distance_meter.clone()),
                                    #list_trait::LIST_VARIABLE_NAME,
                                    #descriptor_index_lit,
                                )
                            )
                        );
                        __solverforge_construction.push(
                            ::solverforge::__internal::ConstructionArgs {
                                element_count: Self::#element_count_ident,
                                assigned_elements: Self::#assigned_elements_ident,
                                entity_count: Self::#n_entities_ident,
                                list_len: Self::#list_len_ident,
                                list_insert: Self::#list_insert_ident,
                                list_remove: Self::#list_remove_for_construction_ident,
                                index_to_element: Self::#index_to_element_ident,
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
                            }
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
                fn __solverforge_default_time_limit_secs() -> u64 {
                    if Self::__solverforge_has_list_variable() {
                        60
                    } else {
                        30
                    }
                }

                #[inline]
                fn __solverforge_has_list_variable() -> bool {
                    false #(|| #has_list_variable_terms)*
                }

                fn __solverforge_is_trivial(solution: &Self) -> bool {
                    let descriptor = Self::descriptor();
                    let has_standard = ::solverforge::__internal::descriptor_has_bindings(&descriptor);
                    let total_entity_count = descriptor
                        .total_entity_count(solution as &dyn ::std::any::Any)
                        .unwrap_or(0);
                    if total_entity_count == 0 {
                        return true;
                    }

                    if !Self::__solverforge_has_list_variable() {
                        return !has_standard;
                    }

                    let has_list = Self::__solverforge_total_list_entities(solution) > 0
                        && Self::__solverforge_total_list_elements(solution) > 0;
                    !has_standard && !has_list
                }

                fn __solverforge_log_scale(solution: &Self) {
                    let descriptor = Self::descriptor();
                    let has_standard = ::solverforge::__internal::descriptor_has_bindings(&descriptor);
                    if Self::__solverforge_has_list_variable() {
                        ::solverforge::__internal::log_solve_start(
                            Self::__solverforge_total_list_entities(solution),
                            ::core::option::Option::Some(
                                Self::__solverforge_total_list_elements(solution),
                            ),
                            ::core::option::Option::Some(has_standard),
                            ::core::option::Option::None,
                        );
                    } else {
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
                    let mut __solverforge_construction = ::std::vec::Vec::new();
                    #(#list_runtime_setup)*
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
                        __solverforge_construction,
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
                    ::std::vec::Vec::new(),
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
                    let mut director = ScoreDirector::with_descriptor_and_shadow_support(
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
