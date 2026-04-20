use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::attr_parse::has_attribute;

use super::type_helpers::extract_collection_inner_type;

pub(super) fn generate_list_operations(
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
) -> TokenStream {
    let list_owners: Vec<_> = fields
        .iter()
        .filter(|field| has_attribute(&field.attrs, "planning_entity_collection"))
        .filter_map(|field| {
            let field_ident = field.ident.as_ref()?;
            let entity_type = extract_collection_inner_type(&field.ty)?;
            Some((field_ident, entity_type))
        })
        .collect();

    if list_owners.is_empty() {
        return TokenStream::new();
    }

    let source_len_arms: Vec<_> = fields
        .iter()
        .filter(|field| {
            has_attribute(&field.attrs, "problem_fact_collection")
                || has_attribute(&field.attrs, "planning_entity_collection")
                || has_attribute(&field.attrs, "planning_list_element_collection")
        })
        .filter_map(|field| {
            let field_ident = field.ident.as_ref()?;
            let field_name = field_ident.to_string();
            Some(quote! { ::core::option::Option::Some(#field_name) => s.#field_ident.len(), })
        })
        .collect();

    let source_element_arms: Vec<_> = fields
        .iter()
        .filter(|field| {
            has_attribute(&field.attrs, "problem_fact_collection")
                || has_attribute(&field.attrs, "planning_entity_collection")
                || has_attribute(&field.attrs, "planning_list_element_collection")
        })
        .filter_map(|field| {
            let field_ident = field.ident.as_ref()?;
            let field_name = field_ident.to_string();
            let value_expr = if has_attribute(&field.attrs, "planning_list_element_collection") {
                quote! { s.#field_ident[idx] }
            } else {
                quote! { idx }
            };
            Some(quote! { ::core::option::Option::Some(#field_name) => { #value_expr } })
        })
        .collect();

    let owner_helpers: Vec<_> = list_owners
        .iter()
        .map(|(field_ident, entity_type)| {
            let field_name = field_ident.to_string();
            let list_trait =
                quote! { <#entity_type as ::solverforge::__internal::ListVariableEntity<Self>> };
            let list_len_ident = format_ident!("__solverforge_list_len_{}", field_name);
            let list_remove_ident = format_ident!("__solverforge_list_remove_{}", field_name);
            let list_insert_ident = format_ident!("__solverforge_list_insert_{}", field_name);
            let list_get_ident = format_ident!("__solverforge_list_get_{}", field_name);
            let list_set_ident = format_ident!("__solverforge_list_set_{}", field_name);
            let list_reverse_ident = format_ident!("__solverforge_list_reverse_{}", field_name);
            let sublist_remove_ident =
                format_ident!("__solverforge_sublist_remove_{}", field_name);
            let sublist_insert_ident =
                format_ident!("__solverforge_sublist_insert_{}", field_name);
            let ruin_remove_ident = format_ident!("__solverforge_ruin_remove_{}", field_name);
            let ruin_insert_ident = format_ident!("__solverforge_ruin_insert_{}", field_name);
            let list_remove_for_construction_ident =
                format_ident!("__solverforge_list_remove_for_construction_{}", field_name);
            let index_to_element_ident =
                format_ident!("__solverforge_index_to_element_{}", field_name);
            let element_count_ident = format_ident!("__solverforge_element_count_{}", field_name);
            let assigned_elements_ident =
                format_ident!("__solverforge_assigned_elements_{}", field_name);
            let n_entities_ident = format_ident!("__solverforge_n_entities_{}", field_name);

            quote! {
                #[inline]
                fn #list_len_ident(s: &Self, entity_idx: usize) -> usize {
                    s.#field_ident
                        .get(entity_idx)
                        .map_or(0, |entity| #list_trait::list_field(entity).len())
                }

                #[inline]
                fn #list_remove_ident(
                    s: &mut Self,
                    entity_idx: usize,
                    pos: usize,
                ) -> ::core::option::Option<usize> {
                    s.#field_ident
                        .get_mut(entity_idx)
                        .map(|entity| #list_trait::list_field_mut(entity).remove(pos))
                }

                #[inline]
                fn #list_insert_ident(s: &mut Self, entity_idx: usize, pos: usize, val: usize) {
                    if let Some(entity) = s.#field_ident.get_mut(entity_idx) {
                        #list_trait::list_field_mut(entity).insert(pos, val);
                    }
                }

                #[inline]
                fn #list_get_ident(s: &Self, entity_idx: usize, pos: usize) -> ::core::option::Option<usize> {
                    s.#field_ident
                        .get(entity_idx)
                        .and_then(|entity| #list_trait::list_field(entity).get(pos).copied())
                }

                #[inline]
                fn #list_set_ident(s: &mut Self, entity_idx: usize, pos: usize, val: usize) {
                    if let Some(entity) = s.#field_ident.get_mut(entity_idx) {
                        let list = #list_trait::list_field_mut(entity);
                        if pos < list.len() {
                            list[pos] = val;
                        }
                    }
                }

                #[inline]
                fn #list_reverse_ident(
                    s: &mut Self,
                    entity_idx: usize,
                    start: usize,
                    end: usize,
                ) {
                    if let Some(entity) = s.#field_ident.get_mut(entity_idx) {
                        #list_trait::list_field_mut(entity)[start..end].reverse();
                    }
                }

                #[inline]
                fn #sublist_remove_ident(
                    s: &mut Self,
                    entity_idx: usize,
                    start: usize,
                    end: usize,
                ) -> Vec<usize> {
                    s.#field_ident
                        .get_mut(entity_idx)
                        .map(|entity| #list_trait::list_field_mut(entity).drain(start..end).collect())
                        .unwrap_or_default()
                }

                #[inline]
                fn #sublist_insert_ident(
                    s: &mut Self,
                    entity_idx: usize,
                    pos: usize,
                    items: Vec<usize>,
                ) {
                    if let Some(entity) = s.#field_ident.get_mut(entity_idx) {
                        let list = #list_trait::list_field_mut(entity);
                        for (offset, item) in items.into_iter().enumerate() {
                            list.insert(pos + offset, item);
                        }
                    }
                }

                #[inline]
                fn #ruin_remove_ident(s: &mut Self, entity_idx: usize, pos: usize) -> usize {
                    #list_trait::list_field_mut(&mut s.#field_ident[entity_idx]).remove(pos)
                }

                #[inline]
                fn #ruin_insert_ident(s: &mut Self, entity_idx: usize, pos: usize, val: usize) {
                    #list_trait::list_field_mut(&mut s.#field_ident[entity_idx]).insert(pos, val);
                }

                #[inline]
                fn #list_remove_for_construction_ident(
                    s: &mut Self,
                    entity_idx: usize,
                    pos: usize,
                ) -> usize {
                    #list_trait::list_field_mut(&mut s.#field_ident[entity_idx]).remove(pos)
                }

                #[inline]
                fn #index_to_element_ident(s: &Self, idx: usize) -> usize {
                    let element_count = Self::#element_count_ident(s);
                    if idx >= element_count {
                        panic!(
                            "list element index {} is out of bounds for {} elements",
                            idx,
                            element_count
                        );
                    }

                    match #list_trait::LIST_ELEMENT_SOURCE {
                        #(#source_element_arms)*
                        ::core::option::Option::Some(source) => {
                            panic!(
                                "list source field `{}` was not found on the planning solution",
                                source
                            );
                        }
                        ::core::option::Option::None => idx,
                    }
                }

                #[inline]
                fn #element_count_ident(s: &Self) -> usize {
                    match #list_trait::LIST_ELEMENT_SOURCE {
                        #(#source_len_arms)*
                        ::core::option::Option::Some(source) => {
                            panic!(
                                "list source field `{}` was not found on the planning solution",
                                source
                            );
                        }
                        ::core::option::Option::None => 0,
                    }
                }

                #[inline]
                fn #assigned_elements_ident(s: &Self) -> Vec<usize> {
                    s.#field_ident
                        .iter()
                        .flat_map(|entity| #list_trait::list_field(entity).iter().copied())
                        .collect()
                }

                #[inline]
                fn #n_entities_ident(s: &Self) -> usize {
                    s.#field_ident.len()
                }
            }
        })
        .collect();

    let total_list_entities_terms: Vec<_> = list_owners
        .iter()
        .map(|(field_ident, entity_type)| {
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

    let total_list_elements_terms: Vec<_> = list_owners
        .iter()
        .map(|(field_ident, entity_type)| {
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

    quote! {
        #(#owner_helpers)*

        #[inline]
        fn __solverforge_total_list_entities(s: &Self) -> usize {
            0 #(+ #total_list_entities_terms)*
        }

        #[inline]
        fn __solverforge_total_list_elements(s: &Self) -> usize {
            0 #(+ #total_list_elements_terms)*
        }
    }
}
