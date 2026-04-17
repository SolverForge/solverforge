use proc_macro2::TokenStream;
use quote::quote;

use crate::attr_parse::has_attribute;

use super::type_helpers::extract_collection_inner_type;

pub(super) fn generate_list_operations(
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
) -> TokenStream {
    let list_owners: Vec<_> = fields
        .iter()
        .filter(|f| has_attribute(&f.attrs, "planning_entity_collection"))
        .enumerate()
        .filter_map(|(idx, field)| {
            let field_ident = field.ident.as_ref()?;
            let entity_type = extract_collection_inner_type(&field.ty)?;
            Some((idx, field_ident, entity_type))
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

    let index_to_element_branches: Vec<_> = list_owners
        .iter()
        .map(|(_, _, entity_type)| {
            let list_trait =
                quote! { <#entity_type as ::solverforge::__internal::ListVariableEntity<Self>> };
            quote! {
                if #list_trait::HAS_LIST_VARIABLE {
                    return match #list_trait::LIST_ELEMENT_SOURCE {
                        #(#source_element_arms)*
                        ::core::option::Option::Some(source) => {
                            panic!(
                                "list source field `{}` was not found on the planning solution",
                                source
                            );
                        }
                        ::core::option::Option::None => idx,
                    };
                }
            }
        })
        .collect();

    let list_len_branches: Vec<_> = list_owners
        .iter()
        .map(|(_, field_ident, entity_type)| {
            let list_trait =
                quote! { <#entity_type as ::solverforge::__internal::ListVariableEntity<Self>> };
            quote! {
                if #list_trait::HAS_LIST_VARIABLE {
                    return self
                        .#field_ident
                        .get(entity_idx)
                        .map_or(0, |entity| #list_trait::list_field(entity).len());
                }
            }
        })
        .collect();

    let list_len_static_branches: Vec<_> = list_owners
        .iter()
        .map(|(_, field_ident, entity_type)| {
            let list_trait =
                quote! { <#entity_type as ::solverforge::__internal::ListVariableEntity<Self>> };
            quote! {
                if #list_trait::HAS_LIST_VARIABLE {
                    return s
                        .#field_ident
                        .get(entity_idx)
                        .map_or(0, |entity| #list_trait::list_field(entity).len());
                }
            }
        })
        .collect();

    let list_remove_branches: Vec<_> = list_owners
        .iter()
        .map(|(_, field_ident, entity_type)| {
            let list_trait =
                quote! { <#entity_type as ::solverforge::__internal::ListVariableEntity<Self>> };
            quote! {
                if #list_trait::HAS_LIST_VARIABLE {
                    return s
                        .#field_ident
                        .get_mut(entity_idx)
                        .map(|entity| #list_trait::list_field_mut(entity).remove(pos));
                }
            }
        })
        .collect();

    let list_insert_branches: Vec<_> = list_owners
        .iter()
        .map(|(_, field_ident, entity_type)| {
            let list_trait =
                quote! { <#entity_type as ::solverforge::__internal::ListVariableEntity<Self>> };
            quote! {
                if #list_trait::HAS_LIST_VARIABLE {
                    if let Some(entity) = s.#field_ident.get_mut(entity_idx) {
                        #list_trait::list_field_mut(entity).insert(pos, val);
                    }
                    return;
                }
            }
        })
        .collect();

    let list_get_branches: Vec<_> = list_owners
        .iter()
        .map(|(_, field_ident, entity_type)| {
            let list_trait =
                quote! { <#entity_type as ::solverforge::__internal::ListVariableEntity<Self>> };
            quote! {
                if #list_trait::HAS_LIST_VARIABLE {
                    return s
                        .#field_ident
                        .get(entity_idx)
                        .and_then(|entity| #list_trait::list_field(entity).get(pos).copied());
                }
            }
        })
        .collect();

    let list_set_branches: Vec<_> = list_owners
        .iter()
        .map(|(_, field_ident, entity_type)| {
            let list_trait =
                quote! { <#entity_type as ::solverforge::__internal::ListVariableEntity<Self>> };
            quote! {
                if #list_trait::HAS_LIST_VARIABLE {
                    if let Some(entity) = s.#field_ident.get_mut(entity_idx) {
                        let list = #list_trait::list_field_mut(entity);
                        if pos < list.len() {
                            list[pos] = val;
                        }
                    }
                    return;
                }
            }
        })
        .collect();

    let list_reverse_branches: Vec<_> = list_owners
        .iter()
        .map(|(_, field_ident, entity_type)| {
            let list_trait =
                quote! { <#entity_type as ::solverforge::__internal::ListVariableEntity<Self>> };
            quote! {
                if #list_trait::HAS_LIST_VARIABLE {
                    if let Some(entity) = s.#field_ident.get_mut(entity_idx) {
                        #list_trait::list_field_mut(entity)[start..end].reverse();
                    }
                    return;
                }
            }
        })
        .collect();

    let sublist_remove_branches: Vec<_> = list_owners
        .iter()
        .map(|(_, field_ident, entity_type)| {
            let list_trait =
                quote! { <#entity_type as ::solverforge::__internal::ListVariableEntity<Self>> };
            quote! {
                if #list_trait::HAS_LIST_VARIABLE {
                    return s
                        .#field_ident
                        .get_mut(entity_idx)
                        .map(|entity| #list_trait::list_field_mut(entity).drain(start..end).collect())
                        .unwrap_or_default();
                }
            }
        })
        .collect();

    let sublist_insert_branches: Vec<_> = list_owners
        .iter()
        .map(|(_, field_ident, entity_type)| {
            let list_trait =
                quote! { <#entity_type as ::solverforge::__internal::ListVariableEntity<Self>> };
            quote! {
                if #list_trait::HAS_LIST_VARIABLE {
                    if let Some(entity) = s.#field_ident.get_mut(entity_idx) {
                        let list = #list_trait::list_field_mut(entity);
                        for (i, item) in items.into_iter().enumerate() {
                            list.insert(pos + i, item);
                        }
                    }
                    return;
                }
            }
        })
        .collect();

    let ruin_remove_branches: Vec<_> = list_owners
        .iter()
        .map(|(_, field_ident, entity_type)| {
            let list_trait =
                quote! { <#entity_type as ::solverforge::__internal::ListVariableEntity<Self>> };
            quote! {
                if #list_trait::HAS_LIST_VARIABLE {
                    return #list_trait::list_field_mut(&mut s.#field_ident[entity_idx]).remove(pos);
                }
            }
        })
        .collect();

    let ruin_insert_branches: Vec<_> = list_owners
        .iter()
        .map(|(_, field_ident, entity_type)| {
            let list_trait =
                quote! { <#entity_type as ::solverforge::__internal::ListVariableEntity<Self>> };
            quote! {
                if #list_trait::HAS_LIST_VARIABLE {
                    #list_trait::list_field_mut(&mut s.#field_ident[entity_idx]).insert(pos, val);
                    return;
                }
            }
        })
        .collect();

    let remove_for_construction_branches: Vec<_> = list_owners
        .iter()
        .map(|(_, field_ident, entity_type)| {
            let list_trait =
                quote! { <#entity_type as ::solverforge::__internal::ListVariableEntity<Self>> };
            quote! {
                if #list_trait::HAS_LIST_VARIABLE {
                    return #list_trait::list_field_mut(&mut s.#field_ident[entity_idx]).remove(pos);
                }
            }
        })
        .collect();

    let descriptor_index_branches: Vec<_> = list_owners
        .iter()
        .map(|(idx, _, entity_type)| {
            let descriptor_index_lit =
                syn::LitInt::new(&idx.to_string(), proc_macro2::Span::call_site());
            let list_trait =
                quote! { <#entity_type as ::solverforge::__internal::ListVariableEntity<Self>> };
            quote! {
                if #list_trait::HAS_LIST_VARIABLE {
                    return #descriptor_index_lit;
                }
            }
        })
        .collect();

    let element_count_branches: Vec<_> = list_owners
        .iter()
        .map(|(_, _, entity_type)| {
            let list_trait =
                quote! { <#entity_type as ::solverforge::__internal::ListVariableEntity<Self>> };
            quote! {
                if #list_trait::HAS_LIST_VARIABLE {
                    return match #list_trait::LIST_ELEMENT_SOURCE {
                        #(#source_len_arms)*
                        ::core::option::Option::Some(source) => {
                            panic!(
                                "list source field `{}` was not found on the planning solution",
                                source
                            );
                        }
                        ::core::option::Option::None => 0,
                    };
                }
            }
        })
        .collect();

    let assigned_elements_branches: Vec<_> = list_owners
        .iter()
        .map(|(_, field_ident, entity_type)| {
            let list_trait =
                quote! { <#entity_type as ::solverforge::__internal::ListVariableEntity<Self>> };
            quote! {
                if #list_trait::HAS_LIST_VARIABLE {
                    return s
                        .#field_ident
                        .iter()
                        .flat_map(|entity| #list_trait::list_field(entity).iter().copied())
                        .collect();
                }
            }
        })
        .collect();

    let n_entities_branches: Vec<_> = list_owners
        .iter()
        .map(|(_, field_ident, entity_type)| {
            let list_trait =
                quote! { <#entity_type as ::solverforge::__internal::ListVariableEntity<Self>> };
            quote! {
                if #list_trait::HAS_LIST_VARIABLE {
                    return s.#field_ident.len();
                }
            }
        })
        .collect();

    let assign_element_branches: Vec<_> = list_owners
        .iter()
        .map(|(_, field_ident, entity_type)| {
            let list_trait =
                quote! { <#entity_type as ::solverforge::__internal::ListVariableEntity<Self>> };
            quote! {
                if #list_trait::HAS_LIST_VARIABLE {
                    if let Some(entity) = s.#field_ident.get_mut(entity_idx) {
                        #list_trait::list_field_mut(entity).push(elem);
                    }
                    return;
                }
            }
        })
        .collect();

    quote! {
        #[inline]
        pub fn list_len(&self, entity_idx: usize) -> usize {
            #(#list_len_branches)*
            0
        }

        #[inline]
        pub fn list_len_static(s: &Self, entity_idx: usize) -> usize {
            #(#list_len_static_branches)*
            0
        }

        #[inline]
        pub fn list_remove(s: &mut Self, entity_idx: usize, pos: usize) -> Option<usize> {
            #(#list_remove_branches)*
            ::core::option::Option::None
        }

        #[inline]
        pub fn list_insert(s: &mut Self, entity_idx: usize, pos: usize, val: usize) {
            #(#list_insert_branches)*
        }

        #[inline]
        pub fn list_get(s: &Self, entity_idx: usize, pos: usize) -> Option<usize> {
            #(#list_get_branches)*
            ::core::option::Option::None
        }

        #[inline]
        pub fn list_set(s: &mut Self, entity_idx: usize, pos: usize, val: usize) {
            #(#list_set_branches)*
        }

        #[inline]
        pub fn list_reverse(s: &mut Self, entity_idx: usize, start: usize, end: usize) {
            #(#list_reverse_branches)*
        }

        #[inline]
        pub fn sublist_remove(
            s: &mut Self,
            entity_idx: usize,
            start: usize,
            end: usize,
        ) -> Vec<usize> {
            #(#sublist_remove_branches)*
            ::std::vec::Vec::new()
        }

        #[inline]
        pub fn sublist_insert(
            s: &mut Self,
            entity_idx: usize,
            pos: usize,
            items: Vec<usize>,
        ) {
            #(#sublist_insert_branches)*
        }

        #[inline]
        pub fn ruin_remove(s: &mut Self, entity_idx: usize, pos: usize) -> usize {
            #(#ruin_remove_branches)*
            panic!("ruin_remove called on a planning solution without a list variable");
        }

        #[inline]
        pub fn ruin_insert(s: &mut Self, entity_idx: usize, pos: usize, val: usize) {
            #(#ruin_insert_branches)*
        }

        #[inline]
        pub fn list_remove_for_construction(s: &mut Self, entity_idx: usize, pos: usize) -> usize {
            #(#remove_for_construction_branches)*
            panic!("list_remove_for_construction called on a planning solution without a list variable");
        }

        #[inline]
        pub fn index_to_element_static(s: &Self, idx: usize) -> usize {
            let element_count = Self::element_count(s);
            if idx >= element_count {
                panic!(
                    "list element index {} is out of bounds for {} elements",
                    idx,
                    element_count
                );
            }
            #(#index_to_element_branches)*
            idx
        }

        #[inline]
        pub fn list_variable_descriptor_index() -> usize {
            #(#descriptor_index_branches)*
            usize::MAX
        }

        #[inline]
        pub fn element_count(s: &Self) -> usize {
            #(#element_count_branches)*
            0
        }

        #[inline]
        pub fn assigned_elements(s: &Self) -> Vec<usize> {
            #(#assigned_elements_branches)*
            ::std::vec::Vec::new()
        }

        #[inline]
        pub fn n_entities(s: &Self) -> usize {
            #(#n_entities_branches)*
            0
        }

        #[inline]
        pub fn assign_element(s: &mut Self, entity_idx: usize, elem: usize) {
            #(#assign_element_branches)*
        }
    }
}
