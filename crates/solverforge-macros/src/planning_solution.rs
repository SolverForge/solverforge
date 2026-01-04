//! #[planning_solution] derive macro implementation

use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Data, Fields, Error};

use crate::has_attribute;

pub fn expand_derive(input: DeriveInput) -> Result<TokenStream, Error> {
    let name = &input.ident;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => return Err(Error::new_spanned(&input, "#[planning_solution] requires named fields")),
        },
        _ => return Err(Error::new_spanned(&input, "#[planning_solution] only works on structs")),
    };

    let score_field = fields
        .iter()
        .find(|f| has_attribute(&f.attrs, "planning_score"))
        .ok_or_else(|| Error::new_spanned(&input, "#[planning_solution] requires a #[planning_score] field"))?;

    let score_field_name = score_field.ident.as_ref().unwrap();
    let score_type = extract_option_inner_type(&score_field.ty)?;

    let entity_descriptors: Vec<_> = fields
        .iter()
        .filter(|f| has_attribute(&f.attrs, "planning_entity_collection"))
        .filter_map(|f| {
            let field_name = f.ident.as_ref().unwrap();
            let field_name_str = field_name.to_string();
            let element_type = extract_collection_inner_type(&f.ty)?;
            Some(quote! {
                .with_entity(::solverforge::EntityDescriptor::new(
                    stringify!(#element_type),
                    ::std::any::TypeId::of::<#element_type>(),
                    #field_name_str,
                ).with_extractor(Box::new(::solverforge::TypedEntityExtractor::new(
                    stringify!(#element_type),
                    #field_name_str,
                    |s: &#name| &s.#field_name,
                    |s: &mut #name| &mut s.#field_name,
                ))))
            })
        })
        .collect();

    let fact_descriptors: Vec<_> = fields
        .iter()
        .filter(|f| has_attribute(&f.attrs, "problem_fact_collection"))
        .filter_map(|f| {
            let field_name = f.ident.as_ref().unwrap();
            let field_name_str = field_name.to_string();
            let element_type = extract_collection_inner_type(&f.ty)?;
            Some(quote! {
                .with_problem_fact(::solverforge::ProblemFactDescriptor::new(
                    stringify!(#element_type),
                    ::std::any::TypeId::of::<#element_type>(),
                    #field_name_str,
                ).with_extractor(Box::new(::solverforge::TypedEntityExtractor::new(
                    stringify!(#element_type),
                    #field_name_str,
                    |s: &#name| &s.#field_name,
                    |s: &mut #name| &mut s.#field_name,
                ))))
            })
        })
        .collect();

    let name_str = name.to_string();
    let score_field_str = score_field_name.to_string();

    let expanded = quote! {
        impl #impl_generics ::solverforge::PlanningSolutionTrait for #name #ty_generics #where_clause {
            type Score = #score_type;
            fn score(&self) -> Option<Self::Score> { self.#score_field_name.clone() }
            fn set_score(&mut self, score: Option<Self::Score>) { self.#score_field_name = score; }
        }

        impl #impl_generics #name #ty_generics #where_clause {
            pub fn descriptor() -> ::solverforge::SolutionDescriptor {
                ::solverforge::SolutionDescriptor::new(
                    #name_str,
                    ::std::any::TypeId::of::<Self>(),
                )
                .with_score_field(#score_field_str)
                #(#entity_descriptors)*
                #(#fact_descriptors)*
            }
        }
    };

    Ok(expanded)
}

fn extract_option_inner_type(ty: &syn::Type) -> Result<&syn::Type, Error> {
    if let syn::Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if segment.ident == "Option" {
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                        return Ok(inner);
                    }
                }
            }
        }
    }
    Err(Error::new_spanned(ty, "Score field must be Option<Score>"))
}

fn extract_collection_inner_type(ty: &syn::Type) -> Option<&syn::Type> {
    if let syn::Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if segment.ident == "Vec" {
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                        return Some(inner);
                    }
                }
            }
        }
    }
    None
}
