//! Macros for SolverForge domain models.

use proc_macro::TokenStream;
use quote::quote;
use syn::parse::Parser;
use syn::{parse_macro_input, Attribute, DeriveInput, Expr, ItemStruct, Lit, Meta};

mod planning_entity;
mod planning_solution;
mod problem_fact;

#[proc_macro_attribute]
pub fn planning_entity(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    let name = &input.ident;
    let vis = &input.vis;
    let generics = &input.generics;
    let attrs: Vec<_> = input.attrs.iter().collect();
    let fields = &input.fields;

    let expanded = quote! {
        #[derive(Clone, Debug, PartialEq, Eq, Hash, ::solverforge::PlanningEntityImpl)]
        #(#attrs)*
        #vis struct #name #generics #fields
    };
    expanded.into()
}

#[proc_macro_attribute]
pub fn planning_solution(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    let name = &input.ident;
    let vis = &input.vis;
    let generics = &input.generics;
    let attrs: Vec<_> = input.attrs.iter().collect();
    let fields = &input.fields;

    // Parse constraints = "path" from attribute
    let constraints_attr = if !attr.is_empty() {
        let parser = syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated;
        if let Ok(nested) = parser.parse(attr) {
            let mut constraints_path = None;
            for meta in nested {
                if let Meta::NameValue(nv) = meta {
                    if nv.path.is_ident("constraints") {
                        if let Expr::Lit(expr_lit) = &nv.value {
                            if let Lit::Str(lit_str) = &expr_lit.lit {
                                constraints_path = Some(lit_str.value());
                            }
                        }
                    }
                }
            }
            constraints_path.map(|p| quote! { #[solverforge_constraints_path = #p] })
        } else {
            None
        }
    } else {
        None
    };

    let expanded = quote! {
        #[derive(Clone, Debug, ::solverforge::PlanningSolutionImpl)]
        #constraints_attr
        #(#attrs)*
        #vis struct #name #generics #fields
    };
    expanded.into()
}

#[proc_macro_attribute]
pub fn problem_fact(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    let name = &input.ident;
    let vis = &input.vis;
    let generics = &input.generics;
    let attrs: Vec<_> = input.attrs.iter().collect();
    let fields = &input.fields;

    let expanded = quote! {
        #[derive(Clone, Debug, PartialEq, Eq, ::solverforge::ProblemFactImpl)]
        #(#attrs)*
        #vis struct #name #generics #fields
    };
    expanded.into()
}

#[proc_macro_derive(
    PlanningEntityImpl,
    attributes(
        planning_id,
        planning_variable,
        planning_list_variable,
        planning_pin,
        inverse_relation_shadow_variable,
        previous_element_shadow_variable,
        next_element_shadow_variable,
        cascading_update_shadow_variable
    )
)]
pub fn derive_planning_entity(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    planning_entity::expand_derive(input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

#[proc_macro_derive(
    PlanningSolutionImpl,
    attributes(
        planning_entity_collection,
        problem_fact_collection,
        planning_score,
        value_range_provider,
        shadow_variable_updates,
        solverforge_constraints_path
    )
)]
pub fn derive_planning_solution(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    planning_solution::expand_derive(input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

#[proc_macro_derive(ProblemFactImpl, attributes(planning_id))]
pub fn derive_problem_fact(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    problem_fact::expand_derive(input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

fn has_attribute(attrs: &[Attribute], name: &str) -> bool {
    attrs.iter().any(|attr| attr.path().is_ident(name))
}

fn get_attribute<'a>(attrs: &'a [Attribute], name: &str) -> Option<&'a Attribute> {
    attrs.iter().find(|attr| attr.path().is_ident(name))
}

fn parse_attribute_bool(attr: &Attribute, key: &str) -> Option<bool> {
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

fn parse_attribute_string(attr: &Attribute, key: &str) -> Option<String> {
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
