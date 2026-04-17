use proc_macro2::TokenStream;
use quote::quote;
use syn::{Error, Ident, Type};

use crate::attr_parse::{get_attribute, parse_attribute_string};
use crate::list_registry::record_list_entity_metadata;

pub(super) fn generate_list_metadata(
    entity_name: &Ident,
    list_variables: &[&syn::Field],
) -> Result<TokenStream, Error> {
    let Some(field) = list_variables.first().copied() else {
        return Ok(TokenStream::new());
    };

    let field_name = field.ident.as_ref().unwrap();
    let field_name_str = field_name.to_string();
    let attr = get_attribute(&field.attrs, "planning_list_variable").unwrap();
    let element_collection = parse_attribute_string(attr, "element_collection").ok_or_else(|| {
        Error::new_spanned(
            field,
            "#[planning_list_variable] requires `element_collection = \"solution_field\"` for the canonical solver path",
        )
    })?;

    ensure_vec_usize(&field.ty, field)?;

    let cross_dm_ty = parse_type_or_default(
        parse_attribute_string(attr, "distance_meter"),
        "::solverforge::__internal::DefaultCrossEntityDistanceMeter",
        "distance_meter",
        field,
    )?;
    let intra_dm_ty = parse_type_or_default(
        parse_attribute_string(attr, "intra_distance_meter"),
        "::solverforge::__internal::DefaultCrossEntityDistanceMeter",
        "intra_distance_meter",
        field,
    )?;
    let cross_dm_expr = parse_default_expr(
        parse_attribute_string(attr, "distance_meter"),
        "::solverforge::__internal::DefaultCrossEntityDistanceMeter",
        "distance_meter",
        field,
    )?;
    let intra_dm_expr = parse_default_expr(
        parse_attribute_string(attr, "intra_distance_meter"),
        "::solverforge::__internal::DefaultCrossEntityDistanceMeter",
        "intra_distance_meter",
        field,
    )?;
    let solution_trait_bound = parse_solution_trait_bound(attr, field)?;
    let solution_where_clause = solution_trait_bound
        .as_ref()
        .map(|bound| quote! { where Solution: #bound });
    let merge_feasible = option_fn_expr(
        parse_attribute_string(attr, "merge_feasible_fn"),
        "merge_feasible_fn",
        field,
    )?;
    let cw_depot = option_fn_expr(
        parse_attribute_string(attr, "cw_depot_fn"),
        "cw_depot_fn",
        field,
    )?;
    let cw_dist = option_fn_expr(
        parse_attribute_string(attr, "cw_distance_fn"),
        "cw_distance_fn",
        field,
    )?;
    let cw_load = option_fn_expr(
        parse_attribute_string(attr, "cw_element_load_fn"),
        "cw_element_load_fn",
        field,
    )?;
    let cw_cap = option_fn_expr(
        parse_attribute_string(attr, "cw_capacity_fn"),
        "cw_capacity_fn",
        field,
    )?;
    let cw_assign = option_fn_expr(
        parse_attribute_string(attr, "cw_assign_route_fn"),
        "cw_assign_route_fn",
        field,
    )?;
    let k_opt_get = option_fn_expr(
        parse_attribute_string(attr, "k_opt_get_route"),
        "k_opt_get_route",
        field,
    )?;
    let k_opt_set = option_fn_expr(
        parse_attribute_string(attr, "k_opt_set_route"),
        "k_opt_set_route",
        field,
    )?;
    let k_opt_depot = option_fn_expr(
        parse_attribute_string(attr, "k_opt_depot_fn"),
        "k_opt_depot_fn",
        field,
    )?;
    let k_opt_dist = option_fn_expr(
        parse_attribute_string(attr, "k_opt_distance_fn"),
        "k_opt_distance_fn",
        field,
    )?;
    let k_opt_feasible = option_fn_expr(
        parse_attribute_string(attr, "k_opt_feasible_fn"),
        "k_opt_feasible_fn",
        field,
    )?;

    record_list_entity_metadata(&entity_name.to_string(), element_collection.clone());

    Ok(quote! {
        pub const __SOLVERFORGE_LIST_VARIABLE_COUNT: usize = 1;
        pub const __SOLVERFORGE_LIST_VARIABLE_NAME: &'static str = #field_name_str;
        pub const __SOLVERFORGE_LIST_ELEMENT_COLLECTION: &'static str = #element_collection;

        #[inline]
        pub fn __solverforge_list_field(entity: &Self) -> &[usize] {
            &entity.#field_name
        }

        #[inline]
        pub fn __solverforge_list_field_mut(entity: &mut Self) -> &mut ::std::vec::Vec<usize> {
            &mut entity.#field_name
        }

        #[inline]
        pub fn __solverforge_list_metadata<Solution>() -> ::solverforge::__internal::ListVariableMetadata<
            Solution,
            #cross_dm_ty,
            #intra_dm_ty,
        >
        #solution_where_clause
        {
            let _ = stringify!(#entity_name);
            let _ = #element_collection;
            ::solverforge::__internal::ListVariableMetadata::new(
                #cross_dm_expr,
                #intra_dm_expr,
                #merge_feasible,
                #cw_depot,
                #cw_dist,
                #cw_load,
                #cw_cap,
                #cw_assign,
                #k_opt_get,
                #k_opt_set,
                #k_opt_depot,
                #k_opt_dist,
                #k_opt_feasible,
            )
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
                        ::core::option::Option::None,
                        ::core::option::Option::None,
                    )
                }
            }
        });
    };

    let attr = get_attribute(&field.attrs, "planning_list_variable").unwrap();
    let cross_dm_ty = parse_type_or_default(
        parse_attribute_string(attr, "distance_meter"),
        "::solverforge::__internal::DefaultCrossEntityDistanceMeter",
        "distance_meter",
        field,
    )?;
    let intra_dm_ty = parse_type_or_default(
        parse_attribute_string(attr, "intra_distance_meter"),
        "::solverforge::__internal::DefaultCrossEntityDistanceMeter",
        "intra_distance_meter",
        field,
    )?;
    let solution_trait_bound = parse_solution_trait_bound(attr, field)?;
    let element_source = parse_attribute_string(attr, "element_collection").ok_or_else(|| {
        Error::new_spanned(
            field,
            "#[planning_list_variable] requires `element_collection = \"solution_collection\"` for the canonical solver path",
        )
    })?;
    let solution_bound = solution_trait_bound
        .as_ref()
        .map(|bound| quote! { + #bound })
        .unwrap_or_default();

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
    attr: &syn::Attribute,
    span: &impl quote::ToTokens,
) -> Result<Option<syn::TypeParamBound>, Error> {
    parse_attribute_string(attr, "solution_trait")
        .map(|path| {
            syn::parse_str(&path)
                .map_err(|_| Error::new_spanned(span, "solution_trait must be a valid trait path"))
        })
        .transpose()
}

fn option_fn_expr(
    path: Option<String>,
    label: &str,
    span: &impl quote::ToTokens,
) -> Result<syn::Expr, Error> {
    if let Some(path) = path {
        let parsed: syn::Path = syn::parse_str(&path)
            .map_err(|_| Error::new_spanned(span, format!("{label} must be a valid path")))?;
        Ok(syn::parse_quote! { ::core::option::Option::Some(#parsed) })
    } else {
        Ok(syn::parse_quote! { ::core::option::Option::None })
    }
}
