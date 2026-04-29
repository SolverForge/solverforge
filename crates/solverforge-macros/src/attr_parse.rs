// Helpers for parsing proc-macro attribute arguments.

use proc_macro::TokenStream;
use syn::parse::Parser;
use syn::{Attribute, Expr, Lit, Meta, Path};

pub(crate) fn path_matches_ident(path: &Path, name: &str) -> bool {
    path.segments
        .last()
        .is_some_and(|segment| segment.ident == name)
}

// Checks if attribute stream contains the `serde` flag.
pub(crate) fn has_serde_flag(attr: TokenStream) -> bool {
    if attr.is_empty() {
        return false;
    }
    let parser = syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated;
    if let Ok(nested) = parser.parse(attr) {
        for meta in nested {
            if let Meta::Path(path) = meta {
                if path_matches_ident(&path, "serde") {
                    return true;
                }
            }
        }
    }
    false
}

// Parses planning_solution attribute flags: serde, constraints = "path",
// config = "path", solver_toml = "path", conflict_repair_providers = "path",
// scalar_groups = "path".
pub(crate) fn parse_solution_flags(
    attr: TokenStream,
) -> (
    bool,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
) {
    let mut has_serde = false;
    let mut constraints_path = None;
    let mut config_path = None;
    let mut solver_toml_path = None;
    let mut conflict_repair_providers_path = None;
    let mut scalar_groups_path = None;

    if attr.is_empty() {
        return (
            has_serde,
            constraints_path,
            config_path,
            solver_toml_path,
            conflict_repair_providers_path,
            scalar_groups_path,
        );
    }

    let parser = syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated;
    if let Ok(nested) = parser.parse(attr) {
        for meta in nested {
            match meta {
                Meta::Path(path) if path_matches_ident(&path, "serde") => {
                    has_serde = true;
                }
                Meta::NameValue(nv) if path_matches_ident(&nv.path, "constraints") => {
                    if let Expr::Lit(expr_lit) = &nv.value {
                        if let Lit::Str(lit_str) = &expr_lit.lit {
                            constraints_path = Some(lit_str.value());
                        }
                    }
                }
                Meta::NameValue(nv) if path_matches_ident(&nv.path, "config") => {
                    if let Expr::Lit(expr_lit) = &nv.value {
                        if let Lit::Str(lit_str) = &expr_lit.lit {
                            config_path = Some(lit_str.value());
                        }
                    }
                }
                Meta::NameValue(nv) if path_matches_ident(&nv.path, "solver_toml") => {
                    if let Expr::Lit(expr_lit) = &nv.value {
                        if let Lit::Str(lit_str) = &expr_lit.lit {
                            solver_toml_path = Some(lit_str.value());
                        }
                    }
                }
                Meta::NameValue(nv)
                    if path_matches_ident(&nv.path, "conflict_repair_providers") =>
                {
                    if let Expr::Lit(expr_lit) = &nv.value {
                        if let Lit::Str(lit_str) = &expr_lit.lit {
                            conflict_repair_providers_path = Some(lit_str.value());
                        }
                    }
                }
                Meta::NameValue(nv) if path_matches_ident(&nv.path, "scalar_groups") => {
                    if let Expr::Lit(expr_lit) = &nv.value {
                        if let Lit::Str(lit_str) = &expr_lit.lit {
                            scalar_groups_path = Some(lit_str.value());
                        }
                    }
                }
                _ => {}
            }
        }
    }

    (
        has_serde,
        constraints_path,
        config_path,
        solver_toml_path,
        conflict_repair_providers_path,
        scalar_groups_path,
    )
}

pub(crate) fn has_attribute(attrs: &[Attribute], name: &str) -> bool {
    attrs
        .iter()
        .any(|attr| path_matches_ident(attr.path(), name))
}

pub(crate) fn get_attribute<'a>(attrs: &'a [Attribute], name: &str) -> Option<&'a Attribute> {
    attrs
        .iter()
        .find(|attr| path_matches_ident(attr.path(), name))
}

pub(crate) fn has_attribute_argument(attr: &Attribute, key: &str) -> bool {
    if let Meta::List(meta_list) = &attr.meta {
        let parser = syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated;
        if let Ok(nested) = parser.parse2(meta_list.tokens.clone()) {
            return nested.into_iter().any(|meta| match meta {
                Meta::Path(path) => path_matches_ident(&path, key),
                Meta::NameValue(nv) => path_matches_ident(&nv.path, key),
                Meta::List(meta_list) => path_matches_ident(&meta_list.path, key),
            });
        }
    }
    false
}

pub(crate) fn attribute_argument_names(attr: &Attribute) -> Vec<String> {
    if let Meta::List(meta_list) = &attr.meta {
        let parser = syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated;
        if let Ok(nested) = parser.parse2(meta_list.tokens.clone()) {
            return nested
                .into_iter()
                .filter_map(|meta| {
                    let path = match meta {
                        Meta::Path(path) => path,
                        Meta::NameValue(nv) => nv.path,
                        Meta::List(meta_list) => meta_list.path,
                    };
                    path.segments
                        .last()
                        .map(|segment| segment.ident.to_string())
                })
                .collect();
        }
    }
    Vec::new()
}

pub(crate) fn parse_attribute_bool(attr: &Attribute, key: &str) -> Option<bool> {
    if let Meta::List(meta_list) = &attr.meta {
        let parser = syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated;
        if let Ok(nested) = parser.parse2(meta_list.tokens.clone()) {
            for meta in nested {
                if let Meta::NameValue(nv) = meta {
                    if path_matches_ident(&nv.path, key) {
                        if let Expr::Lit(expr_lit) = &nv.value {
                            if let Lit::Bool(lit_bool) = &expr_lit.lit {
                                return Some(lit_bool.value());
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

pub(crate) fn parse_attribute_string(attr: &Attribute, key: &str) -> Option<String> {
    if let Meta::List(meta_list) = &attr.meta {
        let parser = syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated;
        if let Ok(nested) = parser.parse2(meta_list.tokens.clone()) {
            for meta in nested {
                if let Meta::NameValue(nv) = meta {
                    if path_matches_ident(&nv.path, key) {
                        if let Expr::Lit(expr_lit) = &nv.value {
                            if let Lit::Str(lit_str) = &expr_lit.lit {
                                return Some(lit_str.value());
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

pub(crate) fn parse_attribute_list(attr: &Attribute, key: &str) -> Vec<String> {
    let mut result = Vec::new();
    if let Meta::List(meta_list) = &attr.meta {
        let parser = syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated;
        if let Ok(nested) = parser.parse2(meta_list.tokens.clone()) {
            for meta in nested {
                if let Meta::NameValue(nv) = meta {
                    if path_matches_ident(&nv.path, key) {
                        if let Expr::Lit(expr_lit) = &nv.value {
                            if let Lit::Str(lit_str) = &expr_lit.lit {
                                result.push(lit_str.value());
                            }
                        }
                    }
                }
            }
        }
    }
    result
}
