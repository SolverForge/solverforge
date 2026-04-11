// #[planning_entity] derive macro implementation

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Error, Fields, Ident, Type};

use crate::attr_parse::{
    get_attribute, has_attribute, parse_attribute_bool, parse_attribute_string,
};
use crate::list_registry::record_list_entity_metadata;

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

    if list_variables.len() > 1 {
        return Err(Error::new_spanned(
            &input,
            "#[planning_entity] currently supports at most one #[planning_list_variable] field",
        ));
    }

    let name_str = name.to_string();

    // Shadow variables
    let inverse_relation_vars: Vec<_> = fields
        .iter()
        .filter(|f| has_attribute(&f.attrs, "inverse_relation_shadow_variable"))
        .collect();

    let previous_element_vars: Vec<_> = fields
        .iter()
        .filter(|f| has_attribute(&f.attrs, "previous_element_shadow_variable"))
        .collect();

    let next_element_vars: Vec<_> = fields
        .iter()
        .filter(|f| has_attribute(&f.attrs, "next_element_shadow_variable"))
        .collect();

    let cascading_update_vars: Vec<_> = fields
        .iter()
        .filter(|f| has_attribute(&f.attrs, "cascading_update_shadow_variable"))
        .collect();

    let variable_descriptors: Vec<_> = planning_variables
        .iter()
        .map(|field| {
            let field_name = field.ident.as_ref().unwrap();
            let field_name_str = field_name.to_string();
            let supports_usize_hooks = field_is_option_usize(&field.ty);
            let attr = get_attribute(&field.attrs, "planning_variable").unwrap();
            let allows_unassigned =
                parse_attribute_bool(attr, "allows_unassigned").unwrap_or(false);
            let is_chained = parse_attribute_bool(attr, "chained").unwrap_or(false);
            let value_range_provider = parse_attribute_string(attr, "value_range_provider")
                .or_else(|| parse_attribute_string(attr, "value_range"));
            let countable_range = parse_attribute_string(attr, "countable_range");
            let getter_name = syn::Ident::new(
                &format!("__solverforge_get_{}", field_name_str),
                proc_macro2::Span::call_site(),
            );
            let setter_name = syn::Ident::new(
                &format!("__solverforge_set_{}", field_name_str),
                proc_macro2::Span::call_site(),
            );

            let base = if is_chained {
                quote! { ::solverforge::__internal::VariableDescriptor::chained(#field_name_str) }
            } else {
                let maybe_usize_accessors = if supports_usize_hooks {
                    quote! { .with_usize_accessors(Self::#getter_name, Self::#setter_name) }
                } else {
                    TokenStream::new()
                };
                quote! {
                    ::solverforge::__internal::VariableDescriptor::genuine(#field_name_str)
                        .with_allows_unassigned(#allows_unassigned)
                        #maybe_usize_accessors
                }
            };

            let provider_is_entity_field =
                value_range_provider.as_ref().is_some_and(|provider_id| {
                    fields.iter().any(|candidate| {
                        candidate
                            .ident
                            .as_ref()
                            .map(|ident| ident == provider_id)
                            .unwrap_or(false)
                    })
                });

            let with_provider = if let Some(provider_id) = value_range_provider {
                let provider_getter_name = syn::Ident::new(
                    &format!("__solverforge_values_for_{}", field_name_str),
                    proc_macro2::Span::call_site(),
                );
                let maybe_entity_provider = if supports_usize_hooks && provider_is_entity_field {
                    quote! { .with_entity_value_provider(Self::#provider_getter_name) }
                } else {
                    TokenStream::new()
                };
                quote! {
                    #base
                        .with_value_range(#provider_id)
                        #maybe_entity_provider
                }
            } else {
                base
            };

            if let Some(range) = countable_range {
                let parts: Vec<_> = range.split("..").collect();
                if parts.len() != 2 {
                    return quote! {
                        compile_error!("countable_range must use `from..to` syntax");
                    };
                }
                let from_lit: i64 = parts[0]
                    .trim()
                    .parse()
                    .expect("countable_range start must be an integer");
                let to_lit: i64 = parts[1]
                    .trim()
                    .parse()
                    .expect("countable_range end must be an integer");
                quote! {
                    #with_provider.with_value_range_type(
                        ::solverforge::__internal::ValueRangeType::CountableRange {
                            from: #from_lit,
                            to: #to_lit,
                        }
                    )
                }
            } else {
                with_provider
            }
        })
        .collect();

    let variable_helpers: Vec<_> = planning_variables
        .iter()
        .map(|field| {
            let field_name = field.ident.as_ref().unwrap();
            let field_name_str = field_name.to_string();
            let supports_usize_hooks = field_is_option_usize(&field.ty);
            let attr = get_attribute(&field.attrs, "planning_variable").unwrap();
            let value_range_provider = parse_attribute_string(attr, "value_range_provider")
                .or_else(|| parse_attribute_string(attr, "value_range"));
            let provider_helper = value_range_provider
                .filter(|provider_id| {
                    fields.iter().any(|candidate| {
                        candidate.ident.as_ref().map(|ident| ident == provider_id).unwrap_or(false)
                    })
                })
                .map(|provider_id| {
                    let provider_field =
                        syn::Ident::new(&provider_id, proc_macro2::Span::call_site());
                    let provider_getter_name = syn::Ident::new(
                        &format!("__solverforge_values_for_{}", field_name_str),
                        proc_macro2::Span::call_site(),
                    );
                    quote! {
                        #[inline]
                        fn #provider_getter_name(entity: &dyn ::std::any::Any) -> ::std::vec::Vec<usize> {
                            let entity = entity
                                .downcast_ref::<Self>()
                                .expect("entity type mismatch for value provider");
                            entity.#provider_field.clone()
                        }
                    }
                });

            if supports_usize_hooks {
                let getter_name = syn::Ident::new(
                    &format!("__solverforge_get_{}", field_name_str),
                    proc_macro2::Span::call_site(),
                );
                let setter_name = syn::Ident::new(
                    &format!("__solverforge_set_{}", field_name_str),
                    proc_macro2::Span::call_site(),
                );

                quote! {
                    #[inline]
                    fn #getter_name(entity: &dyn ::std::any::Any) -> ::core::option::Option<usize> {
                        let entity = entity
                            .downcast_ref::<Self>()
                            .expect("entity type mismatch for planning variable getter");
                        entity.#field_name
                    }

                    #[inline]
                    fn #setter_name(
                        entity: &mut dyn ::std::any::Any,
                        value: ::core::option::Option<usize>,
                    ) {
                        let entity = entity
                            .downcast_mut::<Self>()
                            .expect("entity type mismatch for planning variable setter");
                        entity.#field_name = value;
                    }

                    #provider_helper
                }
            } else {
                provider_helper.unwrap_or_default()
            }
        })
        .collect();

    let optional_planning_variables: Vec<_> = planning_variables
        .iter()
        .filter_map(|field| {
            let field_name = field.ident.as_ref()?;
            field_option_inner_type(&field.ty).map(|field_type| (field_name, field_type))
        })
        .collect();

    let unassigned_filter_extension = if optional_planning_variables.len() == 1 {
        let (field_name, field_type) = optional_planning_variables[0];
        let predicate_name = syn::Ident::new(
            &format!(
                "__{}_{}_unassigned",
                name.to_string().to_lowercase(),
                field_name
            ),
            proc_macro2::Span::call_site(),
        );
        let filter_trait_name = syn::Ident::new(
            &format!("{}UnassignedFilter", name),
            proc_macro2::Span::call_site(),
        );

        quote! {
            #[allow(non_snake_case)]
            fn #predicate_name<Solution>(
                _solution: &Solution,
                entity: &#name,
            ) -> bool
            where
                Solution: ::solverforge::__internal::PlanningSolution,
            {
                let value: &::core::option::Option<#field_type> = &entity.#field_name;
                value.is_none()
            }

            pub trait #filter_trait_name<Sc: ::solverforge::Score + 'static, Solution, E, F> {
                type Output;
                fn unassigned(self) -> Self::Output;
            }

            impl<Sc, Solution, E, F> #filter_trait_name<Sc, Solution, E, F>
                for ::solverforge::__internal::UniConstraintStream<Solution, #name, E, F, Sc>
            where
                Sc: ::solverforge::Score + 'static,
                Solution: ::solverforge::__internal::PlanningSolution,
                E: Fn(&Solution) -> &[#name] + Send + Sync,
                F: ::solverforge::__internal::UniFilter<Solution, #name>,
            {
                type Output = ::solverforge::__internal::UniConstraintStream<
                    Solution,
                    #name,
                    E,
                    ::solverforge::__internal::AndUniFilter<
                        F,
                        ::solverforge::__internal::FnUniFilter<
                            fn(&Solution, &#name) -> bool
                        >,
                    >,
                    Sc,
                >;

                fn unassigned(self) -> Self::Output {
                    let (extractor, filter) = self.into_parts();
                    ::solverforge::__internal::UniConstraintStream::from_parts(
                        extractor,
                        ::solverforge::__internal::AndUniFilter::new(
                            filter,
                            ::solverforge::__internal::FnUniFilter::new(
                                #predicate_name::<Solution> as fn(&Solution, &#name) -> bool
                            ),
                        ),
                    )
                }
            }
        }
    } else {
        TokenStream::new()
    };

    let list_variable_descriptors: Vec<_> = list_variables
        .iter()
        .map(|field| {
            let field_name = field.ident.as_ref().unwrap();
            let field_name_str = field_name.to_string();
            quote! { ::solverforge::__internal::VariableDescriptor::list(#field_name_str) }
        })
        .collect();

    let list_metadata = generate_list_metadata(name, &list_variables)?;
    let list_trait_impl = generate_list_trait_impl(name, &list_variables)?;

    // Shadow variable descriptors
    let inverse_relation_descriptors: Vec<_> = inverse_relation_vars
        .iter()
        .map(|field| {
            let field_name = field.ident.as_ref().unwrap();
            let field_name_str = field_name.to_string();
            let attr = get_attribute(&field.attrs, "inverse_relation_shadow_variable").unwrap();
            let source_var = parse_attribute_string(attr, "source_variable_name")
                .unwrap_or_else(|| "visits".to_string());
            quote! {
                ::solverforge::__internal::VariableDescriptor::shadow(
                    #field_name_str,
                    ::solverforge::__internal::ShadowVariableKind::InverseRelation
                ).with_source(#name_str, #source_var)
            }
        })
        .collect();

    let previous_element_descriptors: Vec<_> = previous_element_vars
        .iter()
        .map(|field| {
            let field_name = field.ident.as_ref().unwrap();
            let field_name_str = field_name.to_string();
            let attr = get_attribute(&field.attrs, "previous_element_shadow_variable").unwrap();
            let source_var = parse_attribute_string(attr, "source_variable_name")
                .unwrap_or_else(|| "visits".to_string());
            quote! {
                ::solverforge::__internal::VariableDescriptor::shadow(
                    #field_name_str,
                    ::solverforge::__internal::ShadowVariableKind::PreviousElement
                ).with_source(#name_str, #source_var)
            }
        })
        .collect();

    let next_element_descriptors: Vec<_> = next_element_vars
        .iter()
        .map(|field| {
            let field_name = field.ident.as_ref().unwrap();
            let field_name_str = field_name.to_string();
            let attr = get_attribute(&field.attrs, "next_element_shadow_variable").unwrap();
            let source_var = parse_attribute_string(attr, "source_variable_name")
                .unwrap_or_else(|| "visits".to_string());
            quote! {
                ::solverforge::__internal::VariableDescriptor::shadow(
                    #field_name_str,
                    ::solverforge::__internal::ShadowVariableKind::NextElement
                ).with_source(#name_str, #source_var)
            }
        })
        .collect();

    let cascading_update_descriptors: Vec<_> = cascading_update_vars
        .iter()
        .map(|field| {
            let field_name = field.ident.as_ref().unwrap();
            let field_name_str = field_name.to_string();
            quote! {
                ::solverforge::__internal::VariableDescriptor::shadow(
                    #field_name_str,
                    ::solverforge::__internal::ShadowVariableKind::Cascading
                )
            }
        })
        .collect();

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

    let pin_field_descriptor = if let Some(field) = pin_field {
        let field_name = field.ident.as_ref().unwrap();
        quote! { desc = desc.with_pin_field(stringify!(#field_name)); }
    } else {
        TokenStream::new()
    };

    let expanded = quote! {
        impl #impl_generics ::solverforge::__internal::PlanningEntity for #name #ty_generics #where_clause {
            fn is_pinned(&self) -> bool { #is_pinned_impl }
            fn as_any(&self) -> &dyn ::std::any::Any { self }
            fn as_any_mut(&mut self) -> &mut dyn ::std::any::Any { self }
        }

        #planning_id_impl

        impl #impl_generics #name #ty_generics #where_clause {
            #(#variable_helpers)*
            #list_metadata

            pub fn entity_descriptor(solution_field: &'static str) -> ::solverforge::__internal::EntityDescriptor {
                let mut desc = ::solverforge::__internal::EntityDescriptor::new(
                    #name_str,
                    ::std::any::TypeId::of::<Self>(),
                    solution_field,
                );
                #id_field_descriptor
                #pin_field_descriptor
                #( desc = desc.with_variable(#variable_descriptors); )*
                #( desc = desc.with_variable(#list_variable_descriptors); )*
                #( desc = desc.with_variable(#inverse_relation_descriptors); )*
                #( desc = desc.with_variable(#previous_element_descriptors); )*
                #( desc = desc.with_variable(#next_element_descriptors); )*
                #( desc = desc.with_variable(#cascading_update_descriptors); )*
                desc
            }
        }

        #list_trait_impl

        #unassigned_filter_extension
    };

    Ok(expanded)
}

fn generate_list_metadata(
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
            "#[planning_list_variable] requires `element_collection = \"solution_field\"` for stock solving",
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

fn generate_list_trait_impl(
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

                const HAS_STOCK_LIST_VARIABLE: bool = false;
                const STOCK_LIST_VARIABLE_NAME: &'static str = "";
                const STOCK_LIST_ELEMENT_SOURCE: ::core::option::Option<&'static str> =
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
            "#[planning_list_variable] requires `element_collection = \"solution_collection\"` for stock solving",
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

            const HAS_STOCK_LIST_VARIABLE: bool = true;
            const STOCK_LIST_VARIABLE_NAME: &'static str = Self::__SOLVERFORGE_LIST_VARIABLE_NAME;
            const STOCK_LIST_ELEMENT_SOURCE: ::core::option::Option<&'static str> =
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
            "#[planning_list_variable] stock solving currently requires Vec<usize>",
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

fn field_is_option_usize(ty: &syn::Type) -> bool {
    field_option_inner_type(ty)
        .and_then(|inner| {
            let syn::Type::Path(inner_path) = inner else {
                return None;
            };
            inner_path.path.segments.last()
        })
        .map(|segment| segment.ident == "usize")
        .unwrap_or(false)
}

fn field_option_inner_type(ty: &syn::Type) -> Option<&syn::Type> {
    let syn::Type::Path(type_path) = ty else {
        return None;
    };
    let segment = type_path.path.segments.last()?;
    if segment.ident != "Option" {
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

#[cfg(test)]
mod tests {
    use super::expand_derive;
    use syn::parse_quote;

    #[test]
    fn golden_entity_expansion_includes_descriptor_and_planning_id() {
        let input = parse_quote! {
            struct Task {
                #[planning_id]
                id: String,
                #[planning_variable(allows_unassigned = true, value_range = "workers")]
                worker_idx: Option<usize>,
                #[planning_list_variable(element_collection = "all_tasks")]
                chain: Vec<usize>,
            }
        };

        let expanded = expand_derive(input)
            .expect("entity expansion should succeed")
            .to_string();

        assert!(expanded.contains("impl :: solverforge :: __internal :: PlanningEntity for Task"));
        assert!(expanded.contains("impl :: solverforge :: __internal :: PlanningId for Task"));
        assert!(expanded.contains("with_allows_unassigned (true)"));
        assert!(expanded.contains("with_value_range (\"workers\")"));
        assert!(expanded.contains("with_id_field (stringify ! (id))"));
        assert!(expanded.contains("pub fn entity_descriptor"));
        assert!(expanded.contains("pub const __SOLVERFORGE_LIST_VARIABLE_COUNT : usize = 1"));
        assert!(expanded.contains(
            "pub const __SOLVERFORGE_LIST_ELEMENT_COLLECTION : & 'static str = \"all_tasks\""
        ));
        assert!(expanded.contains("const HAS_STOCK_LIST_VARIABLE : bool = true"));
        assert!(expanded.contains("STOCK_LIST_ELEMENT_SOURCE"));
        assert!(expanded.contains("pub fn __solverforge_list_metadata < Solution >"));
        assert!(expanded.contains("pub trait TaskUnassignedFilter"));
        assert!(expanded.contains("fn unassigned (self)"));
    }
}
