fn validate_planning_entity_field_attributes(
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
) -> Result<(), Error> {
    for field in fields {
        if let Some(attr) = get_attribute(&field.attrs, "planning_id") {
            validate_no_attribute_args(attr, "planning_id")?;
        }
        if let Some(attr) = get_attribute(&field.attrs, "planning_pin") {
            validate_no_attribute_args(attr, "planning_pin")?;
        }
        if let Some(attr) = get_attribute(&field.attrs, "planning_variable") {
            validate_planning_variable_attribute(attr)?;
        }
        if let Some(attr) = get_attribute(&field.attrs, "planning_list_variable") {
            validate_planning_list_variable_attribute(attr)?;
        }
        if let Some(attr) = get_attribute(&field.attrs, "inverse_relation_shadow_variable") {
            validate_shadow_variable_attribute(attr, "inverse_relation_shadow_variable")?;
        }
        if let Some(attr) = get_attribute(&field.attrs, "previous_element_shadow_variable") {
            validate_shadow_variable_attribute(attr, "previous_element_shadow_variable")?;
        }
        if let Some(attr) = get_attribute(&field.attrs, "next_element_shadow_variable") {
            validate_shadow_variable_attribute(attr, "next_element_shadow_variable")?;
        }
        if let Some(attr) = get_attribute(&field.attrs, "cascading_update_shadow_variable") {
            validate_no_attribute_args(attr, "cascading_update_shadow_variable")?;
        }
    }
    Ok(())
}

fn validate_scalar_hook_targets(planning_variables: &[&syn::Field]) -> Result<(), Error> {
    const SCALAR_HOOK_ATTRIBUTES: &[&str] = &[
        "candidate_values",
        "nearby_value_candidates",
        "nearby_entity_candidates",
        "nearby_value_distance_meter",
        "nearby_entity_distance_meter",
        "construction_entity_order_key",
        "construction_value_order_key",
    ];

    for field in planning_variables {
        let attr = get_attribute(&field.attrs, "planning_variable").unwrap();
        let has_scalar_hook = SCALAR_HOOK_ATTRIBUTES
            .iter()
            .any(|key| has_attribute_argument(attr, key));
        if !has_scalar_hook {
            continue;
        }

        if parse_attribute_bool(attr, "chained").unwrap_or(false) {
            return Err(Error::new_spanned(
                *field,
                "chained planning variables cannot declare scalar runtime hook attributes",
            ));
        }

        if !field_is_option_usize(&field.ty) {
            return Err(Error::new_spanned(
                *field,
                "scalar runtime hook attributes require a non-chained Option<usize> planning variable",
            ));
        }
    }

    Ok(())
}
