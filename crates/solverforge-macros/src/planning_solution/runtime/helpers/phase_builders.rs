struct ListBuildPhasesInput<'a> {
    solution_name: &'a Ident,
    cross_enum_ident: &'a Ident,
    intra_enum_ident: &'a Ident,
    scalar_setup: &'a TokenStream,
    list_runtime_setup: &'a [TokenStream],
    scalar_groups_expr: &'a TokenStream,
    conflict_repair_expr: &'a TokenStream,
    search_fn: Option<&'a syn::Path>,
}

fn generate_list_build_phases_fn(input: ListBuildPhasesInput<'_>) -> TokenStream {
    let ListBuildPhasesInput {
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
            fn __solverforge_build_phases<D, ProgressCb>(
                config: &::solverforge::__internal::SolverConfig,
            ) -> impl ::solverforge::__internal::Phase<
                #solution_name,
                D,
                ProgressCb
            > + ::std::fmt::Debug + ::core::marker::Send
            where
                D: ::solverforge::__internal::Director<#solution_name>,
                ProgressCb: ::solverforge::__internal::ProgressCallback<#solution_name>,
            {
                let descriptor = Self::descriptor();
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
                let context = ::solverforge::__internal::SearchContext::new(
                    descriptor,
                    model,
                    config.random_seed,
                );
                ::solverforge::__internal::build_search::<
                    #solution_name,
                    usize,
                    #cross_enum_ident,
                    #intra_enum_ident,
                    D,
                    ProgressCb,
                    _
                >(#search_fn(context), config)
            }
        }
    } else {
        quote! {
            fn __solverforge_build_phases(
                config: &::solverforge::__internal::SolverConfig,
            ) -> ::solverforge::__internal::PhaseSequence<
                ::solverforge::__internal::RuntimePhase<
                    ::solverforge::__internal::Construction<
                        #solution_name,
                        usize,
                        #cross_enum_ident,
                        #intra_enum_ident
                    >,
                    ::solverforge::__internal::LocalSearchStrategy<
                        #solution_name,
                        usize,
                        #cross_enum_ident,
                        #intra_enum_ident
                    >
                >
            > {
                let descriptor = Self::descriptor();
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
                ::solverforge::__internal::build_phases(config, &descriptor, &model)
            }
        }
    }
}

fn generate_scalar_build_phases_fn(
    solution_name: &Ident,
    scalar_setup: &TokenStream,
    scalar_groups_expr: &TokenStream,
    conflict_repair_expr: &TokenStream,
    search_fn: Option<&syn::Path>,
) -> TokenStream {
    if let Some(search_fn) = search_fn {
        quote! {
            fn __solverforge_build_phases<D, ProgressCb>(
                config: &::solverforge::__internal::SolverConfig,
            ) -> impl ::solverforge::__internal::Phase<
                #solution_name,
                D,
                ProgressCb
            > + ::std::fmt::Debug + ::core::marker::Send
            where
                D: ::solverforge::__internal::Director<#solution_name>,
                ProgressCb: ::solverforge::__internal::ProgressCallback<#solution_name>,
            {
                let descriptor = Self::descriptor();
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
                let context = ::solverforge::__internal::SearchContext::new(
                    descriptor,
                    model,
                    config.random_seed,
                );
                ::solverforge::__internal::build_search::<
                    #solution_name,
                    usize,
                    ::solverforge::__internal::DefaultCrossEntityDistanceMeter,
                    ::solverforge::__internal::DefaultCrossEntityDistanceMeter,
                    D,
                    ProgressCb,
                    _
                >(#search_fn(context), config)
            }
        }
    } else {
        quote! {
            fn __solverforge_build_phases(
                config: &::solverforge::__internal::SolverConfig,
            ) -> ::solverforge::__internal::PhaseSequence<
                ::solverforge::__internal::RuntimePhase<
                    ::solverforge::__internal::Construction<
                        #solution_name,
                        usize,
                        ::solverforge::__internal::DefaultCrossEntityDistanceMeter,
                        ::solverforge::__internal::DefaultCrossEntityDistanceMeter
                    >,
                    ::solverforge::__internal::LocalSearchStrategy<
                        #solution_name,
                        usize,
                        ::solverforge::__internal::DefaultCrossEntityDistanceMeter,
                        ::solverforge::__internal::DefaultCrossEntityDistanceMeter
                    >
                >
            > {
                let descriptor = Self::descriptor();
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
                ::solverforge::__internal::build_phases(config, &descriptor, &model)
            }
        }
    }
}
