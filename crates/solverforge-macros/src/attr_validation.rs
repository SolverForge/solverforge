// Strict validation for user-authored proc-macro attribute arguments.

use std::collections::BTreeSet;

use proc_macro2::TokenStream;
use syn::{Attribute, Error, Meta};

use crate::attr_parse::{lit_bool_value, lit_string_value, parse_meta_args, path_matches_ident};

#[derive(Clone, Copy)]
enum AttributeArgKind {
    Flag,
    Bool,
    String,
}

#[derive(Clone, Copy)]
struct AttributeArgSpec {
    name: &'static str,
    kind: AttributeArgKind,
    repeatable: bool,
}

impl AttributeArgSpec {
    const fn flag(name: &'static str) -> Self {
        Self {
            name,
            kind: AttributeArgKind::Flag,
            repeatable: false,
        }
    }

    const fn bool(name: &'static str) -> Self {
        Self {
            name,
            kind: AttributeArgKind::Bool,
            repeatable: false,
        }
    }

    const fn string(name: &'static str) -> Self {
        Self {
            name,
            kind: AttributeArgKind::String,
            repeatable: false,
        }
    }

    const fn repeated_string(name: &'static str) -> Self {
        Self {
            name,
            kind: AttributeArgKind::String,
            repeatable: true,
        }
    }
}

const SERDE_FLAG_ARGS: &[AttributeArgSpec] = &[AttributeArgSpec::flag("serde")];

const SOLUTION_ARGS: &[AttributeArgSpec] = &[
    AttributeArgSpec::flag("serde"),
    AttributeArgSpec::string("constraints"),
    AttributeArgSpec::string("config"),
    AttributeArgSpec::string("solver_toml"),
    AttributeArgSpec::string("search"),
    AttributeArgSpec::string("conflict_repairs"),
    AttributeArgSpec::string("scalar_groups"),
];

const PLANNING_VARIABLE_ARGS: &[AttributeArgSpec] = &[
    AttributeArgSpec::bool("allows_unassigned"),
    AttributeArgSpec::bool("chained"),
    AttributeArgSpec::string("value_range_provider"),
    AttributeArgSpec::string("countable_range"),
    AttributeArgSpec::string("candidate_values"),
    AttributeArgSpec::string("nearby_value_candidates"),
    AttributeArgSpec::string("nearby_entity_candidates"),
    AttributeArgSpec::string("nearby_value_distance_meter"),
    AttributeArgSpec::string("nearby_entity_distance_meter"),
    AttributeArgSpec::string("construction_entity_order_key"),
    AttributeArgSpec::string("construction_value_order_key"),
];

const PLANNING_LIST_VARIABLE_ARGS: &[AttributeArgSpec] = &[
    AttributeArgSpec::string("element_collection"),
    AttributeArgSpec::string("distance_meter"),
    AttributeArgSpec::string("intra_distance_meter"),
    AttributeArgSpec::string("route_get_fn"),
    AttributeArgSpec::string("route_set_fn"),
    AttributeArgSpec::string("route_depot_fn"),
    AttributeArgSpec::string("route_distance_fn"),
    AttributeArgSpec::string("route_feasible_fn"),
    AttributeArgSpec::string("solution_trait"),
];

const SHADOW_VARIABLE_ARGS: &[AttributeArgSpec] =
    &[AttributeArgSpec::string("source_variable_name")];

const SHADOW_UPDATES_ARGS: &[AttributeArgSpec] = &[
    AttributeArgSpec::string("list_owner"),
    AttributeArgSpec::string("inverse_field"),
    AttributeArgSpec::string("previous_field"),
    AttributeArgSpec::string("next_field"),
    AttributeArgSpec::string("cascading_listener"),
    AttributeArgSpec::string("post_update_listener"),
    AttributeArgSpec::repeated_string("entity_aggregate"),
    AttributeArgSpec::repeated_string("entity_compute"),
];

const LIST_ELEMENT_COLLECTION_ARGS: &[AttributeArgSpec] = &[AttributeArgSpec::string("owner")];

pub(crate) fn parse_serde_flag(attr: TokenStream, macro_name: &str) -> Result<bool, Error> {
    if attr.is_empty() {
        return Ok(false);
    }
    let nested = parse_meta_args(attr)?;
    validate_meta_args(macro_name, nested.iter(), SERDE_FLAG_ARGS)?;
    Ok(nested
        .iter()
        .any(|meta| matches!(meta, Meta::Path(path) if path_matches_ident(path, "serde"))))
}

// Parses planning_solution attribute flags: serde, constraints = "path",
// config = "path", solver_toml = "path", search = "path",
// conflict_repairs = "path", scalar_groups = "path".
#[derive(Debug, Default)]
pub(crate) struct SolutionFlags {
    pub(crate) has_serde: bool,
    pub(crate) constraints_path: Option<String>,
    pub(crate) config_path: Option<String>,
    pub(crate) solver_toml_path: Option<String>,
    pub(crate) search_path: Option<String>,
    pub(crate) conflict_repairs_path: Option<String>,
    pub(crate) scalar_groups_path: Option<String>,
}

pub(crate) fn parse_solution_flags(attr: TokenStream) -> Result<SolutionFlags, Error> {
    let mut flags = SolutionFlags::default();

    if attr.is_empty() {
        return Ok(flags);
    }

    let nested = parse_meta_args(attr)?;
    validate_meta_args("planning_solution", nested.iter(), SOLUTION_ARGS)?;
    for meta in nested {
        match meta {
            Meta::Path(path) if path_matches_ident(&path, "serde") => {
                flags.has_serde = true;
            }
            Meta::NameValue(nv) if path_matches_ident(&nv.path, "constraints") => {
                flags.constraints_path = lit_string_value(&nv.value);
            }
            Meta::NameValue(nv) if path_matches_ident(&nv.path, "config") => {
                flags.config_path = lit_string_value(&nv.value);
            }
            Meta::NameValue(nv) if path_matches_ident(&nv.path, "solver_toml") => {
                flags.solver_toml_path = lit_string_value(&nv.value);
            }
            Meta::NameValue(nv) if path_matches_ident(&nv.path, "search") => {
                if let Some(path) = lit_string_value(&nv.value) {
                    validate_rust_path("search", &path, &nv)?;
                    flags.search_path = Some(path);
                }
            }
            Meta::NameValue(nv) if path_matches_ident(&nv.path, "conflict_repairs") => {
                flags.conflict_repairs_path = lit_string_value(&nv.value);
            }
            Meta::NameValue(nv) if path_matches_ident(&nv.path, "scalar_groups") => {
                flags.scalar_groups_path = lit_string_value(&nv.value);
            }
            _ => {}
        }
    }

    Ok(flags)
}

fn validate_rust_path(arg_name: &str, path: &str, meta: &syn::MetaNameValue) -> Result<(), Error> {
    syn::parse_str::<syn::Path>(path).map(|_| ()).map_err(|_| {
        Error::new_spanned(
            meta,
            format!("planning_solution argument `{arg_name}` must be a valid Rust path string"),
        )
    })
}

pub(crate) fn validate_planning_entity_attribute(attr: &Attribute) -> Result<(), Error> {
    validate_attribute_args(attr, "planning_entity", SERDE_FLAG_ARGS)
}

pub(crate) fn validate_problem_fact_attribute(attr: &Attribute) -> Result<(), Error> {
    validate_attribute_args(attr, "problem_fact", SERDE_FLAG_ARGS)
}

pub(crate) fn validate_planning_solution_attribute(attr: &Attribute) -> Result<(), Error> {
    validate_attribute_args(attr, "planning_solution", SOLUTION_ARGS)
}

pub(crate) fn validate_planning_variable_attribute(attr: &Attribute) -> Result<(), Error> {
    validate_attribute_args(attr, "planning_variable", PLANNING_VARIABLE_ARGS)
}

pub(crate) fn validate_planning_list_variable_attribute(attr: &Attribute) -> Result<(), Error> {
    validate_attribute_args(attr, "planning_list_variable", PLANNING_LIST_VARIABLE_ARGS)
}

pub(crate) fn validate_shadow_variable_attribute(
    attr: &Attribute,
    attr_name: &str,
) -> Result<(), Error> {
    validate_attribute_args(attr, attr_name, SHADOW_VARIABLE_ARGS)
}

pub(crate) fn validate_shadow_updates_attribute(attr: &Attribute) -> Result<(), Error> {
    validate_attribute_args(attr, "shadow_variable_updates", SHADOW_UPDATES_ARGS)
}

pub(crate) fn validate_list_element_collection_attribute(attr: &Attribute) -> Result<(), Error> {
    validate_attribute_args(
        attr,
        "planning_list_element_collection",
        LIST_ELEMENT_COLLECTION_ARGS,
    )
}

pub(crate) fn validate_no_attribute_args(attr: &Attribute, attr_name: &str) -> Result<(), Error> {
    if matches!(attr.meta, Meta::Path(_)) {
        return Ok(());
    }
    Err(Error::new_spanned(
        attr,
        format!("{attr_name} does not accept arguments"),
    ))
}

fn validate_attribute_args(
    attr: &Attribute,
    attr_name: &str,
    specs: &[AttributeArgSpec],
) -> Result<(), Error> {
    let Meta::List(meta_list) = &attr.meta else {
        if matches!(attr.meta, Meta::Path(_)) {
            return Ok(());
        }
        return Err(Error::new_spanned(
            attr,
            format!("{attr_name} arguments must be written inside parentheses"),
        ));
    };
    let nested = parse_meta_args(meta_list.tokens.clone())?;
    validate_meta_args(attr_name, nested.iter(), specs)
}

fn validate_meta_args<'a>(
    attr_name: &str,
    metas: impl IntoIterator<Item = &'a Meta>,
    specs: &[AttributeArgSpec],
) -> Result<(), Error> {
    let mut seen = BTreeSet::new();
    for meta in metas {
        let Some(name) = meta_arg_name(meta) else {
            return Err(Error::new_spanned(
                meta,
                format!("{attr_name} argument must be an identifier"),
            ));
        };
        let Some(spec) = specs.iter().find(|spec| spec.name == name) else {
            return Err(Error::new_spanned(
                meta,
                format!(
                    "unsupported {attr_name} argument `{name}`; supported arguments are {}",
                    supported_args(specs)
                ),
            ));
        };
        if !spec.repeatable && !seen.insert(name.clone()) {
            return Err(Error::new_spanned(
                meta,
                format!("duplicate {attr_name} argument `{name}`"),
            ));
        }
        validate_meta_arg_shape(attr_name, meta, spec)?;
    }
    Ok(())
}

fn validate_meta_arg_shape(
    attr_name: &str,
    meta: &Meta,
    spec: &AttributeArgSpec,
) -> Result<(), Error> {
    match spec.kind {
        AttributeArgKind::Flag => {
            if matches!(meta, Meta::Path(_)) {
                Ok(())
            } else {
                Err(Error::new_spanned(
                    meta,
                    format!("{attr_name} argument `{}` must be a flag", spec.name),
                ))
            }
        }
        AttributeArgKind::Bool => {
            if matches!(meta, Meta::NameValue(nv) if lit_bool_value(&nv.value).is_some()) {
                Ok(())
            } else {
                Err(Error::new_spanned(
                    meta,
                    format!(
                        "{attr_name} argument `{}` must be a boolean literal",
                        spec.name
                    ),
                ))
            }
        }
        AttributeArgKind::String => {
            if matches!(meta, Meta::NameValue(nv) if lit_string_value(&nv.value).is_some()) {
                Ok(())
            } else {
                Err(Error::new_spanned(
                    meta,
                    format!(
                        "{attr_name} argument `{}` must be a string literal",
                        spec.name
                    ),
                ))
            }
        }
    }
}

fn supported_args(specs: &[AttributeArgSpec]) -> String {
    specs
        .iter()
        .map(|spec| spec.name)
        .collect::<Vec<_>>()
        .join(", ")
}

fn meta_arg_name(meta: &Meta) -> Option<String> {
    let path = match meta {
        Meta::Path(path) => path,
        Meta::NameValue(nv) => &nv.path,
        Meta::List(meta_list) => &meta_list.path,
    };
    path.segments
        .last()
        .map(|segment| segment.ident.to_string())
}

#[cfg(test)]
#[path = "attr_validation_tests.rs"]
mod tests;
