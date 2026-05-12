use crate::attr_parse::has_attribute;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::Ident;

use super::type_helpers::extract_collection_inner_type;

pub(super) fn generate_collection_source_methods(
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
) -> TokenStream {
    let entity_fields: Vec<_> = fields
        .iter()
        .filter(|f| has_attribute(&f.attrs, "planning_entity_collection"))
        .collect();

    let fact_fields: Vec<_> = fields
        .iter()
        .filter(|f| has_attribute(&f.attrs, "problem_fact_collection"))
        .collect();

    let list_element_fields: Vec<_> = fields
        .iter()
        .filter(|f| has_attribute(&f.attrs, "planning_list_element_collection"))
        .collect();

    let mut source_methods: Vec<TokenStream> = Vec::new();

    for (descriptor_index, f) in entity_fields.iter().enumerate() {
        let field_name = match f.ident.as_ref() {
            Some(n) => n,
            None => continue,
        };
        let element_type = match extract_collection_inner_type(&f.ty) {
            Some(t) => t,
            None => continue,
        };
        let descriptor_index_lit = syn::Index::from(descriptor_index);

        source_methods.push(quote! {
            pub fn #field_name() -> impl ::solverforge::stream::CollectionExtract<Self, Item = #element_type> {
                ::solverforge::__internal::source(
                    (|s: &Self| s.#field_name.as_slice()) as fn(&Self) -> &[#element_type],
                    ::solverforge::__internal::ChangeSource::Descriptor(#descriptor_index_lit),
                )
            }
        });
    }

    for f in fact_fields.iter() {
        let field_name = match f.ident.as_ref() {
            Some(n) => n,
            None => continue,
        };
        let element_type = match extract_collection_inner_type(&f.ty) {
            Some(t) => t,
            None => continue,
        };

        source_methods.push(quote! {
            pub fn #field_name() -> impl ::solverforge::stream::CollectionExtract<Self, Item = #element_type> {
                ::solverforge::__internal::source(
                    (|s: &Self| s.#field_name.as_slice()) as fn(&Self) -> &[#element_type],
                    ::solverforge::__internal::ChangeSource::Static,
                )
            }
        });
    }

    for f in list_element_fields.iter() {
        let field_name = match f.ident.as_ref() {
            Some(n) => n,
            None => continue,
        };
        let element_type = match extract_collection_inner_type(&f.ty) {
            Some(t) => t,
            None => continue,
        };

        source_methods.push(quote! {
            pub fn #field_name() -> impl ::solverforge::stream::CollectionExtract<Self, Item = #element_type> {
                ::solverforge::__internal::source(
                    (|s: &Self| s.#field_name.as_slice()) as fn(&Self) -> &[#element_type],
                    ::solverforge::__internal::ChangeSource::Static,
                )
            }
        });
    }

    quote! { #(#source_methods)* }
}

pub(super) fn generate_constraint_stream_extensions(
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
    solution_name: &Ident,
) -> TokenStream {
    let trait_name = format_ident!("{}ConstraintStreams", solution_name);

    let entity_fields: Vec<_> = fields
        .iter()
        .filter(|f| has_attribute(&f.attrs, "planning_entity_collection"))
        .collect();

    let fact_fields: Vec<_> = fields
        .iter()
        .filter(|f| has_attribute(&f.attrs, "problem_fact_collection"))
        .collect();

    let list_element_fields: Vec<_> = fields
        .iter()
        .filter(|f| has_attribute(&f.attrs, "planning_list_element_collection"))
        .collect();

    let mut trait_methods: Vec<TokenStream> = Vec::new();
    let mut impl_methods: Vec<TokenStream> = Vec::new();

    for (descriptor_index, f) in entity_fields.iter().enumerate() {
        let field_name = match f.ident.as_ref() {
            Some(n) => n,
            None => continue,
        };
        let element_type = match extract_collection_inner_type(&f.ty) {
            Some(t) => t,
            None => continue,
        };
        let descriptor_index_lit = syn::Index::from(descriptor_index);
        let change_source = quote! {
            ::solverforge::__internal::ChangeSource::Descriptor(#descriptor_index_lit)
        };

        push_stream_extension_method(
            &mut trait_methods,
            &mut impl_methods,
            solution_name,
            field_name,
            &element_type,
            change_source,
        );
    }

    for f in fact_fields.iter().chain(list_element_fields.iter()) {
        let field_name = match f.ident.as_ref() {
            Some(n) => n,
            None => continue,
        };
        let element_type = match extract_collection_inner_type(&f.ty) {
            Some(t) => t,
            None => continue,
        };
        let change_source = quote! { ::solverforge::__internal::ChangeSource::Static };

        push_stream_extension_method(
            &mut trait_methods,
            &mut impl_methods,
            solution_name,
            field_name,
            &element_type,
            change_source,
        );
    }

    quote! {
        pub trait #trait_name<Sc: ::solverforge::Score + 'static> {
            #(#trait_methods)*
        }

        impl<Sc: ::solverforge::Score + 'static> #trait_name<Sc>
            for ::solverforge::stream::ConstraintFactory<#solution_name, Sc>
        {
            #(#impl_methods)*
        }
    }
}

fn push_stream_extension_method(
    trait_methods: &mut Vec<TokenStream>,
    impl_methods: &mut Vec<TokenStream>,
    solution_name: &Ident,
    field_name: &Ident,
    element_type: &syn::Type,
    change_source: TokenStream,
) {
    let return_type = quote! {
        ::solverforge::__internal::UniConstraintStream<
            #solution_name,
            #element_type,
            ::solverforge::__internal::SourceExtract<fn(&#solution_name) -> &[#element_type]>,
            ::solverforge::__internal::TrueFilter,
            Sc,
        >
    };

    trait_methods.push(quote! {
        fn #field_name(self) -> #return_type;
    });

    impl_methods.push(quote! {
        fn #field_name(self) -> #return_type {
            ::solverforge::stream::ConstraintFactory::for_each(
                self,
                ::solverforge::__internal::source(
                    (|s: &#solution_name| s.#field_name.as_slice())
                        as fn(&#solution_name) -> &[#element_type],
                    #change_source,
                ),
            )
        }
    });
}
