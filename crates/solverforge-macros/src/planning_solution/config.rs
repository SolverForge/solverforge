use syn::{Error, Lit, LitStr, Meta, Result};

use crate::attr_parse::{get_attribute, parse_attribute_list, parse_attribute_string};

#[derive(Default)]
pub(super) struct ShadowConfig {
    pub(super) list_owner: Option<String>,
    pub(super) inverse_field: Option<String>,
    pub(super) index_field: Option<String>,
    pub(super) previous_field: Option<String>,
    pub(super) next_field: Option<String>,
    pub(super) cascading_listener: Option<String>,
    pub(super) post_update_listener: Option<String>,
    pub(super) entity_aggregates: Vec<String>,
    pub(super) entity_computes: Vec<String>,
}

fn hidden_string_attr(attrs: &[syn::Attribute], attr_name: &str) -> Option<LitStr> {
    for attr in attrs {
        if attr.path().is_ident(attr_name) {
            if let Meta::NameValue(nv) = &attr.meta {
                if let syn::Expr::Lit(expr_lit) = &nv.value {
                    if let Lit::Str(lit_str) = &expr_lit.lit {
                        return Some(lit_str.clone());
                    }
                }
            }
        }
    }
    None
}

/// Reads a hidden attribute that names a Rust item and parses it as a path,
/// reporting an unparsable value as a spanned error instead of panicking.
fn hidden_rust_path_attr(attrs: &[syn::Attribute], attr_name: &str) -> Result<Option<syn::Path>> {
    hidden_string_attr(attrs, attr_name)
        .map(|lit| {
            lit.parse::<syn::Path>().map_err(|_| {
                Error::new_spanned(
                    &lit,
                    format!("`{attr_name}` must be a valid Rust path string"),
                )
            })
        })
        .transpose()
}

pub(super) fn parse_constraints_path(attrs: &[syn::Attribute]) -> Result<Option<syn::Path>> {
    hidden_rust_path_attr(attrs, "solverforge_constraints_path")
}

pub(super) fn parse_config_path(attrs: &[syn::Attribute]) -> Result<Option<syn::Path>> {
    hidden_rust_path_attr(attrs, "solverforge_config_path")
}

pub(super) fn parse_solver_toml_path(attrs: &[syn::Attribute]) -> Option<String> {
    hidden_string_attr(attrs, "solverforge_solver_toml_path").map(|lit| lit.value())
}

pub(super) fn parse_search_path(attrs: &[syn::Attribute]) -> Result<Option<syn::Path>> {
    hidden_rust_path_attr(attrs, "solverforge_search_path")
}

pub(super) fn parse_conflict_repairs_path(attrs: &[syn::Attribute]) -> Result<Option<syn::Path>> {
    hidden_rust_path_attr(attrs, "solverforge_conflict_repairs_path")
}

pub(super) fn parse_scalar_groups_path(attrs: &[syn::Attribute]) -> Result<Option<syn::Path>> {
    hidden_rust_path_attr(attrs, "solverforge_scalar_groups_path")
}

pub(super) fn parse_shadow_config(attrs: &[syn::Attribute]) -> ShadowConfig {
    let mut config = ShadowConfig::default();

    if let Some(attr) = get_attribute(attrs, "shadow_variable_updates") {
        config.list_owner = parse_attribute_string(attr, "list_owner");
        config.inverse_field = parse_attribute_string(attr, "inverse_field");
        config.index_field = parse_attribute_string(attr, "index_field");
        config.previous_field = parse_attribute_string(attr, "previous_field");
        config.next_field = parse_attribute_string(attr, "next_field");
        config.cascading_listener = parse_attribute_string(attr, "cascading_listener");
        config.post_update_listener = parse_attribute_string(attr, "post_update_listener");
        config.entity_aggregates = parse_attribute_list(attr, "entity_aggregate");
        config.entity_computes = parse_attribute_list(attr, "entity_compute");
    }

    config
}
