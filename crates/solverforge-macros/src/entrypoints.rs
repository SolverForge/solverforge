use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::spanned::Spanned;
use syn::{parse_macro_input, parse_quote, Attribute, DeriveInput, ItemFn, ItemStruct, LitStr};

use crate::attr_validation::{parse_serde_flag, parse_solution_flags};

pub(crate) fn planning_entity_attr(attr: TokenStream, item: TokenStream) -> TokenStream {
    let has_serde = match parse_serde_flag(attr.into(), "planning_entity") {
        Ok(has_serde) => has_serde,
        Err(error) => return error.to_compile_error().into(),
    };
    let item = parse_macro_input!(item as ItemStruct);
    let serde_derives = serde_derives(has_serde);
    let derive: Attribute = parse_quote! {
        #[derive(Clone, Debug, PartialEq, Eq, Hash, #serde_derives ::solverforge::__internal::PlanningEntityImpl)]
    };
    emit_with_leading_attrs(item, vec![derive])
}

pub(crate) fn planning_solution_attr(attr: TokenStream, item: TokenStream) -> TokenStream {
    let flags = match parse_solution_flags(attr.into()) {
        Ok(flags) => flags,
        Err(error) => return error.to_compile_error().into(),
    };
    let item = parse_macro_input!(item as ItemStruct);
    let serde_derives = serde_derives(flags.has_serde);
    let mut leading: Vec<Attribute> = vec![parse_quote! {
        #[derive(Clone, Debug, #serde_derives ::solverforge::__internal::PlanningSolutionImpl)]
    }];
    leading.extend(flags.constraints_path.map(|p| {
        let p = hidden_path_literal(&p);
        parse_quote! { #[solverforge_constraints_path = #p] }
    }));
    leading.extend(flags.config_path.map(|p| {
        let p = hidden_path_literal(&p);
        parse_quote! { #[solverforge_config_path = #p] }
    }));
    leading.extend(
        flags
            .solver_toml_path
            .map(|p| parse_quote! { #[solverforge_solver_toml_path = #p] }),
    );
    leading.extend(flags.search_path.map(|p| {
        let p = hidden_path_literal(&p);
        parse_quote! { #[solverforge_search_path = #p] }
    }));
    leading.extend(flags.conflict_repairs_path.map(|p| {
        let p = hidden_path_literal(&p);
        parse_quote! { #[solverforge_conflict_repairs_path = #p] }
    }));
    leading.extend(flags.scalar_groups_path.map(|p| {
        let p = hidden_path_literal(&p);
        parse_quote! { #[solverforge_scalar_groups_path = #p] }
    }));
    emit_with_leading_attrs(item, leading)
}

pub(crate) fn problem_fact_attr(attr: TokenStream, item: TokenStream) -> TokenStream {
    let has_serde = match parse_serde_flag(attr.into(), "problem_fact") {
        Ok(has_serde) => has_serde,
        Err(error) => return error.to_compile_error().into(),
    };
    let item = parse_macro_input!(item as ItemStruct);
    let serde_derives = serde_derives(has_serde);
    let derive: Attribute = parse_quote! {
        #[derive(Clone, Debug, PartialEq, Eq, #serde_derives ::solverforge::__internal::ProblemFactImpl)]
    };
    emit_with_leading_attrs(item, vec![derive])
}

/// The derive reads the hidden path attributes as string literals; the
/// literal keeps the span of the user's argument.
fn hidden_path_literal(path: &syn::Path) -> LitStr {
    LitStr::new(&path.to_token_stream().to_string(), path.span())
}

fn serde_derives(has_serde: bool) -> proc_macro2::TokenStream {
    if has_serde {
        quote! { ::serde::Serialize, ::serde::Deserialize, }
    } else {
        quote! {}
    }
}

/// Re-emits the user's struct unchanged apart from the generated attributes
/// placed in front of its own, so where clauses, the trailing `;` of tuple and
/// unit structs, and every other part of the item survive the attribute.
fn emit_with_leading_attrs(mut item: ItemStruct, mut leading: Vec<Attribute>) -> TokenStream {
    leading.append(&mut item.attrs);
    item.attrs = leading;
    quote!(#item).into()
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
