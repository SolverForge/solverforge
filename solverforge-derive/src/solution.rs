//! Implementation of the PlanningSolution derive macro.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse_macro_input, Attribute, Data, DeriveInput, Fields, Ident, LitStr, Type};

/// Information about a field in the planning solution.
struct SolutionFieldInfo {
    name: Ident,
    ty: Type,
    is_problem_fact_collection: bool,
    is_planning_entity_collection: bool,
    is_planning_score: bool,
    value_range_provider_id: Option<String>,
}

/// Information parsed from the struct-level attributes.
struct SolutionInfo {
    constraint_provider: Option<String>,
}

/// Implementation of the `#[derive(PlanningSolution)]` macro.
pub fn derive_planning_solution_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let name_str = name.to_string();

    // Parse struct-level attributes
    let solution_info = parse_solution_attrs(&input.attrs);

    // Extract fields from struct
    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => {
                return syn::Error::new_spanned(
                    &input,
                    "PlanningSolution can only be derived for structs with named fields",
                )
                .to_compile_error()
                .into();
            }
        },
        _ => {
            return syn::Error::new_spanned(
                &input,
                "PlanningSolution can only be derived for structs",
            )
            .to_compile_error()
            .into();
        }
    };

    // Parse field information
    let mut field_infos: Vec<SolutionFieldInfo> = Vec::new();
    let mut score_field: Option<&Ident> = None;
    let mut score_type: Option<Type> = None;

    for field in fields {
        let field_name = field.ident.as_ref().unwrap();
        let field_ty = &field.ty;

        let is_problem_fact_collection = has_attribute(&field.attrs, "problem_fact_collection");
        let is_planning_entity_collection =
            has_attribute(&field.attrs, "planning_entity_collection");
        let is_planning_score = has_attribute(&field.attrs, "planning_score");
        let value_range_provider_id = parse_value_range_provider_attr(&field.attrs);

        if is_planning_score {
            if score_field.is_some() {
                return syn::Error::new_spanned(
                    field_name,
                    "Only one field can be marked with #[planning_score]",
                )
                .to_compile_error()
                .into();
            }
            score_field = Some(field_name);
            score_type = Some(field_ty.clone());
        }

        field_infos.push(SolutionFieldInfo {
            name: field_name.clone(),
            ty: field_ty.clone(),
            is_problem_fact_collection,
            is_planning_entity_collection,
            is_planning_score,
            value_range_provider_id,
        });
    }

    // Verify we have a score field
    let score_field = match score_field {
        Some(f) => f,
        None => {
            return syn::Error::new_spanned(
                &input,
                "PlanningSolution requires exactly one field marked with #[planning_score]",
            )
            .to_compile_error()
            .into();
        }
    };

    // Extract inner score type from Option<ScoreType>
    let score_type = score_type.unwrap();
    let inner_score_type = extract_inner_option_type(&score_type);

    // Generate constraint provider call
    let constraints_impl = match &solution_info.constraint_provider {
        Some(fn_name) => {
            let fn_ident = syn::Ident::new(fn_name, proc_macro2::Span::call_site());
            quote! {
                #fn_ident()
            }
        }
        None => {
            quote! {
                ::solverforge_core::constraints::ConstraintSet::new()
            }
        }
    };

    // Generate domain_model() implementation
    let domain_model_impl = generate_domain_model(&name_str, &field_infos);

    // Generate to_json() implementation
    let to_json_impl = generate_to_json(&field_infos);

    // Generate from_json() implementation
    let from_json_impl = generate_from_json(name, &field_infos);

    let expanded = quote! {
        impl ::solverforge_core::PlanningSolution for #name {
            type Score = #inner_score_type;

            fn domain_model() -> ::solverforge_core::domain::DomainModel {
                #domain_model_impl
            }

            fn constraints() -> ::solverforge_core::constraints::ConstraintSet {
                #constraints_impl
            }

            fn score(&self) -> Option<Self::Score> {
                self.#score_field.clone()
            }

            fn set_score(&mut self, score: Self::Score) {
                self.#score_field = Some(score);
            }

            fn to_json(&self) -> ::solverforge_core::SolverForgeResult<String> {
                #to_json_impl
            }

            fn from_json(json: &str) -> ::solverforge_core::SolverForgeResult<Self> {
                #from_json_impl
            }
        }
    };

    TokenStream::from(expanded)
}

/// Parse struct-level attributes for PlanningSolution.
fn parse_solution_attrs(attrs: &[Attribute]) -> SolutionInfo {
    let mut constraint_provider = None;

    for attr in attrs {
        if attr.path().is_ident("constraint_provider") {
            // Parse #[constraint_provider = "fn_name"]
            if let syn::Meta::NameValue(nv) = &attr.meta {
                if let syn::Expr::Lit(expr_lit) = &nv.value {
                    if let syn::Lit::Str(lit_str) = &expr_lit.lit {
                        constraint_provider = Some(lit_str.value());
                    }
                }
            }
        }
    }

    SolutionInfo {
        constraint_provider,
    }
}

/// Check if a field has a specific attribute.
fn has_attribute(attrs: &[Attribute], name: &str) -> bool {
    attrs.iter().any(|attr| attr.path().is_ident(name))
}

/// Parse the #[value_range_provider(id = "...")] attribute.
fn parse_value_range_provider_attr(attrs: &[Attribute]) -> Option<String> {
    for attr in attrs {
        if attr.path().is_ident("value_range_provider") {
            let mut id = None;

            // Parse nested meta using syn 2.x API
            let _ = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("id") {
                    let value: LitStr = meta.value()?.parse()?;
                    id = Some(value.value());
                }
                Ok(())
            });

            return id;
        }
    }
    None
}

/// Extract the inner type from Option<T>.
fn extract_inner_option_type(ty: &Type) -> TokenStream2 {
    let type_str = quote!(#ty).to_string().replace(' ', "");

    if type_str.starts_with("Option<") && type_str.ends_with('>') {
        let inner = &type_str[7..type_str.len() - 1];
        let inner_ident = syn::parse_str::<Type>(inner).ok();
        if let Some(inner_ty) = inner_ident {
            return quote! { #inner_ty };
        }
    }

    // If not an Option or parsing fails, return the original type
    quote! { #ty }
}

/// Generate the domain_model() method implementation.
fn generate_domain_model(struct_name: &str, fields: &[SolutionFieldInfo]) -> TokenStream2 {
    // Collect entity classes from planning_entity_collection fields
    let entity_class_additions: Vec<TokenStream2> = fields
        .iter()
        .filter(|f| f.is_planning_entity_collection)
        .map(|f| {
            let element_type = extract_vec_element_type(&f.ty);
            quote! {
                .add_class(<#element_type as ::solverforge_core::PlanningEntity>::domain_class())
            }
        })
        .collect();

    // Generate field descriptors for the solution class
    let field_descriptors: Vec<TokenStream2> = fields
        .iter()
        .map(|field| {
            let field_name = field.name.to_string();
            let field_type = rust_type_to_field_type(&field.ty);

            let mut annotations = Vec::new();

            if field.is_problem_fact_collection {
                annotations.push(quote! {
                    .with_planning_annotation(
                        ::solverforge_core::domain::PlanningAnnotation::ProblemFactCollectionProperty
                    )
                });
            }

            if field.is_planning_entity_collection {
                annotations.push(quote! {
                    .with_planning_annotation(
                        ::solverforge_core::domain::PlanningAnnotation::PlanningEntityCollectionProperty
                    )
                });
            }

            if let Some(id) = &field.value_range_provider_id {
                annotations.push(quote! {
                    .with_planning_annotation(
                        ::solverforge_core::domain::PlanningAnnotation::value_range_provider_with_id(#id)
                    )
                });
            }

            if field.is_planning_score {
                annotations.push(quote! {
                    .with_planning_annotation(
                        ::solverforge_core::domain::PlanningAnnotation::planning_score()
                    )
                });
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
        ::solverforge_core::domain::DomainModel::builder()
            #(#entity_class_additions)*
            .add_class(
                ::solverforge_core::domain::DomainClass::new(#struct_name)
                    .with_annotation(::solverforge_core::domain::PlanningAnnotation::PlanningSolution)
                    #(#field_descriptors)*
            )
            .build()
    }
}

/// Extract the element type from Vec<T>.
fn extract_vec_element_type(ty: &Type) -> TokenStream2 {
    let type_str = quote!(#ty).to_string().replace(' ', "");

    if type_str.starts_with("Vec<") && type_str.ends_with('>') {
        let inner = &type_str[4..type_str.len() - 1];
        if let Ok(inner_ty) = syn::parse_str::<Type>(inner) {
            return quote! { #inner_ty };
        }
    }

    // Fallback - return a placeholder
    quote! { () }
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

    // Handle Option<T> - extract inner type for Score types
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
        // Score types
        "HardSoftScore"
        | "solverforge_core::HardSoftScore"
        | "::solverforge_core::HardSoftScore" => {
            quote! {
                ::solverforge_core::domain::FieldType::Score(
                    ::solverforge_core::domain::ScoreType::HardSoft
                )
            }
        }
        "SimpleScore" | "solverforge_core::SimpleScore" | "::solverforge_core::SimpleScore" => {
            quote! {
                ::solverforge_core::domain::FieldType::Score(
                    ::solverforge_core::domain::ScoreType::Simple
                )
            }
        }
        "HardMediumSoftScore"
        | "solverforge_core::HardMediumSoftScore"
        | "::solverforge_core::HardMediumSoftScore" => {
            quote! {
                ::solverforge_core::domain::FieldType::Score(
                    ::solverforge_core::domain::ScoreType::HardMediumSoft
                )
            }
        }
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

/// Generate the to_json() method implementation.
fn generate_to_json(fields: &[SolutionFieldInfo]) -> TokenStream2 {
    let field_insertions: Vec<TokenStream2> = fields
        .iter()
        .map(|field| {
            let field_name = &field.name;
            let field_name_str = field.name.to_string();
            let field_ty = &field.ty;

            let type_str = quote!(#field_ty).to_string();

            if field.is_planning_score {
                // Handle score field (Option<ScoreType>)
                quote! {
                    if let Some(score) = &self.#field_name {
                        map.insert(
                            #field_name_str.to_string(),
                            ::solverforge_core::Value::String(format!("{}", score))
                        );
                    }
                }
            } else if type_str.contains("Vec") {
                // Handle Vec<T> - check if it's a PlanningEntity collection
                if field.is_planning_entity_collection {
                    quote! {
                        let arr: Vec<::solverforge_core::Value> = self.#field_name
                            .iter()
                            .map(|item| ::solverforge_core::PlanningEntity::to_value(item))
                            .collect();
                        map.insert(#field_name_str.to_string(), ::solverforge_core::Value::Array(arr));
                    }
                } else {
                    // Problem fact collection - serialize as array using serde_json
                    quote! {
                        let arr: Vec<::solverforge_core::Value> = self.#field_name
                            .iter()
                            .filter_map(|item| {
                                ::serde_json::to_value(item).ok()
                                    .map(|json| ::solverforge_core::Value::from_json_value(json))
                            })
                            .collect();
                        map.insert(#field_name_str.to_string(), ::solverforge_core::Value::Array(arr));
                    }
                }
            } else if type_str.contains("Option") {
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
        ::serde_json::to_string(&::solverforge_core::Value::Object(map))
            .map_err(|e| ::solverforge_core::SolverForgeError::Serialization(e.to_string()))
    }
}

/// Generate the from_json() method implementation.
fn generate_from_json(struct_name: &Ident, fields: &[SolutionFieldInfo]) -> TokenStream2 {
    let field_extractions: Vec<TokenStream2> = fields
        .iter()
        .map(|field| {
            let field_name = &field.name;
            let field_name_str = field.name.to_string();
            let field_ty = &field.ty;

            let type_str = quote!(#field_ty).to_string();

            if field.is_planning_score {
                // Score field is Option<ScoreType>, default to None
                quote! {
                    let #field_name: #field_ty = None;
                }
            } else if type_str.contains("Vec") {
                if field.is_planning_entity_collection {
                    // Planning entity collection - use from_value
                    let element_type = extract_vec_element_type(&field.ty);
                    quote! {
                        let #field_name: #field_ty = match map.get(#field_name_str) {
                            Some(::solverforge_core::Value::Array(arr)) => {
                                arr.iter()
                                    .filter_map(|v| <#element_type as ::solverforge_core::PlanningEntity>::from_value(v).ok())
                                    .collect()
                            }
                            _ => Vec::new(),
                        };
                    }
                } else {
                    // Problem fact collection - basic deserialization
                    let element_type = extract_vec_element_type(&field.ty);
                    quote! {
                        let #field_name: #field_ty = match map.get(#field_name_str) {
                            Some(::solverforge_core::Value::Array(arr)) => {
                                arr.iter()
                                    .filter_map(|v| {
                                        ::serde_json::from_value::<#element_type>(
                                            ::serde_json::to_value(v).ok()?
                                        ).ok()
                                    })
                                    .collect()
                            }
                            _ => Vec::new(),
                        };
                    }
                }
            } else if type_str.contains("Option") {
                quote! {
                    let #field_name: #field_ty = map.get(#field_name_str)
                        .and_then(|v| {
                            match v {
                                ::solverforge_core::Value::Null => None,
                                _ => ::serde_json::from_value(
                                    ::serde_json::to_value(v).ok()?
                                ).ok(),
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
                // For other types, try JSON deserialization
                quote! {
                    let #field_name: #field_ty = map.get(#field_name_str)
                        .and_then(|v| {
                            ::serde_json::from_value(
                                ::serde_json::to_value(v).ok()?
                            ).ok()
                        })
                        .ok_or_else(|| ::solverforge_core::SolverForgeError::Serialization(
                            format!("Missing or invalid field: {}", #field_name_str)
                        ))?;
                }
            }
        })
        .collect();

    let field_names: Vec<&Ident> = fields.iter().map(|f| &f.name).collect();

    quote! {
        let value: ::solverforge_core::Value = ::serde_json::from_str(json)
            .map_err(|e| ::solverforge_core::SolverForgeError::Serialization(e.to_string()))?;

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
