//! Implementation of the PlanningEntity derive macro.

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

/// Implementation of the `#[derive(PlanningEntity)]` macro.
pub fn derive_planning_entity_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    // TODO: Parse attributes and generate proper implementation
    // For now, generate a compile error to indicate this is not yet implemented

    let expanded = quote! {
        compile_error!("PlanningEntity derive macro is not yet implemented. \
            Please implement the PlanningEntity trait manually for now.");
    };

    // Suppress unused variable warning during development
    let _ = name;

    TokenStream::from(expanded)
}
