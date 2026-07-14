fn generate_shadow_methods(model: &ModelMetadata) -> Result<TokenStream> {
    let solution_module = &model.solution.module_ident;
    let solution_ident = &model.solution.ident;
    let solution_path = quote! { #solution_module::#solution_ident };
    let (list_update, list_descriptor) = generate_list_shadow_update(model, &solution_path)?;

    let Some((descriptor_index, collection_accessor)) = list_descriptor else {
        return Ok(quote! {
            fn update_entity_shadows(
                _solution: &mut Self,
                _descriptor_index: usize,
                _entity_index: usize,
            ) -> bool {
                false
            }

            fn update_all_shadows(_solution: &mut Self) -> bool {
                false
            }
        });
    };

    Ok(quote! {
        fn update_entity_shadows(
            solution: &mut Self,
            descriptor_index: usize,
            entity_index: usize,
        ) -> bool {
            let mut updated = false;
            #list_update
            updated
        }

        fn update_all_shadows(solution: &mut Self) -> bool {
            for entity_index in 0..#solution_path::#collection_accessor(solution).len() {
                let _ = <Self as ::solverforge::__internal::PlanningModelSupport>::update_entity_shadows(
                    solution,
                    #descriptor_index,
                    entity_index,
                );
            }
            true
        }
    })
}

fn generate_list_shadow_update(
    model: &ModelMetadata,
    solution_path: &TokenStream,
) -> Result<(TokenStream, Option<(usize, Ident)>)> {
    let config = &model.solution.shadow_config;
    if !list_shadow_updates_requested(config) {
        return Ok((TokenStream::new(), None));
    }

    let list_owner = config.list_owner.as_deref().ok_or_else(|| {
        Error::new(
            proc_macro2::Span::call_site(),
            "#[shadow_variable_updates(...)] requires `list_owner = \"entity_collection_field\"` when list shadow updates are configured",
        )
    })?;
    let owner_collection = model
        .solution
        .collections
        .iter()
        .find(|collection| {
            collection.field_name == list_owner && collection.descriptor_index.is_some()
        })
        .ok_or_else(|| {
            Error::new(
                proc_macro2::Span::call_site(),
                format!(
                    "#[shadow_variable_updates(list_owner = \"{list_owner}\")] must name a #[planning_entity_collection] field",
                ),
            )
        })?;
    let owner_ident = &owner_collection.field_ident;
    let owner_accessor = format_ident!("__solverforge_collection_{}", owner_ident);
    let owner_mut_accessor = format_ident!("__solverforge_collection_{}_mut", owner_ident);
    let descriptor_index = owner_collection.descriptor_index.unwrap();
    let entity_type_name = canonical_type_name(&model.aliases, &owner_collection.type_name);
    let entity = model
        .entities
        .get(entity_type_name)
        .expect("list owner entity should have been validated");
    let element_collection_name = entity.list_element_collection.as_deref().ok_or_else(|| {
        Error::new(
            proc_macro2::Span::call_site(),
            format!("list owner `{list_owner}` does not declare #[planning_list_variable]"),
        )
    })?;
    let list_variable_ident = entity
        .list_variable_name
        .as_deref()
        .map(|name| Ident::new(name, proc_macro2::Span::call_site()))
        .ok_or_else(|| {
            Error::new(
                proc_macro2::Span::call_site(),
                format!("list owner `{list_owner}` does not declare #[planning_list_variable]"),
            )
        })?;
    let element_collection = model
        .solution
        .collections
        .iter()
        .find(|collection| collection.field_name == element_collection_name)
        .ok_or_else(|| {
            Error::new(
                proc_macro2::Span::call_site(),
                format!(
                    "planning solution with list owner `{list_owner}` requires a collection field named `{element_collection_name}`",
                ),
            )
        })?;
    let element_accessor =
        format_ident!("__solverforge_collection_{}", element_collection.field_ident);
    let element_mut_accessor =
        format_ident!("__solverforge_collection_{}_mut", element_collection.field_ident);

    let inverse_update = config.inverse_field.as_ref().map(|field| {
        let field_ident = Ident::new(field, proc_macro2::Span::call_site());
        quote! {
            {
                let elements = #solution_path::#element_mut_accessor(solution);
                for &element_idx in &element_indices {
                    elements[element_idx].#field_ident = Some(entity_index);
                }
            }
        }
    });
    let previous_update = config.previous_field.as_ref().map(|field| {
        let field_ident = Ident::new(field, proc_macro2::Span::call_site());
        quote! {
            {
                let elements = #solution_path::#element_mut_accessor(solution);
                let mut previous = None;
                for &element_idx in &element_indices {
                    elements[element_idx].#field_ident = previous;
                    previous = Some(element_idx);
                }
            }
        }
    });
    let next_update = config.next_field.as_ref().map(|field| {
        let field_ident = Ident::new(field, proc_macro2::Span::call_site());
        quote! {
            {
                let elements = #solution_path::#element_mut_accessor(solution);
                for (offset, &element_idx) in element_indices.iter().enumerate() {
                    elements[element_idx].#field_ident = element_indices.get(offset + 1).copied();
                }
            }
        }
    });
    let cascading_update = config.cascading_listener.as_ref().map(|method| {
        let method_ident = Ident::new(method, proc_macro2::Span::call_site());
        quote! {
            for &element_idx in &element_indices {
                solution.#method_ident(element_idx);
            }
        }
    });
    let post_update = config.post_update_listener.as_ref().map(|method| {
        let method_ident = Ident::new(method, proc_macro2::Span::call_site());
        quote! { solution.#method_ident(entity_index); }
    });
    let aggregate_updates = generate_list_aggregate_updates(
        config,
        solution_path,
        &element_accessor,
        &owner_mut_accessor,
    );
    let compute_updates = generate_list_compute_updates(config, solution_path, &owner_mut_accessor);

    let update = quote! {
        if descriptor_index == #descriptor_index
            && entity_index < #solution_path::#owner_accessor(solution).len()
        {
            let element_indices = #solution_path::#owner_accessor(solution)[entity_index]
                .#list_variable_ident
                .to_vec();
            #inverse_update
            #previous_update
            #next_update
            #cascading_update
            #(#aggregate_updates)*
            #(#compute_updates)*
            #post_update
            updated = true;
        }
    };
    Ok((update, Some((descriptor_index, owner_accessor))))
}

fn generate_list_aggregate_updates(
    config: &ShadowConfig,
    solution_path: &TokenStream,
    element_accessor: &Ident,
    owner_mut_accessor: &Ident,
) -> Vec<TokenStream> {
    config
        .entity_aggregates
        .iter()
        .filter_map(|spec| {
            let parts = spec.split(':').collect::<Vec<_>>();
            if parts.len() != 3 || parts[1] != "sum" {
                return None;
            }
            let target = Ident::new(parts[0], proc_macro2::Span::call_site());
            let source = Ident::new(parts[2], proc_macro2::Span::call_site());
            Some(quote! {
                let aggregate_value = {
                    let elements = #solution_path::#element_accessor(solution);
                    element_indices.iter().map(|&index| elements[index].#source).sum()
                };
                #solution_path::#owner_mut_accessor(solution)[entity_index].#target = aggregate_value;
            })
        })
        .collect()
}

fn generate_list_compute_updates(
    config: &ShadowConfig,
    solution_path: &TokenStream,
    owner_mut_accessor: &Ident,
) -> Vec<TokenStream> {
    config
        .entity_computes
        .iter()
        .filter_map(|spec| {
            let parts = spec.split(':').collect::<Vec<_>>();
            if parts.len() != 2 {
                return None;
            }
            let target = Ident::new(parts[0], proc_macro2::Span::call_site());
            let method = Ident::new(parts[1], proc_macro2::Span::call_site());
            Some(quote! {
                let computed_value = solution.#method(entity_index);
                #solution_path::#owner_mut_accessor(solution)[entity_index].#target = computed_value;
            })
        })
        .collect()
}

fn list_shadow_updates_requested(config: &ShadowConfig) -> bool {
    config.inverse_field.is_some()
        || config.previous_field.is_some()
        || config.next_field.is_some()
        || config.cascading_listener.is_some()
        || config.post_update_listener.is_some()
        || !config.entity_aggregates.is_empty()
        || !config.entity_computes.is_empty()
}
