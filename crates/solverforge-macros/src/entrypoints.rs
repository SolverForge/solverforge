use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, ItemFn, ItemStruct};

use crate::attr_validation::{parse_serde_flag, parse_solution_flags};

pub(crate) fn planning_entity_attr(attr: TokenStream, item: TokenStream) -> TokenStream {
    let has_serde = match parse_serde_flag(attr.into(), "planning_entity") {
        Ok(has_serde) => has_serde,
        Err(error) => return error.to_compile_error().into(),
    };
    let input = parse_macro_input!(item as ItemStruct);
    let name = &input.ident;
    let vis = &input.vis;
    let generics = &input.generics;
    let attrs: Vec<_> = input.attrs.iter().collect();
    let fields = &input.fields;

    let serde_derives = if has_serde {
        quote! { ::serde::Serialize, ::serde::Deserialize, }
    } else {
        quote! {}
    };

    let expanded = quote! {
        #[derive(Clone, Debug, PartialEq, Eq, Hash, #serde_derives ::solverforge::__internal::PlanningEntityImpl)]
        #(#attrs)*
        #vis struct #name #generics #fields
    };
    expanded.into()
}

pub(crate) fn planning_solution_attr(attr: TokenStream, item: TokenStream) -> TokenStream {
    let flags = match parse_solution_flags(attr.into()) {
        Ok(flags) => flags,
        Err(error) => return error.to_compile_error().into(),
    };
    let input = parse_macro_input!(item as ItemStruct);
    let name = &input.ident;
    let vis = &input.vis;
    let generics = &input.generics;
    let attrs: Vec<_> = input.attrs.iter().collect();
    let fields = &input.fields;

    let serde_derives = if flags.has_serde {
        quote! { ::serde::Serialize, ::serde::Deserialize, }
    } else {
        quote! {}
    };

    let constraints_attr = flags
        .constraints_path
        .map(|p| quote! { #[solverforge_constraints_path = #p] });
    let config_attr = flags
        .config_path
        .map(|p| quote! { #[solverforge_config_path = #p] });
    let solver_toml_attr = flags
        .solver_toml_path
        .map(|p| quote! { #[solverforge_solver_toml_path = #p] });
    let search_attr = flags
        .search_path
        .map(|p| quote! { #[solverforge_search_path = #p] });
    let conflict_repairs_attr = flags
        .conflict_repairs_path
        .map(|p| quote! { #[solverforge_conflict_repairs_path = #p] });
    let scalar_groups_attr = flags
        .scalar_groups_path
        .map(|p| quote! { #[solverforge_scalar_groups_path = #p] });

    let expanded = quote! {
        #[derive(Clone, Debug, #serde_derives ::solverforge::__internal::PlanningSolutionImpl)]
        #constraints_attr
        #config_attr
        #solver_toml_attr
        #search_attr
        #conflict_repairs_attr
        #scalar_groups_attr
        #(#attrs)*
        #vis struct #name #generics #fields
    };
    expanded.into()
}

pub(crate) fn problem_fact_attr(attr: TokenStream, item: TokenStream) -> TokenStream {
    let has_serde = match parse_serde_flag(attr.into(), "problem_fact") {
        Ok(has_serde) => has_serde,
        Err(error) => return error.to_compile_error().into(),
    };
    let input = parse_macro_input!(item as ItemStruct);
    let name = &input.ident;
    let vis = &input.vis;
    let generics = &input.generics;
    let attrs: Vec<_> = input.attrs.iter().collect();
    let fields = &input.fields;

    let serde_derives = if has_serde {
        quote! { ::serde::Serialize, ::serde::Deserialize, }
    } else {
        quote! {}
    };

    let expanded = quote! {
        #[derive(Clone, Debug, PartialEq, Eq, #serde_derives ::solverforge::__internal::ProblemFactImpl)]
        #(#attrs)*
        #vis struct #name #generics #fields
    };
    expanded.into()
}

pub(crate) fn solverforge_constraints_attr(attr: TokenStream, item: TokenStream) -> TokenStream {
    if !attr.is_empty() {
        return syn::Error::new(
            proc_macro2::Span::call_site(),
            "solverforge_constraints does not accept arguments",
        )
        .to_compile_error()
        .into();
    }
    let input = parse_macro_input!(item as ItemFn);
    crate::constraints::expand(input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

pub(crate) fn planning_model_macro(input: TokenStream) -> TokenStream {
    crate::planning_model::expand(input.into())
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

pub(crate) fn derive_planning_entity(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    crate::planning_entity::expand_derive(input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

pub(crate) fn derive_planning_solution(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    crate::planning_solution::expand_derive(input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

pub(crate) fn derive_problem_fact(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    crate::problem_fact::expand_derive(input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}
