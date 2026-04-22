use proc_macro2::TokenStream;
use quote::quote;
use syn::{Error, Ident};

use crate::attr_parse::{get_attribute, parse_attribute_bool, parse_attribute_string};
use crate::scalar_registry::{record_scalar_entity_metadata, ScalarVariableMetadata};

use super::utils::field_is_option_usize;

pub(super) fn generate_scalar_helpers(
    entity_name: &Ident,
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
    planning_variables: &[&syn::Field],
) -> Result<TokenStream, Error> {
    let mut helpers = Vec::new();
    let mut metadata = Vec::new();

    for field in planning_variables {
        let field_name = field.ident.as_ref().unwrap();
        let field_name_str = field_name.to_string();
        let attr = get_attribute(&field.attrs, "planning_variable").unwrap();
        let value_range_provider = parse_attribute_string(attr, "value_range_provider")
            .or_else(|| parse_attribute_string(attr, "value_range"));
        let countable_range = parse_attribute_string(attr, "countable_range");
        let nearby_value_distance_meter =
            parse_attribute_string(attr, "nearby_value_distance_meter");
        let nearby_entity_distance_meter =
            parse_attribute_string(attr, "nearby_entity_distance_meter");

        if !field_is_option_usize(&field.ty) {
            continue;
        }

        let typed_getter_name = syn::Ident::new(
            &format!("__solverforge_get_{}_typed", field_name_str),
            proc_macro2::Span::call_site(),
        );
        let typed_setter_name = syn::Ident::new(
            &format!("__solverforge_set_{}_typed", field_name_str),
            proc_macro2::Span::call_site(),
        );

        helpers.push(quote! {
            #[inline]
            pub(crate) fn #typed_getter_name(entity: &Self) -> ::core::option::Option<usize> {
                entity.#field_name
            }

            #[inline]
            pub(crate) fn #typed_setter_name(
                entity: &mut Self,
                value: ::core::option::Option<usize>,
            ) {
                entity.#field_name = value;
            }
        });

        let provider_is_entity_field = value_range_provider.as_ref().is_some_and(|provider_id| {
            fields.iter().any(|candidate| {
                candidate
                    .ident
                    .as_ref()
                    .map(|ident| ident == provider_id)
                    .unwrap_or(false)
            })
        });

        if provider_is_entity_field {
            let provider_field = syn::Ident::new(
                value_range_provider.as_ref().unwrap(),
                proc_macro2::Span::call_site(),
            );
            let typed_provider_name = syn::Ident::new(
                &format!("__solverforge_values_for_{}_typed", field_name_str),
                proc_macro2::Span::call_site(),
            );
            helpers.push(quote! {
                #[inline]
                pub(crate) fn #typed_provider_name(entity: &Self) -> &[usize] {
                    &entity.#provider_field
                }
            });
        }

        metadata.push(ScalarVariableMetadata {
            field_name: field_name_str,
            allows_unassigned: parse_attribute_bool(attr, "allows_unassigned").unwrap_or(false),
            value_range_provider,
            countable_range: countable_range
                .as_ref()
                .map(|range| parse_range(field, range))
                .transpose()?,
            provider_is_entity_field,
            nearby_value_distance_meter,
            nearby_entity_distance_meter,
        });
    }

    record_scalar_entity_metadata(&entity_name.to_string(), metadata);

    Ok(quote! { #(#helpers)* })
}

fn parse_range(field: &syn::Field, range: &str) -> Result<(i64, i64), Error> {
    let parts: Vec<_> = range.split("..").collect();
    if parts.len() != 2 {
        return Err(Error::new_spanned(
            field,
            "countable_range must use `from..to` syntax",
        ));
    }
    let from = parts[0]
        .trim()
        .parse::<i64>()
        .map_err(|_| Error::new_spanned(field, "countable_range start must be an integer"))?;
    let to = parts[1]
        .trim()
        .parse::<i64>()
        .map_err(|_| Error::new_spanned(field, "countable_range end must be an integer"))?;
    Ok((from, to))
}
