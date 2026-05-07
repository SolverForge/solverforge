// Helpers for parsing proc-macro attribute arguments.

use proc_macro2::TokenStream;
use syn::parse::Parser;
use syn::{Attribute, Error, Expr, Lit, Meta, Path};

pub(crate) fn path_matches_ident(path: &Path, name: &str) -> bool {
    path.segments
        .last()
        .is_some_and(|segment| segment.ident == name)
}

pub(crate) fn parse_meta_args(
    tokens: TokenStream,
) -> Result<syn::punctuated::Punctuated<Meta, syn::Token![,]>, Error> {
    let parser = syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated;
    parser.parse2(tokens)
}

pub(crate) fn lit_bool_value(expr: &Expr) -> Option<bool> {
    if let Expr::Lit(expr_lit) = expr {
        if let Lit::Bool(lit_bool) = &expr_lit.lit {
            return Some(lit_bool.value());
        }
    }
    None
}

pub(crate) fn lit_string_value(expr: &Expr) -> Option<String> {
    if let Expr::Lit(expr_lit) = expr {
        if let Lit::Str(lit_str) = &expr_lit.lit {
            return Some(lit_str.value());
        }
    }
    None
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

pub(crate) fn parse_attribute_bool(attr: &Attribute, key: &str) -> Option<bool> {
    if let Meta::List(meta_list) = &attr.meta {
        let parser = syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated;
        if let Ok(nested) = parser.parse2(meta_list.tokens.clone()) {
            for meta in nested {
                if let Meta::NameValue(nv) = meta {
                    if path_matches_ident(&nv.path, key) {
                        return lit_bool_value(&nv.value);
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
                        return lit_string_value(&nv.value);
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
                        if let Some(value) = lit_string_value(&nv.value) {
                            result.push(value);
                        }
                    }
                }
            }
        }
    }
    result
}
