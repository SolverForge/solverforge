// Helpers for parsing proc-macro attribute arguments.

use proc_macro::TokenStream;
use syn::parse::Parser;
use syn::{Attribute, Expr, Lit, Meta};

// Checks if attribute stream contains the `serde` flag.
pub(crate) fn has_serde_flag(attr: TokenStream) -> bool {
    if attr.is_empty() {
        return false;
    }
    let parser = syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated;
    if let Ok(nested) = parser.parse(attr) {
        for meta in nested {
            if let Meta::Path(path) = meta {
                if path.is_ident("serde") {
                    return true;
                }
            }
        }
    }
    false
}

// Parses planning_solution attribute flags: serde, constraints = "path",
// config = "path", solver_toml = "path".
pub(crate) fn parse_solution_flags(
    attr: TokenStream,
) -> (bool, Option<String>, Option<String>, Option<String>) {
    let mut has_serde = false;
    let mut constraints_path = None;
    let mut config_path = None;
    let mut solver_toml_path = None;

    if attr.is_empty() {
        return (has_serde, constraints_path, config_path, solver_toml_path);
    }

    let parser = syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated;
    if let Ok(nested) = parser.parse(attr) {
        for meta in nested {
            match meta {
                Meta::Path(path) if path.is_ident("serde") => {
                    has_serde = true;
                }
                Meta::NameValue(nv) if nv.path.is_ident("constraints") => {
                    if let Expr::Lit(expr_lit) = &nv.value {
                        if let Lit::Str(lit_str) = &expr_lit.lit {
                            constraints_path = Some(lit_str.value());
                        }
                    }
                }
                Meta::NameValue(nv) if nv.path.is_ident("config") => {
                    if let Expr::Lit(expr_lit) = &nv.value {
                        if let Lit::Str(lit_str) = &expr_lit.lit {
                            config_path = Some(lit_str.value());
                        }
                    }
                }
                Meta::NameValue(nv) if nv.path.is_ident("solver_toml") => {
                    if let Expr::Lit(expr_lit) = &nv.value {
                        if let Lit::Str(lit_str) = &expr_lit.lit {
                            solver_toml_path = Some(lit_str.value());
                        }
                    }
                }
                _ => {}
            }
        }
    }

    (has_serde, constraints_path, config_path, solver_toml_path)
}

pub(crate) fn has_attribute(attrs: &[Attribute], name: &str) -> bool {
    attrs.iter().any(|attr| attr.path().is_ident(name))
}

pub(crate) fn get_attribute<'a>(attrs: &'a [Attribute], name: &str) -> Option<&'a Attribute> {
    attrs.iter().find(|attr| attr.path().is_ident(name))
}

pub(crate) fn parse_attribute_bool(attr: &Attribute, key: &str) -> Option<bool> {
    if let Meta::List(meta_list) = &attr.meta {
        let parser = syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated;
        if let Ok(nested) = parser.parse2(meta_list.tokens.clone()) {
            for meta in nested {
                if let Meta::NameValue(nv) = meta {
                    if nv.path.is_ident(key) {
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
                    if nv.path.is_ident(key) {
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
                    if nv.path.is_ident(key) {
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
