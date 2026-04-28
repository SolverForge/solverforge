fn validate_planning_variable_arguments(planning_variables: &[&syn::Field]) -> Result<(), Error> {
    const PLANNING_VARIABLE_ATTRIBUTES: &[&str] = &[
        "allows_unassigned",
        "chained",
        "value_range_provider",
        "countable_range",
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
        for argument in attribute_argument_names(attr) {
            if argument == "value_range" {
                return Err(Error::new_spanned(
                    *field,
                    "`value_range` is not a supported planning_variable argument; use `value_range_provider`",
                ));
            }
            if !PLANNING_VARIABLE_ATTRIBUTES.contains(&argument.as_str()) {
                return Err(Error::new_spanned(
                    *field,
                    format!("unsupported planning_variable argument `{argument}`"),
                ));
            }
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
