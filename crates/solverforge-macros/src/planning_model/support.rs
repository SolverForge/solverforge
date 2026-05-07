fn generate_support_impl(model: &ModelMetadata) -> Result<TokenStream> {
    let solution_module = &model.solution.module_ident;
    let solution_ident = &model.solution.ident;
    let solution_path = quote! { #solution_module::#solution_ident };

    let mut descriptor_helpers = Vec::new();
    let mut descriptor_attachments = Vec::new();
    let mut runtime_helpers = Vec::new();
    let mut runtime_attachments = Vec::new();
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

            if let Some(path) = &variable.hooks.candidate_values {
                let helper = format_ident!(
                    "__solverforge_descriptor_candidate_values_{}_{}",
                    entity_field,
                    variable.field_name
                );
                let runtime_helper = format_ident!(
                    "__solverforge_runtime_candidate_values_{}_{}",
                    entity_field,
                    variable.field_name
                );
                descriptor_helpers.push(quote! {
                    fn #helper(
                        solution: &dyn ::std::any::Any,
                        entity_index: usize,
                        variable_index: usize,
                    ) -> &[usize] {
                        let solution = solution
                            .downcast_ref::<#solution_path>()
                            .expect("solution type mismatch for scalar candidate values");
                        #path(solution, entity_index, variable_index)
                    }
                });
                descriptor_attachments.push(quote! {
                    attach_scalar_variable_hook(
                        descriptor,
                        #descriptor_index,
                        #variable_name,
                        |variable| {
                            variable.candidate_values = ::core::option::Option::Some(#helper);
                        },
                    );
                });
                runtime_helpers.push(quote! {
                    fn #runtime_helper(
                        solution: &#solution_path,
                        entity_index: usize,
                        variable_index: usize,
                    ) -> &[usize] {
                        #path(solution, entity_index, variable_index)
                    }
                });
                runtime_attachments.push(quote! {
                    if context.descriptor_index == #descriptor_index
                        && context.variable_name == #variable_name
                    {
                        context = context.with_candidate_values(#runtime_helper);
                    }
                });
            }

            if let Some(path) = &variable.hooks.nearby_value_candidates {
                let helper = format_ident!(
                    "__solverforge_descriptor_nearby_value_candidates_{}_{}",
                    entity_field,
                    variable.field_name
                );
                let runtime_helper = format_ident!(
                    "__solverforge_runtime_nearby_value_candidates_{}_{}",
                    entity_field,
                    variable.field_name
                );
                descriptor_helpers.push(quote! {
                    fn #helper(
                        solution: &dyn ::std::any::Any,
                        entity_index: usize,
                        variable_index: usize,
                    ) -> &[usize] {
                        let solution = solution
                            .downcast_ref::<#solution_path>()
                            .expect("solution type mismatch for nearby value candidates");
                        #path(solution, entity_index, variable_index)
                    }
                });
                descriptor_attachments.push(quote! {
                    attach_scalar_variable_hook(
                        descriptor,
                        #descriptor_index,
                        #variable_name,
                        |variable| {
                            variable.nearby_value_candidates = ::core::option::Option::Some(#helper);
                        },
                    );
                });
                runtime_helpers.push(quote! {
                    fn #runtime_helper(
                        solution: &#solution_path,
                        entity_index: usize,
                        variable_index: usize,
                    ) -> &[usize] {
                        #path(solution, entity_index, variable_index)
                    }
                });
                runtime_attachments.push(quote! {
                    if context.descriptor_index == #descriptor_index
                        && context.variable_name == #variable_name
                    {
                        context = context.with_nearby_value_candidates(#runtime_helper);
                    }
                });
            }

            if let Some(path) = &variable.hooks.nearby_entity_candidates {
                let helper = format_ident!(
                    "__solverforge_descriptor_nearby_entity_candidates_{}_{}",
                    entity_field,
                    variable.field_name
                );
                let runtime_helper = format_ident!(
                    "__solverforge_runtime_nearby_entity_candidates_{}_{}",
                    entity_field,
                    variable.field_name
                );
                descriptor_helpers.push(quote! {
                    fn #helper(
                        solution: &dyn ::std::any::Any,
                        entity_index: usize,
                        variable_index: usize,
                    ) -> &[usize] {
                        let solution = solution
                            .downcast_ref::<#solution_path>()
                            .expect("solution type mismatch for nearby entity candidates");
                        #path(solution, entity_index, variable_index)
                    }
                });
                descriptor_attachments.push(quote! {
                    attach_scalar_variable_hook(
                        descriptor,
                        #descriptor_index,
                        #variable_name,
                        |variable| {
                            variable.nearby_entity_candidates = ::core::option::Option::Some(#helper);
                        },
                    );
                });
                runtime_helpers.push(quote! {
                    fn #runtime_helper(
                        solution: &#solution_path,
                        entity_index: usize,
                        variable_index: usize,
                    ) -> &[usize] {
                        #path(solution, entity_index, variable_index)
                    }
                });
                runtime_attachments.push(quote! {
                    if context.descriptor_index == #descriptor_index
                        && context.variable_name == #variable_name
                    {
                        context = context.with_nearby_entity_candidates(#runtime_helper);
                    }
                });
            }

            if let Some(path) = &variable.hooks.nearby_value_distance_meter {
                let helper = format_ident!(
                    "__solverforge_descriptor_nearby_value_distance_{}_{}",
                    entity_field,
                    variable.field_name
                );
                let runtime_helper = format_ident!(
                    "__solverforge_runtime_nearby_value_distance_{}_{}",
                    entity_field,
                    variable.field_name
                );
                descriptor_helpers.push(quote! {
                    fn #helper(
                        solution: &dyn ::std::any::Any,
                        entity_index: usize,
                        value: usize,
                    ) -> f64 {
                        let solution = solution
                            .downcast_ref::<#solution_path>()
                            .expect("solution type mismatch for nearby value distance meter");
                        let entity = #solution_path::#entity_accessor(solution, entity_index);
                        #path(solution, entity, value)
                    }
                });
                descriptor_attachments.push(quote! {
                    attach_scalar_variable_hook(
                        descriptor,
                        #descriptor_index,
                        #variable_name,
                        |variable| {
                            variable.nearby_value_distance_meter = ::core::option::Option::Some(#helper);
                        },
                    );
                });
                runtime_helpers.push(quote! {
                    fn #runtime_helper(
                        solution: &#solution_path,
                        entity_index: usize,
                        _variable_index: usize,
                        value: usize,
                    ) -> ::core::option::Option<f64> {
                        let entity = #solution_path::#entity_accessor(solution, entity_index);
                        ::core::option::Option::Some(#path(solution, entity, value))
                    }
                });
                runtime_attachments.push(quote! {
                    if context.descriptor_index == #descriptor_index
                        && context.variable_name == #variable_name
                    {
                        context = context.with_nearby_value_distance_meter(#runtime_helper);
                    }
                });
            }

            if let Some(path) = &variable.hooks.nearby_entity_distance_meter {
                let helper = format_ident!(
                    "__solverforge_descriptor_nearby_entity_distance_{}_{}",
                    entity_field,
                    variable.field_name
                );
                let runtime_helper = format_ident!(
                    "__solverforge_runtime_nearby_entity_distance_{}_{}",
                    entity_field,
                    variable.field_name
                );
                descriptor_helpers.push(quote! {
                    fn #helper(
                        solution: &dyn ::std::any::Any,
                        left_entity_index: usize,
                        right_entity_index: usize,
                    ) -> f64 {
                        let solution = solution
                            .downcast_ref::<#solution_path>()
                            .expect("solution type mismatch for nearby entity distance meter");
                        let left = #solution_path::#entity_accessor(solution, left_entity_index);
                        let right = #solution_path::#entity_accessor(solution, right_entity_index);
                        #path(solution, left, right)
                    }
                });
                descriptor_attachments.push(quote! {
                    attach_scalar_variable_hook(
                        descriptor,
                        #descriptor_index,
                        #variable_name,
                        |variable| {
                            variable.nearby_entity_distance_meter = ::core::option::Option::Some(#helper);
                        },
                    );
                });
                runtime_helpers.push(quote! {
                    fn #runtime_helper(
                        solution: &#solution_path,
                        left_entity_index: usize,
                        right_entity_index: usize,
                        _variable_index: usize,
                    ) -> ::core::option::Option<f64> {
                        let left = #solution_path::#entity_accessor(solution, left_entity_index);
                        let right = #solution_path::#entity_accessor(solution, right_entity_index);
                        ::core::option::Option::Some(#path(solution, left, right))
                    }
                });
                runtime_attachments.push(quote! {
                    if context.descriptor_index == #descriptor_index
                        && context.variable_name == #variable_name
                    {
                        context = context.with_nearby_entity_distance_meter(#runtime_helper);
                    }
                });
            }

            if let Some(path) = &variable.hooks.construction_entity_order_key {
                let helper = format_ident!(
                    "__solverforge_descriptor_construction_entity_order_key_{}_{}",
                    entity_field,
                    variable.field_name
                );
                let runtime_helper = format_ident!(
                    "__solverforge_runtime_construction_entity_order_key_{}_{}",
                    entity_field,
                    variable.field_name
                );
                descriptor_helpers.push(quote! {
                    fn #helper(
                        solution: &dyn ::std::any::Any,
                        entity_index: usize,
                    ) -> i64 {
                        let solution = solution
                            .downcast_ref::<#solution_path>()
                            .expect("solution type mismatch for construction entity order key");
                        let entity = #solution_path::#entity_accessor(solution, entity_index);
                        #path(solution, entity)
                    }
                });
                descriptor_attachments.push(quote! {
                    attach_scalar_variable_hook(
                        descriptor,
                        #descriptor_index,
                        #variable_name,
                        |variable| {
                            variable.construction_entity_order_key = ::core::option::Option::Some(#helper);
                        },
                    );
                });
                runtime_helpers.push(quote! {
                    fn #runtime_helper(
                        solution: &#solution_path,
                        entity_index: usize,
                        _variable_index: usize,
                    ) -> ::core::option::Option<i64> {
                        let entity = #solution_path::#entity_accessor(solution, entity_index);
                        ::core::option::Option::Some(#path(solution, entity))
                    }
                });
                runtime_attachments.push(quote! {
                    if context.descriptor_index == #descriptor_index
                        && context.variable_name == #variable_name
                    {
                        context = context.with_construction_entity_order_key(#runtime_helper);
                    }
                });
            }

            if let Some(path) = &variable.hooks.construction_value_order_key {
                let helper = format_ident!(
                    "__solverforge_descriptor_construction_value_order_key_{}_{}",
                    entity_field,
                    variable.field_name
                );
                let runtime_helper = format_ident!(
                    "__solverforge_runtime_construction_value_order_key_{}_{}",
                    entity_field,
                    variable.field_name
                );
                descriptor_helpers.push(quote! {
                    fn #helper(
                        solution: &dyn ::std::any::Any,
                        entity_index: usize,
                        value: usize,
                    ) -> i64 {
                        let solution = solution
                            .downcast_ref::<#solution_path>()
                            .expect("solution type mismatch for construction value order key");
                        let entity = #solution_path::#entity_accessor(solution, entity_index);
                        #path(solution, entity, value)
                    }
                });
                descriptor_attachments.push(quote! {
                    attach_scalar_variable_hook(
                        descriptor,
                        #descriptor_index,
                        #variable_name,
                        |variable| {
                            variable.construction_value_order_key = ::core::option::Option::Some(#helper);
                        },
                    );
                });
                runtime_helpers.push(quote! {
                    fn #runtime_helper(
                        solution: &#solution_path,
                        entity_index: usize,
                        _variable_index: usize,
                        value: usize,
                    ) -> ::core::option::Option<i64> {
                        let entity = #solution_path::#entity_accessor(solution, entity_index);
                        ::core::option::Option::Some(#path(solution, entity, value))
                    }
                });
                runtime_attachments.push(quote! {
                    if context.descriptor_index == #descriptor_index
                        && context.variable_name == #variable_name
                    {
                        context = context.with_construction_value_order_key(#runtime_helper);
                    }
                });
            }
        }
    }

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
                mut context: ::solverforge::__internal::ScalarVariableContext<Self>,
            ) -> ::solverforge::__internal::ScalarVariableContext<Self> {
                #(#runtime_helpers)*
                #(#runtime_attachments)*
                context
            }

            fn attach_scalar_groups(
                scalar_variables: &[::solverforge::__internal::ScalarVariableContext<Self>],
            ) -> ::std::vec::Vec<::solverforge::__internal::ScalarGroupContext<Self>> {
                #scalar_groups_impl
            }

            fn validate_model(descriptor: &::solverforge::__internal::SolutionDescriptor) {
                #(#validation_checks)*
            }

            #shadow_methods
        }
    })
}
