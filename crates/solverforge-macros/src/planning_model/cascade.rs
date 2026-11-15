/* Cascading list shadows: an element field computed from the element's
previous element in its list and from the elements it declares it depends
on, which other owners' lists may hold. The generated hooks give
`CascadePropagation` the dependency graph and the per-element update; the
propagation itself, its ordering and its change reporting are SolverForge's.
*/

struct CascadeContext<'a> {
    solution_path: &'a TokenStream,
    element_accessor: &'a Ident,
    element_mut_accessor: &'a Ident,
}

fn validate_cascade_config(config: &ShadowConfig) -> Result<()> {
    let error = |message: &str| Err(Error::new(proc_macro2::Span::call_site(), message));
    if config.cascading_field.is_none() {
        if config.cascading_compute.is_some() || config.cascading_dependencies.is_some() {
            return error(
                "#[shadow_variable_updates(...)] `cascading_compute` and `cascading_dependencies` require `cascading_field = \"element_field\"`",
            );
        }
        return Ok(());
    }
    if config.cascading_compute.is_none() {
        return error(
            "#[shadow_variable_updates(cascading_field = ...)] requires `cascading_compute = \"solution_method\"`, a `fn(&self, element: usize) -> Option<_>` on the solution",
        );
    }
    if config.inverse_field.is_none()
        || config.previous_field.is_none()
        || config.next_field.is_none()
    {
        return error(
            "#[shadow_variable_updates(cascading_field = ...)] requires `inverse_field`, `previous_field` and `next_field`: propagation follows each element's list position",
        );
    }
    if config.cascading_listener.is_some()
        || config.post_update_listener.is_some()
        || !config.entity_aggregates.is_empty()
        || !config.entity_computes.is_empty()
    {
        return error(
            "#[shadow_variable_updates(cascading_field = ...)] cannot be combined with `cascading_listener`, `post_update_listener`, `entity_aggregate` or `entity_compute`: those run per changed owner and would not see cascading values that a move changes in other owners' lists",
        );
    }
    Ok(())
}

fn generate_cascade_hooks(config: &ShadowConfig, context: &CascadeContext<'_>) -> TokenStream {
    let span = proc_macro2::Span::call_site();
    let CascadeContext {
        solution_path,
        element_accessor,
        element_mut_accessor,
    } = context;
    let field = Ident::new(config.cascading_field.as_deref().unwrap(), span);
    let compute = Ident::new(config.cascading_compute.as_deref().unwrap(), span);
    let inverse = Ident::new(config.inverse_field.as_deref().unwrap(), span);
    let next = Ident::new(config.next_field.as_deref().unwrap(), span);
    let dependencies = config
        .cascading_dependencies
        .as_deref()
        .map(|method| {
            let method = Ident::new(method, span);
            quote! { dependencies.extend(solution.#method(element)); }
        })
        .unwrap_or_default();
    quote! {
        fn cascading_element_count(solution: &Self) -> usize {
            #solution_path::#element_accessor(solution).len()
        }

        fn cascading_assigned(solution: &Self, element: usize) -> bool {
            #solution_path::#element_accessor(solution)[element].#inverse.is_some()
        }

        fn cascading_next(solution: &Self, element: usize) -> ::core::option::Option<usize> {
            #solution_path::#element_accessor(solution)[element].#next
        }

        #[allow(unused_variables)]
        fn cascading_dependencies(
            solution: &Self,
            element: usize,
            dependencies: &mut ::std::vec::Vec<usize>,
        ) {
            #dependencies
        }

        fn cascading_unset(solution: &Self, element: usize) -> bool {
            #solution_path::#element_accessor(solution)[element].#field.is_none()
        }

        fn update_cascading_shadow<W: ::core::ops::FnOnce(&Self)>(
            solution: &mut Self,
            element: usize,
            computable: bool,
            before_write: W,
        ) -> bool {
            let value = if computable {
                solution.#compute(element)
            } else {
                ::core::option::Option::None
            };
            if #solution_path::#element_accessor(solution)[element].#field == value {
                return false;
            }
            before_write(solution);
            #solution_path::#element_mut_accessor(solution)[element].#field = value;
            true
        }
    }
}

/* Shadow methods for a model with a cascading shadow: the owner update
refreshes list structure only, `update_entity_shadows` then propagates the
cascading values from the owner's elements, and `update_all_shadows`
refreshes every owner's structure before recomputing every value.
*/
fn generate_cascading_shadow_methods(
    generated: &GeneratedListShadowUpdate,
    solution_path: &TokenStream,
    descriptor_index: usize,
    owner_accessor: &Ident,
) -> TokenStream {
    let list_update = &generated.update;
    let list_reset = &generated.reset;
    let list_elements = &generated.elements;
    let cascade_hooks = &generated.cascade;
    let list_variable = &generated.list_variable;
    quote! {
        fn update_list_structure_shadows(
            solution: &mut Self,
            descriptor_index: usize,
            entity_index: usize,
        ) -> bool {
            let mut updated = false;
            #list_update
            updated
        }

        fn update_entity_shadows(
            solution: &mut Self,
            descriptor_index: usize,
            entity_index: usize,
        ) -> bool {
            let updated = <Self as ::solverforge::__internal::PlanningModelSupport>::update_list_structure_shadows(
                solution,
                descriptor_index,
                entity_index,
            );
            if updated {
                let changed = #solution_path::#owner_accessor(solution)[entity_index]
                    .#list_variable
                    .to_vec();
                ::solverforge::__internal::CascadePropagation::new(&*solution)
                    .propagate(solution, changed, |_, _| {});
            }
            updated
        }

        fn update_all_shadows(solution: &mut Self) -> bool {
            #list_reset
            for entity_index in 0..#solution_path::#owner_accessor(solution).len() {
                let _ = <Self as ::solverforge::__internal::PlanningModelSupport>::update_list_structure_shadows(
                    solution,
                    #descriptor_index,
                    entity_index,
                );
            }
            ::solverforge::__internal::CascadePropagation::refresh_all(solution);
            true
        }

        #list_elements

        #cascade_hooks
    }
}
