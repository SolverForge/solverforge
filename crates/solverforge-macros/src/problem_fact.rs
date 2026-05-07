// #[problem_fact] derive macro implementation

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Error, Fields};

use crate::attr_parse::{get_attribute, has_attribute};
use crate::attr_validation::validate_no_attribute_args;

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
                    "#[problem_fact] requires named fields",
                ))
            }
        },
        _ => {
            return Err(Error::new_spanned(
                &input,
                "#[problem_fact] only works on structs",
            ))
        }
    };

    for field in fields {
        if let Some(attr) = get_attribute(&field.attrs, "planning_id") {
            validate_no_attribute_args(attr, "planning_id")?;
        }
    }

    let id_field = fields
        .iter()
        .find(|f| has_attribute(&f.attrs, "planning_id"));

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

    let expanded = quote! {
        impl #impl_generics ::solverforge::__internal::ProblemFact for #name #ty_generics #where_clause {
            fn as_any(&self) -> &dyn ::std::any::Any { self }
        }

        #planning_id_impl

        impl #impl_generics #name #ty_generics #where_clause {
            pub fn problem_fact_descriptor(
                solution_field: &'static str,
            ) -> ::solverforge::__internal::ProblemFactDescriptor {
                let mut desc = ::solverforge::__internal::ProblemFactDescriptor::new(
                    stringify!(#name),
                    ::std::any::TypeId::of::<Self>(),
                    solution_field,
                );
                #id_field_descriptor
                desc
            }
        }
    };

    Ok(expanded)
}
