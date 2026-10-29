/* Construction and search order-key hooks of one scalar variable. */
fn push_scalar_order_hooks(hooks: &mut ScalarHookTokens, site: ScalarHookSite<'_>) {
    let ScalarHookSite {
        variable,
        variable_name,
        descriptor_index,
        entity_field,
        entity_accessor,
        solution_path,
    } = site;

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
        hooks.descriptor_helpers.push(quote! {
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
        hooks.descriptor_attachments.push(quote! {
            attach_scalar_variable_hook(
                descriptor,
                #descriptor_index,
                #variable_name,
                |variable| {
                    variable.construction_entity_order_key = ::core::option::Option::Some(#helper);
                },
            );
        });
        hooks.runtime_helpers.push(quote! {
            fn #runtime_helper(
                solution: &#solution_path,
                entity_index: usize,
                _variable_index: usize,
            ) -> ::core::option::Option<i64> {
                let entity = #solution_path::#entity_accessor(solution, entity_index);
                ::core::option::Option::Some(#path(solution, entity))
            }
        });
        hooks.runtime_attachments.push(quote! {
            if slot.descriptor_index == #descriptor_index
                && slot.variable_name == #variable_name
            {
                slot = slot.with_construction_entity_order_key(#runtime_helper);
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
        hooks.descriptor_helpers.push(quote! {
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
        hooks.descriptor_attachments.push(quote! {
            attach_scalar_variable_hook(
                descriptor,
                #descriptor_index,
                #variable_name,
                |variable| {
                    variable.construction_value_order_key = ::core::option::Option::Some(#helper);
                },
            );
        });
        hooks.runtime_helpers.push(quote! {
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
        hooks.runtime_attachments.push(quote! {
            if slot.descriptor_index == #descriptor_index
                && slot.variable_name == #variable_name
            {
                slot = slot.with_construction_value_order_key(#runtime_helper);
            }
        });
    }

    if let Some(path) = &variable.hooks.search_entity_order_key {
        let helper = format_ident!(
            "__solverforge_descriptor_search_entity_order_key_{}_{}",
            entity_field,
            variable.field_name
        );
        let runtime_helper = format_ident!(
            "__solverforge_runtime_search_entity_order_key_{}_{}",
            entity_field,
            variable.field_name
        );
        hooks.descriptor_helpers.push(quote! {
            fn #helper(
                solution: &dyn ::std::any::Any,
                entity_index: usize,
            ) -> i64 {
                let solution = solution
                    .downcast_ref::<#solution_path>()
                    .expect("solution type mismatch for search entity order key");
                let entity = #solution_path::#entity_accessor(solution, entity_index);
                #path(solution, entity)
            }
        });
        hooks.descriptor_attachments.push(quote! {
            attach_scalar_variable_hook(
                descriptor,
                #descriptor_index,
                #variable_name,
                |variable| {
                    variable.search_entity_order_key = ::core::option::Option::Some(#helper);
                },
            );
        });
        hooks.runtime_helpers.push(quote! {
            fn #runtime_helper(
                solution: &#solution_path,
                entity_index: usize,
                _variable_index: usize,
            ) -> ::core::option::Option<i64> {
                let entity = #solution_path::#entity_accessor(solution, entity_index);
                ::core::option::Option::Some(#path(solution, entity))
            }
        });
        hooks.runtime_attachments.push(quote! {
            if slot.descriptor_index == #descriptor_index
                && slot.variable_name == #variable_name
            {
                slot = slot.with_search_entity_order_key(#runtime_helper);
            }
        });
    }

    if let Some(path) = &variable.hooks.search_value_order_key {
        let helper = format_ident!(
            "__solverforge_descriptor_search_value_order_key_{}_{}",
            entity_field,
            variable.field_name
        );
        let runtime_helper = format_ident!(
            "__solverforge_runtime_search_value_order_key_{}_{}",
            entity_field,
            variable.field_name
        );
        hooks.descriptor_helpers.push(quote! {
            fn #helper(
                solution: &dyn ::std::any::Any,
                entity_index: usize,
                value: usize,
            ) -> i64 {
                let solution = solution
                    .downcast_ref::<#solution_path>()
                    .expect("solution type mismatch for search value order key");
                let entity = #solution_path::#entity_accessor(solution, entity_index);
                #path(solution, entity, value)
            }
        });
        hooks.descriptor_attachments.push(quote! {
            attach_scalar_variable_hook(
                descriptor,
                #descriptor_index,
                #variable_name,
                |variable| {
                    variable.search_value_order_key = ::core::option::Option::Some(#helper);
                },
            );
        });
        hooks.runtime_helpers.push(quote! {
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
        hooks.runtime_attachments.push(quote! {
            if slot.descriptor_index == #descriptor_index
                && slot.variable_name == #variable_name
            {
                slot = slot.with_search_value_order_key(#runtime_helper);
            }
        });
    }
}
