use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::attr_parse::has_attribute;

use super::type_helpers::extract_collection_inner_type;

include!("list_operations/setup.rs");
include!("list_operations/quote.rs");

pub(super) fn generate_list_operations(
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
) -> TokenStream {
    __solverforge_list_setup!(
        fields,
        entity_collections,
        source_len_arms,
        source_element_arms,
        owner_helpers
    );
    let list_owner_count_terms: Vec<_> = entity_collections
        .iter()
        .map(|(_, _, entity_type)| quote! { #entity_type::__SOLVERFORGE_LIST_VARIABLE_COUNT })
        .collect();
    let total_list_entities_terms: Vec<_> = entity_collections
        .iter()
        .map(|(_, field_ident, entity_type)| {
            let field_name = field_ident.to_string();
            let list_trait =
                quote! { <#entity_type as ::solverforge::__internal::ListVariableEntity<Self>> };
            let n_entities_ident = format_ident!("__solverforge_n_entities_{}", field_name);
            quote! {
                if #list_trait::HAS_LIST_VARIABLE {
                    Self::#n_entities_ident(s)
                } else {
                    0
                }
            }
        })
        .collect();
    let total_list_elements_terms: Vec<_> = entity_collections
        .iter()
        .map(|(_, field_ident, entity_type)| {
            let field_name = field_ident.to_string();
            let list_trait =
                quote! { <#entity_type as ::solverforge::__internal::ListVariableEntity<Self>> };
            let element_count_ident = format_ident!("__solverforge_element_count_{}", field_name);
            quote! {
                if #list_trait::HAS_LIST_VARIABLE {
                    Self::#element_count_ident(s)
                } else {
                    0
                }
            }
        })
        .collect();
    __solverforge_list_quote!(
        owner_helpers,
        list_owner_count_terms,
        total_list_entities_terms,
        total_list_elements_terms
    )
}
