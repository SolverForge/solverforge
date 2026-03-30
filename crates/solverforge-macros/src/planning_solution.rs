// #[planning_solution] derive macro implementation

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Error, Fields, Ident, Lit, Meta};

use crate::attr_parse::{
    get_attribute, has_attribute, parse_attribute_list, parse_attribute_string,
};

#[derive(Default)]
struct ShadowConfig {
    list_owner: Option<String>,
    inverse_field: Option<String>,
    previous_field: Option<String>,
    next_field: Option<String>,
    cascading_listener: Option<String>,
    post_update_listener: Option<String>,

    // Aggregate shadow fields on the list owner entity.
    // Format: "field_name:aggregation:source_field" (e.g., "total_demand:sum:demand")
    entity_aggregates: Vec<String>,

    // Computed shadow fields on the list owner entity.
    // Format: "field_name:method_name" (e.g., "total_driving_time:compute_driving_time")
    entity_computes: Vec<String>,
}

// Parse the constraints path from #[solverforge_constraints_path = "path"]
fn parse_constraints_path(attrs: &[syn::Attribute]) -> Option<String> {
    for attr in attrs {
        if attr.path().is_ident("solverforge_constraints_path") {
            if let Meta::NameValue(nv) = &attr.meta {
                if let syn::Expr::Lit(expr_lit) = &nv.value {
                    if let Lit::Str(lit_str) = &expr_lit.lit {
                        return Some(lit_str.value());
                    }
                }
            }
        }
    }
    None
}

fn parse_shadow_config(attrs: &[syn::Attribute]) -> ShadowConfig {
    let mut config = ShadowConfig::default();

    if let Some(attr) = get_attribute(attrs, "shadow_variable_updates") {
        config.list_owner = parse_attribute_string(attr, "list_owner");
        config.inverse_field = parse_attribute_string(attr, "inverse_field");
        config.previous_field = parse_attribute_string(attr, "previous_field");
        config.next_field = parse_attribute_string(attr, "next_field");
        config.cascading_listener = parse_attribute_string(attr, "cascading_listener");
        config.post_update_listener = parse_attribute_string(attr, "post_update_listener");
        config.entity_aggregates = parse_attribute_list(attr, "entity_aggregate");
        config.entity_computes = parse_attribute_list(attr, "entity_compute");
    }

    config
}

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
                    "#[planning_solution] requires named fields",
                ))
            }
        },
        _ => {
            return Err(Error::new_spanned(
                &input,
                "#[planning_solution] only works on structs",
            ))
        }
    };

    let score_field = fields
        .iter()
        .find(|f| has_attribute(&f.attrs, "planning_score"))
        .ok_or_else(|| {
            Error::new_spanned(
                &input,
                "#[planning_solution] requires a #[planning_score] field",
            )
        })?;

    let score_field_name = score_field.ident.as_ref().unwrap();
    let score_type = extract_option_inner_type(&score_field.ty)?;

    let entity_descriptors: Vec<_> = fields
        .iter()
        .filter(|f| has_attribute(&f.attrs, "planning_entity_collection"))
        .filter_map(|f| {
            let field_name = f.ident.as_ref().unwrap();
            let field_name_str = field_name.to_string();
            let element_type = extract_collection_inner_type(&f.ty)?;
            Some(quote! {
                .with_entity(#element_type::entity_descriptor(#field_name_str).with_extractor(
                    Box::new(::solverforge::__internal::TypedEntityExtractor::new(
                        stringify!(#element_type),
                        #field_name_str,
                        |s: &#name| &s.#field_name,
                        |s: &mut #name| &mut s.#field_name,
                    ))
                ))
            })
        })
        .collect();

    let fact_descriptors: Vec<_> = fields
        .iter()
        .filter(|f| has_attribute(&f.attrs, "problem_fact_collection"))
        .filter_map(|f| {
            let field_name = f.ident.as_ref().unwrap();
            let field_name_str = field_name.to_string();
            let element_type = extract_collection_inner_type(&f.ty)?;
            Some(quote! {
                .with_problem_fact(#element_type::problem_fact_descriptor(#field_name_str).with_extractor(
                    Box::new(::solverforge::__internal::TypedEntityExtractor::new(
                        stringify!(#element_type),
                        #field_name_str,
                        |s: &#name| &s.#field_name,
                        |s: &mut #name| &mut s.#field_name,
                    ))
                ))
            })
        })
        .collect();

    let name_str = name.to_string();
    let score_field_str = score_field_name.to_string();

    let shadow_config = parse_shadow_config(&input.attrs);
    let shadow_support_impl = generate_shadow_support(&shadow_config, fields, name)?;
    let constraints_path = parse_constraints_path(&input.attrs);
    let entity_count_arms: Vec<_> = fields
        .iter()
        .filter(|f| has_attribute(&f.attrs, "planning_entity_collection"))
        .enumerate()
        .map(|(idx, f)| {
            let field_name = f.ident.as_ref().unwrap();
            quote! { #idx => this.#field_name.len(), }
        })
        .collect();

    let list_operations = generate_list_operations(&shadow_config, fields, name)?;
    let stock_phase_support =
        generate_stock_phase_support(&shadow_config, fields, &constraints_path, name);
    let stock_solve_internal =
        generate_stock_solve_internal(&shadow_config, fields, &constraints_path, name);
    let solvable_solution_impl = generate_solvable_solution(name, &constraints_path);

    let stream_extensions = generate_constraint_stream_extensions(fields, name);

    let expanded = quote! {
        impl #impl_generics ::solverforge::__internal::PlanningSolution for #name #ty_generics #where_clause {
            type Score = #score_type;
            fn score(&self) -> Option<Self::Score> { self.#score_field_name.clone() }
            fn set_score(&mut self, score: Option<Self::Score>) { self.#score_field_name = score; }
        }

        impl #impl_generics #name #ty_generics #where_clause {
            pub fn descriptor() -> ::solverforge::__internal::SolutionDescriptor {
                ::solverforge::__internal::SolutionDescriptor::new(
                    #name_str,
                    ::std::any::TypeId::of::<Self>(),
                )
                .with_score_field(#score_field_str)
                #(#entity_descriptors)*
                #(#fact_descriptors)*
            }

            #[inline]
            pub fn entity_count(this: &Self, descriptor_index: usize) -> usize {
                match descriptor_index {
                    #(#entity_count_arms)*
                    _ => 0,
                }
            }

            #list_operations
            #stock_solve_internal
        }

        #stock_phase_support
        #shadow_support_impl

        #solvable_solution_impl

        #stream_extensions
    };

    Ok(expanded)
}

struct ListOwnerConfig<'a> {
    field_ident: &'a Ident,
    entity_type: &'a syn::Type,
    descriptor_index: usize,
}

struct ListElementCollectionConfig<'a> {
    field_ident: &'a Ident,
    owner_field: String,
}

fn find_list_owner_config<'a>(
    config: &ShadowConfig,
    fields: &'a syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
) -> Result<Option<ListOwnerConfig<'a>>, Error> {
    let Some(list_owner) = config.list_owner.as_deref() else {
        return Ok(None);
    };

    fields
        .iter()
        .filter(|f| has_attribute(&f.attrs, "planning_entity_collection"))
        .enumerate()
        .find_map(|(descriptor_index, field)| {
            let field_ident = field.ident.as_ref()?;
            if field_ident != list_owner {
                return None;
            }
            let entity_type = extract_collection_inner_type(&field.ty)?;
            Some(ListOwnerConfig {
                field_ident,
                entity_type,
                descriptor_index,
            })
        })
        .map(Some)
        .ok_or_else(|| {
            Error::new(
                proc_macro2::Span::call_site(),
                format!(
                    "#[shadow_variable_updates(list_owner = \"{list_owner}\")] must name a #[planning_entity_collection] field"
                ),
            )
        })
}

fn find_list_element_collection_config<'a>(
    fields: &'a syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
) -> Result<Option<ListElementCollectionConfig<'a>>, Error> {
    let mut matches = fields
        .iter()
        .filter_map(|field| {
            let attr = get_attribute(&field.attrs, "planning_list_element_collection")?;
            let owner = parse_attribute_string(attr, "owner")?;
            let field_ident = field.ident.as_ref()?;
            let inner = extract_collection_inner_type(&field.ty)?;
            let syn::Type::Path(type_path) = inner else {
                return None;
            };
            let segment = type_path.path.segments.last()?;
            if segment.ident != "usize" {
                return None;
            }
            Some(ListElementCollectionConfig {
                field_ident,
                owner_field: owner,
            })
        })
        .collect::<Vec<_>>();

    if matches.len() > 1 {
        return Err(Error::new(
            proc_macro2::Span::call_site(),
            "#[planning_solution] currently supports at most one #[planning_list_element_collection(...)] field",
        ));
    }

    Ok(matches.pop())
}

fn find_stock_list_config<'a>(
    fields: &'a syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
) -> Result<Option<(ListOwnerConfig<'a>, ListElementCollectionConfig<'a>)>, Error> {
    let Some(element_collection) = find_list_element_collection_config(fields)? else {
        return Ok(None);
    };

    let owner = fields
        .iter()
        .filter(|f| has_attribute(&f.attrs, "planning_entity_collection"))
        .enumerate()
        .find_map(|(descriptor_index, field)| {
            let field_ident = field.ident.as_ref()?;
            if *field_ident != element_collection.owner_field {
                return None;
            }
            let entity_type = extract_collection_inner_type(&field.ty)?;
            Some(ListOwnerConfig {
                field_ident,
                entity_type,
                descriptor_index,
            })
        })
        .ok_or_else(|| {
            Error::new(
                proc_macro2::Span::call_site(),
                format!(
                    "planning solution with list owner `{}` requires a `#[planning_list_element_collection(owner = \"{}\")]` field of type Vec<usize>",
                    element_collection.owner_field,
                    element_collection.owner_field,
                ),
            )
        })?;

    Ok(Some((owner, element_collection)))
}

fn shadow_updates_requested(config: &ShadowConfig) -> bool {
    config.inverse_field.is_some()
        || config.previous_field.is_some()
        || config.next_field.is_some()
        || config.cascading_listener.is_some()
        || config.post_update_listener.is_some()
        || !config.entity_aggregates.is_empty()
        || !config.entity_computes.is_empty()
}

fn generate_list_operations(
    _config: &ShadowConfig,
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
    _solution_name: &Ident,
) -> Result<TokenStream, Error> {
    let Some((list_owner, element_collection)) = find_stock_list_config(fields)? else {
        return Ok(TokenStream::new());
    };

    let descriptor_index_lit = syn::LitInt::new(
        &list_owner.descriptor_index.to_string(),
        proc_macro2::Span::call_site(),
    );
    let list_owner_ident = list_owner.field_ident;
    let element_collection_ident = element_collection.field_ident;
    let list_owner_type = list_owner.entity_type;
    let list_trait =
        quote! { <#list_owner_type as ::solverforge::__internal::StockListEntity<Self>> };

    Ok(quote! {
        #[inline]
        pub fn list_len(&self, entity_idx: usize) -> usize {
            self.#list_owner_ident
                .get(entity_idx)
                .map_or(0, |entity| #list_trait::list_field(entity).len())
        }

        #[inline]
        pub fn list_len_static(s: &Self, entity_idx: usize) -> usize {
            s.#list_owner_ident
                .get(entity_idx)
                .map_or(0, |entity| #list_trait::list_field(entity).len())
        }

        #[inline]
        pub fn list_remove(s: &mut Self, entity_idx: usize, pos: usize) -> Option<usize> {
            s.#list_owner_ident
                .get_mut(entity_idx)
                .map(|entity| #list_trait::list_field_mut(entity).remove(pos))
        }

        #[inline]
        pub fn list_insert(s: &mut Self, entity_idx: usize, pos: usize, val: usize) {
            if let Some(entity) = s.#list_owner_ident.get_mut(entity_idx) {
                #list_trait::list_field_mut(entity).insert(pos, val);
            }
        }

        #[inline]
        pub fn list_get(s: &Self, entity_idx: usize, pos: usize) -> Option<usize> {
            s.#list_owner_ident
                .get(entity_idx)
                .and_then(|entity| #list_trait::list_field(entity).get(pos).copied())
        }

        #[inline]
        pub fn list_set(s: &mut Self, entity_idx: usize, pos: usize, val: usize) {
            if let Some(entity) = s.#list_owner_ident.get_mut(entity_idx) {
                let list = #list_trait::list_field_mut(entity);
                if pos < list.len() {
                    list[pos] = val;
                }
            }
        }

        #[inline]
        pub fn list_reverse(s: &mut Self, entity_idx: usize, start: usize, end: usize) {
            if let Some(entity) = s.#list_owner_ident.get_mut(entity_idx) {
                #list_trait::list_field_mut(entity)[start..end].reverse();
            }
        }

        #[inline]
        pub fn sublist_remove(
            s: &mut Self,
            entity_idx: usize,
            start: usize,
            end: usize,
        ) -> Vec<usize> {
            s.#list_owner_ident
                .get_mut(entity_idx)
                .map(|entity| #list_trait::list_field_mut(entity).drain(start..end).collect())
                .unwrap_or_default()
        }

        #[inline]
        pub fn sublist_insert(
            s: &mut Self,
            entity_idx: usize,
            pos: usize,
            items: Vec<usize>,
        ) {
            if let Some(entity) = s.#list_owner_ident.get_mut(entity_idx) {
                let list = #list_trait::list_field_mut(entity);
                for (i, item) in items.into_iter().enumerate() {
                    list.insert(pos + i, item);
                }
            }
        }

        #[inline]
        pub fn ruin_remove(s: &mut Self, entity_idx: usize, pos: usize) -> usize {
            #list_trait::list_field_mut(&mut s.#list_owner_ident[entity_idx]).remove(pos)
        }

        #[inline]
        pub fn ruin_insert(s: &mut Self, entity_idx: usize, pos: usize, val: usize) {
            #list_trait::list_field_mut(&mut s.#list_owner_ident[entity_idx]).insert(pos, val);
        }

        #[inline]
        pub fn list_remove_for_construction(s: &mut Self, entity_idx: usize, pos: usize) -> usize {
            #list_trait::list_field_mut(&mut s.#list_owner_ident[entity_idx]).remove(pos)
        }

        #[inline]
        pub fn index_to_element_static(s: &Self, idx: usize) -> usize {
            s.#element_collection_ident[idx]
        }

        #[inline]
        pub const fn list_variable_descriptor_index() -> usize {
            #descriptor_index_lit
        }

        #[inline]
        pub fn element_count(s: &Self) -> usize {
            s.#element_collection_ident.len()
        }

        #[inline]
        pub fn assigned_elements(s: &Self) -> Vec<usize> {
            s.#list_owner_ident
                .iter()
                .flat_map(|entity| #list_trait::list_field(entity).iter().copied())
                .collect()
        }

        #[inline]
        pub fn n_entities(s: &Self) -> usize {
            s.#list_owner_ident.len()
        }

        #[inline]
        pub fn assign_element(s: &mut Self, entity_idx: usize, elem: usize) {
            if let Some(entity) = s.#list_owner_ident.get_mut(entity_idx) {
                #list_trait::list_field_mut(entity).push(elem);
            }
        }
    })
}

fn generate_stock_solve_internal(
    _shadow_config: &ShadowConfig,
    _fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
    constraints_path: &Option<String>,
    _solution_name: &Ident,
) -> TokenStream {
    let Some(path) = constraints_path.as_ref() else {
        return TokenStream::new();
    };

    let constraints_fn: syn::Path =
        syn::parse_str(path).expect("constraints path must be a valid Rust path");
    quote! {
        fn solve_internal(
            self,
            terminate: Option<&std::sync::atomic::AtomicBool>,
            sender: ::solverforge::__internal::UnboundedSender<::solverforge::SolverEvent<Self>>,
        ) -> Self {
            ::solverforge::__internal::init_console();

            ::solverforge::__internal::run_stock_solver(
                self,
                #constraints_fn,
                Self::descriptor,
                Self::entity_count,
                terminate,
                sender,
                Self::__solverforge_stock_default_time_limit_secs(),
                Self::__solverforge_stock_is_trivial,
                Self::__solverforge_stock_log_scale,
                Self::__solverforge_build_stock_phases,
            )
        }
    }
}

fn generate_stock_phase_support(
    _shadow_config: &ShadowConfig,
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
    constraints_path: &Option<String>,
    solution_name: &Ident,
) -> TokenStream {
    if constraints_path.is_none() {
        return TokenStream::new();
    }

    if let Some((list_owner, _element_collection)) =
        find_stock_list_config(fields).expect("stock list config validation should have succeeded")
    {
        let list_owner_type = list_owner.entity_type;
        let list_trait = quote! {
            <#list_owner_type as ::solverforge::__internal::StockListEntity<#solution_name>>
        };
        let descriptor_index_lit = syn::LitInt::new(
            &list_owner.descriptor_index.to_string(),
            proc_macro2::Span::call_site(),
        );

        return quote! {
            impl #solution_name {
                const fn __solverforge_stock_default_time_limit_secs() -> u64 {
                    60
                }

                fn __solverforge_stock_is_trivial(solution: &Self) -> bool {
                    let descriptor = Self::descriptor();
                    let has_standard = ::solverforge::__internal::descriptor_has_bindings(&descriptor);
                    let has_list = Self::n_entities(solution) > 0 && Self::element_count(solution) > 0;
                    (!has_standard && !has_list)
                        || (Self::n_entities(solution) == 0)
                        || (has_list && Self::element_count(solution) == 0)
                }

                fn __solverforge_stock_log_scale(solution: &Self) {
                    let descriptor = Self::descriptor();
                    let has_standard = ::solverforge::__internal::descriptor_has_bindings(&descriptor);
                    ::solverforge::__internal::log_stock_solve_start(
                        Self::n_entities(solution),
                        ::core::option::Option::Some(Self::element_count(solution)),
                        ::core::option::Option::Some(has_standard),
                        ::core::option::Option::None,
                    );
                }

                fn __solverforge_build_stock_phases(
                    config: &::solverforge::__internal::SolverConfig,
                ) -> ::solverforge::__internal::PhaseSequence<
                    ::solverforge::__internal::UnifiedMixedStockPhase<
                        #solution_name,
                        usize,
                        #list_trait::CrossDistanceMeter,
                        #list_trait::IntraDistanceMeter,
                    >
                > {
                    let descriptor = Self::descriptor();
                    let metadata = #list_trait::stock_list_metadata();
                    let list_ctx = ::solverforge::__internal::ListContext::new(
                        Self::list_len_static,
                        Self::list_remove,
                        Self::list_insert,
                        Self::list_get,
                        Self::list_set,
                        Self::list_reverse,
                        Self::sublist_remove,
                        Self::sublist_insert,
                        Self::ruin_remove,
                        Self::ruin_insert,
                        Self::n_entities,
                        metadata.cross_distance_meter.clone(),
                        metadata.intra_distance_meter.clone(),
                        #list_trait::STOCK_LIST_VARIABLE_NAME,
                        #descriptor_index_lit,
                    );
                    let construction = ::solverforge::__internal::MixedStockConstructionArgs {
                        element_count: Self::element_count,
                        assigned_elements: Self::assigned_elements,
                        entity_count: Self::n_entities,
                        list_len: Self::list_len_static,
                        list_insert: Self::list_insert,
                        list_remove: Self::list_remove_for_construction,
                        index_to_element: Self::index_to_element_static,
                        descriptor_index: #descriptor_index_lit,
                        depot_fn: metadata.cw_depot_fn,
                        distance_fn: metadata.cw_distance_fn,
                        element_load_fn: metadata.cw_element_load_fn,
                        capacity_fn: metadata.cw_capacity_fn,
                        assign_route_fn: metadata.cw_assign_route_fn,
                        merge_feasible_fn: metadata.merge_feasible_fn,
                        k_opt_get_route: metadata.k_opt_get_route,
                        k_opt_set_route: metadata.k_opt_set_route,
                        k_opt_depot_fn: metadata.k_opt_depot_fn,
                        k_opt_distance_fn: metadata.k_opt_distance_fn,
                        k_opt_feasible_fn: metadata.k_opt_feasible_fn,
                    };
                    ::solverforge::__internal::build_mixed_stock_phases(
                        config,
                        &descriptor,
                        &list_ctx,
                        construction,
                        #list_trait::STOCK_LIST_VARIABLE_NAME,
                    )
                }
            }
        };
    }

    quote! {
        impl #solution_name {
            const fn __solverforge_stock_default_time_limit_secs() -> u64 {
                30
            }

            fn __solverforge_stock_is_trivial(solution: &Self) -> bool {
                let descriptor = Self::descriptor();
                !::solverforge::__internal::descriptor_has_bindings(&descriptor)
                    || descriptor
                        .total_entity_count(solution as &dyn ::std::any::Any)
                        .unwrap_or(0)
                        == 0
            }

            fn __solverforge_stock_log_scale(solution: &Self) {
                let descriptor = Self::descriptor();
                ::solverforge::__internal::log_stock_solve_start(
                    descriptor
                        .total_entity_count(solution as &dyn ::std::any::Any)
                        .unwrap_or(0),
                    ::core::option::Option::None,
                    ::core::option::Option::None,
                    ::core::option::Option::Some(
                        descriptor.genuine_variable_descriptors().len(),
                    ),
                );
            }

            fn __solverforge_build_stock_phases(
                config: &::solverforge::__internal::SolverConfig,
            ) -> ::solverforge::__internal::PhaseSequence<
                ::solverforge::__internal::StandardStockPhase<#solution_name>
            > {
                let descriptor = Self::descriptor();
                ::solverforge::__internal::build_standard_stock_phases(config, &descriptor)
            }
        }
    }
}

fn generate_solvable_solution(
    solution_name: &Ident,
    constraints_path: &Option<String>,
) -> TokenStream {
    let solvable_solution_impl = quote! {
        impl ::solverforge::__internal::SolvableSolution for #solution_name {
            fn descriptor() -> ::solverforge::__internal::SolutionDescriptor {
                #solution_name::descriptor()
            }

            fn entity_count(solution: &Self, descriptor_index: usize) -> usize {
                #solution_name::entity_count(solution, descriptor_index)
            }
        }
    };

    // Generate Solvable and Analyzable trait impls only if constraints are specified
    let solvable_impl = constraints_path.as_ref().map(|path| {
        let constraints_fn: syn::Path =
            syn::parse_str(path).expect("constraints path must be a valid Rust path");

        quote! {
            impl ::solverforge::Solvable for #solution_name {
                fn solve(
                    self,
                    terminate: Option<&std::sync::atomic::AtomicBool>,
                    sender: ::solverforge::__internal::UnboundedSender<::solverforge::SolverEvent<Self>>,
                ) {
                    let _ = #solution_name::solve_internal(self, terminate, sender);
                }
            }

            impl ::solverforge::Analyzable for #solution_name {
                fn analyze(&self) -> ::solverforge::ScoreAnalysis<<Self as ::solverforge::__internal::PlanningSolution>::Score> {
                    use ::solverforge::__internal::{
                        ScoreDirector, Director,
                    };

                    let constraints = #constraints_fn();
                    let mut director = ScoreDirector::with_descriptor(
                        self.clone(),
                        constraints,
                        Self::descriptor(),
                        Self::entity_count,
                    );

                    let score = director.calculate_score();
                    let constraint_scores = director.constraint_match_totals();

                    let constraints = constraint_scores
                        .into_iter()
                        .map(|(name, weight, contribution, match_count)| {
                            ::solverforge::ConstraintAnalysis {
                                name,
                                weight,
                                score: contribution,
                                match_count,
                            }
                        })
                        .collect();

                    ::solverforge::ScoreAnalysis { score, constraints }
                }
            }
        }
    });

    quote! {
        #solvable_solution_impl
        #solvable_impl
    }
}

fn generate_shadow_support(
    config: &ShadowConfig,
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
    solution_name: &Ident,
) -> Result<TokenStream, Error> {
    if !shadow_updates_requested(config) {
        return Ok(quote! {
            impl ::solverforge::__internal::ShadowVariableSupport for #solution_name {
                #[inline]
                fn update_entity_shadows(&mut self, _entity_idx: usize) {}
            }
        });
    }

    let Some(list_owner) = find_list_owner_config(config, fields)? else {
        return Err(Error::new(
            proc_macro2::Span::call_site(),
            "#[shadow_variable_updates(...)] requires `list_owner = \"entity_collection_field\"` when shadow updates are configured",
        ));
    };
    let Some(element_collection) = find_list_element_collection_config(fields)?
        .filter(|element_collection| *list_owner.field_ident == element_collection.owner_field)
    else {
        return Err(Error::new(
            proc_macro2::Span::call_site(),
            format!(
                "planning solution with list owner `{}` requires a `#[planning_list_element_collection(owner = \"{}\")]` field of type Vec<usize>",
                list_owner.field_ident,
                list_owner.field_ident,
            ),
        ));
    };

    let list_owner_ident = list_owner.field_ident;
    let element_collection_ident = element_collection.field_ident;
    let list_owner_type = list_owner.entity_type;
    let list_trait =
        quote! { <#list_owner_type as ::solverforge::__internal::StockListEntity<Self>> };

    let inverse_update = config.inverse_field.as_ref().map(|field| {
        let field_ident = Ident::new(field, proc_macro2::Span::call_site());
        quote! {
            for &element_idx in &element_indices {
                self.#element_collection_ident[element_idx].#field_ident = Some(entity_idx);
            }
        }
    });

    let previous_update = config.previous_field.as_ref().map(|field| {
        let field_ident = Ident::new(field, proc_macro2::Span::call_site());
        quote! {
            let mut prev_idx: Option<usize> = None;
            for &element_idx in &element_indices {
                self.#element_collection_ident[element_idx].#field_ident = prev_idx;
                prev_idx = Some(element_idx);
            }
        }
    });

    let next_update = config.next_field.as_ref().map(|field| {
        let field_ident = Ident::new(field, proc_macro2::Span::call_site());
        quote! {
            let len = element_indices.len();
            for (i, &element_idx) in element_indices.iter().enumerate() {
                let next_idx = if i + 1 < len { Some(element_indices[i + 1]) } else { None };
                self.#element_collection_ident[element_idx].#field_ident = next_idx;
            }
        }
    });

    let cascading_update = config.cascading_listener.as_ref().map(|method| {
        let method_ident = Ident::new(method, proc_macro2::Span::call_site());
        quote! {
            for &element_idx in &element_indices {
                self.#method_ident(element_idx);
            }
        }
    });

    let post_update = config.post_update_listener.as_ref().map(|method| {
        let method_ident = Ident::new(method, proc_macro2::Span::call_site());
        quote! {
            self.#method_ident(entity_idx);
        }
    });

    let aggregate_updates: Vec<_> = config
        .entity_aggregates
        .iter()
        .filter_map(|spec| {
            let parts: Vec<&str> = spec.split(':').collect();
            if parts.len() != 3 {
                return None;
            }
            let target_field = Ident::new(parts[0], proc_macro2::Span::call_site());
            let aggregation = parts[1];
            let source_field = Ident::new(parts[2], proc_macro2::Span::call_site());

            match aggregation {
                "sum" => Some(quote! {
                    self.#list_owner_ident[entity_idx].#target_field = element_indices
                        .iter()
                        .map(|&idx| self.#element_collection_ident[idx].#source_field)
                        .sum();
                }),
                _ => None,
            }
        })
        .collect();

    let compute_updates: Vec<_> = config
        .entity_computes
        .iter()
        .filter_map(|spec| {
            let parts: Vec<&str> = spec.split(':').collect();
            if parts.len() != 2 {
                return None;
            }
            let target_field = Ident::new(parts[0], proc_macro2::Span::call_site());
            let method_name = Ident::new(parts[1], proc_macro2::Span::call_site());

            Some(quote! {
                self.#list_owner_ident[entity_idx].#target_field = self.#method_name(entity_idx);
            })
        })
        .collect();

    Ok(quote! {
        impl ::solverforge::__internal::ShadowVariableSupport for #solution_name {
            #[inline]
            fn update_entity_shadows(&mut self, entity_idx: usize) {
                let element_indices: Vec<usize> =
                    #list_trait::list_field(&self.#list_owner_ident[entity_idx]).to_vec();

                #inverse_update
                #previous_update
                #next_update
                #cascading_update
                #(#aggregate_updates)*
                #(#compute_updates)*
                #post_update
            }
        }
    })
}

fn generate_constraint_stream_extensions(
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
    solution_name: &Ident,
) -> TokenStream {
    // Collect entity collection fields
    let entity_fields: Vec<_> = fields
        .iter()
        .filter(|f| has_attribute(&f.attrs, "planning_entity_collection"))
        .collect();

    // Collect problem fact collection fields
    let fact_fields: Vec<_> = fields
        .iter()
        .filter(|f| has_attribute(&f.attrs, "problem_fact_collection"))
        .collect();

    // Build accessor methods for constraint factory extension trait
    let mut accessor_methods: Vec<TokenStream> = Vec::new();
    let mut accessor_impls: Vec<TokenStream> = Vec::new();

    for f in entity_fields.iter().chain(fact_fields.iter()) {
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
                fn(&#solution_name) -> &[#element_type],
                ::solverforge::__internal::TrueFilter,
                Sc>;
        });

        accessor_impls.push(quote! {
            fn #field_name(self) -> ::solverforge::__internal::UniConstraintStream<
                #solution_name,
                #element_type,
                fn(&#solution_name) -> &[#element_type],
                ::solverforge::__internal::TrueFilter,
                Sc>
            {
                self.for_each((|s: &#solution_name| s.#field_name.as_slice()) as fn(&#solution_name) -> &[#element_type])
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

fn extract_option_inner_type(ty: &syn::Type) -> Result<&syn::Type, Error> {
    if let syn::Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if segment.ident == "Option" {
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                        return Ok(inner);
                    }
                }
            }
        }
    }
    Err(Error::new_spanned(ty, "Score field must be Option<Score>"))
}

fn extract_collection_inner_type(ty: &syn::Type) -> Option<&syn::Type> {
    if let syn::Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if segment.ident == "Vec" {
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                        return Some(inner);
                    }
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::expand_derive;
    use syn::parse_quote;

    #[test]
    fn golden_solution_expansion_emits_constraint_streams_and_descriptor() {
        let input = parse_quote! {
            #[solverforge_constraints_path = "crate::constraints::create_constraints"]
            struct Plan {
                #[problem_fact_collection]
                workers: Vec<Worker>,
                #[planning_entity_collection]
                tasks: Vec<Task>,
                #[planning_score]
                score: Option<HardSoftScore>,
            }
        };

        let expanded = expand_derive(input)
            .expect("solution expansion should succeed")
            .to_string();

        assert!(expanded.contains("impl :: solverforge :: __internal :: PlanningSolution for Plan"));
        assert!(expanded.contains("pub trait PlanConstraintStreams"));
        assert!(expanded.contains(
            "pub fn descriptor () -> :: solverforge :: __internal :: SolutionDescriptor"
        ));
        assert!(expanded.contains("create_constraints"));
    }
}
