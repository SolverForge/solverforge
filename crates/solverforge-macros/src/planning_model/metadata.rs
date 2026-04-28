
fn parse_solution(module: &ModuleSource, item_struct: &ItemStruct) -> Result<SolutionMetadata> {
    let fields = named_fields(item_struct, "#[planning_solution] requires named fields")?;
    let mut collections = Vec::new();
    let mut collection_field_names = BTreeSet::new();
    let mut descriptor_index = 0usize;

    for field in fields {
        let Some(field_ident) = field.ident.clone() else {
            continue;
        };
        let field_name = field_ident.to_string();
        if has_attribute(&field.attrs, "planning_entity_collection")
            || has_attribute(&field.attrs, "problem_fact_collection")
            || has_attribute(&field.attrs, "planning_list_element_collection")
        {
            collection_field_names.insert(field_name.clone());
        }
        if has_attribute(&field.attrs, "planning_entity_collection") {
            let type_name = collection_type_name(&field.ty).ok_or_else(|| {
                Error::new_spanned(
                    field,
                    "#[planning_entity_collection] requires a Vec<T> field",
                )
            })?;
            collections.push(SolutionCollection {
                field_ident,
                field_name,
                type_name,
                descriptor_index: Some(descriptor_index),
            });
            descriptor_index += 1;
        } else if has_attribute(&field.attrs, "problem_fact_collection") {
            let type_name = collection_type_name(&field.ty).ok_or_else(|| {
                Error::new_spanned(field, "#[problem_fact_collection] requires a Vec<T> field")
            })?;
            collections.push(SolutionCollection {
                field_ident,
                field_name,
                type_name,
                descriptor_index: None,
            });
        }
    }

    Ok(SolutionMetadata {
        module_ident: module.ident.clone(),
        ident: item_struct.ident.clone(),
        collections,
        collection_field_names,
        shadow_config: parse_shadow_config(&item_struct.attrs),
    })
}

fn parse_shadow_config(attrs: &[Attribute]) -> ShadowConfig {
    let mut config = ShadowConfig::default();
    if let Some(attr) = get_attribute(attrs, "shadow_variable_updates") {
        config.list_owner = parse_attribute_string(attr, "list_owner");
        config.inverse_field = parse_attribute_string(attr, "inverse_field");
        config.previous_field = parse_attribute_string(attr, "previous_field");
        config.next_field = parse_attribute_string(attr, "next_field");
        config.cascading_listener = parse_attribute_string(attr, "cascading_listener");
        config.post_update_listener = parse_attribute_string(attr, "post_update_listener");
        config.entity_aggregates = parse_attribute_list(attr, "entity_aggregate");
        config.entity_computes = parse_attribute_list(attr, "entity_compute");
    }
    config
}

fn parse_entity(module: &ModuleSource, item_struct: &ItemStruct) -> Result<EntityMetadata> {
    let fields = named_fields(item_struct, "#[planning_entity] requires named fields")?;
    let mut scalar_variables = Vec::new();
    let mut list_variable_name = None;
    let mut list_element_collection = None;

    for field in fields {
        if has_attribute(&field.attrs, "planning_variable") {
            let Some(field_ident) = field.ident.as_ref() else {
                continue;
            };
            if !field_is_option_usize(&field.ty) {
                continue;
            }
            let attr = get_attribute(&field.attrs, "planning_variable").unwrap();
            if parse_attribute_bool(attr, "chained").unwrap_or(false) {
                continue;
            }
            scalar_variables.push(ScalarVariableMetadata {
                field_name: field_ident.to_string(),
                hooks: HookPaths {
                    candidate_values: parse_hook_path(
                        attr,
                        "candidate_values",
                        &module.ident,
                        field,
                    )?,
                    nearby_value_candidates: parse_hook_path(
                        attr,
                        "nearby_value_candidates",
                        &module.ident,
                        field,
                    )?,
                    nearby_entity_candidates: parse_hook_path(
                        attr,
                        "nearby_entity_candidates",
                        &module.ident,
                        field,
                    )?,
                    nearby_value_distance_meter: parse_hook_path(
                        attr,
                        "nearby_value_distance_meter",
                        &module.ident,
                        field,
                    )?,
                    nearby_entity_distance_meter: parse_hook_path(
                        attr,
                        "nearby_entity_distance_meter",
                        &module.ident,
                        field,
                    )?,
                    construction_entity_order_key: parse_hook_path(
                        attr,
                        "construction_entity_order_key",
                        &module.ident,
                        field,
                    )?,
                    construction_value_order_key: parse_hook_path(
                        attr,
                        "construction_value_order_key",
                        &module.ident,
                        field,
                    )?,
                },
            });
        }

        if has_attribute(&field.attrs, "planning_list_variable") {
            if let Some(field_ident) = field.ident.as_ref() {
                list_variable_name = Some(field_ident.to_string());
            }
            let attr = get_attribute(&field.attrs, "planning_list_variable").unwrap();
            let element_collection =
                parse_attribute_string(attr, "element_collection").ok_or_else(|| {
                    Error::new_spanned(
                        field,
                        "#[planning_list_variable] requires `element_collection = \"solution_field\"`",
                    )
                })?;
            list_element_collection = Some(element_collection);
        }
    }

    Ok(EntityMetadata {
        type_name: item_struct.ident.to_string(),
        scalar_variables,
        list_variable_name,
        list_element_collection,
    })
}

fn named_fields<'a>(
    item_struct: &'a ItemStruct,
    message: &'static str,
) -> Result<&'a syn::punctuated::Punctuated<syn::Field, syn::token::Comma>> {
    let Fields::Named(fields) = &item_struct.fields else {
        return Err(Error::new_spanned(item_struct, message));
    };
    Ok(&fields.named)
}

fn parse_hook_path(
    attr: &Attribute,
    key: &str,
    module_ident: &Ident,
    span: &impl ToTokens,
) -> Result<Option<syn::Path>> {
    let Some(raw) = parse_attribute_string(attr, key) else {
        return Ok(None);
    };
    let mut path: syn::Path = syn::parse_str(&raw)
        .map_err(|_| Error::new_spanned(span, format!("{key} must be a valid Rust path")))?;
    if path.leading_colon.is_none() && path.segments.len() == 1 {
        path = syn::parse_quote! { #module_ident::#path };
    }
    Ok(Some(path))
}

fn validate_collections(
    solution: &SolutionMetadata,
    entities: &BTreeMap<String, EntityMetadata>,
    facts: &BTreeSet<String>,
    aliases: &BTreeMap<String, String>,
) -> Result<()> {
    for collection in &solution.collections {
        let resolved_type_name = canonical_type_name(aliases, &collection.type_name);
        if collection.descriptor_index.is_some() {
            if !entities.contains_key(resolved_type_name) {
                return Err(Error::new_spanned(
                    &collection.field_ident,
                    format!(
                        "planning_model! entity collection `{}` references unknown #[planning_entity] type `{}`",
                        collection.field_name, collection.type_name,
                    ),
                ));
            }
        } else if !facts.contains(resolved_type_name) && !entities.contains_key(resolved_type_name)
        {
            return Err(Error::new_spanned(
                &collection.field_ident,
                format!(
                    "planning_model! problem fact collection `{}` references unknown #[problem_fact] type `{}`",
                    collection.field_name, collection.type_name,
                ),
            ));
        }
    }
    Ok(())
}

fn validate_list_element_sources(
    solution: &SolutionMetadata,
    entities: &BTreeMap<String, EntityMetadata>,
    aliases: &BTreeMap<String, String>,
) -> Result<()> {
    for collection in solution
        .collections
        .iter()
        .filter(|collection| collection.descriptor_index.is_some())
    {
        let resolved_type_name = canonical_type_name(aliases, &collection.type_name);
        let Some(entity) = entities.get(resolved_type_name) else {
            continue;
        };
        let Some(element_collection) = entity.list_element_collection.as_deref() else {
            continue;
        };
        if !solution.collection_field_names.contains(element_collection) {
            return Err(Error::new_spanned(
                &collection.field_ident,
                format!(
                    "planning_model! list entity `{}` requires a solution collection field named `{}`",
                    entity.type_name, element_collection,
                ),
            ));
        }
    }
    Ok(())
}

fn collection_type_name(ty: &Type) -> Option<String> {
    let inner = collection_inner_type(ty)?;
    type_name(inner)
}

fn collection_inner_type(ty: &Type) -> Option<&Type> {
    let Type::Path(type_path) = ty else {
        return None;
    };
    let segment = type_path.path.segments.last()?;
    if segment.ident != "Vec" {
        return None;
    }
    let syn::PathArguments::AngleBracketed(args) = &segment.arguments else {
        return None;
    };
    let Some(syn::GenericArgument::Type(inner)) = args.args.first() else {
        return None;
    };
    Some(inner)
}

fn type_name(ty: &Type) -> Option<String> {
    let Type::Path(type_path) = ty else {
        return None;
    };
    Some(type_path.path.segments.last()?.ident.to_string())
}

fn field_is_option_usize(ty: &Type) -> bool {
    let Type::Path(type_path) = ty else {
        return false;
    };
    let Some(segment) = type_path.path.segments.last() else {
        return false;
    };
    if segment.ident != "Option" {
        return false;
    }
    let syn::PathArguments::AngleBracketed(args) = &segment.arguments else {
        return false;
    };
    let Some(syn::GenericArgument::Type(Type::Path(inner))) = args.args.first() else {
        return false;
    };
    inner
        .path
        .segments
        .last()
        .is_some_and(|segment| segment.ident == "usize")
}
