//! Implementation of the PlanningEntity derive macro.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse_macro_input, Attribute, Data, DeriveInput, Fields, Ident, LitBool, LitStr, Type};

/// Information about a field in the planning entity.
struct FieldInfo {
    name: Ident,
    ty: Type,
    is_planning_id: bool,
    planning_variable: Option<PlanningVariableInfo>,
    list_variable: Option<ListVariableInfo>,
    shadow_variable: Option<ShadowVariableInfo>,
}

/// Information about a planning variable attribute.
struct PlanningVariableInfo {
    value_range_provider_refs: Vec<String>,
    allows_unassigned: bool,
}

/// Information about a planning list variable attribute.
struct ListVariableInfo {
    value_range_provider_refs: Vec<String>,
    allows_unassigned_values: bool,
}

/// Information about a shadow variable attribute.
enum ShadowVariableInfo {
    InverseRelation { source: String },
    Index { source: String },
    NextElement { source: String },
    PreviousElement { source: String },
    Anchor { source: String },
}

/// Implementation of the `#[derive(PlanningEntity)]` macro.
pub fn derive_planning_entity_impl(input: TokenStream) -> TokenStream {
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
                    "PlanningEntity can only be derived for structs with named fields",
                )
                .to_compile_error()
                .into();
            }
        },
        _ => {
            return syn::Error::new_spanned(
                &input,
                "PlanningEntity can only be derived for structs",
            )
            .to_compile_error()
            .into();
        }
    };

    // Parse field information
    let mut field_infos: Vec<FieldInfo> = Vec::new();
    let mut planning_id_field: Option<&Ident> = None;

    for field in fields {
        let field_name = field.ident.as_ref().unwrap();
        let field_ty = &field.ty;

        let is_planning_id = has_attribute(&field.attrs, "planning_id");
        let planning_variable = parse_planning_variable_attr(&field.attrs);
        let list_variable = parse_planning_list_variable_attr(&field.attrs);
        let shadow_variable = parse_shadow_variable_attr(&field.attrs);

        if is_planning_id {
            if planning_id_field.is_some() {
                return syn::Error::new_spanned(
                    field_name,
                    "Only one field can be marked with #[planning_id]",
                )
                .to_compile_error()
                .into();
            }
            planning_id_field = Some(field_name);
        }

        field_infos.push(FieldInfo {
            name: field_name.clone(),
            ty: field_ty.clone(),
            is_planning_id,
            planning_variable,
            list_variable,
            shadow_variable,
        });
    }

    // Verify we have a planning_id
    let planning_id_field = match planning_id_field {
        Some(f) => f,
        None => {
            return syn::Error::new_spanned(
                &input,
                "PlanningEntity requires exactly one field marked with #[planning_id]",
            )
            .to_compile_error()
            .into();
        }
    };

    // Generate domain_class() implementation
    let domain_class_impl = generate_domain_class(&name_str, &field_infos);

    // Generate planning_id() implementation
    let planning_id_impl = generate_planning_id(planning_id_field, &field_infos);

    // Generate to_value() implementation
    let to_value_impl = generate_to_value(&field_infos);

    // Generate from_value() implementation
    let from_value_impl = generate_from_value(name, &field_infos);

    let expanded = quote! {
        impl ::solverforge_core::PlanningEntity for #name {
            fn domain_class() -> ::solverforge_core::domain::DomainClass {
                #domain_class_impl
            }

            fn planning_id(&self) -> ::solverforge_core::Value {
                #planning_id_impl
            }

            fn to_value(&self) -> ::solverforge_core::Value {
                #to_value_impl
            }

            fn from_value(value: &::solverforge_core::Value) -> ::solverforge_core::SolverForgeResult<Self> {
                #from_value_impl
            }
        }
    };

    TokenStream::from(expanded)
}

/// Check if a field has a specific attribute.
fn has_attribute(attrs: &[Attribute], name: &str) -> bool {
    attrs.iter().any(|attr| attr.path().is_ident(name))
}

/// Parse the #[planning_variable(...)] attribute using syn 2.x API.
fn parse_planning_variable_attr(attrs: &[Attribute]) -> Option<PlanningVariableInfo> {
    for attr in attrs {
        if attr.path().is_ident("planning_variable") {
            let mut value_range_provider_refs = Vec::new();
            let mut allows_unassigned = false;

            // Parse nested meta using syn 2.x API
            let _ = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("value_range_provider") {
                    let value: LitStr = meta.value()?.parse()?;
                    value_range_provider_refs.push(value.value());
                } else if meta.path.is_ident("allows_unassigned") {
                    let value: LitBool = meta.value()?.parse()?;
                    allows_unassigned = value.value();
                }
                Ok(())
            });

            return Some(PlanningVariableInfo {
                value_range_provider_refs,
                allows_unassigned,
            });
        }
    }
    None
}

/// Parse the #[planning_list_variable(...)] attribute using syn 2.x API.
fn parse_planning_list_variable_attr(attrs: &[Attribute]) -> Option<ListVariableInfo> {
    for attr in attrs {
        if attr.path().is_ident("planning_list_variable") {
            let mut value_range_provider_refs = Vec::new();
            let mut allows_unassigned_values = false;

            // Parse nested meta using syn 2.x API
            let _ = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("value_range_provider") {
                    let value: LitStr = meta.value()?.parse()?;
                    value_range_provider_refs.push(value.value());
                } else if meta.path.is_ident("allows_unassigned_values") {
                    let value: LitBool = meta.value()?.parse()?;
                    allows_unassigned_values = value.value();
                }
                Ok(())
            });

            return Some(ListVariableInfo {
                value_range_provider_refs,
                allows_unassigned_values,
            });
        }
    }
    None
}

/// Parse shadow variable attributes from a field.
/// Supports: inverse_relation_shadow, index_shadow, next_element_shadow,
/// previous_element_shadow, anchor_shadow
fn parse_shadow_variable_attr(attrs: &[Attribute]) -> Option<ShadowVariableInfo> {
    for attr in attrs {
        // Check each shadow variable attribute type
        if attr.path().is_ident("inverse_relation_shadow") {
            if let Some(source) = parse_source_attr(attr) {
                return Some(ShadowVariableInfo::InverseRelation { source });
            }
        } else if attr.path().is_ident("index_shadow") {
            if let Some(source) = parse_source_attr(attr) {
                return Some(ShadowVariableInfo::Index { source });
            }
        } else if attr.path().is_ident("next_element_shadow") {
            if let Some(source) = parse_source_attr(attr) {
                return Some(ShadowVariableInfo::NextElement { source });
            }
        } else if attr.path().is_ident("previous_element_shadow") {
            if let Some(source) = parse_source_attr(attr) {
                return Some(ShadowVariableInfo::PreviousElement { source });
            }
        } else if attr.path().is_ident("anchor_shadow") {
            if let Some(source) = parse_source_attr(attr) {
                return Some(ShadowVariableInfo::Anchor { source });
            }
        }
    }
    None
}

/// Parse the source = "..." from a shadow variable attribute.
fn parse_source_attr(attr: &Attribute) -> Option<String> {
    let mut source = None;
    let _ = attr.parse_nested_meta(|meta| {
        if meta.path.is_ident("source") {
            let value: LitStr = meta.value()?.parse()?;
            source = Some(value.value());
        }
        Ok(())
    });
    source
}

/// Generate the domain_class() method implementation.
fn generate_domain_class(struct_name: &str, fields: &[FieldInfo]) -> TokenStream2 {
    let field_descriptors: Vec<TokenStream2> = fields
        .iter()
        .map(|field| {
            let field_name = field.name.to_string();
            let field_type = rust_type_to_field_type(&field.ty);

            let mut annotations = Vec::new();

            if field.is_planning_id {
                annotations.push(quote! {
                    .with_planning_annotation(::solverforge_core::domain::PlanningAnnotation::PlanningId)
                });
            }

            if let Some(pv) = &field.planning_variable {
                let refs: Vec<_> = pv
                    .value_range_provider_refs
                    .iter()
                    .map(|s| quote! { #s.to_string() })
                    .collect();
                let allows_unassigned = pv.allows_unassigned;

                if allows_unassigned {
                    annotations.push(quote! {
                        .with_planning_annotation(
                            ::solverforge_core::domain::PlanningAnnotation::planning_variable_unassigned(
                                vec![#(#refs),*]
                            )
                        )
                    });
                } else {
                    annotations.push(quote! {
                        .with_planning_annotation(
                            ::solverforge_core::domain::PlanningAnnotation::planning_variable(
                                vec![#(#refs),*]
                            )
                        )
                    });
                }
            }

            if let Some(lv) = &field.list_variable {
                let refs: Vec<_> = lv
                    .value_range_provider_refs
                    .iter()
                    .map(|s| quote! { #s.to_string() })
                    .collect();
                let allows_unassigned_values = lv.allows_unassigned_values;

                if allows_unassigned_values {
                    annotations.push(quote! {
                        .with_planning_annotation(
                            ::solverforge_core::domain::PlanningAnnotation::planning_list_variable_unassigned(
                                vec![#(#refs),*]
                            )
                        )
                    });
                } else {
                    annotations.push(quote! {
                        .with_planning_annotation(
                            ::solverforge_core::domain::PlanningAnnotation::planning_list_variable(
                                vec![#(#refs),*]
                            )
                        )
                    });
                }
            }

            if let Some(sv) = &field.shadow_variable {
                match sv {
                    ShadowVariableInfo::InverseRelation { source } => {
                        annotations.push(quote! {
                            .with_planning_annotation(
                                ::solverforge_core::domain::PlanningAnnotation::inverse_relation_shadow(#source)
                            )
                        });
                    }
                    ShadowVariableInfo::Index { source } => {
                        annotations.push(quote! {
                            .with_planning_annotation(
                                ::solverforge_core::domain::PlanningAnnotation::index_shadow(#source)
                            )
                        });
                    }
                    ShadowVariableInfo::NextElement { source } => {
                        annotations.push(quote! {
                            .with_planning_annotation(
                                ::solverforge_core::domain::PlanningAnnotation::next_element_shadow(#source)
                            )
                        });
                    }
                    ShadowVariableInfo::PreviousElement { source } => {
                        annotations.push(quote! {
                            .with_planning_annotation(
                                ::solverforge_core::domain::PlanningAnnotation::previous_element_shadow(#source)
                            )
                        });
                    }
                    ShadowVariableInfo::Anchor { source } => {
                        annotations.push(quote! {
                            .with_planning_annotation(
                                ::solverforge_core::domain::PlanningAnnotation::anchor_shadow(#source)
                            )
                        });
                    }
                }
            }

            quote! {
                .with_field(
                    ::solverforge_core::domain::FieldDescriptor::new(#field_name, #field_type)
                    #(#annotations)*
                )
            }
        })
        .collect();

    quote! {
        ::solverforge_core::domain::DomainClass::new(#struct_name)
            .with_annotation(::solverforge_core::domain::PlanningAnnotation::PlanningEntity)
            #(#field_descriptors)*
    }
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
    if type_str == "i32" || type_str == "i64" || type_str == "isize" {
        return quote! {
            ::solverforge_core::domain::FieldType::Primitive(
                ::solverforge_core::domain::PrimitiveType::Long
            )
        };
    }
    if type_str == "u32" || type_str == "u64" || type_str == "usize" {
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

    // Handle Option<T> - extract inner type
    if type_str.starts_with("Option<") && type_str.ends_with('>') {
        let inner = &type_str[7..type_str.len() - 1];
        let inner_type = inner_type_to_field_type(inner);
        return inner_type;
    }

    // Handle Vec<T>
    if type_str.starts_with("Vec<") && type_str.ends_with('>') {
        let inner = &type_str[4..type_str.len() - 1];
        let inner_type = inner_type_to_field_type(inner);
        return quote! {
            ::solverforge_core::domain::FieldType::list(#inner_type)
        };
    }

    // Default to Object type with the type name
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
        "i32" | "i64" | "isize" | "u32" | "u64" | "usize" => quote! {
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
        _ => {
            let type_name = extract_type_name(type_str);
            quote! {
                ::solverforge_core::domain::FieldType::object(#type_name)
            }
        }
    }
}

/// Extract the simple type name from a potentially qualified type.
fn extract_type_name(type_str: &str) -> String {
    // Handle paths like "crate::Room" -> "Room"
    type_str.rsplit("::").next().unwrap_or(type_str).to_string()
}

/// Generate the planning_id() method implementation.
fn generate_planning_id(planning_id_field: &Ident, fields: &[FieldInfo]) -> TokenStream2 {
    let field_info = fields
        .iter()
        .find(|f| f.name == *planning_id_field)
        .unwrap();
    let field_name = &field_info.name;
    let field_ty = &field_info.ty;

    // Generate appropriate Value conversion based on type
    let type_str = quote!(#field_ty).to_string();
    if type_str.contains("String") {
        quote! {
            ::solverforge_core::Value::String(self.#field_name.clone())
        }
    } else {
        // For other types, try to convert to Value
        quote! {
            ::solverforge_core::Value::from(self.#field_name.clone())
        }
    }
}

/// Generate the to_value() method implementation.
fn generate_to_value(fields: &[FieldInfo]) -> TokenStream2 {
    let field_insertions: Vec<TokenStream2> = fields
        .iter()
        .map(|field| {
            let field_name = &field.name;
            let field_name_str = field.name.to_string();
            let field_ty = &field.ty;

            // Check if this is an Option type
            let type_str = quote!(#field_ty).to_string();
            if type_str.contains("Option") {
                quote! {
                    map.insert(
                        #field_name_str.to_string(),
                        self.#field_name.as_ref()
                            .map(|v| ::solverforge_core::Value::from(v.clone()))
                            .unwrap_or(::solverforge_core::Value::Null)
                    );
                }
            } else {
                quote! {
                    map.insert(
                        #field_name_str.to_string(),
                        ::solverforge_core::Value::from(self.#field_name.clone())
                    );
                }
            }
        })
        .collect();

    quote! {
        let mut map = ::std::collections::HashMap::new();
        #(#field_insertions)*
        ::solverforge_core::Value::Object(map)
    }
}

/// Generate the from_value() method implementation.
fn generate_from_value(struct_name: &Ident, fields: &[FieldInfo]) -> TokenStream2 {
    let field_extractions: Vec<TokenStream2> = fields
        .iter()
        .map(|field| {
            let field_name = &field.name;
            let field_name_str = field.name.to_string();
            let field_ty = &field.ty;

            // Check if this is an Option type
            let type_str = quote!(#field_ty).to_string();
            if type_str.contains("Option < String >") || type_str.contains("Option<String>") {
                quote! {
                    let #field_name: #field_ty = map.get(#field_name_str)
                        .and_then(|v| {
                            match v {
                                ::solverforge_core::Value::Null => None,
                                ::solverforge_core::Value::String(s) => Some(s.clone()),
                                _ => None,
                            }
                        });
                }
            } else if type_str.contains("Option < i64 >")
                || type_str.contains("Option<i64>")
                || type_str.contains("Option < i32 >")
                || type_str.contains("Option<i32>")
            {
                quote! {
                    let #field_name: #field_ty = map.get(#field_name_str)
                        .and_then(|v| {
                            match v {
                                ::solverforge_core::Value::Null => None,
                                ::solverforge_core::Value::Int(i) => Some(*i as _),
                                _ => None,
                            }
                        });
                }
            } else if type_str.contains("Option") {
                // Generic Option handling
                quote! {
                    let #field_name: #field_ty = map.get(#field_name_str)
                        .and_then(|v| {
                            match v {
                                ::solverforge_core::Value::Null => None,
                                ::solverforge_core::Value::String(s) => s.parse().ok(),
                                ::solverforge_core::Value::Int(i) => Some(*i as _),
                                _ => None,
                            }
                        });
                }
            } else if type_str.contains("String") {
                quote! {
                    let #field_name: #field_ty = map.get(#field_name_str)
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                        .ok_or_else(|| ::solverforge_core::SolverForgeError::Serialization(
                            format!("Missing or invalid field: {}", #field_name_str)
                        ))?;
                }
            } else if type_str.contains("i64") || type_str.contains("i32") {
                quote! {
                    let #field_name: #field_ty = map.get(#field_name_str)
                        .and_then(|v| v.as_int())
                        .map(|i| i as #field_ty)
                        .ok_or_else(|| ::solverforge_core::SolverForgeError::Serialization(
                            format!("Missing or invalid field: {}", #field_name_str)
                        ))?;
                }
            } else if type_str.contains("bool") {
                quote! {
                    let #field_name: #field_ty = map.get(#field_name_str)
                        .and_then(|v| v.as_bool())
                        .ok_or_else(|| ::solverforge_core::SolverForgeError::Serialization(
                            format!("Missing or invalid field: {}", #field_name_str)
                        ))?;
                }
            } else {
                // For other types, try a generic conversion
                quote! {
                    let #field_name: #field_ty = map.get(#field_name_str)
                        .and_then(|v| v.as_str())
                        .and_then(|s| s.parse().ok())
                        .ok_or_else(|| ::solverforge_core::SolverForgeError::Serialization(
                            format!("Missing or invalid field: {}", #field_name_str)
                        ))?;
                }
            }
        })
        .collect();

    let field_names: Vec<&Ident> = fields.iter().map(|f| &f.name).collect();

    quote! {
        match value {
            ::solverforge_core::Value::Object(map) => {
                #(#field_extractions)*
                Ok(#struct_name {
                    #(#field_names),*
                })
            }
            _ => Err(::solverforge_core::SolverForgeError::Serialization(
                "Expected object value".to_string()
            )),
        }
    }
}
