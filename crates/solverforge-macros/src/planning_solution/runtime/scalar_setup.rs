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
            let _ = type_path.path.segments.last()?;
            Some((idx, field_name, field_type))
        })
        .collect();

    let provider_fields: Vec<_> = fields
        .iter()
        .filter(|f| {
            has_attribute(&f.attrs, "planning_entity_collection")
                || has_attribute(&f.attrs, "problem_fact_collection")
        })
        .filter_map(|field| field.ident.as_ref())
        .collect();

    let provider_names: Vec<_> = provider_fields
        .iter()
        .map(|field_name| field_name.to_string())
        .collect();
    let provider_count_arms: Vec<_> = provider_fields
        .iter()
        .enumerate()
        .map(|(idx, field_name)| {
            quote! { #idx => solution.#field_name.len(), }
        })
        .collect();

    let entity_helpers: Vec<_> = entity_fields
        .iter()
        .map(|(_, field_name, field_type)| {
            let count_fn_ident = format_ident!("__solverforge_scalar_count_{}", field_name);
            let getter_ident = format_ident!("__solverforge_scalar_get_{}", field_name);
            let setter_ident = format_ident!("__solverforge_scalar_set_{}", field_name);
            let values_ident = format_ident!("__solverforge_scalar_values_{}", field_name);
            quote! {
                fn #count_fn_ident(solution: &#solution_name) -> usize {
                    solution.#field_name.len()
                }

                fn #getter_ident(
                    solution: &#solution_name,
                    entity_index: usize,
                    variable_index: usize,
                ) -> ::core::option::Option<usize> {
                    <#field_type>::__solverforge_scalar_get_by_index(
                        &solution.#field_name[entity_index],
                        variable_index,
                    )
                }

                fn #setter_ident(
                    solution: &mut #solution_name,
                    entity_index: usize,
                    variable_index: usize,
                    value: ::core::option::Option<usize>,
                ) {
                    <#field_type>::__solverforge_scalar_set_by_index(
                        &mut solution.#field_name[entity_index],
                        variable_index,
                        value,
                    );
                }

                fn #values_ident(
                    solution: &#solution_name,
                    entity_index: usize,
                    variable_index: usize,
                ) -> &[usize] {
                    <#field_type>::__solverforge_scalar_values_by_index(
                        &solution.#field_name[entity_index],
                        variable_index,
                    )
                }
            }
        })
        .collect();

    let scalar_slot_pushes: Vec<_> = entity_fields
        .iter()
        .map(|(descriptor_index, field_name, field_type)| {
            let entity_count_fn_ident = format_ident!("__solverforge_scalar_count_{}", field_name);
            let getter_ident = format_ident!("__solverforge_scalar_get_{}", field_name);
            let setter_ident = format_ident!("__solverforge_scalar_set_{}", field_name);
            let values_ident = format_ident!("__solverforge_scalar_values_{}", field_name);
            quote! {
                {
                    let __solverforge_descriptor_index = #descriptor_index;
                    let __solverforge_entity_descriptor = descriptor
                        .entity_descriptors
                        .get(__solverforge_descriptor_index)
                        .expect("entity descriptor missing for scalar runtime setup");
                    for __solverforge_variable_index in 0..<#field_type>::__solverforge_scalar_variable_count() {
                        let Some(__solverforge_variable_name) =
                            <#field_type>::__solverforge_scalar_variable_name_by_index(
                                __solverforge_variable_index,
                            )
                        else {
                            continue;
                        };
                        let Some(__solverforge_variable_descriptor) = __solverforge_entity_descriptor
                            .genuine_variable_descriptors()
                            .find(|variable| {
                                variable.name == __solverforge_variable_name
                                    && variable.usize_getter.is_some()
                                    && variable.usize_setter.is_some()
                            })
                        else {
                            continue;
                        };

                        let __solverforge_value_source = if __solverforge_variable_descriptor
                            .entity_value_provider
                            .is_some()
                            || <#field_type>::__solverforge_scalar_provider_is_entity_field_by_index(
                                __solverforge_variable_index,
                            )
                        {
                            ::solverforge::__internal::ValueSource::EntitySlice {
                                values_for_entity: #values_ident,
                            }
                        } else {
                            match &__solverforge_variable_descriptor.value_range_type {
                                ::solverforge::__internal::ValueRangeType::CountableRange { from, to } => {
                                    let from = usize::try_from(*from).expect(
                                        "countable_range start must be non-negative for canonical scalar solving",
                                    );
                                    let to = usize::try_from(*to).expect(
                                        "countable_range end must be non-negative for canonical scalar solving",
                                    );
                                    ::solverforge::__internal::ValueSource::CountableRange { from, to }
                                }
                                _ => {
                                    if let Some(provider_name) =
                                        __solverforge_variable_descriptor.value_range_provider
                                    {
                                        let provider_index = __solverforge_scalar_provider_fields
                                            .iter()
                                            .position(|field| *field == provider_name)
                                            .expect("scalar value range provider must be a solution collection");
                                        ::solverforge::__internal::ValueSource::SolutionCount {
                                            count_fn: __solverforge_scalar_collection_count,
                                            provider_index,
                                        }
                                    } else {
                                        ::solverforge::__internal::ValueSource::Empty
                                    }
                                }
                            }
                        };

                        let __solverforge_slot =
                            ::solverforge::__internal::ScalarVariableSlot::new(
                                __solverforge_descriptor_index,
                                __solverforge_variable_index,
                                __solverforge_entity_descriptor.type_name,
                                #entity_count_fn_ident,
                                __solverforge_variable_name,
                                #getter_ident,
                                #setter_ident,
                                __solverforge_value_source,
                                __solverforge_variable_descriptor.allows_unassigned,
                            );
                        let __solverforge_slot =
                            <#solution_name as ::solverforge::__internal::PlanningModelSupport>::attach_runtime_scalar_hooks(
                                __solverforge_slot,
                            );
                        __solverforge_variables.push(
                            ::solverforge::__internal::VariableSlot::Scalar(
                                __solverforge_slot,
                            )
                        );
                    }
                }
            }
        })
        .collect();

    quote! {
        let mut __solverforge_variables = ::std::vec::Vec::new();
        let __solverforge_scalar_provider_fields: &[&str] = &[
            #(#provider_names),*
        ];
        #(#entity_helpers)*
        fn __solverforge_scalar_collection_count(
            solution: &#solution_name,
            provider_index: usize,
        ) -> usize {
            match provider_index {
                #(#provider_count_arms)*
                _ => 0,
            }
        }
        #(#scalar_slot_pushes)*
    }
}

