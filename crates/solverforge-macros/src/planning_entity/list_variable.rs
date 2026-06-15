use proc_macro2::TokenStream;
use quote::quote;
use syn::{Error, Ident, Type};

use crate::attr_parse::{get_attribute, parse_attribute_string};

pub(super) fn generate_list_metadata(
    entity_name: &Ident,
    list_variables: &[&syn::Field],
) -> Result<TokenStream, Error> {
    let Some(field) = list_variables.first().copied() else {
        return Ok(quote! {
            pub const __SOLVERFORGE_LIST_VARIABLE_COUNT: usize = 0;
        });
    };

    let field_name = field.ident.as_ref().unwrap();
    let entity_name_str = entity_name.to_string();
    let field_name_str = field_name.to_string();
    let attr = get_attribute(&field.attrs, "planning_list_variable").unwrap();
    let element_collection = parse_attribute_string(attr, "element_collection").ok_or_else(|| {
        Error::new_spanned(
            field,
            "#[planning_list_variable] requires `element_collection = \"solution_field\"` for the canonical solver path",
        )
    })?;

    ensure_vec_usize(&field.ty, field)?;

    let distance_meter = parse_profiled_attribute(attr, "distance_meter", field)?;
    let intra_distance_meter = parse_profiled_attribute(attr, "intra_distance_meter", field)?;
    let solution_trait = parse_profiled_attribute(attr, "solution_trait", field)?;
    let route_hooks_attr = parse_profiled_attribute(attr, "route_hooks", field)?;
    let savings_hooks_attr = parse_profiled_attribute(attr, "savings_hooks", field)?;
    let savings_metric_class_attr =
        parse_profiled_attribute(attr, "savings_metric_class_fn", field)?;

    let cross_dm_ty = parse_type_or_default(
        distance_meter.clone(),
        "::solverforge::__internal::DefaultCrossEntityDistanceMeter",
        "distance_meter",
        field,
    )?;
    let intra_dm_ty = parse_type_or_default(
        intra_distance_meter.clone(),
        "::solverforge::__internal::DefaultCrossEntityDistanceMeter",
        "intra_distance_meter",
        field,
    )?;
    let cross_dm_expr = parse_default_expr(
        distance_meter,
        "::solverforge::__internal::DefaultCrossEntityDistanceMeter",
        "distance_meter",
        field,
    )?;
    let intra_dm_expr = parse_default_expr(
        intra_distance_meter,
        "::solverforge::__internal::DefaultCrossEntityDistanceMeter",
        "intra_distance_meter",
        field,
    )?;
    let solution_trait_bound = parse_solution_trait_bound(solution_trait, field)?;
    let element_owner_path = parse_optional_path(
        parse_attribute_string(attr, "element_owner_fn"),
        "element_owner_fn",
        field,
    )?;
    let solution_where_clause = match (solution_trait_bound.as_ref(), element_owner_path.is_some())
    {
        (Some(bound), true) => {
            quote! { where Solution: #bound + ::solverforge::__internal::PlanningModelSupport }
        }
        (Some(bound), false) => quote! { where Solution: #bound },
        (None, true) => {
            quote! { where Solution: ::solverforge::__internal::PlanningModelSupport }
        }
        (None, false) => quote! {},
    };
    let route_hooks = parse_optional_path(route_hooks_attr, "route_hooks", field)?;
    let savings_hooks = parse_optional_path(savings_hooks_attr, "savings_hooks", field)?;
    let savings_metric_class_path =
        parse_optional_path(savings_metric_class_attr, "savings_metric_class_fn", field)?;
    let route_get = hook_fn_expr(route_hooks.as_ref(), "get");
    let route_set = hook_fn_expr(route_hooks.as_ref(), "set");
    let route_depot = hook_fn_expr(route_hooks.as_ref(), "depot");
    let route_distance = hook_fn_expr(route_hooks.as_ref(), "distance");
    let route_feasible = hook_fn_expr(route_hooks.as_ref(), "feasible");
    let savings_depot = hook_fn_expr(savings_hooks.as_ref(), "depot");
    let savings_metric_class = optional_hook_fn_expr(savings_metric_class_path.as_ref());
    let savings_distance = hook_fn_expr(savings_hooks.as_ref(), "distance");
    let savings_feasible = hook_fn_expr(savings_hooks.as_ref(), "feasible");
    let (element_owner_adapter, element_owner) = if element_owner_path.is_some() {
        (
            quote! {
                #[inline]
                fn __solverforge_list_element_owner<Solution>(
                    solution: &Solution,
                    element: &usize,
                ) -> ::core::option::Option<usize>
                #solution_where_clause
                {
                    <Solution as ::solverforge::__internal::PlanningModelSupport>::list_element_owner(
                        #entity_name_str,
                        #field_name_str,
                        solution,
                        element,
                    )
                }
            },
            quote! {
                ::core::option::Option::Some(
                    __solverforge_list_element_owner::<Solution>
                        as fn(&Solution, &usize) -> ::core::option::Option<usize>
                )
            },
        )
    } else {
        (quote! {}, quote! { ::core::option::Option::None })
    };
    Ok(quote! {
        pub const __SOLVERFORGE_LIST_VARIABLE_COUNT: usize = 1;
        const __SOLVERFORGE_LIST_VARIABLE_NAME: &'static str = #field_name_str;
        const __SOLVERFORGE_LIST_ELEMENT_COLLECTION: &'static str = #element_collection;

        #[inline]
        fn __solverforge_list_field(entity: &Self) -> &[usize] {
            &entity.#field_name
        }

        #[inline]
        fn __solverforge_list_field_mut(entity: &mut Self) -> &mut ::std::vec::Vec<usize> {
            &mut entity.#field_name
        }

        #[inline]
        fn __solverforge_list_metadata<Solution>() -> ::solverforge::__internal::ListVariableMetadata<
            Solution,
            #cross_dm_ty,
            #intra_dm_ty,
        >
        #solution_where_clause
        {
            let _ = #entity_name_str;
            let _ = #element_collection;
            #element_owner_adapter
            ::solverforge::__internal::ListVariableMetadata::new(
                #cross_dm_expr,
                #intra_dm_expr,
                #route_get,
                #route_set,
                #route_depot,
                #route_distance,
                #route_feasible,
                #savings_depot,
                #savings_metric_class,
                #savings_distance,
                #savings_feasible,
            )
            .with_element_owner_fn(#element_owner)
        }

    })
}

pub(super) fn generate_list_trait_impl(
    entity_name: &Ident,
    list_variables: &[&syn::Field],
) -> Result<TokenStream, Error> {
    let Some(field) = list_variables.first().copied() else {
        return Ok(quote! {
            impl<Solution> ::solverforge::__internal::ListVariableEntity<Solution> for #entity_name
            where
                Solution: ::solverforge::__internal::PlanningSolution,
            {
                type CrossDistanceMeter = ::solverforge::__internal::DefaultCrossEntityDistanceMeter;
                type IntraDistanceMeter = ::solverforge::__internal::DefaultCrossEntityDistanceMeter;

                const HAS_LIST_VARIABLE: bool = false;
                const LIST_VARIABLE_NAME: &'static str = "";
                const LIST_ELEMENT_SOURCE: ::core::option::Option<&'static str> =
                    ::core::option::Option::None;

                #[inline]
                fn list_field(_entity: &Self) -> &[usize] {
                    panic!("ListVariableEntity::list_field called on an entity without #[planning_list_variable]");
                }

                #[inline]
                fn list_field_mut(_entity: &mut Self) -> &mut ::std::vec::Vec<usize> {
                    panic!("ListVariableEntity::list_field_mut called on an entity without #[planning_list_variable]");
                }

                #[inline]
                fn list_metadata() -> ::solverforge::__internal::ListVariableMetadata<
                    Solution,
                    Self::CrossDistanceMeter,
                    Self::IntraDistanceMeter,
                > {
                    ::solverforge::__internal::ListVariableMetadata::new(
                        ::solverforge::__internal::DefaultCrossEntityDistanceMeter::default(),
                        ::solverforge::__internal::DefaultCrossEntityDistanceMeter::default(),
                        ::core::option::Option::None,
                        ::core::option::Option::None,
                        ::core::option::Option::None,
                        ::core::option::Option::None,
                        ::core::option::Option::None,
                        ::core::option::Option::None,
                        ::core::option::Option::None,
                        ::core::option::Option::None,
                        ::core::option::Option::None,
                    )
                }
            }
        });
    };

    let attr = get_attribute(&field.attrs, "planning_list_variable").unwrap();
    let distance_meter = parse_profiled_attribute(attr, "distance_meter", field)?;
    let intra_distance_meter = parse_profiled_attribute(attr, "intra_distance_meter", field)?;
    let solution_trait = parse_profiled_attribute(attr, "solution_trait", field)?;
    let cross_dm_ty = parse_type_or_default(
        distance_meter,
        "::solverforge::__internal::DefaultCrossEntityDistanceMeter",
        "distance_meter",
        field,
    )?;
    let intra_dm_ty = parse_type_or_default(
        intra_distance_meter,
        "::solverforge::__internal::DefaultCrossEntityDistanceMeter",
        "intra_distance_meter",
        field,
    )?;
    let solution_trait_bound = parse_solution_trait_bound(solution_trait, field)?;
    let has_element_owner = parse_attribute_string(attr, "element_owner_fn").is_some();
    let element_source = parse_attribute_string(attr, "element_collection").ok_or_else(|| {
        Error::new_spanned(
            field,
            "#[planning_list_variable] requires `element_collection = \"solution_collection\"` for the canonical solver path",
        )
    })?;
    let solution_bound = match (solution_trait_bound.as_ref(), has_element_owner) {
        (Some(bound), true) => {
            quote! { + #bound + ::solverforge::__internal::PlanningModelSupport }
        }
        (Some(bound), false) => quote! { + #bound },
        (None, true) => quote! { + ::solverforge::__internal::PlanningModelSupport },
        (None, false) => quote! {},
    };

    Ok(quote! {
        impl<Solution> ::solverforge::__internal::ListVariableEntity<Solution> for #entity_name
        where
            Solution: ::solverforge::__internal::PlanningSolution #solution_bound,
        {
            type CrossDistanceMeter = #cross_dm_ty;
            type IntraDistanceMeter = #intra_dm_ty;

            const HAS_LIST_VARIABLE: bool = true;
            const LIST_VARIABLE_NAME: &'static str = Self::__SOLVERFORGE_LIST_VARIABLE_NAME;
            const LIST_ELEMENT_SOURCE: ::core::option::Option<&'static str> =
                ::core::option::Option::Some(#element_source);

            #[inline]
            fn list_field(entity: &Self) -> &[usize] {
                Self::__solverforge_list_field(entity)
            }

            #[inline]
            fn list_field_mut(entity: &mut Self) -> &mut ::std::vec::Vec<usize> {
                Self::__solverforge_list_field_mut(entity)
            }

            #[inline]
            fn list_metadata() -> ::solverforge::__internal::ListVariableMetadata<
                Solution,
                Self::CrossDistanceMeter,
                Self::IntraDistanceMeter,
            > {
                Self::__solverforge_list_metadata::<Solution>()
            }
        }
    })
}

fn ensure_vec_usize(ty: &Type, field: &syn::Field) -> Result<(), Error> {
    let Some(inner) = field_vec_inner_type(ty) else {
        return Err(Error::new_spanned(
            field,
            "#[planning_list_variable] requires a field of type Vec<usize>",
        ));
    };
    let Type::Path(type_path) = inner else {
        return Err(Error::new_spanned(
            field,
            "#[planning_list_variable] requires a field of type Vec<usize>",
        ));
    };
    let Some(segment) = type_path.path.segments.last() else {
        return Err(Error::new_spanned(
            field,
            "#[planning_list_variable] requires a field of type Vec<usize>",
        ));
    };
    if segment.ident != "usize" {
        return Err(Error::new_spanned(
            field,
            "#[planning_list_variable] currently requires Vec<usize> on the canonical solver path",
        ));
    }
    Ok(())
}

fn field_vec_inner_type(ty: &Type) -> Option<&Type> {
    let Type::Path(type_path) = ty else {
        return None;
    };
    let segment = type_path.path.segments.last()?;
    if segment.ident != "Vec" {
        return None;
    }
    let syn::PathArguments::AngleBracketed(args) = &segment.arguments else {
        return None;
    };
    let Some(syn::GenericArgument::Type(inner)) = args.args.first() else {
        return None;
    };
    Some(inner)
}

fn parse_type_or_default(
    path: Option<String>,
    default: &str,
    label: &str,
    span: &impl quote::ToTokens,
) -> Result<Type, Error> {
    let raw = path.unwrap_or_else(|| default.to_string());
    syn::parse_str(&raw)
        .map_err(|_| Error::new_spanned(span, format!("{label} must be a valid type path")))
}

fn parse_default_expr(
    path: Option<String>,
    default: &str,
    label: &str,
    span: &impl quote::ToTokens,
) -> Result<syn::Expr, Error> {
    if let Some(path) = path {
        let parsed: syn::Path = syn::parse_str(&path)
            .map_err(|_| Error::new_spanned(span, format!("{label} must be a valid path")))?;
        Ok(syn::parse_quote! { #parsed::default() })
    } else {
        syn::parse_str(default)
            .map_err(|_| Error::new_spanned(span, format!("{label} must be a valid path")))
    }
}

fn parse_solution_trait_bound(
    path: Option<String>,
    span: &impl quote::ToTokens,
) -> Result<Option<syn::TypeParamBound>, Error> {
    path.map(|path| {
        syn::parse_str(&path)
            .map_err(|_| Error::new_spanned(span, "solution_trait must be a valid trait path"))
    })
    .transpose()
}

fn hook_fn_expr(module_path: Option<&syn::Path>, fn_name: &str) -> TokenStream {
    if let Some(module_path) = module_path {
        let fn_ident = Ident::new(fn_name, proc_macro2::Span::call_site());
        quote! { ::core::option::Option::Some(#module_path::#fn_ident) }
    } else {
        quote! { ::core::option::Option::None }
    }
}

fn optional_hook_fn_expr(path: Option<&syn::Path>) -> TokenStream {
    if let Some(path) = path {
        quote! { ::core::option::Option::Some(#path) }
    } else {
        quote! { ::core::option::Option::None }
    }
}

fn parse_optional_path(
    path: Option<String>,
    label: &str,
    span: &impl quote::ToTokens,
) -> Result<Option<syn::Path>, Error> {
    path.map(|path| {
        syn::parse_str(&path)
            .map_err(|_| Error::new_spanned(span, format!("{label} must be a valid path")))
    })
    .transpose()
}

#[derive(Clone, Copy)]
enum ListVariableDomain {
    Cvrp,
}

impl ListVariableDomain {
    fn from_attr(
        attr: &syn::Attribute,
        span: &impl quote::ToTokens,
    ) -> Result<Option<Self>, Error> {
        parse_attribute_string(attr, "domain")
            .map(|domain| match domain.as_str() {
                "cvrp" => Ok(Self::Cvrp),
                _ => Err(Error::new_spanned(
                    span,
                    format!(
                        "unsupported planning_list_variable domain `{domain}`; supported domains are cvrp"
                    ),
                )),
            })
            .transpose()
    }

    fn default_for(self, name: &str) -> Option<&'static str> {
        match self {
            Self::Cvrp => match name {
                "distance_meter" => Some("::solverforge::cvrp::MatrixDistanceMeter"),
                "intra_distance_meter" => Some("::solverforge::cvrp::MatrixIntraDistanceMeter"),
                "solution_trait" => Some("::solverforge::cvrp::VrpSolution"),
                "route_hooks" => Some("::solverforge::cvrp::route_hooks"),
                "savings_hooks" => Some("::solverforge::cvrp::savings_hooks"),
                "savings_metric_class_fn" => Some("::solverforge::cvrp::savings_metric_class"),
                _ => None,
            },
        }
    }
}

fn parse_profiled_attribute(
    attr: &syn::Attribute,
    name: &str,
    span: &impl quote::ToTokens,
) -> Result<Option<String>, Error> {
    let explicit = parse_attribute_string(attr, name);
    let Some(domain) = ListVariableDomain::from_attr(attr, span)? else {
        return Ok(explicit);
    };
    let Some(default) = domain.default_for(name) else {
        return Ok(explicit);
    };

    if let Some(value) = explicit {
        if normalize_path(&value) == normalize_path(default) {
            Ok(Some(default.to_string()))
        } else {
            Err(Error::new_spanned(
                span,
                format!(
                    "`domain = \"cvrp\"` provides `{name}`; remove `{name}` or use the stock path `{default}`"
                ),
            ))
        }
    } else {
        Ok(Some(default.to_string()))
    }
}

fn normalize_path(path: &str) -> String {
    path.chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<String>()
        .trim_start_matches("::")
        .to_string()
}
