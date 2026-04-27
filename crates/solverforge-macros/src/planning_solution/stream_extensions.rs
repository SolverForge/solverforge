use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

use crate::attr_parse::has_attribute;

use super::type_helpers::extract_collection_inner_type;

pub(super) fn generate_constraint_stream_extensions(
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
    solution_name: &Ident,
) -> TokenStream {
    let entity_fields: Vec<_> = fields
        .iter()
        .filter(|f| has_attribute(&f.attrs, "planning_entity_collection"))
        .collect();

    let fact_fields: Vec<_> = fields
        .iter()
        .filter(|f| has_attribute(&f.attrs, "problem_fact_collection"))
        .collect();

    let mut accessor_methods: Vec<TokenStream> = Vec::new();
    let mut accessor_impls: Vec<TokenStream> = Vec::new();

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

        accessor_methods.push(quote! {
            fn #field_name(self) -> ::solverforge::__internal::UniConstraintStream<
                #solution_name,
                #element_type,
                ::solverforge::__internal::SourceExtract<fn(&#solution_name) -> &[#element_type]>,
                ::solverforge::__internal::TrueFilter,
                Sc>;
        });

        accessor_impls.push(quote! {
            fn #field_name(self) -> ::solverforge::__internal::UniConstraintStream<
                #solution_name,
                #element_type,
                ::solverforge::__internal::SourceExtract<fn(&#solution_name) -> &[#element_type]>,
                ::solverforge::__internal::TrueFilter,
                Sc>
            {
                self.for_each(::solverforge::__internal::source(
                    (|s: &#solution_name| s.#field_name.as_slice()) as fn(&#solution_name) -> &[#element_type],
                    ::solverforge::__internal::ChangeSource::Descriptor(#descriptor_index_lit),
                ))
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

        accessor_methods.push(quote! {
            fn #field_name(self) -> ::solverforge::__internal::UniConstraintStream<
                #solution_name,
                #element_type,
                ::solverforge::__internal::SourceExtract<fn(&#solution_name) -> &[#element_type]>,
                ::solverforge::__internal::TrueFilter,
                Sc>;
        });

        accessor_impls.push(quote! {
            fn #field_name(self) -> ::solverforge::__internal::UniConstraintStream<
                #solution_name,
                #element_type,
                ::solverforge::__internal::SourceExtract<fn(&#solution_name) -> &[#element_type]>,
                ::solverforge::__internal::TrueFilter,
                Sc>
            {
                self.for_each(::solverforge::__internal::source(
                    (|s: &#solution_name| s.#field_name.as_slice()) as fn(&#solution_name) -> &[#element_type],
                    ::solverforge::__internal::ChangeSource::Static,
                ))
            }
        });
    }

    if accessor_methods.is_empty() {
        return TokenStream::new();
    }

    let trait_name = Ident::new(
        &format!("{}ConstraintStreams", solution_name),
        proc_macro2::Span::call_site(),
    );

    quote! {
        pub trait #trait_name<Sc: ::solverforge::Score + 'static> {
            #(#accessor_methods)*
        }

        impl<Sc: ::solverforge::Score + 'static> #trait_name<Sc>
            for ::solverforge::stream::ConstraintFactory<#solution_name, Sc>
        {
            #(#accessor_impls)*
        }
    }
}
