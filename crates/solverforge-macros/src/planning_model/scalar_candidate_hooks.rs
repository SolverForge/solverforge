/* Generated helper functions and attachment statements for a model's scalar
variable hooks, filled per variable and spliced into the support impl. */
#[derive(Default)]
struct ScalarHookTokens {
    descriptor_helpers: Vec<TokenStream>,
    descriptor_attachments: Vec<TokenStream>,
    runtime_helpers: Vec<TokenStream>,
    runtime_attachments: Vec<TokenStream>,
}

/* One scalar variable being wired, with the identifiers its hooks need. */
#[derive(Clone, Copy)]
struct ScalarHookSite<'a> {
    variable: &'a ScalarVariableMetadata,
    variable_name: &'a String,
    descriptor_index: usize,
    entity_field: &'a Ident,
    entity_accessor: &'a Ident,
    solution_path: &'a TokenStream,
}

/* Candidate-value, nearby-candidate and nearby-distance hooks of one scalar
variable. */
fn push_scalar_candidate_hooks(hooks: &mut ScalarHookTokens, site: ScalarHookSite<'_>) {
    let ScalarHookSite {
        variable,
        variable_name,
        descriptor_index,
        entity_field,
        entity_accessor,
        solution_path,
    } = site;

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
        hooks.descriptor_helpers.push(quote! {
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
        hooks.descriptor_attachments.push(quote! {
            attach_scalar_variable_hook(
                descriptor,
                #descriptor_index,
                #variable_name,
                |variable| {
                    variable.candidate_values = ::core::option::Option::Some(#helper);
                },
            );
        });
        hooks.runtime_helpers.push(quote! {
            fn #runtime_helper(
                solution: &#solution_path,
                entity_index: usize,
                variable_index: usize,
            ) -> &[usize] {
                #path(solution, entity_index, variable_index)
            }
        });
        hooks.runtime_attachments.push(quote! {
            if slot.descriptor_index == #descriptor_index
                && slot.variable_name == #variable_name
            {
                slot = slot.with_candidate_values(#runtime_helper);
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
        hooks.descriptor_helpers.push(quote! {
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
        hooks.descriptor_attachments.push(quote! {
            attach_scalar_variable_hook(
                descriptor,
                #descriptor_index,
                #variable_name,
                |variable| {
                    variable.nearby_value_candidates = ::core::option::Option::Some(#helper);
                },
            );
        });
        hooks.runtime_helpers.push(quote! {
            fn #runtime_helper(
                solution: &#solution_path,
                entity_index: usize,
                variable_index: usize,
            ) -> &[usize] {
                #path(solution, entity_index, variable_index)
            }
        });
        hooks.runtime_attachments.push(quote! {
            if slot.descriptor_index == #descriptor_index
                && slot.variable_name == #variable_name
            {
                slot = slot.with_nearby_value_candidates(#runtime_helper);
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
        hooks.descriptor_helpers.push(quote! {
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
        hooks.descriptor_attachments.push(quote! {
            attach_scalar_variable_hook(
                descriptor,
                #descriptor_index,
                #variable_name,
                |variable| {
                    variable.nearby_entity_candidates = ::core::option::Option::Some(#helper);
                },
            );
        });
        hooks.runtime_helpers.push(quote! {
            fn #runtime_helper(
                solution: &#solution_path,
                entity_index: usize,
                variable_index: usize,
            ) -> &[usize] {
                #path(solution, entity_index, variable_index)
            }
        });
        hooks.runtime_attachments.push(quote! {
            if slot.descriptor_index == #descriptor_index
                && slot.variable_name == #variable_name
            {
                slot = slot.with_nearby_entity_candidates(#runtime_helper);
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
        hooks.descriptor_helpers.push(quote! {
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
        hooks.descriptor_attachments.push(quote! {
            attach_scalar_variable_hook(
                descriptor,
                #descriptor_index,
                #variable_name,
                |variable| {
                    variable.nearby_value_distance_meter = ::core::option::Option::Some(#helper);
                },
            );
        });
        hooks.runtime_helpers.push(quote! {
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
        hooks.runtime_attachments.push(quote! {
            if slot.descriptor_index == #descriptor_index
                && slot.variable_name == #variable_name
            {
                slot = slot.with_nearby_value_distance_meter(#runtime_helper);
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
        hooks.descriptor_helpers.push(quote! {
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
        hooks.descriptor_attachments.push(quote! {
            attach_scalar_variable_hook(
                descriptor,
                #descriptor_index,
                #variable_name,
                |variable| {
                    variable.nearby_entity_distance_meter = ::core::option::Option::Some(#helper);
                },
            );
        });
        hooks.runtime_helpers.push(quote! {
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
        hooks.runtime_attachments.push(quote! {
            if slot.descriptor_index == #descriptor_index
                && slot.variable_name == #variable_name
            {
                slot = slot.with_nearby_entity_distance_meter(#runtime_helper);
            }
        });
    }
}
