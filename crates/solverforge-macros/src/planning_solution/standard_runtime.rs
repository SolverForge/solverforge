use std::collections::BTreeSet;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::Ident;

use crate::attr_parse::has_attribute;
use crate::standard_registry::lookup_standard_entity_metadata;

use super::type_helpers::extract_collection_inner_type;

pub(super) struct StandardRuntimeSupport {
    pub setup: TokenStream,
}

pub(super) fn generate_standard_runtime_support(
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
    solution_name: &Ident,
) -> StandardRuntimeSupport {
    let entity_fields: Vec<_> = fields
        .iter()
        .filter(|f| has_attribute(&f.attrs, "planning_entity_collection"))
        .enumerate()
        .filter_map(|(idx, field)| {
            let field_name = field.ident.as_ref()?.clone();
            let field_type = extract_collection_inner_type(&field.ty)?;
            let syn::Type::Path(type_path) = field_type else {
                return None;
            };
            let type_ident = type_path.path.segments.last()?.ident.clone();
            let metadata = lookup_standard_entity_metadata(&type_ident.to_string())?;
            if metadata.variables.is_empty() {
                return None;
            }
            Some((idx, field_name, type_ident, metadata))
        })
        .collect();

    if entity_fields.is_empty() {
        return StandardRuntimeSupport {
            setup: quote! {
                let mut __solverforge_variables = ::std::vec::Vec::new();
            },
        };
    }

    let mut provider_fields = BTreeSet::new();
    for (_, _, _, metadata) in &entity_fields {
        for variable in &metadata.variables {
            if !variable.provider_is_entity_field {
                if let Some(provider) = &variable.value_range_provider {
                    provider_fields.insert(provider.clone());
                }
            }
        }
    }

    let provider_count_helpers: Vec<_> = provider_fields
        .into_iter()
        .map(|provider_field_name| {
            let provider_ident = format_ident!("{provider_field_name}");
            let count_fn_ident =
                format_ident!("__solverforge_standard_count_{}", provider_field_name);
            quote! {
                fn #count_fn_ident(solution: &#solution_name) -> usize {
                    solution.#provider_ident.len()
                }
            }
        })
        .collect();

    let entity_count_helpers: Vec<_> = entity_fields
        .iter()
        .map(|(_, field_name, _, _)| {
            let count_fn_ident = format_ident!("__solverforge_standard_count_{}", field_name);
            quote! {
                fn #count_fn_ident(solution: &#solution_name) -> usize {
                    solution.#field_name.len()
                }
            }
        })
        .collect();

    let variable_helpers: Vec<_> = entity_fields
        .iter()
        .flat_map(|(descriptor_index, field_name, entity_type, metadata)| {
            metadata.variables.iter().map(move |variable| {
                let variable_name = &variable.field_name;
                let allows_unassigned = variable.allows_unassigned;
                let getter_ident = format_ident!(
                    "__solverforge_standard_get_{}_{}",
                    field_name,
                    variable.field_name
                );
                let setter_ident = format_ident!(
                    "__solverforge_standard_set_{}_{}",
                    field_name,
                    variable.field_name
                );
                let entity_count_fn_ident =
                    format_ident!("__solverforge_standard_count_{}", field_name);
                let typed_getter_ident =
                    format_ident!("__solverforge_get_{}_typed", variable.field_name);
                let typed_setter_ident =
                    format_ident!("__solverforge_set_{}_typed", variable.field_name);
                let maybe_slice_helper = if variable.provider_is_entity_field {
                    let slice_ident = format_ident!(
                        "__solverforge_standard_values_{}_{}",
                        field_name,
                        variable.field_name
                    );
                    let typed_slice_ident =
                        format_ident!("__solverforge_values_for_{}_typed", variable.field_name);
                    quote! {
                        fn #slice_ident(
                            solution: &#solution_name,
                            entity_index: usize,
                        ) -> &[usize] {
                            <#entity_type>::#typed_slice_ident(&solution.#field_name[entity_index])
                        }
                    }
                } else {
                    TokenStream::new()
                };

                let value_source = if variable.provider_is_entity_field {
                    let slice_ident = format_ident!(
                        "__solverforge_standard_values_{}_{}",
                        field_name,
                        variable.field_name
                    );
                    quote! {
                        ::solverforge::__internal::ValueSource::EntitySlice {
                            values_for_entity: #slice_ident,
                        }
                    }
                } else if let Some((from, to)) = variable.countable_range {
                    let from_usize = usize::try_from(from).expect(
                        "countable_range start must be non-negative for canonical standard solving",
                    );
                    let to_usize = usize::try_from(to).expect(
                        "countable_range end must be non-negative for canonical standard solving",
                    );
                    quote! {
                        ::solverforge::__internal::ValueSource::CountableRange {
                            from: #from_usize,
                            to: #to_usize,
                        }
                    }
                } else if let Some(provider_field_name) = &variable.value_range_provider {
                    let count_fn_ident =
                        format_ident!("__solverforge_standard_count_{}", provider_field_name);
                    quote! {
                        ::solverforge::__internal::ValueSource::SolutionCount {
                            count_fn: #count_fn_ident,
                        }
                    }
                } else {
                    quote! { ::solverforge::__internal::ValueSource::Empty }
                };

                quote! {
                    fn #getter_ident(
                        solution: &#solution_name,
                        entity_index: usize,
                    ) -> ::core::option::Option<usize> {
                        <#entity_type>::#typed_getter_ident(&solution.#field_name[entity_index])
                    }

                    fn #setter_ident(
                        solution: &mut #solution_name,
                        entity_index: usize,
                        value: ::core::option::Option<usize>,
                    ) {
                        <#entity_type>::#typed_setter_ident(
                            &mut solution.#field_name[entity_index],
                            value,
                        );
                    }

                    #maybe_slice_helper

                    __solverforge_variables.push(
                        ::solverforge::__internal::VariableContext::Scalar(
                            ::solverforge::__internal::ScalarVariableContext::new(
                                #descriptor_index,
                                stringify!(#entity_type),
                                #entity_count_fn_ident,
                                #variable_name,
                                #getter_ident,
                                #setter_ident,
                                #value_source,
                                #allows_unassigned,
                            )
                        )
                    );
                }
            })
        })
        .collect();

    StandardRuntimeSupport {
        setup: quote! {
            let mut __solverforge_variables = ::std::vec::Vec::new();
            #(#provider_count_helpers)*
            #(#entity_count_helpers)*
            #(#variable_helpers)*
        },
    }
}
