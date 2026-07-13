struct ListSearchDeclarationInput<'a> {
    solution_name: &'a Ident,
    cross_enum_ident: &'a Ident,
    intra_enum_ident: &'a Ident,
    scalar_setup: &'a TokenStream,
    list_runtime_setup: &'a [TokenStream],
    scalar_groups_expr: &'a TokenStream,
    conflict_repair_expr: &'a TokenStream,
    search_fn: Option<&'a syn::Path>,
}

fn generate_list_search_declaration_fn(input: ListSearchDeclarationInput<'_>) -> TokenStream {
    let ListSearchDeclarationInput {
        solution_name,
        cross_enum_ident,
        intra_enum_ident,
        scalar_setup,
        list_runtime_setup,
        scalar_groups_expr,
        conflict_repair_expr,
        search_fn,
    } = input;

    if let Some(search_fn) = search_fn {
        quote! {
            fn __solverforge_search_declaration(
                config: &::solverforge::__internal::SolverConfig,
                descriptor: ::solverforge::__internal::SolutionDescriptor,
            ) -> ::solverforge::__internal::RuntimeBuildResult<
                impl ::solverforge::__internal::Search<
                    #solution_name,
                    usize,
                    #cross_enum_ident,
                    #intra_enum_ident
                >
            > {
                #scalar_setup
                #(#list_runtime_setup)*
                let __solverforge_descriptor_variable_order =
                    |descriptor_index: usize, variable_name: &str| {
                        descriptor
                            .entity_descriptors
                            .get(descriptor_index)
                            .and_then(|entity| {
                                entity
                                    .variable_descriptors
                                    .iter()
                                    .position(|descriptor_var| descriptor_var.name == variable_name)
                            })
                            .unwrap_or(::core::usize::MAX)
                    };
                __solverforge_variables.sort_by_key(|variable| {
                    match variable {
                        ::solverforge::__internal::VariableSlot::Scalar(ctx) => {
                            (
                                ctx.descriptor_index,
                                __solverforge_descriptor_variable_order(
                                    ctx.descriptor_index,
                                    ctx.variable_name,
                                ),
                            )
                        }
                        ::solverforge::__internal::VariableSlot::List(ctx) => {
                            (
                                ctx.descriptor_index,
                                __solverforge_descriptor_variable_order(
                                    ctx.descriptor_index,
                                    ctx.variable_name,
                                ),
                            )
                        }
                        ::solverforge::__internal::VariableSlot::DynamicScalar(_)
                        | ::solverforge::__internal::VariableSlot::DynamicList(_) => {
                            (::core::usize::MAX, ::core::usize::MAX)
                        }
                    }
                });
                let __solverforge_scalar_slots = __solverforge_variables
                    .iter()
                    .filter_map(|variable| match variable {
                        ::solverforge::__internal::VariableSlot::Scalar(ctx) => {
                            ::core::option::Option::Some(*ctx)
                        }
                        ::solverforge::__internal::VariableSlot::List(_) => {
                            ::core::option::Option::None
                        }
                        ::solverforge::__internal::VariableSlot::DynamicScalar(_)
                        | ::solverforge::__internal::VariableSlot::DynamicList(_) => {
                            ::core::option::Option::None
                        }
                    })
                    .collect::<::std::vec::Vec<_>>();
                let __solverforge_scalar_groups = #scalar_groups_expr;
                let model = ::solverforge::__internal::RuntimeModel::<
                    #solution_name,
                    usize,
                    #cross_enum_ident,
                    #intra_enum_ident
                >::new(__solverforge_variables)
                .with_scalar_groups(__solverforge_scalar_groups)
                #conflict_repair_expr;
                let context = ::solverforge::__internal::SearchContext::try_new(
                    descriptor,
                    model,
                    config.random_seed,
                )?;
                ::core::result::Result::Ok(#search_fn(context))
            }
        }
    } else {
        quote! {
            fn __solverforge_search_declaration(
                config: &::solverforge::__internal::SolverConfig,
                descriptor: ::solverforge::__internal::SolutionDescriptor,
            ) -> ::solverforge::__internal::RuntimeBuildResult<
                impl ::solverforge::__internal::Search<
                    #solution_name,
                    usize,
                    #cross_enum_ident,
                    #intra_enum_ident
                >
            > {
                #scalar_setup
                #(#list_runtime_setup)*
                let __solverforge_descriptor_variable_order =
                    |descriptor_index: usize, variable_name: &str| {
                        descriptor
                            .entity_descriptors
                            .get(descriptor_index)
                            .and_then(|entity| {
                                entity
                                    .variable_descriptors
                                    .iter()
                                    .position(|descriptor_var| descriptor_var.name == variable_name)
                            })
                            .unwrap_or(::core::usize::MAX)
                    };
                __solverforge_variables.sort_by_key(|variable| {
                    match variable {
                        ::solverforge::__internal::VariableSlot::Scalar(ctx) => {
                            (
                                ctx.descriptor_index,
                                __solverforge_descriptor_variable_order(
                                    ctx.descriptor_index,
                                    ctx.variable_name,
                                ),
                            )
                        }
                        ::solverforge::__internal::VariableSlot::List(ctx) => {
                            (
                                ctx.descriptor_index,
                                __solverforge_descriptor_variable_order(
                                    ctx.descriptor_index,
                                    ctx.variable_name,
                                ),
                            )
                        }
                        ::solverforge::__internal::VariableSlot::DynamicScalar(_)
                        | ::solverforge::__internal::VariableSlot::DynamicList(_) => {
                            (::core::usize::MAX, ::core::usize::MAX)
                        }
                    }
                });
                let __solverforge_scalar_slots = __solverforge_variables
                    .iter()
                    .filter_map(|variable| match variable {
                        ::solverforge::__internal::VariableSlot::Scalar(ctx) => {
                            ::core::option::Option::Some(*ctx)
                        }
                        ::solverforge::__internal::VariableSlot::List(_) => {
                            ::core::option::Option::None
                        }
                        ::solverforge::__internal::VariableSlot::DynamicScalar(_)
                        | ::solverforge::__internal::VariableSlot::DynamicList(_) => {
                            ::core::option::Option::None
                        }
                    })
                    .collect::<::std::vec::Vec<_>>();
                let __solverforge_scalar_groups = #scalar_groups_expr;
                let model = ::solverforge::__internal::RuntimeModel::<
                    #solution_name,
                    usize,
                    #cross_enum_ident,
                    #intra_enum_ident
                >::new(__solverforge_variables)
                .with_scalar_groups(__solverforge_scalar_groups)
                #conflict_repair_expr;
                let context = ::solverforge::__internal::SearchContext::try_new(
                    descriptor,
                    model,
                    config.random_seed,
                )?;
                ::core::result::Result::Ok(context.defaults())
            }
        }
    }
}

fn generate_scalar_search_declaration_fn(
    solution_name: &Ident,
    scalar_setup: &TokenStream,
    scalar_groups_expr: &TokenStream,
    conflict_repair_expr: &TokenStream,
    search_fn: Option<&syn::Path>,
) -> TokenStream {
    if let Some(search_fn) = search_fn {
        quote! {
            fn __solverforge_search_declaration(
                config: &::solverforge::__internal::SolverConfig,
                descriptor: ::solverforge::__internal::SolutionDescriptor,
            ) -> ::solverforge::__internal::RuntimeBuildResult<
                impl ::solverforge::__internal::Search<
                    #solution_name,
                    usize,
                    ::solverforge::__internal::DefaultCrossEntityDistanceMeter,
                    ::solverforge::__internal::DefaultCrossEntityDistanceMeter
                >
            > {
                #scalar_setup
                let __solverforge_scalar_slots = __solverforge_variables
                    .iter()
                    .filter_map(|variable| match variable {
                        ::solverforge::__internal::VariableSlot::Scalar(ctx) => {
                            ::core::option::Option::Some(*ctx)
                        }
                        ::solverforge::__internal::VariableSlot::List(_) => {
                            ::core::option::Option::None
                        }
                        ::solverforge::__internal::VariableSlot::DynamicScalar(_)
                        | ::solverforge::__internal::VariableSlot::DynamicList(_) => {
                            ::core::option::Option::None
                        }
                    })
                    .collect::<::std::vec::Vec<_>>();
                let __solverforge_scalar_groups = #scalar_groups_expr;
                let model = ::solverforge::__internal::RuntimeModel::<
                    #solution_name,
                    usize,
                    ::solverforge::__internal::DefaultCrossEntityDistanceMeter,
                    ::solverforge::__internal::DefaultCrossEntityDistanceMeter
                >::new(__solverforge_variables)
                .with_scalar_groups(__solverforge_scalar_groups)
                #conflict_repair_expr;
                let context = ::solverforge::__internal::SearchContext::try_new(
                    descriptor,
                    model,
                    config.random_seed,
                )?;
                ::core::result::Result::Ok(#search_fn(context))
            }
        }
    } else {
        quote! {
            fn __solverforge_search_declaration(
                config: &::solverforge::__internal::SolverConfig,
                descriptor: ::solverforge::__internal::SolutionDescriptor,
            ) -> ::solverforge::__internal::RuntimeBuildResult<
                impl ::solverforge::__internal::Search<
                    #solution_name,
                    usize,
                    ::solverforge::__internal::DefaultCrossEntityDistanceMeter,
                    ::solverforge::__internal::DefaultCrossEntityDistanceMeter
                >
            > {
                #scalar_setup
                let __solverforge_scalar_slots = __solverforge_variables
                    .iter()
                    .filter_map(|variable| match variable {
                        ::solverforge::__internal::VariableSlot::Scalar(ctx) => {
                            ::core::option::Option::Some(*ctx)
                        }
                        ::solverforge::__internal::VariableSlot::List(_) => {
                            ::core::option::Option::None
                        }
                        ::solverforge::__internal::VariableSlot::DynamicScalar(_)
                        | ::solverforge::__internal::VariableSlot::DynamicList(_) => {
                            ::core::option::Option::None
                        }
                    })
                    .collect::<::std::vec::Vec<_>>();
                let __solverforge_scalar_groups = #scalar_groups_expr;
                let model = ::solverforge::__internal::RuntimeModel::<
                    #solution_name,
                    usize,
                    ::solverforge::__internal::DefaultCrossEntityDistanceMeter,
                    ::solverforge::__internal::DefaultCrossEntityDistanceMeter
                >::new(__solverforge_variables)
                .with_scalar_groups(__solverforge_scalar_groups)
                #conflict_repair_expr;
                let context = ::solverforge::__internal::SearchContext::try_new(
                    descriptor,
                    model,
                    config.random_seed,
                )?;
                ::core::result::Result::Ok(context.defaults())
            }
        }
    }
}
