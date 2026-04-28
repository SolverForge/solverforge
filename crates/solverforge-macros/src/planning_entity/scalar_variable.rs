use proc_macro2::TokenStream;
use quote::quote;
use syn::{Error, Ident};

use crate::attr_parse::{get_attribute, parse_attribute_bool, parse_attribute_string};

use super::utils::field_is_option_usize;

pub(super) fn generate_scalar_helpers(
    _entity_name: &Ident,
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
    planning_variables: &[&syn::Field],
) -> Result<TokenStream, Error> {
    let mut helpers = Vec::new();
    let mut getter_arms = Vec::new();
    let mut setter_arms = Vec::new();
    let mut name_arms = Vec::new();
    let mut allows_unassigned_arms = Vec::new();
    let mut value_range_provider_arms = Vec::new();
    let mut countable_range_arms = Vec::new();
    let mut provider_is_entity_field_arms = Vec::new();
    let mut entity_slice_arms = Vec::new();
    let mut scalar_variable_index = 0usize;

    for field in planning_variables {
        let field_name = field.ident.as_ref().unwrap();
        let field_name_str = field_name.to_string();
        let attr = get_attribute(&field.attrs, "planning_variable").unwrap();
        let value_range_provider = parse_attribute_string(attr, "value_range_provider");
        let countable_range = parse_attribute_string(attr, "countable_range");
        let is_chained = parse_attribute_bool(attr, "chained").unwrap_or(false);

        let supports_scalar_helpers = field_is_option_usize(&field.ty) && !is_chained;

        let typed_getter_name = syn::Ident::new(
            &format!("__solverforge_get_{}_typed", field_name_str),
            proc_macro2::Span::call_site(),
        );
        let typed_setter_name = syn::Ident::new(
            &format!("__solverforge_set_{}_typed", field_name_str),
            proc_macro2::Span::call_site(),
        );

        let provider_is_entity_field = value_range_provider.as_ref().is_some_and(|provider_id| {
            fields.iter().any(|candidate| {
                candidate
                    .ident
                    .as_ref()
                    .map(|ident| ident == provider_id)
                    .unwrap_or(false)
            })
        });

        if supports_scalar_helpers {
            let index = scalar_variable_index;
            scalar_variable_index += 1;

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

            getter_arms.push(quote! { #index => Self::#typed_getter_name(entity), });
            setter_arms.push(quote! {
                #index => {
                    Self::#typed_setter_name(entity, value);
                }
            });
            name_arms.push(quote! { #index => ::core::option::Option::Some(#field_name_str), });
            let allows_unassigned =
                parse_attribute_bool(attr, "allows_unassigned").unwrap_or(false);
            allows_unassigned_arms.push(quote! { #index => #allows_unassigned, });
            let provider_arm = if let Some(provider) = &value_range_provider {
                quote! { #index => ::core::option::Option::Some(#provider), }
            } else {
                quote! { #index => ::core::option::Option::None, }
            };
            value_range_provider_arms.push(provider_arm);
            let range_arm = if let Some(range) = &countable_range {
                let (from, to) = parse_range(field, range)?;
                quote! { #index => ::core::option::Option::Some((#from, #to)), }
            } else {
                quote! { #index => ::core::option::Option::None, }
            };
            countable_range_arms.push(range_arm);
            provider_is_entity_field_arms.push(quote! { #index => #provider_is_entity_field, });
        }

        if supports_scalar_helpers && provider_is_entity_field {
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
            let index = scalar_variable_index - 1;
            entity_slice_arms.push(quote! {
                #index => Self::#typed_provider_name(entity),
            });
        }
    }

    let scalar_variable_count = scalar_variable_index;

    Ok(quote! {
        #(#helpers)*

        #[inline]
        pub(crate) const fn __solverforge_scalar_variable_count() -> usize {
            #scalar_variable_count
        }

        #[inline]
        pub(crate) const fn __solverforge_scalar_variable_name_by_index(
            variable_index: usize,
        ) -> ::core::option::Option<&'static str> {
            match variable_index {
                #(#name_arms)*
                _ => ::core::option::Option::None,
            }
        }

        #[inline]
        pub(crate) const fn __solverforge_scalar_allows_unassigned_by_index(
            variable_index: usize,
        ) -> bool {
            match variable_index {
                #(#allows_unassigned_arms)*
                _ => false,
            }
        }

        #[inline]
        pub(crate) const fn __solverforge_scalar_value_range_provider_by_index(
            variable_index: usize,
        ) -> ::core::option::Option<&'static str> {
            match variable_index {
                #(#value_range_provider_arms)*
                _ => ::core::option::Option::None,
            }
        }

        #[inline]
        pub(crate) const fn __solverforge_scalar_countable_range_by_index(
            variable_index: usize,
        ) -> ::core::option::Option<(i64, i64)> {
            match variable_index {
                #(#countable_range_arms)*
                _ => ::core::option::Option::None,
            }
        }

        #[inline]
        pub(crate) const fn __solverforge_scalar_provider_is_entity_field_by_index(
            variable_index: usize,
        ) -> bool {
            match variable_index {
                #(#provider_is_entity_field_arms)*
                _ => false,
            }
        }

        #[inline]
        pub(crate) fn __solverforge_scalar_get_by_index(
            entity: &Self,
            variable_index: usize,
        ) -> ::core::option::Option<usize> {
            match variable_index {
                #(#getter_arms)*
                _ => ::core::option::Option::None,
            }
        }

        #[inline]
        pub(crate) fn __solverforge_scalar_set_by_index(
            entity: &mut Self,
            variable_index: usize,
            value: ::core::option::Option<usize>,
        ) {
            match variable_index {
                #(#setter_arms)*
                _ => {}
            }
        }

        #[inline]
        pub(crate) fn __solverforge_scalar_values_by_index(
            entity: &Self,
            variable_index: usize,
        ) -> &[usize] {
            match variable_index {
                #(#entity_slice_arms)*
                _ => &[],
            }
        }

    })
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
