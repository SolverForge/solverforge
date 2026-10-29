fn generate_support_impl(model: &ModelMetadata) -> Result<TokenStream> {
    let solution_module = &model.solution.module_ident;
    let solution_ident = &model.solution.ident;
    let solution_path = quote! { #solution_module::#solution_ident };

    let mut hooks = ScalarHookTokens::default();
    let mut list_runtime_helpers = Vec::new();
    let mut list_runtime_attachments = Vec::new();
    let mut list_owner_match_arms = Vec::new();
    let mut validation_checks = Vec::new();
    let shadow_methods = generate_shadow_methods(model)?;
    let scalar_groups_impl = generate_scalar_groups_impl(model);

    for collection in model
        .solution
        .collections
        .iter()
        .filter(|collection| collection.descriptor_index.is_some())
    {
        let entity = model
            .entities
            .get(canonical_type_name(&model.aliases, &collection.type_name))
            .expect("entity collection should have been validated");
        let descriptor_index = collection.descriptor_index.unwrap();
        let entity_field = &collection.field_ident;
        let entity_accessor = format_ident!("__solverforge_entity_{}", entity_field);
        let entity_type_name = &entity.type_name;
        let solution_field_name = &collection.field_name;

        validation_checks.push(quote! {
            {
                let entity_descriptor = descriptor
                    .entity_descriptors
                    .get(#descriptor_index)
                    .expect("planning_model! entity descriptor missing");
                assert_eq!(
                    entity_descriptor.solution_field,
                    #solution_field_name,
                    "planning_model! entity descriptor field mismatch",
                );
                assert_eq!(
                    entity_descriptor.type_name,
                    #entity_type_name,
                    "planning_model! entity descriptor type mismatch",
                );
            }
        });

        for variable in &entity.scalar_variables {
            let variable_name = &variable.field_name;
            validation_checks.push(quote! {
                {
                    let entity_descriptor = descriptor
                        .entity_descriptors
                        .get(#descriptor_index)
                        .expect("planning_model! entity descriptor missing");
                    let _ = entity_descriptor
                        .variable_descriptors
                        .iter()
                        .find(|variable| {
                            variable.name == #variable_name
                                && variable.usize_getter.is_some()
                                && variable.usize_setter.is_some()
                        })
                        .expect("planning_model! scalar variable descriptor missing");
                }
            });

            let site = ScalarHookSite {
                variable,
                variable_name,
                descriptor_index,
                entity_field,
                entity_accessor: &entity_accessor,
                solution_path: &solution_path,
            };
            push_scalar_candidate_hooks(&mut hooks, site);
            push_scalar_order_hooks(&mut hooks, site);
        }

        if let (Some(variable_name), Some(path)) =
            (&entity.list_variable_name, &entity.list_element_owner_fn)
        {
            let runtime_helper = format_ident!(
                "__solverforge_runtime_list_element_owner_{}",
                entity_field
            );
            list_runtime_helpers.push(quote! {
                fn #runtime_helper(
                    solution: &#solution_path,
                    element: &usize,
                ) -> ::core::option::Option<usize> {
                    #path(solution, *element)
                }
            });
            list_runtime_attachments.push(quote! {
                if slot.descriptor_index == #descriptor_index
                    && slot.variable_name == #variable_name
                {
                    slot = slot.with_element_owner_fn(::core::option::Option::Some(#runtime_helper));
                }
            });
            list_owner_match_arms.push(quote! {
                (#entity_type_name, #variable_name) => #path(solution, *element),
            });
        }

        if let (Some(variable_name), Some(path)) = (
            &entity.list_variable_name,
            &entity.list_construction_element_order_key,
        ) {
            let runtime_helper = format_ident!(
                "__solverforge_runtime_list_construction_element_order_{}",
                entity_field
            );
            list_runtime_helpers.push(quote! {
                fn #runtime_helper(
                    solution: &#solution_path,
                    element: usize,
                ) -> i64 {
                    #path(solution, element)
                }
            });
            list_runtime_attachments.push(quote! {
                if slot.descriptor_index == #descriptor_index
                    && slot.variable_name == #variable_name
                {
                    slot = slot.with_construction_element_order_key(
                        ::core::option::Option::Some(#runtime_helper),
                    );
                }
            });
        }

        if let (
            Some(variable_name),
            Some(duration_path),
            Some(successors_path),
        ) = (
            &entity.list_variable_name,
            &entity.list_precedence_duration_fn,
            &entity.list_precedence_successors_fn,
        ) {
            let duration_helper = format_ident!(
                "__solverforge_runtime_list_precedence_duration_{}",
                entity_field
            );
            let successors_helper = format_ident!(
                "__solverforge_runtime_list_precedence_successors_{}",
                entity_field
            );
            list_runtime_helpers.push(quote! {
                fn #duration_helper(
                    solution: &#solution_path,
                    element: usize,
                ) -> usize {
                    #duration_path(solution, element)
                }

                fn #successors_helper(
                    solution: &#solution_path,
                    element: usize,
                    out: &mut ::std::vec::Vec<usize>,
                ) {
                    #successors_path(solution, element, out)
                }
            });
            list_runtime_attachments.push(quote! {
                if slot.descriptor_index == #descriptor_index
                    && slot.variable_name == #variable_name
                {
                    slot = slot.with_precedence_hooks(
                        ::core::option::Option::Some(#duration_helper),
                        ::core::option::Option::Some(#successors_helper),
                    );
                }
            });
        }
    }

    let ScalarHookTokens {
        descriptor_helpers,
        descriptor_attachments,
        runtime_helpers,
        runtime_attachments,
    } = hooks;

    Ok(quote! {
        impl ::solverforge::__internal::PlanningModelSupport for #solution_path {
            fn attach_descriptor_hooks(
                descriptor: &mut ::solverforge::__internal::SolutionDescriptor,
            ) {
                fn attach_scalar_variable_hook(
                    descriptor: &mut ::solverforge::__internal::SolutionDescriptor,
                    descriptor_index: usize,
                    variable_name: &'static str,
                    attach: impl FnOnce(&mut ::solverforge::__internal::VariableDescriptor),
                ) {
                    let entity_descriptor = descriptor
                        .entity_descriptors
                        .get_mut(descriptor_index)
                        .expect("planning_model! entity descriptor missing for scalar hook");
                    let variable_descriptor = entity_descriptor
                        .variable_descriptors
                        .iter_mut()
                        .find(|variable| {
                            variable.name == variable_name
                                && variable.usize_getter.is_some()
                                && variable.usize_setter.is_some()
                        })
                        .expect("planning_model! scalar hook target variable missing");
                    attach(variable_descriptor);
                }

                #(#descriptor_helpers)*
                #(#descriptor_attachments)*
            }

            fn attach_runtime_scalar_hooks(
                mut slot: ::solverforge::__internal::ScalarVariableSlot<Self>,
            ) -> ::solverforge::__internal::ScalarVariableSlot<Self> {
                #(#runtime_helpers)*
                #(#runtime_attachments)*
                slot
            }

            fn attach_runtime_list_hooks<DM, IDM>(
                mut slot: ::solverforge::__internal::ListVariableSlot<Self, usize, DM, IDM>,
            ) -> ::solverforge::__internal::ListVariableSlot<Self, usize, DM, IDM> {
                #(#list_runtime_helpers)*
                #(#list_runtime_attachments)*
                slot
            }

            fn list_element_owner(
                entity_type_name: &'static str,
                variable_name: &'static str,
                solution: &Self,
                element: &usize,
            ) -> ::core::option::Option<usize> {
                match (entity_type_name, variable_name) {
                    #(#list_owner_match_arms)*
                    _ => ::core::option::Option::None,
                }
            }

            fn attach_scalar_groups(
                scalar_variables: &[::solverforge::__internal::ScalarVariableSlot<Self>],
            ) -> ::std::vec::Vec<::solverforge::__internal::ScalarGroupBinding<Self>> {
                #scalar_groups_impl
            }

            fn validate_model(descriptor: &::solverforge::__internal::SolutionDescriptor) {
                #(#validation_checks)*
            }

            #shadow_methods
        }
    })
}
