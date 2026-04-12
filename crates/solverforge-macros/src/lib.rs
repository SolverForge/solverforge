// Macros for SolverForge domain models.

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, ItemStruct};

mod attr_parse;
mod list_registry;
mod planning_entity;
mod planning_solution;
mod problem_fact;

use attr_parse::{has_serde_flag, parse_solution_flags};

#[proc_macro_attribute]
pub fn planning_entity(attr: TokenStream, item: TokenStream) -> TokenStream {
    let has_serde = has_serde_flag(attr);
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
        #[derive(Clone, Debug, PartialEq, Eq, Hash, #serde_derives ::solverforge::PlanningEntityImpl)]
        #(#attrs)*
        #vis struct #name #generics #fields
    };
    expanded.into()
}

#[proc_macro_attribute]
pub fn planning_solution(attr: TokenStream, item: TokenStream) -> TokenStream {
    let (has_serde, constraints_path, config_path, solver_toml_path) = parse_solution_flags(attr);
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

    let constraints_attr =
        constraints_path.map(|p| quote! { #[solverforge_constraints_path = #p] });
    let config_attr = config_path.map(|p| quote! { #[solverforge_config_path = #p] });
    let solver_toml_attr =
        solver_toml_path.map(|p| quote! { #[solverforge_solver_toml_path = #p] });

    let expanded = quote! {
        #[derive(Clone, Debug, #serde_derives ::solverforge::PlanningSolutionImpl)]
        #constraints_attr
        #config_attr
        #solver_toml_attr
        #(#attrs)*
        #vis struct #name #generics #fields
    };
    expanded.into()
}

#[proc_macro_attribute]
pub fn problem_fact(attr: TokenStream, item: TokenStream) -> TokenStream {
    let has_serde = has_serde_flag(attr);
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
        #[derive(Clone, Debug, PartialEq, Eq, #serde_derives ::solverforge::ProblemFactImpl)]
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
        planning_list_element_collection,
        problem_fact_collection,
        planning_score,
        value_range_provider,
        shadow_variable_updates,
        solverforge_constraints_path,
        solverforge_config_path,
        solverforge_solver_toml_path
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
