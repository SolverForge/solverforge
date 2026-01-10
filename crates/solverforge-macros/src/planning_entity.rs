//! #[planning_entity] derive macro implementation

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Error, Fields};

use crate::{get_attribute, has_attribute, parse_attribute_bool, parse_attribute_string};

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

    let variable_descriptors: Vec<_> = planning_variables
        .iter()
        .map(|field| {
            let field_name = field.ident.as_ref().unwrap();
            let field_name_str = field_name.to_string();
            let attr = get_attribute(&field.attrs, "planning_variable").unwrap();
            let allows_unassigned =
                parse_attribute_bool(attr, "allows_unassigned").unwrap_or(false);
            let is_chained = parse_attribute_bool(attr, "chained").unwrap_or(false);
            let value_range_provider = parse_attribute_string(attr, "value_range_provider");

            let base = if is_chained {
                quote! { ::solverforge::VariableDescriptor::chained(#field_name_str) }
            } else {
                quote! {
                    ::solverforge::VariableDescriptor::genuine(#field_name_str)
                        .with_allows_unassigned(#allows_unassigned)
                }
            };

            if let Some(provider_id) = value_range_provider {
                quote! { #base.with_value_range(#provider_id) }
            } else {
                base
            }
        })
        .collect();

    let list_variable_descriptors: Vec<_> = list_variables
        .iter()
        .map(|field| {
            let field_name = field.ident.as_ref().unwrap();
            let field_name_str = field_name.to_string();
            quote! { ::solverforge::VariableDescriptor::list(#field_name_str) }
        })
        .collect();

    let planning_id_impl = if let Some(field) = id_field {
        let field_name = field.ident.as_ref().unwrap();
        let field_type = &field.ty;
        quote! {
            impl #impl_generics ::solverforge::PlanningId for #name #ty_generics #where_clause {
                type Id = #field_type;
                fn planning_id(&self) -> Self::Id { self.#field_name.clone() }
            }
        }
    } else {
        TokenStream::new()
    };

    let name_str = name.to_string();

    let expanded = quote! {
        impl #impl_generics ::solverforge::PlanningEntityTrait for #name #ty_generics #where_clause {
            fn is_pinned(&self) -> bool { #is_pinned_impl }
            fn as_any(&self) -> &dyn ::std::any::Any { self }
            fn as_any_mut(&mut self) -> &mut dyn ::std::any::Any { self }
        }

        #planning_id_impl

        impl #impl_generics #name #ty_generics #where_clause {
            pub fn entity_descriptor(solution_field: &'static str) -> ::solverforge::EntityDescriptor {
                let mut desc = ::solverforge::EntityDescriptor::new(
                    #name_str,
                    ::std::any::TypeId::of::<Self>(),
                    solution_field,
                );
                #( desc = desc.with_variable(#variable_descriptors); )*
                #( desc = desc.with_variable(#list_variable_descriptors); )*
                desc
            }
        }
    };

    Ok(expanded)
}
