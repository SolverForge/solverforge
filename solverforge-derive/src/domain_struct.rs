//! Implementation of the DomainStruct derive macro.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields, Type};

/// Implementation of the `#[derive(DomainStruct)]` macro.
pub fn derive_domain_struct_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let name_str = name.to_string();

    // Extract fields from struct
    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => {
                return syn::Error::new_spanned(
                    &input,
                    "DomainStruct can only be derived for structs with named fields",
                )
                .to_compile_error()
                .into();
            }
        },
        _ => {
            return syn::Error::new_spanned(&input, "DomainStruct can only be derived for structs")
                .to_compile_error()
                .into();
        }
    };

    // Generate field descriptors
    let field_descriptors: Vec<TokenStream2> = fields
        .iter()
        .map(|field| {
            let field_name = field.ident.as_ref().unwrap();
            let field_name_str = field_name.to_string();
            let field_type = rust_type_to_field_type(&field.ty);

            quote! {
                .with_field(
                    ::solverforge_core::domain::FieldDescriptor::new(#field_name_str, #field_type)
                )
            }
        })
        .collect();

    let expanded = quote! {
        impl ::solverforge_core::DomainStruct for #name {
            fn domain_class() -> ::solverforge_core::domain::DomainClass {
                ::solverforge_core::domain::DomainClass::new(#name_str)
                    #(#field_descriptors)*
            }
        }
    };

    TokenStream::from(expanded)
}

/// Convert a Rust type to a FieldType expression.
fn rust_type_to_field_type(ty: &Type) -> TokenStream2 {
    let type_str = quote!(#ty).to_string().replace(' ', "");

    // Handle common primitive types
    if type_str == "String" || type_str == "&str" {
        return quote! {
            ::solverforge_core::domain::FieldType::Primitive(
                ::solverforge_core::domain::PrimitiveType::String
            )
        };
    }
    if type_str == "i32" || type_str == "u32" {
        return quote! {
            ::solverforge_core::domain::FieldType::Primitive(
                ::solverforge_core::domain::PrimitiveType::Int
            )
        };
    }
    if type_str == "i64" || type_str == "u64" || type_str == "isize" || type_str == "usize" {
        return quote! {
            ::solverforge_core::domain::FieldType::Primitive(
                ::solverforge_core::domain::PrimitiveType::Long
            )
        };
    }
    if type_str == "f32" || type_str == "f64" {
        return quote! {
            ::solverforge_core::domain::FieldType::Primitive(
                ::solverforge_core::domain::PrimitiveType::Double
            )
        };
    }
    if type_str == "bool" {
        return quote! {
            ::solverforge_core::domain::FieldType::Primitive(
                ::solverforge_core::domain::PrimitiveType::Bool
            )
        };
    }

    // Handle DateTime types
    if is_datetime_type(&type_str) {
        return quote! {
            ::solverforge_core::domain::FieldType::Primitive(
                ::solverforge_core::domain::PrimitiveType::DateTime
            )
        };
    }

    // Handle Option<T>
    if type_str.starts_with("Option<") && type_str.ends_with('>') {
        let inner = &type_str[7..type_str.len() - 1];
        return inner_type_to_field_type(inner);
    }

    // Handle Vec<T>
    if type_str.starts_with("Vec<") && type_str.ends_with('>') {
        let inner = &type_str[4..type_str.len() - 1];
        let inner_type = inner_type_to_field_type(inner);
        return quote! {
            ::solverforge_core::domain::FieldType::list(#inner_type)
        };
    }

    // Default to Object type
    let type_name = extract_type_name(&type_str);
    quote! {
        ::solverforge_core::domain::FieldType::object(#type_name)
    }
}

/// Convert an inner type string to a FieldType expression.
fn inner_type_to_field_type(type_str: &str) -> TokenStream2 {
    match type_str {
        "String" | "&str" => quote! {
            ::solverforge_core::domain::FieldType::Primitive(
                ::solverforge_core::domain::PrimitiveType::String
            )
        },
        "i32" | "u32" => quote! {
            ::solverforge_core::domain::FieldType::Primitive(
                ::solverforge_core::domain::PrimitiveType::Int
            )
        },
        "i64" | "u64" | "isize" | "usize" => quote! {
            ::solverforge_core::domain::FieldType::Primitive(
                ::solverforge_core::domain::PrimitiveType::Long
            )
        },
        "f32" | "f64" => quote! {
            ::solverforge_core::domain::FieldType::Primitive(
                ::solverforge_core::domain::PrimitiveType::Double
            )
        },
        "bool" => quote! {
            ::solverforge_core::domain::FieldType::Primitive(
                ::solverforge_core::domain::PrimitiveType::Bool
            )
        },
        _ if is_datetime_type(type_str) => quote! {
            ::solverforge_core::domain::FieldType::Primitive(
                ::solverforge_core::domain::PrimitiveType::DateTime
            )
        },
        _ => {
            let type_name = extract_type_name(type_str);
            quote! {
                ::solverforge_core::domain::FieldType::object(#type_name)
            }
        }
    }
}

/// Check if a type string represents a DateTime type.
fn is_datetime_type(type_str: &str) -> bool {
    type_str == "NaiveDateTime"
        || type_str == "chrono::NaiveDateTime"
        || type_str == "::chrono::NaiveDateTime"
        || type_str == "DateTime<Utc>"
        || type_str == "chrono::DateTime<Utc>"
        || type_str == "::chrono::DateTime<Utc>"
        || type_str == "chrono::DateTime<chrono::Utc>"
        || type_str == "NaiveDate"
        || type_str == "chrono::NaiveDate"
        || type_str == "::chrono::NaiveDate"
}

/// Extract the simple type name from a potentially qualified type.
fn extract_type_name(type_str: &str) -> String {
    type_str.rsplit("::").next().unwrap_or(type_str).to_string()
}
