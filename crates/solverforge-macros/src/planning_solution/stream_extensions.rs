use crate::attr_parse::has_attribute;
use proc_macro2::TokenStream;
use quote::quote;

use super::type_helpers::extract_collection_inner_type;

pub(super) fn generate_collection_source_methods(
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
) -> TokenStream {
    let entity_fields: Vec<_> = fields
        .iter()
        .filter(|f| has_attribute(&f.attrs, "planning_entity_collection"))
        .collect();

    let fact_fields: Vec<_> = fields
        .iter()
        .filter(|f| has_attribute(&f.attrs, "problem_fact_collection"))
        .collect();

    let list_element_fields: Vec<_> = fields
        .iter()
        .filter(|f| has_attribute(&f.attrs, "planning_list_element_collection"))
        .collect();

    let mut source_methods: Vec<TokenStream> = Vec::new();

    for (descriptor_index, f) in entity_fields.iter().enumerate() {
        let field_name = match f.ident.as_ref() {
            Some(n) => n,
            None => continue,
        };
        let element_type = match extract_collection_inner_type(&f.ty) {
            Some(t) => t,
            None => continue,
        };
        let descriptor_index_lit = syn::Index::from(descriptor_index);

        source_methods.push(quote! {
            pub fn #field_name() -> impl ::solverforge::stream::CollectionExtract<Self, Item = #element_type> {
                ::solverforge::__internal::source(
                    (|s: &Self| s.#field_name.as_slice()) as fn(&Self) -> &[#element_type],
                    ::solverforge::__internal::ChangeSource::Descriptor(#descriptor_index_lit),
                )
            }
        });
    }

    for f in fact_fields.iter() {
        let field_name = match f.ident.as_ref() {
            Some(n) => n,
            None => continue,
        };
        let element_type = match extract_collection_inner_type(&f.ty) {
            Some(t) => t,
            None => continue,
        };

        source_methods.push(quote! {
            pub fn #field_name() -> impl ::solverforge::stream::CollectionExtract<Self, Item = #element_type> {
                ::solverforge::__internal::source(
                    (|s: &Self| s.#field_name.as_slice()) as fn(&Self) -> &[#element_type],
                    ::solverforge::__internal::ChangeSource::Static,
                )
            }
        });
    }

    for f in list_element_fields.iter() {
        let field_name = match f.ident.as_ref() {
            Some(n) => n,
            None => continue,
        };
        let element_type = match extract_collection_inner_type(&f.ty) {
            Some(t) => t,
            None => continue,
        };

        source_methods.push(quote! {
            pub fn #field_name() -> impl ::solverforge::stream::CollectionExtract<Self, Item = #element_type> {
                ::solverforge::__internal::source(
                    (|s: &Self| s.#field_name.as_slice()) as fn(&Self) -> &[#element_type],
                    ::solverforge::__internal::ChangeSource::Static,
                )
            }
        });
    }

    quote! { #(#source_methods)* }
}
