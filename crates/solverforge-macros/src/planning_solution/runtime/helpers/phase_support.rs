pub(super) fn generate_runtime_phase_support(
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
    constraints_path: &Option<String>,
    conflict_repairs_path: &Option<String>,
    scalar_groups_path: &Option<String>,
    search_path: &Option<String>,
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
            let syn::Type::Path(type_path) = &field_type else {
                return None;
            };
            let type_name = type_path.path.segments.last()?.ident.to_string();
            Some((idx, field_ident, field_type, type_name))
        })
        .collect();
    let scalar_setup = generate_scalar_runtime_setup(fields, solution_name);
    let conflict_repair_expr = conflict_repairs_path
        .as_ref()
        .map(|path| {
            let repairs_fn: syn::Path =
                syn::parse_str(path).expect("conflict repairs path must be valid");
            quote! { .with_conflict_repairs(#repairs_fn()) }
        })
        .unwrap_or_else(|| quote! {});
    let scalar_groups_expr = scalar_groups_path
        .as_ref()
        .map(|path| {
            let groups_fn: syn::Path =
                syn::parse_str(path).expect("scalar groups path must be valid");
            quote! {
                ::solverforge::__internal::bind_scalar_groups(
                    #groups_fn(),
                    &__solverforge_scalar_slots,
                )
            }
        })
        .unwrap_or_else(|| {
            quote! {
                <#solution_name as ::solverforge::__internal::PlanningModelSupport>::attach_scalar_groups(
                    &__solverforge_scalar_slots,
                )
            }
        });
    let search_fn = search_path.as_ref().map(|path| {
        syn::parse_str::<syn::Path>(path).expect("search path must be a valid Rust path")
    });
    let scalar_candidate_count_helper =
        generate_scalar_candidate_count_helper(fields, solution_name);

    if !list_owners.is_empty() {
        let cross_enum_ident = format_ident!("__{}CrossDistanceMeter", solution_name);
        let intra_enum_ident = format_ident!("__{}IntraDistanceMeter", solution_name);
        let has_list_variable_terms: Vec<_> = list_owners
            .iter()
            .map(|(_, _, field_type, _)| {
                let list_trait =
                    quote! { <#field_type as ::solverforge::__internal::ListVariableEntity<#solution_name>> };
                quote! { #list_trait::HAS_LIST_VARIABLE }
            })
            .collect();

        let cross_variants: Vec<_> = list_owners
            .iter()
            .map(|(idx, _, field_type, _)| {
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
            .map(|(idx, _, field_type, _)| {
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
            .map(|(idx, _, _, _)| {
                let variant = format_ident!("Entity{idx}");
                quote! {
                    Self::#variant(meter) => meter.distance(solution, src_entity, src_pos, dst_entity, dst_pos),
                }
            })
            .collect();
        let intra_match_arms: Vec<_> = list_owners
            .iter()
            .map(|(idx, _, _, _)| {
                let variant = format_ident!("Entity{idx}");
                quote! {
                    Self::#variant(meter) => meter.distance(solution, src_entity, src_pos, dst_entity, dst_pos),
                }
            })
            .collect();
        let list_runtime_setup: Vec<_> = list_owners
            .iter()
            .map(|(idx, field_ident, field_type, _type_name)| {
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
                        let __solverforge_entity_type_name = descriptor
                            .entity_descriptors
                            .get(#descriptor_index_lit)
                            .expect("entity descriptor missing for list runtime setup")
                            .type_name;
                        let metadata = #list_trait::list_metadata();
                        __solverforge_variables.push(
                            ::solverforge::__internal::VariableSlot::List(
                                ::solverforge::__internal::ListVariableSlot::new(
                                    __solverforge_entity_type_name,
                                    Self::#element_count_ident,
                                    Self::#assigned_elements_ident,
                                    Self::#list_len_ident,
                                    Self::#list_remove_ident,
                                    Self::#list_remove_for_construction_ident,
                                    Self::#list_insert_ident,
                                    Self::#list_get_ident,
                                    Self::#list_set_ident,
                                    Self::#list_reverse_ident,
                                    Self::#sublist_remove_ident,
                                    Self::#sublist_insert_ident,
                                    Self::#ruin_remove_ident,
                                    Self::#ruin_insert_ident,
                                    Self::#index_to_element_ident,
                                    Self::#n_entities_ident,
                                    #cross_enum_ident::#variant(metadata.cross_distance_meter.clone()),
                                    #intra_enum_ident::#variant(metadata.intra_distance_meter.clone()),
                                    #list_trait::LIST_VARIABLE_NAME,
                                    #descriptor_index_lit,
                                    metadata.route_get_fn,
                                    metadata.route_set_fn,
                                    metadata.route_depot_fn,
                                    metadata.route_distance_fn,
                                    metadata.route_feasible_fn,
                                )
                            )
                        );
                    }
                }
            })
            .collect();

        let build_phases_fn = generate_list_build_phases_fn(ListBuildPhasesInput {
            solution_name,
            cross_enum_ident: &cross_enum_ident,
            intra_enum_ident: &intra_enum_ident,
            scalar_setup: &scalar_setup,
            list_runtime_setup: &list_runtime_setup,
            scalar_groups_expr: &scalar_groups_expr,
            conflict_repair_expr: &conflict_repair_expr,
            search_fn: search_fn.as_ref(),
        });

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
                #scalar_candidate_count_helper

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
                    let has_scalar = ::solverforge::__internal::descriptor_has_bindings(&descriptor);
                    let total_entity_count = descriptor
                        .total_entity_count(solution as &dyn ::std::any::Any)
                        .unwrap_or(0);
                    if total_entity_count == 0 {
                        return true;
                    }

                    if !Self::__solverforge_has_list_variable() {
                        return !has_scalar;
                    }

                    let has_list = Self::__solverforge_total_list_entities(solution) > 0
                        && Self::__solverforge_total_list_elements(solution) > 0;
                    !has_scalar && !has_list
                }

                fn __solverforge_log_scale(solution: &Self) {
                    let descriptor = Self::descriptor();
                    if Self::__solverforge_has_list_variable() {
                        ::solverforge::__internal::log_solve_start(
                            Self::__solverforge_total_list_entities(solution),
                            ::core::option::Option::Some(
                                Self::__solverforge_total_list_elements(solution),
                            ),
                            ::core::option::Option::None,
                        );
                    } else {
                        ::solverforge::__internal::log_solve_start(
                            descriptor
                                .total_entity_count(solution as &dyn ::std::any::Any)
                                .unwrap_or(0),
                            ::core::option::Option::None,
                            ::core::option::Option::Some(
                                Self::__solverforge_scalar_candidate_count(solution),
                            ),
                        );
                    }
                }

                #build_phases_fn
            }
        };
    }

    let build_phases_fn = generate_scalar_build_phases_fn(
        solution_name,
        &scalar_setup,
        &scalar_groups_expr,
        &conflict_repair_expr,
        search_fn.as_ref(),
    );

    quote! {
        impl #solution_name {
            #scalar_candidate_count_helper

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
                    ::core::option::Option::Some(
                        Self::__solverforge_scalar_candidate_count(solution),
                    ),
                );
            }

            #build_phases_fn
        }
    }
}
