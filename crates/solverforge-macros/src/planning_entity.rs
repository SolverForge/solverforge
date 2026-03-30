// #[planning_entity] derive macro implementation

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Error, Fields};

use crate::attr_parse::{
    get_attribute, has_attribute, parse_attribute_bool, parse_attribute_string,
};

pub fn expand_derive(input: DeriveInput) -> Result<TokenStream, Error> {
    let name = &input.ident;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => {
                return Err(Error::new_spanned(
                    &input,
                    "#[planning_entity] requires named fields",
                ))
            }
        },
        _ => {
            return Err(Error::new_spanned(
                &input,
                "#[planning_entity] only works on structs",
            ))
        }
    };

    let id_field = fields
        .iter()
        .find(|f| has_attribute(&f.attrs, "planning_id"));
    let pin_field = fields
        .iter()
        .find(|f| has_attribute(&f.attrs, "planning_pin"));

    let is_pinned_impl = if let Some(field) = pin_field {
        let field_name = field.ident.as_ref().unwrap();
        quote! { self.#field_name }
    } else {
        quote! { false }
    };

    let planning_variables: Vec<_> = fields
        .iter()
        .filter(|f| has_attribute(&f.attrs, "planning_variable"))
        .collect();

    let list_variables: Vec<_> = fields
        .iter()
        .filter(|f| has_attribute(&f.attrs, "planning_list_variable"))
        .collect();

    let name_str = name.to_string();

    // Shadow variables
    let inverse_relation_vars: Vec<_> = fields
        .iter()
        .filter(|f| has_attribute(&f.attrs, "inverse_relation_shadow_variable"))
        .collect();

    let previous_element_vars: Vec<_> = fields
        .iter()
        .filter(|f| has_attribute(&f.attrs, "previous_element_shadow_variable"))
        .collect();

    let next_element_vars: Vec<_> = fields
        .iter()
        .filter(|f| has_attribute(&f.attrs, "next_element_shadow_variable"))
        .collect();

    let cascading_update_vars: Vec<_> = fields
        .iter()
        .filter(|f| has_attribute(&f.attrs, "cascading_update_shadow_variable"))
        .collect();

    let variable_descriptors: Vec<_> = planning_variables
        .iter()
        .map(|field| {
            let field_name = field.ident.as_ref().unwrap();
            let field_name_str = field_name.to_string();
            let supports_usize_hooks = field_is_option_usize(&field.ty);
            let attr = get_attribute(&field.attrs, "planning_variable").unwrap();
            let allows_unassigned =
                parse_attribute_bool(attr, "allows_unassigned").unwrap_or(false);
            let is_chained = parse_attribute_bool(attr, "chained").unwrap_or(false);
            let value_range_provider = parse_attribute_string(attr, "value_range_provider")
                .or_else(|| parse_attribute_string(attr, "value_range"));
            let countable_range = parse_attribute_string(attr, "countable_range");
            let getter_name = syn::Ident::new(
                &format!("__solverforge_get_{}", field_name_str),
                proc_macro2::Span::call_site(),
            );
            let setter_name = syn::Ident::new(
                &format!("__solverforge_set_{}", field_name_str),
                proc_macro2::Span::call_site(),
            );

            let base = if is_chained {
                quote! { ::solverforge::__internal::VariableDescriptor::chained(#field_name_str) }
            } else {
                let maybe_usize_accessors = if supports_usize_hooks {
                    quote! { .with_usize_accessors(Self::#getter_name, Self::#setter_name) }
                } else {
                    TokenStream::new()
                };
                quote! {
                    ::solverforge::__internal::VariableDescriptor::genuine(#field_name_str)
                        .with_allows_unassigned(#allows_unassigned)
                        #maybe_usize_accessors
                }
            };

            let provider_is_entity_field =
                value_range_provider.as_ref().is_some_and(|provider_id| {
                    fields.iter().any(|candidate| {
                        candidate
                            .ident
                            .as_ref()
                            .map(|ident| ident == provider_id)
                            .unwrap_or(false)
                    })
                });

            let with_provider = if let Some(provider_id) = value_range_provider {
                let provider_getter_name = syn::Ident::new(
                    &format!("__solverforge_values_for_{}", field_name_str),
                    proc_macro2::Span::call_site(),
                );
                let maybe_entity_provider = if supports_usize_hooks && provider_is_entity_field {
                    quote! { .with_entity_value_provider(Self::#provider_getter_name) }
                } else {
                    TokenStream::new()
                };
                quote! {
                    #base
                        .with_value_range(#provider_id)
                        #maybe_entity_provider
                }
            } else {
                base
            };

            if let Some(range) = countable_range {
                let parts: Vec<_> = range.split("..").collect();
                if parts.len() != 2 {
                    return quote! {
                        compile_error!("countable_range must use `from..to` syntax");
                    };
                }
                let from_lit: i64 = parts[0]
                    .trim()
                    .parse()
                    .expect("countable_range start must be an integer");
                let to_lit: i64 = parts[1]
                    .trim()
                    .parse()
                    .expect("countable_range end must be an integer");
                quote! {
                    #with_provider.with_value_range_type(
                        ::solverforge::__internal::ValueRangeType::CountableRange {
                            from: #from_lit,
                            to: #to_lit,
                        }
                    )
                }
            } else {
                with_provider
            }
        })
        .collect();

    let variable_helpers: Vec<_> = planning_variables
        .iter()
        .map(|field| {
            let field_name = field.ident.as_ref().unwrap();
            let field_name_str = field_name.to_string();
            let supports_usize_hooks = field_is_option_usize(&field.ty);
            let attr = get_attribute(&field.attrs, "planning_variable").unwrap();
            let value_range_provider = parse_attribute_string(attr, "value_range_provider")
                .or_else(|| parse_attribute_string(attr, "value_range"));
            let provider_helper = value_range_provider
                .filter(|provider_id| {
                    fields.iter().any(|candidate| {
                        candidate.ident.as_ref().map(|ident| ident == provider_id).unwrap_or(false)
                    })
                })
                .map(|provider_id| {
                    let provider_field =
                        syn::Ident::new(&provider_id, proc_macro2::Span::call_site());
                    let provider_getter_name = syn::Ident::new(
                        &format!("__solverforge_values_for_{}", field_name_str),
                        proc_macro2::Span::call_site(),
                    );
                    quote! {
                        #[inline]
                        fn #provider_getter_name(entity: &dyn ::std::any::Any) -> ::std::vec::Vec<usize> {
                            let entity = entity
                                .downcast_ref::<Self>()
                                .expect("entity type mismatch for value provider");
                            entity.#provider_field.clone()
                        }
                    }
                });

            if supports_usize_hooks {
                let getter_name = syn::Ident::new(
                    &format!("__solverforge_get_{}", field_name_str),
                    proc_macro2::Span::call_site(),
                );
                let setter_name = syn::Ident::new(
                    &format!("__solverforge_set_{}", field_name_str),
                    proc_macro2::Span::call_site(),
                );

                quote! {
                    #[inline]
                    fn #getter_name(entity: &dyn ::std::any::Any) -> ::core::option::Option<usize> {
                        let entity = entity
                            .downcast_ref::<Self>()
                            .expect("entity type mismatch for planning variable getter");
                        entity.#field_name
                    }

                    #[inline]
                    fn #setter_name(
                        entity: &mut dyn ::std::any::Any,
                        value: ::core::option::Option<usize>,
                    ) {
                        let entity = entity
                            .downcast_mut::<Self>()
                            .expect("entity type mismatch for planning variable setter");
                        entity.#field_name = value;
                    }

                    #provider_helper
                }
            } else {
                provider_helper.unwrap_or_default()
            }
        })
        .collect();

    let list_variable_descriptors: Vec<_> = list_variables
        .iter()
        .map(|field| {
            let field_name = field.ident.as_ref().unwrap();
            let field_name_str = field_name.to_string();
            quote! { ::solverforge::__internal::VariableDescriptor::list(#field_name_str) }
        })
        .collect();

    // Shadow variable descriptors
    let inverse_relation_descriptors: Vec<_> = inverse_relation_vars
        .iter()
        .map(|field| {
            let field_name = field.ident.as_ref().unwrap();
            let field_name_str = field_name.to_string();
            let attr = get_attribute(&field.attrs, "inverse_relation_shadow_variable").unwrap();
            let source_var = parse_attribute_string(attr, "source_variable_name")
                .unwrap_or_else(|| "visits".to_string());
            quote! {
                ::solverforge::__internal::VariableDescriptor::shadow(
                    #field_name_str,
                    ::solverforge::__internal::ShadowVariableKind::InverseRelation
                ).with_source(#name_str, #source_var)
            }
        })
        .collect();

    let previous_element_descriptors: Vec<_> = previous_element_vars
        .iter()
        .map(|field| {
            let field_name = field.ident.as_ref().unwrap();
            let field_name_str = field_name.to_string();
            let attr = get_attribute(&field.attrs, "previous_element_shadow_variable").unwrap();
            let source_var = parse_attribute_string(attr, "source_variable_name")
                .unwrap_or_else(|| "visits".to_string());
            quote! {
                ::solverforge::__internal::VariableDescriptor::shadow(
                    #field_name_str,
                    ::solverforge::__internal::ShadowVariableKind::PreviousElement
                ).with_source(#name_str, #source_var)
            }
        })
        .collect();

    let next_element_descriptors: Vec<_> = next_element_vars
        .iter()
        .map(|field| {
            let field_name = field.ident.as_ref().unwrap();
            let field_name_str = field_name.to_string();
            let attr = get_attribute(&field.attrs, "next_element_shadow_variable").unwrap();
            let source_var = parse_attribute_string(attr, "source_variable_name")
                .unwrap_or_else(|| "visits".to_string());
            quote! {
                ::solverforge::__internal::VariableDescriptor::shadow(
                    #field_name_str,
                    ::solverforge::__internal::ShadowVariableKind::NextElement
                ).with_source(#name_str, #source_var)
            }
        })
        .collect();

    let cascading_update_descriptors: Vec<_> = cascading_update_vars
        .iter()
        .map(|field| {
            let field_name = field.ident.as_ref().unwrap();
            let field_name_str = field_name.to_string();
            quote! {
                ::solverforge::__internal::VariableDescriptor::shadow(
                    #field_name_str,
                    ::solverforge::__internal::ShadowVariableKind::Cascading
                )
            }
        })
        .collect();

    let planning_id_impl = if let Some(field) = id_field {
        let field_name = field.ident.as_ref().unwrap();
        let field_type = &field.ty;
        quote! {
            impl #impl_generics ::solverforge::__internal::PlanningId for #name #ty_generics #where_clause {
                type Id = #field_type;
                fn planning_id(&self) -> Self::Id { self.#field_name.clone() }
            }
        }
    } else {
        TokenStream::new()
    };

    let id_field_descriptor = if let Some(field) = id_field {
        let field_name = field.ident.as_ref().unwrap();
        quote! { desc = desc.with_id_field(stringify!(#field_name)); }
    } else {
        TokenStream::new()
    };

    let pin_field_descriptor = if let Some(field) = pin_field {
        let field_name = field.ident.as_ref().unwrap();
        quote! { desc = desc.with_pin_field(stringify!(#field_name)); }
    } else {
        TokenStream::new()
    };

    let expanded = quote! {
        impl #impl_generics ::solverforge::__internal::PlanningEntity for #name #ty_generics #where_clause {
            fn is_pinned(&self) -> bool { #is_pinned_impl }
            fn as_any(&self) -> &dyn ::std::any::Any { self }
            fn as_any_mut(&mut self) -> &mut dyn ::std::any::Any { self }
        }

        #planning_id_impl

        impl #impl_generics #name #ty_generics #where_clause {
            #(#variable_helpers)*

            pub fn entity_descriptor(solution_field: &'static str) -> ::solverforge::__internal::EntityDescriptor {
                let mut desc = ::solverforge::__internal::EntityDescriptor::new(
                    #name_str,
                    ::std::any::TypeId::of::<Self>(),
                    solution_field,
                );
                #id_field_descriptor
                #pin_field_descriptor
                #( desc = desc.with_variable(#variable_descriptors); )*
                #( desc = desc.with_variable(#list_variable_descriptors); )*
                #( desc = desc.with_variable(#inverse_relation_descriptors); )*
                #( desc = desc.with_variable(#previous_element_descriptors); )*
                #( desc = desc.with_variable(#next_element_descriptors); )*
                #( desc = desc.with_variable(#cascading_update_descriptors); )*
                desc
            }
        }
    };

    Ok(expanded)
}

fn field_is_option_usize(ty: &syn::Type) -> bool {
    let syn::Type::Path(type_path) = ty else {
        return false;
    };
    let Some(segment) = type_path.path.segments.last() else {
        return false;
    };
    if segment.ident != "Option" {
        return false;
    }
    let syn::PathArguments::AngleBracketed(args) = &segment.arguments else {
        return false;
    };
    let Some(syn::GenericArgument::Type(syn::Type::Path(inner_path))) = args.args.first() else {
        return false;
    };
    inner_path
        .path
        .segments
        .last()
        .map(|segment| segment.ident == "usize")
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::expand_derive;
    use syn::parse_quote;

    #[test]
    fn golden_entity_expansion_includes_descriptor_and_planning_id() {
        let input = parse_quote! {
            struct Task {
                #[planning_id]
                id: String,
                #[planning_variable(allows_unassigned = true, value_range = "workers")]
                worker_idx: Option<usize>,
            }
        };

        let expanded = expand_derive(input)
            .expect("entity expansion should succeed")
            .to_string();

        assert!(expanded.contains("impl :: solverforge :: __internal :: PlanningEntity for Task"));
        assert!(expanded.contains("impl :: solverforge :: __internal :: PlanningId for Task"));
        assert!(expanded.contains("with_allows_unassigned (true)"));
        assert!(expanded.contains("with_value_range (\"workers\")"));
        assert!(expanded.contains("with_id_field (stringify ! (id))"));
        assert!(expanded.contains("pub fn entity_descriptor"));
    }
}
