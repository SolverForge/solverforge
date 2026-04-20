use proc_macro2::TokenStream;
use quote::quote;
use syn::{Error, Ident};

use crate::attr_parse::has_attribute;
use crate::list_registry::lookup_list_entity_metadata;

use super::config::ShadowConfig;
use super::type_helpers::extract_collection_inner_type;

struct ListOwnerConfig<'a> {
    field_ident: &'a Ident,
    entity_type: &'a syn::Type,
    element_collection_name: String,
}

struct ListShadowConfig<'a> {
    list_owner: ListOwnerConfig<'a>,
    element_collection_ident: &'a Ident,
}

fn type_name_from_collection(ty: &syn::Type) -> Option<String> {
    let entity_type = extract_collection_inner_type(ty)?;
    let syn::Type::Path(type_path) = entity_type else {
        return None;
    };
    let segment = type_path.path.segments.last()?;
    Some(segment.ident.to_string())
}

fn find_list_owner_config<'a>(
    config: &ShadowConfig,
    fields: &'a syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
) -> Result<Option<ListOwnerConfig<'a>>, Error> {
    let Some(list_owner) = config.list_owner.as_deref() else {
        return Ok(None);
    };

    fields
        .iter()
        .filter(|f| has_attribute(&f.attrs, "planning_entity_collection"))
        .find_map(|field| {
            let field_ident = field.ident.as_ref()?;
            if field_ident != list_owner {
                return None;
            }
            let entity_type = extract_collection_inner_type(&field.ty)?;
            let element_collection_name = type_name_from_collection(&field.ty)
                .and_then(|type_name| lookup_list_entity_metadata(&type_name))
                .map(|metadata| metadata.element_collection_name)
                .unwrap_or_default();
            Some(ListOwnerConfig {
                field_ident,
                entity_type,
                element_collection_name,
            })
        })
        .map(Some)
        .ok_or_else(|| {
            Error::new(
                proc_macro2::Span::call_site(),
                format!(
                    "#[shadow_variable_updates(list_owner = \"{list_owner}\")] must name a #[planning_entity_collection] field"
                ),
            )
        })
}

fn find_matching_element_collection<'a>(
    list_owner: &ListOwnerConfig<'a>,
    fields: &'a syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
) -> Result<&'a Ident, Error> {
    fields
        .iter()
        .find_map(|field| {
            let field_ident = field.ident.as_ref()?;
            if field_ident.to_string() != list_owner.element_collection_name {
                return None;
            }
            if has_attribute(&field.attrs, "planning_entity_collection")
                || has_attribute(&field.attrs, "problem_fact_collection")
            {
                Some(field_ident)
            } else {
                None
            }
        })
        .ok_or_else(|| {
            Error::new(
                proc_macro2::Span::call_site(),
                format!(
                    "planning solution with list owner `{}` requires a `#[planning_entity_collection]` or `#[problem_fact_collection]` field named `{}`",
                    list_owner.field_ident,
                    list_owner.element_collection_name,
                ),
            )
        })
}

fn find_list_shadow_config<'a>(
    list_owner: ListOwnerConfig<'a>,
    fields: &'a syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
) -> Result<ListShadowConfig<'a>, Error> {
    let element_collection_ident = find_matching_element_collection(&list_owner, fields)?;
    Ok(ListShadowConfig {
        list_owner,
        element_collection_ident,
    })
}

fn shadow_updates_requested(config: &ShadowConfig) -> bool {
    config.inverse_field.is_some()
        || config.previous_field.is_some()
        || config.next_field.is_some()
        || config.cascading_listener.is_some()
        || config.post_update_listener.is_some()
        || !config.entity_aggregates.is_empty()
        || !config.entity_computes.is_empty()
}

pub(super) fn generate_shadow_support(
    config: &ShadowConfig,
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
    solution_name: &Ident,
) -> Result<TokenStream, Error> {
    if !shadow_updates_requested(config) {
        return Ok(quote! {
            impl ::solverforge::__internal::ShadowVariableSupport for #solution_name {
                #[inline]
                fn update_entity_shadows(&mut self, _entity_idx: usize) {}
            }
        });
    }

    let Some(list_owner) = find_list_owner_config(config, fields)? else {
        return Err(Error::new(
            proc_macro2::Span::call_site(),
            "#[shadow_variable_updates(...)] requires `list_owner = \"entity_collection_field\"` when shadow updates are configured",
        ));
    };

    let runtime_config = find_list_shadow_config(list_owner, fields)?;

    let list_owner_ident = runtime_config.list_owner.field_ident;
    let element_collection_ident = runtime_config.element_collection_ident;
    let list_owner_type = runtime_config.list_owner.entity_type;
    let list_trait =
        quote! { <#list_owner_type as ::solverforge::__internal::ListVariableEntity<Self>> };

    let inverse_update = config.inverse_field.as_ref().map(|field| {
        let field_ident = Ident::new(field, proc_macro2::Span::call_site());
        quote! {
            for &element_idx in &element_indices {
                self.#element_collection_ident[element_idx].#field_ident = Some(entity_idx);
            }
        }
    });

    let previous_update = config.previous_field.as_ref().map(|field| {
        let field_ident = Ident::new(field, proc_macro2::Span::call_site());
        quote! {
            let mut prev_idx: Option<usize> = None;
            for &element_idx in &element_indices {
                self.#element_collection_ident[element_idx].#field_ident = prev_idx;
                prev_idx = Some(element_idx);
            }
        }
    });

    let next_update = config.next_field.as_ref().map(|field| {
        let field_ident = Ident::new(field, proc_macro2::Span::call_site());
        quote! {
            let len = element_indices.len();
            for (i, &element_idx) in element_indices.iter().enumerate() {
                let next_idx = if i + 1 < len { Some(element_indices[i + 1]) } else { None };
                self.#element_collection_ident[element_idx].#field_ident = next_idx;
            }
        }
    });

    let cascading_update = config.cascading_listener.as_ref().map(|method| {
        let method_ident = Ident::new(method, proc_macro2::Span::call_site());
        quote! {
            for &element_idx in &element_indices {
                self.#method_ident(element_idx);
            }
        }
    });

    let post_update = config.post_update_listener.as_ref().map(|method| {
        let method_ident = Ident::new(method, proc_macro2::Span::call_site());
        quote! {
            self.#method_ident(entity_idx);
        }
    });

    let aggregate_updates: Vec<_> = config
        .entity_aggregates
        .iter()
        .filter_map(|spec| {
            let parts: Vec<&str> = spec.split(':').collect();
            if parts.len() != 3 {
                return None;
            }
            let target_field = Ident::new(parts[0], proc_macro2::Span::call_site());
            let aggregation = parts[1];
            let source_field = Ident::new(parts[2], proc_macro2::Span::call_site());

            match aggregation {
                "sum" => Some(quote! {
                    self.#list_owner_ident[entity_idx].#target_field = element_indices
                        .iter()
                        .map(|&idx| self.#element_collection_ident[idx].#source_field)
                        .sum();
                }),
                _ => None,
            }
        })
        .collect();

    let compute_updates: Vec<_> = config
        .entity_computes
        .iter()
        .filter_map(|spec| {
            let parts: Vec<&str> = spec.split(':').collect();
            if parts.len() != 2 {
                return None;
            }
            let target_field = Ident::new(parts[0], proc_macro2::Span::call_site());
            let method_name = Ident::new(parts[1], proc_macro2::Span::call_site());

            Some(quote! {
                self.#list_owner_ident[entity_idx].#target_field = self.#method_name(entity_idx);
            })
        })
        .collect();

    Ok(quote! {
        impl ::solverforge::__internal::ShadowVariableSupport for #solution_name {
            #[inline]
            fn update_entity_shadows(&mut self, entity_idx: usize) {
                let element_indices: Vec<usize> =
                    #list_trait::list_field(&self.#list_owner_ident[entity_idx]).to_vec();

                #inverse_update
                #previous_update
                #next_update
                #cascading_update
                #(#aggregate_updates)*
                #(#compute_updates)*
                #post_update
            }
        }
    })
}
