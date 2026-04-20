use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::attr_parse::has_attribute;

use super::type_helpers::extract_collection_inner_type;

pub(super) fn generate_list_operations(
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
) -> TokenStream {
    let entity_collections: Vec<_> = fields
        .iter()
        .filter(|field| has_attribute(&field.attrs, "planning_entity_collection"))
        .enumerate()
        .filter_map(|(descriptor_index, field)| {
            let field_ident = field.ident.as_ref()?;
            let entity_type = extract_collection_inner_type(&field.ty)?;
            Some((descriptor_index, field_ident, entity_type))
        })
        .collect();

    if entity_collections.is_empty() {
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

    let owner_helpers: Vec<_> = entity_collections
        .iter()
        .map(|(_, field_ident, entity_type)| {
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
            let assign_element_ident =
                format_ident!("__solverforge_assign_element_{}", field_name);

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
                fn #list_get_ident(
                    s: &Self,
                    entity_idx: usize,
                    pos: usize,
                ) -> ::core::option::Option<usize> {
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

                #[inline]
                fn #assign_element_ident(s: &mut Self, entity_idx: usize, elem: usize) {
                    if let Some(entity) = s.#field_ident.get_mut(entity_idx) {
                        #list_trait::list_field_mut(entity).push(elem);
                    }
                }
            }
        })
        .collect();

    let owner_public_methods: Vec<_> = entity_collections
        .iter()
        .map(|(descriptor_index, field_ident, entity_type)| {
            let field_name = field_ident.to_string();
            let list_trait =
                quote! { <#entity_type as ::solverforge::__internal::ListVariableEntity<Self>> };
            let owner_guard = quote! {
                if !#list_trait::HAS_LIST_VARIABLE {
                    panic!(
                        "`{}` is not a planning list owner on this solution",
                        stringify!(#field_ident)
                    );
                }
            };
            let list_len_ident = format_ident!("__solverforge_list_len_{}", field_name);
            let list_remove_ident = format_ident!("__solverforge_list_remove_{}", field_name);
            let list_insert_ident = format_ident!("__solverforge_list_insert_{}", field_name);
            let list_get_ident = format_ident!("__solverforge_list_get_{}", field_name);
            let list_set_ident = format_ident!("__solverforge_list_set_{}", field_name);
            let list_reverse_ident = format_ident!("__solverforge_list_reverse_{}", field_name);
            let sublist_remove_ident = format_ident!("__solverforge_sublist_remove_{}", field_name);
            let sublist_insert_ident = format_ident!("__solverforge_sublist_insert_{}", field_name);
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
            let assign_element_ident = format_ident!("__solverforge_assign_element_{}", field_name);

            let owner_list_len_method = format_ident!("{}_list_len", field_name);
            let owner_list_len_static_method = format_ident!("{}_list_len_static", field_name);
            let owner_list_remove_method = format_ident!("{}_list_remove", field_name);
            let owner_list_insert_method = format_ident!("{}_list_insert", field_name);
            let owner_list_get_method = format_ident!("{}_list_get", field_name);
            let owner_list_set_method = format_ident!("{}_list_set", field_name);
            let owner_list_reverse_method = format_ident!("{}_list_reverse", field_name);
            let owner_sublist_remove_method = format_ident!("{}_sublist_remove", field_name);
            let owner_sublist_insert_method = format_ident!("{}_sublist_insert", field_name);
            let owner_ruin_remove_method = format_ident!("{}_ruin_remove", field_name);
            let owner_ruin_insert_method = format_ident!("{}_ruin_insert", field_name);
            let owner_list_remove_for_construction_method =
                format_ident!("{}_list_remove_for_construction", field_name);
            let owner_index_to_element_method =
                format_ident!("{}_index_to_element_static", field_name);
            let owner_descriptor_index_method =
                format_ident!("{}_list_variable_descriptor_index", field_name);
            let owner_element_count_method = format_ident!("{}_element_count", field_name);
            let owner_assigned_elements_method = format_ident!("{}_assigned_elements", field_name);
            let owner_n_entities_method = format_ident!("{}_n_entities", field_name);
            let owner_assign_element_method = format_ident!("{}_assign_element", field_name);

            let descriptor_index_lit = syn::LitInt::new(
                &descriptor_index.to_string(),
                proc_macro2::Span::call_site(),
            );

            quote! {
                #[inline]
                pub fn #owner_list_len_method(&self, entity_idx: usize) -> usize {
                    #owner_guard
                    Self::#list_len_ident(self, entity_idx)
                }

                #[inline]
                pub fn #owner_list_len_static_method(s: &Self, entity_idx: usize) -> usize {
                    #owner_guard
                    Self::#list_len_ident(s, entity_idx)
                }

                #[inline]
                pub fn #owner_list_remove_method(
                    s: &mut Self,
                    entity_idx: usize,
                    pos: usize,
                ) -> ::core::option::Option<usize> {
                    #owner_guard
                    Self::#list_remove_ident(s, entity_idx, pos)
                }

                #[inline]
                pub fn #owner_list_insert_method(
                    s: &mut Self,
                    entity_idx: usize,
                    pos: usize,
                    val: usize,
                ) {
                    #owner_guard
                    Self::#list_insert_ident(s, entity_idx, pos, val)
                }

                #[inline]
                pub fn #owner_list_get_method(
                    s: &Self,
                    entity_idx: usize,
                    pos: usize,
                ) -> ::core::option::Option<usize> {
                    #owner_guard
                    Self::#list_get_ident(s, entity_idx, pos)
                }

                #[inline]
                pub fn #owner_list_set_method(
                    s: &mut Self,
                    entity_idx: usize,
                    pos: usize,
                    val: usize,
                ) {
                    #owner_guard
                    Self::#list_set_ident(s, entity_idx, pos, val)
                }

                #[inline]
                pub fn #owner_list_reverse_method(
                    s: &mut Self,
                    entity_idx: usize,
                    start: usize,
                    end: usize,
                ) {
                    #owner_guard
                    Self::#list_reverse_ident(s, entity_idx, start, end)
                }

                #[inline]
                pub fn #owner_sublist_remove_method(
                    s: &mut Self,
                    entity_idx: usize,
                    start: usize,
                    end: usize,
                ) -> Vec<usize> {
                    #owner_guard
                    Self::#sublist_remove_ident(s, entity_idx, start, end)
                }

                #[inline]
                pub fn #owner_sublist_insert_method(
                    s: &mut Self,
                    entity_idx: usize,
                    pos: usize,
                    items: Vec<usize>,
                ) {
                    #owner_guard
                    Self::#sublist_insert_ident(s, entity_idx, pos, items)
                }

                #[inline]
                pub fn #owner_ruin_remove_method(
                    s: &mut Self,
                    entity_idx: usize,
                    pos: usize,
                ) -> usize {
                    #owner_guard
                    Self::#ruin_remove_ident(s, entity_idx, pos)
                }

                #[inline]
                pub fn #owner_ruin_insert_method(
                    s: &mut Self,
                    entity_idx: usize,
                    pos: usize,
                    val: usize,
                ) {
                    #owner_guard
                    Self::#ruin_insert_ident(s, entity_idx, pos, val)
                }

                #[inline]
                pub fn #owner_list_remove_for_construction_method(
                    s: &mut Self,
                    entity_idx: usize,
                    pos: usize,
                ) -> usize {
                    #owner_guard
                    Self::#list_remove_for_construction_ident(s, entity_idx, pos)
                }

                #[inline]
                pub fn #owner_index_to_element_method(s: &Self, idx: usize) -> usize {
                    #owner_guard
                    Self::#index_to_element_ident(s, idx)
                }

                #[inline]
                pub fn #owner_descriptor_index_method() -> usize {
                    #owner_guard
                    #descriptor_index_lit
                }

                #[inline]
                pub fn #owner_element_count_method(s: &Self) -> usize {
                    #owner_guard
                    Self::#element_count_ident(s)
                }

                #[inline]
                pub fn #owner_assigned_elements_method(s: &Self) -> Vec<usize> {
                    #owner_guard
                    Self::#assigned_elements_ident(s)
                }

                #[inline]
                pub fn #owner_n_entities_method(s: &Self) -> usize {
                    #owner_guard
                    Self::#n_entities_ident(s)
                }

                #[inline]
                pub fn #owner_assign_element_method(
                    s: &mut Self,
                    entity_idx: usize,
                    elem: usize,
                ) {
                    #owner_guard
                    Self::#assign_element_ident(s, entity_idx, elem)
                }
            }
        })
        .collect();

    let list_owner_count_terms: Vec<_> = entity_collections
        .iter()
        .map(|(_, _, entity_type)| quote! { #entity_type::__SOLVERFORGE_LIST_VARIABLE_COUNT })
        .collect();

    let single_owner_list_len_branches: Vec<_> = entity_collections
        .iter()
        .map(|(_, field_ident, entity_type)| {
            let field_name = field_ident.to_string();
            let list_trait =
                quote! { <#entity_type as ::solverforge::__internal::ListVariableEntity<Self>> };
            let list_len_ident = format_ident!("__solverforge_list_len_{}", field_name);
            quote! {
                if #list_trait::HAS_LIST_VARIABLE {
                    return Self::#list_len_ident(s, entity_idx);
                }
            }
        })
        .collect();

    let single_owner_list_remove_branches: Vec<_> = entity_collections
        .iter()
        .map(|(_, field_ident, entity_type)| {
            let field_name = field_ident.to_string();
            let list_trait =
                quote! { <#entity_type as ::solverforge::__internal::ListVariableEntity<Self>> };
            let list_remove_ident = format_ident!("__solverforge_list_remove_{}", field_name);
            quote! {
                if #list_trait::HAS_LIST_VARIABLE {
                    return Self::#list_remove_ident(s, entity_idx, pos);
                }
            }
        })
        .collect();

    let single_owner_list_insert_branches: Vec<_> = entity_collections
        .iter()
        .map(|(_, field_ident, entity_type)| {
            let field_name = field_ident.to_string();
            let list_trait =
                quote! { <#entity_type as ::solverforge::__internal::ListVariableEntity<Self>> };
            let list_insert_ident = format_ident!("__solverforge_list_insert_{}", field_name);
            quote! {
                if #list_trait::HAS_LIST_VARIABLE {
                    Self::#list_insert_ident(s, entity_idx, pos, val);
                    return;
                }
            }
        })
        .collect();

    let single_owner_list_get_branches: Vec<_> = entity_collections
        .iter()
        .map(|(_, field_ident, entity_type)| {
            let field_name = field_ident.to_string();
            let list_trait =
                quote! { <#entity_type as ::solverforge::__internal::ListVariableEntity<Self>> };
            let list_get_ident = format_ident!("__solverforge_list_get_{}", field_name);
            quote! {
                if #list_trait::HAS_LIST_VARIABLE {
                    return Self::#list_get_ident(s, entity_idx, pos);
                }
            }
        })
        .collect();

    let single_owner_list_set_branches: Vec<_> = entity_collections
        .iter()
        .map(|(_, field_ident, entity_type)| {
            let field_name = field_ident.to_string();
            let list_trait =
                quote! { <#entity_type as ::solverforge::__internal::ListVariableEntity<Self>> };
            let list_set_ident = format_ident!("__solverforge_list_set_{}", field_name);
            quote! {
                if #list_trait::HAS_LIST_VARIABLE {
                    Self::#list_set_ident(s, entity_idx, pos, val);
                    return;
                }
            }
        })
        .collect();

    let single_owner_list_reverse_branches: Vec<_> = entity_collections
        .iter()
        .map(|(_, field_ident, entity_type)| {
            let field_name = field_ident.to_string();
            let list_trait =
                quote! { <#entity_type as ::solverforge::__internal::ListVariableEntity<Self>> };
            let list_reverse_ident = format_ident!("__solverforge_list_reverse_{}", field_name);
            quote! {
                if #list_trait::HAS_LIST_VARIABLE {
                    Self::#list_reverse_ident(s, entity_idx, start, end);
                    return;
                }
            }
        })
        .collect();

    let single_owner_sublist_remove_branches: Vec<_> = entity_collections
        .iter()
        .map(|(_, field_ident, entity_type)| {
            let field_name = field_ident.to_string();
            let list_trait =
                quote! { <#entity_type as ::solverforge::__internal::ListVariableEntity<Self>> };
            let sublist_remove_ident = format_ident!("__solverforge_sublist_remove_{}", field_name);
            quote! {
                if #list_trait::HAS_LIST_VARIABLE {
                    return Self::#sublist_remove_ident(s, entity_idx, start, end);
                }
            }
        })
        .collect();

    let single_owner_sublist_insert_branches: Vec<_> = entity_collections
        .iter()
        .map(|(_, field_ident, entity_type)| {
            let field_name = field_ident.to_string();
            let list_trait =
                quote! { <#entity_type as ::solverforge::__internal::ListVariableEntity<Self>> };
            let sublist_insert_ident = format_ident!("__solverforge_sublist_insert_{}", field_name);
            quote! {
                if #list_trait::HAS_LIST_VARIABLE {
                    Self::#sublist_insert_ident(s, entity_idx, pos, items);
                    return;
                }
            }
        })
        .collect();

    let single_owner_ruin_remove_branches: Vec<_> = entity_collections
        .iter()
        .map(|(_, field_ident, entity_type)| {
            let field_name = field_ident.to_string();
            let list_trait =
                quote! { <#entity_type as ::solverforge::__internal::ListVariableEntity<Self>> };
            let ruin_remove_ident = format_ident!("__solverforge_ruin_remove_{}", field_name);
            quote! {
                if #list_trait::HAS_LIST_VARIABLE {
                    return Self::#ruin_remove_ident(s, entity_idx, pos);
                }
            }
        })
        .collect();

    let single_owner_ruin_insert_branches: Vec<_> = entity_collections
        .iter()
        .map(|(_, field_ident, entity_type)| {
            let field_name = field_ident.to_string();
            let list_trait =
                quote! { <#entity_type as ::solverforge::__internal::ListVariableEntity<Self>> };
            let ruin_insert_ident = format_ident!("__solverforge_ruin_insert_{}", field_name);
            quote! {
                if #list_trait::HAS_LIST_VARIABLE {
                    Self::#ruin_insert_ident(s, entity_idx, pos, val);
                    return;
                }
            }
        })
        .collect();

    let single_owner_remove_for_construction_branches: Vec<_> = entity_collections
        .iter()
        .map(|(_, field_ident, entity_type)| {
            let field_name = field_ident.to_string();
            let list_trait =
                quote! { <#entity_type as ::solverforge::__internal::ListVariableEntity<Self>> };
            let list_remove_for_construction_ident =
                format_ident!("__solverforge_list_remove_for_construction_{}", field_name);
            quote! {
                if #list_trait::HAS_LIST_VARIABLE {
                    return Self::#list_remove_for_construction_ident(s, entity_idx, pos);
                }
            }
        })
        .collect();

    let single_owner_index_to_element_branches: Vec<_> = entity_collections
        .iter()
        .map(|(_, field_ident, entity_type)| {
            let field_name = field_ident.to_string();
            let list_trait =
                quote! { <#entity_type as ::solverforge::__internal::ListVariableEntity<Self>> };
            let index_to_element_ident =
                format_ident!("__solverforge_index_to_element_{}", field_name);
            quote! {
                if #list_trait::HAS_LIST_VARIABLE {
                    return Self::#index_to_element_ident(s, idx);
                }
            }
        })
        .collect();

    let single_owner_descriptor_index_branches: Vec<_> = entity_collections
        .iter()
        .map(|(descriptor_index, _, entity_type)| {
            let descriptor_index_lit = syn::LitInt::new(
                &descriptor_index.to_string(),
                proc_macro2::Span::call_site(),
            );
            let list_trait =
                quote! { <#entity_type as ::solverforge::__internal::ListVariableEntity<Self>> };
            quote! {
                if #list_trait::HAS_LIST_VARIABLE {
                    return #descriptor_index_lit;
                }
            }
        })
        .collect();

    let single_owner_element_count_branches: Vec<_> = entity_collections
        .iter()
        .map(|(_, field_ident, entity_type)| {
            let field_name = field_ident.to_string();
            let list_trait =
                quote! { <#entity_type as ::solverforge::__internal::ListVariableEntity<Self>> };
            let element_count_ident = format_ident!("__solverforge_element_count_{}", field_name);
            quote! {
                if #list_trait::HAS_LIST_VARIABLE {
                    return Self::#element_count_ident(s);
                }
            }
        })
        .collect();

    let single_owner_assigned_elements_branches: Vec<_> = entity_collections
        .iter()
        .map(|(_, field_ident, entity_type)| {
            let field_name = field_ident.to_string();
            let list_trait =
                quote! { <#entity_type as ::solverforge::__internal::ListVariableEntity<Self>> };
            let assigned_elements_ident =
                format_ident!("__solverforge_assigned_elements_{}", field_name);
            quote! {
                if #list_trait::HAS_LIST_VARIABLE {
                    return Self::#assigned_elements_ident(s);
                }
            }
        })
        .collect();

    let single_owner_n_entities_branches: Vec<_> = entity_collections
        .iter()
        .map(|(_, field_ident, entity_type)| {
            let field_name = field_ident.to_string();
            let list_trait =
                quote! { <#entity_type as ::solverforge::__internal::ListVariableEntity<Self>> };
            let n_entities_ident = format_ident!("__solverforge_n_entities_{}", field_name);
            quote! {
                if #list_trait::HAS_LIST_VARIABLE {
                    return Self::#n_entities_ident(s);
                }
            }
        })
        .collect();

    let single_owner_assign_element_branches: Vec<_> = entity_collections
        .iter()
        .map(|(_, field_ident, entity_type)| {
            let field_name = field_ident.to_string();
            let list_trait =
                quote! { <#entity_type as ::solverforge::__internal::ListVariableEntity<Self>> };
            let assign_element_ident = format_ident!("__solverforge_assign_element_{}", field_name);
            quote! {
                if #list_trait::HAS_LIST_VARIABLE {
                    Self::#assign_element_ident(s, entity_idx, elem);
                    return;
                }
            }
        })
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

    quote! {
        #(#owner_helpers)*

        const __SOLVERFORGE_LIST_OWNER_COUNT: usize = 0 #(+ #list_owner_count_terms)*;

        #[inline]
        fn __solverforge_assert_single_list_owner() {
            assert!(
                Self::__SOLVERFORGE_LIST_OWNER_COUNT == 1,
                "single-owner list helper called on a solution with {} list owners",
                Self::__SOLVERFORGE_LIST_OWNER_COUNT,
            );
        }

        #(#owner_public_methods)*

        #[inline]
        pub fn list_len(&self, entity_idx: usize) -> usize {
            Self::list_len_static(self, entity_idx)
        }

        #[inline]
        pub fn list_len_static(s: &Self, entity_idx: usize) -> usize {
            Self::__solverforge_assert_single_list_owner();
            #(#single_owner_list_len_branches)*
            unreachable!("single-owner list helper called without a canonical list owner");
        }

        #[inline]
        pub fn list_remove(s: &mut Self, entity_idx: usize, pos: usize) -> ::core::option::Option<usize> {
            Self::__solverforge_assert_single_list_owner();
            #(#single_owner_list_remove_branches)*
            unreachable!("single-owner list helper called without a canonical list owner");
        }

        #[inline]
        pub fn list_insert(s: &mut Self, entity_idx: usize, pos: usize, val: usize) {
            Self::__solverforge_assert_single_list_owner();
            #(#single_owner_list_insert_branches)*
            unreachable!("single-owner list helper called without a canonical list owner");
        }

        #[inline]
        pub fn list_get(s: &Self, entity_idx: usize, pos: usize) -> ::core::option::Option<usize> {
            Self::__solverforge_assert_single_list_owner();
            #(#single_owner_list_get_branches)*
            unreachable!("single-owner list helper called without a canonical list owner");
        }

        #[inline]
        pub fn list_set(s: &mut Self, entity_idx: usize, pos: usize, val: usize) {
            Self::__solverforge_assert_single_list_owner();
            #(#single_owner_list_set_branches)*
            unreachable!("single-owner list helper called without a canonical list owner");
        }

        #[inline]
        pub fn list_reverse(s: &mut Self, entity_idx: usize, start: usize, end: usize) {
            Self::__solverforge_assert_single_list_owner();
            #(#single_owner_list_reverse_branches)*
            unreachable!("single-owner list helper called without a canonical list owner");
        }

        #[inline]
        pub fn sublist_remove(
            s: &mut Self,
            entity_idx: usize,
            start: usize,
            end: usize,
        ) -> Vec<usize> {
            Self::__solverforge_assert_single_list_owner();
            #(#single_owner_sublist_remove_branches)*
            unreachable!("single-owner list helper called without a canonical list owner");
        }

        #[inline]
        pub fn sublist_insert(
            s: &mut Self,
            entity_idx: usize,
            pos: usize,
            items: Vec<usize>,
        ) {
            Self::__solverforge_assert_single_list_owner();
            #(#single_owner_sublist_insert_branches)*
            unreachable!("single-owner list helper called without a canonical list owner");
        }

        #[inline]
        pub fn ruin_remove(s: &mut Self, entity_idx: usize, pos: usize) -> usize {
            Self::__solverforge_assert_single_list_owner();
            #(#single_owner_ruin_remove_branches)*
            unreachable!("single-owner list helper called without a canonical list owner");
        }

        #[inline]
        pub fn ruin_insert(s: &mut Self, entity_idx: usize, pos: usize, val: usize) {
            Self::__solverforge_assert_single_list_owner();
            #(#single_owner_ruin_insert_branches)*
            unreachable!("single-owner list helper called without a canonical list owner");
        }

        #[inline]
        pub fn list_remove_for_construction(s: &mut Self, entity_idx: usize, pos: usize) -> usize {
            Self::__solverforge_assert_single_list_owner();
            #(#single_owner_remove_for_construction_branches)*
            unreachable!("single-owner list helper called without a canonical list owner");
        }

        #[inline]
        pub fn index_to_element_static(s: &Self, idx: usize) -> usize {
            Self::__solverforge_assert_single_list_owner();
            #(#single_owner_index_to_element_branches)*
            unreachable!("single-owner list helper called without a canonical list owner");
        }

        #[inline]
        pub fn list_variable_descriptor_index() -> usize {
            Self::__solverforge_assert_single_list_owner();
            #(#single_owner_descriptor_index_branches)*
            unreachable!("single-owner list helper called without a canonical list owner");
        }

        #[inline]
        pub fn element_count(s: &Self) -> usize {
            Self::__solverforge_assert_single_list_owner();
            #(#single_owner_element_count_branches)*
            unreachable!("single-owner list helper called without a canonical list owner");
        }

        #[inline]
        pub fn assigned_elements(s: &Self) -> Vec<usize> {
            Self::__solverforge_assert_single_list_owner();
            #(#single_owner_assigned_elements_branches)*
            unreachable!("single-owner list helper called without a canonical list owner");
        }

        #[inline]
        pub fn n_entities(s: &Self) -> usize {
            Self::__solverforge_assert_single_list_owner();
            #(#single_owner_n_entities_branches)*
            unreachable!("single-owner list helper called without a canonical list owner");
        }

        #[inline]
        pub fn assign_element(s: &mut Self, entity_idx: usize, elem: usize) {
            Self::__solverforge_assert_single_list_owner();
            #(#single_owner_assign_element_branches)*
            unreachable!("single-owner list helper called without a canonical list owner");
        }

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
