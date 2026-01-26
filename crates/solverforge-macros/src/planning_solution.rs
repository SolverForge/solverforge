//! #[planning_solution] derive macro implementation

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Error, Fields, Ident, Lit, Meta};

use crate::{get_attribute, has_attribute, parse_attribute_list, parse_attribute_string};

#[derive(Default)]
struct ShadowConfig {
    list_owner: Option<String>,
    list_field: Option<String>,
    element_collection: Option<String>,
    inverse_field: Option<String>,
    previous_field: Option<String>,
    next_field: Option<String>,
    cascading_listener: Option<String>,
    post_update_listener: Option<String>,
    element_type: Option<String>,
    /// Aggregate shadow fields on the list owner entity.
    /// Format: "field_name:aggregation:source_field" (e.g., "total_demand:sum:demand")
    entity_aggregates: Vec<String>,
    /// O(1) incremental delta shadow fields on the list owner entity.
    /// Format: "field_name:delta_method" (e.g., "total_driving_time_seconds:compute_driving_time_delta")
    /// Method signature: fn(&self, entity_idx: usize, position: usize, element_idx: usize, is_insert: bool) -> T
    entity_deltas: Vec<String>,
    /// Computed shadow fields on the list owner entity.
    /// Format: "field_name:compute_method" (e.g., "total_driving_time_seconds:compute_vehicle_driving_time")
    /// Method signature: fn(&self, entity_idx: usize) -> T
    entity_computes: Vec<String>,
}

/// Configuration for basic (non-list) planning variables.
///
/// Used with `#[basic_variable_config(...)]` attribute to specify:
/// - Which entity collection contains planning entities
/// - Which field is the planning variable
/// - The type of the variable
/// - Where to get valid values from
#[derive(Default)]
struct BasicVariableConfig {
    /// Entity collection field name (e.g., "shifts")
    entity_collection: Option<String>,
    /// Planning variable field name (e.g., "employee_idx")
    variable_field: Option<String>,
    /// Variable type (e.g., "usize")
    variable_type: Option<String>,
    /// Value range source - either a field name or "0..entity_count"
    value_range: Option<String>,
}

/// Parse the constraints path from #[solverforge_constraints_path = "path"]
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
        config.list_field = parse_attribute_string(attr, "list_field");
        config.element_collection = parse_attribute_string(attr, "element_collection");
        config.inverse_field = parse_attribute_string(attr, "inverse_field");
        config.previous_field = parse_attribute_string(attr, "previous_field");
        config.next_field = parse_attribute_string(attr, "next_field");
        config.cascading_listener = parse_attribute_string(attr, "cascading_listener");
        config.post_update_listener = parse_attribute_string(attr, "post_update_listener");
        config.element_type = parse_attribute_string(attr, "element_type");
        config.entity_aggregates = parse_attribute_list(attr, "entity_aggregate");
        config.entity_deltas = parse_attribute_list(attr, "entity_delta");
        config.entity_computes = parse_attribute_list(attr, "entity_compute");
    }

    config
}

fn parse_basic_variable_config(attrs: &[syn::Attribute]) -> BasicVariableConfig {
    let mut config = BasicVariableConfig::default();

    if let Some(attr) = get_attribute(attrs, "basic_variable_config") {
        config.entity_collection = parse_attribute_string(attr, "entity_collection");
        config.variable_field = parse_attribute_string(attr, "variable_field");
        config.variable_type = parse_attribute_string(attr, "variable_type");
        config.value_range = parse_attribute_string(attr, "value_range");
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

    // Entity descriptors are metadata-only (zero-erasure architecture)
    // Entity access is done through generated methods on the solution type
    let entity_descriptors: Vec<_> = fields
        .iter()
        .filter(|f| has_attribute(&f.attrs, "planning_entity_collection"))
        .filter_map(|f| {
            let field_name = f.ident.as_ref().unwrap();
            let field_name_str = field_name.to_string();
            let element_type = extract_collection_inner_type(&f.ty)?;
            Some(quote! {
                .with_entity(::solverforge::__internal::EntityDescriptor::new(
                    stringify!(#element_type),
                    ::std::any::TypeId::of::<#element_type>(),
                    #field_name_str,
                ))
            })
        })
        .collect();

    // Problem fact descriptors are metadata-only (zero-erasure architecture)
    let fact_descriptors: Vec<_> = fields
        .iter()
        .filter(|f| has_attribute(&f.attrs, "problem_fact_collection"))
        .filter_map(|f| {
            let field_name = f.ident.as_ref().unwrap();
            let field_name_str = field_name.to_string();
            let element_type = extract_collection_inner_type(&f.ty)?;
            Some(quote! {
                .with_problem_fact(::solverforge::__internal::ProblemFactDescriptor::new(
                    stringify!(#element_type),
                    ::std::any::TypeId::of::<#element_type>(),
                    #field_name_str,
                ))
            })
        })
        .collect();

    let name_str = name.to_string();
    let score_field_str = score_field_name.to_string();

    let shadow_config = parse_shadow_config(&input.attrs);
    let shadow_support_impl = generate_shadow_support(&shadow_config, name, fields);
    let constraints_path = parse_constraints_path(&input.attrs);
    let basic_config = parse_basic_variable_config(&input.attrs);

    let entity_count_arms: Vec<_> = fields
        .iter()
        .filter(|f| has_attribute(&f.attrs, "planning_entity_collection"))
        .enumerate()
        .map(|(idx, f)| {
            let field_name = f.ident.as_ref().unwrap();
            quote! { #idx => this.#field_name.len(), }
        })
        .collect();

    let list_operations = generate_list_operations(&shadow_config, fields, &constraints_path);
    let basic_operations =
        generate_basic_variable_operations(&basic_config, fields, &constraints_path, name);
    let solvable_solution_impl =
        generate_solvable_solution(&shadow_config, &basic_config, name, &constraints_path);

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
            #basic_operations
        }

        #shadow_support_impl

        #solvable_solution_impl
    };

    Ok(expanded)
}

fn generate_list_operations(
    config: &ShadowConfig,
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
    constraints_path: &Option<String>,
) -> TokenStream {
    let (list_owner, list_field, element_type, element_collection) = match (
        &config.list_owner,
        &config.list_field,
        &config.element_type,
        &config.element_collection,
    ) {
        (Some(lo), Some(lf), Some(et), Some(ec)) => (lo, lf, et, ec),
        _ => return TokenStream::new(),
    };

    let list_owner_ident = Ident::new(list_owner, proc_macro2::Span::call_site());
    let list_field_ident = Ident::new(list_field, proc_macro2::Span::call_site());
    let element_type_ident = Ident::new(element_type, proc_macro2::Span::call_site());
    let list_field_str = list_field.as_str();

    let entity_fields: Vec<_> = fields
        .iter()
        .filter(|f| has_attribute(&f.attrs, "planning_entity_collection"))
        .collect();

    let descriptor_index = entity_fields
        .iter()
        .position(|f| f.ident.as_ref().map(|i| i.to_string()) == Some(list_owner.clone()))
        .expect("list_owner must be a planning_entity_collection field");

    let descriptor_index_lit = syn::LitInt::new(
        &descriptor_index.to_string(),
        proc_macro2::Span::call_site(),
    );

    let element_collection_ident2 = Ident::new(element_collection, proc_macro2::Span::call_site());

    // Generate finalize calls for all problem_fact_collection fields
    let finalize_calls: Vec<_> = fields
        .iter()
        .filter(|f| has_attribute(&f.attrs, "problem_fact_collection"))
        .filter_map(|f| {
            let field_name = f.ident.as_ref()?;
            Some(quote! {
                for item in &mut s.#field_name {
                    item.finalize();
                }
            })
        })
        .collect();

    // Generate solve_internal() only if constraints path is provided
    let solve_impl = constraints_path.as_ref().map(|path| {
        let constraints_fn: syn::Path =
            syn::parse_str(path).expect("constraints path must be a valid Rust path");

        quote! {
            /// Internal solve implementation called by the Solvable trait.
            fn solve_internal(
                mut self,
                terminate: Option<&std::sync::atomic::AtomicBool>,
                sender: ::tokio::sync::mpsc::UnboundedSender<(Self, <Self as ::solverforge::__internal::PlanningSolution>::Score)>,
            ) -> Self {
                ::solverforge::__internal::init_console();

                Self::list_finalize_all(&mut self);

                let entity_count = Self::n_entities(&self);
                let value_count = Self::element_count(&self);
                ::solverforge::__internal::tracing::info!(
                    event = "solve_start",
                    entity_count = entity_count,
                    value_count = value_count,
                );

                let config = ::solverforge::__internal::SolverConfig::load("solver.toml").unwrap_or_default();
                let constraints = #constraints_fn();
                let director = ::solverforge::__internal::ScoreDirector::with_descriptor(
                    self,
                    constraints,
                    Self::descriptor(),
                    |s, descriptor_idx| {
                        if descriptor_idx == #descriptor_index_lit {
                            Self::n_entities(s)
                        } else {
                            0
                        }
                    },
                );

                // Direct construction - no factory
                let placer = ::solverforge::__internal::ListEntityPlacer::<Self, #element_type_ident>::new(
                    Self::element_count,
                    Self::assigned_elements,
                    Self::n_entities,
                    Self::assign_element,
                    Self::list_len_fn,
                    Self::list_remove_fn,
                    Self::index_to_element,
                    #descriptor_index_lit,
                );
                let construction = ::solverforge::__internal::ConstructionHeuristicPhase::new(
                    placer,
                    ::solverforge::__internal::ConstructionForagerImpl::from_config(
                        ::solverforge::__internal::ConstructionHeuristicType::FirstFit
                    ),
                );

                // Direct construction - MoveSelectorImpl for list local search
                let list_fn_ptrs = ::solverforge::__internal::ListVariableFnPtrs {
                    entity_count: Self::n_entities,
                    element_count: Self::element_count,
                    assigned_elements: Self::assigned_elements,
                    list_len: Self::list_len_fn,
                    list_get: Self::list_get_fn,
                    list_set: Self::list_set_fn,
                    list_remove: Self::list_remove_fn,
                    list_insert: Self::list_insert_fn,
                    sublist_remove: |s, e, start, end| Self::sublist_remove(s, e, start, end),
                    sublist_insert: |s, e, pos, items| Self::sublist_insert(s, e, pos, items),
                    list_reverse: Self::list_reverse_fn,
                    list_get_element_idx: Self::list_get_element_idx_fn,
                    assign: Self::assign_element,
                    variable_name: #list_field_str,
                    descriptor_index: #descriptor_index_lit,
                };
                let move_selector = ::solverforge::__internal::MoveSelectorImpl::k_opt(list_fn_ptrs, 3, 1);
                let acceptor = ::solverforge::__internal::AcceptorImpl::late_acceptance();
                let forager = ::solverforge::__internal::LocalSearchForagerImpl::accepted_count(1000);
                let local_search = ::solverforge::__internal::LocalSearchPhase::new(
                    move_selector,
                    acceptor,
                    forager,
                    None,
                    100, // default stagnation threshold
                ).with_sender(sender.clone());

                let time_limit = config.termination.as_ref()
                    .and_then(|t| t.time_limit())
                    .unwrap_or(std::time::Duration::from_secs(30));

                let mut solver = ::solverforge::__internal::Solver::new((construction, local_search))
                    .with_time_limit(time_limit);

                if let Some(flag) = terminate {
                    solver = solver.with_terminate(flag);
                }

                let result = solver.solve(director);

                {
                    use ::solverforge::__internal::PlanningSolution;
                    let final_score = result.score()
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| "N/A".to_string());
                    ::solverforge::__internal::tracing::info!(
                        event = "solve_end",
                        score = %final_score,
                    );

                    // Send final solution through channel
                    if let Some(score) = result.score() {
                        let _ = sender.send((result.clone(), score));
                    }
                }

                result
            }
        }
    });

    quote! {
        #[inline]
        pub fn list_len(&self, entity_idx: usize) -> usize {
            self.#list_owner_ident
                .get(entity_idx)
                .map_or(0, |e| e.#list_field_ident.len())
        }

        #[inline]
        pub fn list_remove(&mut self, entity_idx: usize, pos: usize) -> Option<#element_type_ident> {
            self.#list_owner_ident
                .get_mut(entity_idx)
                .map(|e| e.#list_field_ident.remove(pos))
        }

        #[inline]
        pub fn list_insert(&mut self, entity_idx: usize, pos: usize, val: #element_type_ident) {
            if let Some(e) = self.#list_owner_ident.get_mut(entity_idx) {
                e.#list_field_ident.insert(pos, val);
            }
        }

        #[inline]
        pub fn sublist_remove(
            &mut self,
            entity_idx: usize,
            start: usize,
            end: usize,
        ) -> Vec<#element_type_ident> {
            self.#list_owner_ident
                .get_mut(entity_idx)
                .map(|e| e.#list_field_ident.drain(start..end).collect())
                .unwrap_or_default()
        }

        #[inline]
        pub fn sublist_insert(
            &mut self,
            entity_idx: usize,
            pos: usize,
            items: Vec<#element_type_ident>,
        ) {
            if let Some(e) = self.#list_owner_ident.get_mut(entity_idx) {
                for (i, item) in items.into_iter().enumerate() {
                    e.#list_field_ident.insert(pos + i, item);
                }
            }
        }

        #[inline]
        pub const fn list_variable_descriptor_index() -> usize {
            #descriptor_index_lit
        }

        /// Total number of elements to assign.
        #[inline]
        pub fn element_count(s: &Self) -> usize {
            s.#element_collection_ident2.len()
        }

        /// Elements already assigned to entities.
        #[inline]
        pub fn assigned_elements(s: &Self) -> Vec<#element_type_ident> {
            s.#list_owner_ident
                .iter()
                .flat_map(|e| e.#list_field_ident.iter().copied())
                .collect()
        }

        /// Number of entities (for construction).
        #[inline]
        pub fn n_entities(s: &Self) -> usize {
            s.#list_owner_ident.len()
        }

        /// Assign element to entity (appends to list).
        #[inline]
        pub fn assign_element(s: &mut Self, entity_idx: usize, elem: #element_type_ident) {
            if let Some(e) = s.#list_owner_ident.get_mut(entity_idx) {
                e.#list_field_ident.push(elem);
            }
        }

        /// Insert element at position (function pointer version for solver).
        #[inline]
        pub fn list_insert_fn(s: &mut Self, entity_idx: usize, pos: usize, elem: #element_type_ident) {
            if let Some(e) = s.#list_owner_ident.get_mut(entity_idx) {
                e.#list_field_ident.insert(pos, elem);
            }
        }

        /// Remove element at position (function pointer version for solver).
        #[inline]
        pub fn list_remove_fn(s: &mut Self, entity_idx: usize, pos: usize) -> Option<#element_type_ident> {
            s.#list_owner_ident
                .get_mut(entity_idx)
                .map(|e| e.#list_field_ident.remove(pos))
        }

        /// Get list length (function pointer version for solver).
        #[inline]
        pub fn list_len_fn(s: &Self, entity_idx: usize) -> usize {
            s.#list_owner_ident
                .get(entity_idx)
                .map_or(0, |e| e.#list_field_ident.len())
        }

        /// Get element at position (function pointer version for solver).
        #[inline]
        pub fn list_get_fn(s: &Self, entity_idx: usize, pos: usize) -> Option<#element_type_ident> {
            s.#list_owner_ident
                .get(entity_idx)
                .and_then(|e| e.#list_field_ident.get(pos).copied())
        }

        /// Set element at position (function pointer version for solver).
        #[inline]
        pub fn list_set_fn(s: &mut Self, entity_idx: usize, pos: usize, val: #element_type_ident) {
            if let Some(e) = s.#list_owner_ident.get_mut(entity_idx) {
                if let Some(slot) = e.#list_field_ident.get_mut(pos) {
                    *slot = val;
                }
            }
        }

        /// Reverse elements in range [start, end) (function pointer version for solver).
        #[inline]
        pub fn list_reverse_fn(s: &mut Self, entity_idx: usize, start: usize, end: usize) {
            if let Some(e) = s.#list_owner_ident.get_mut(entity_idx) {
                if end <= e.#list_field_ident.len() && start < end {
                    e.#list_field_ident[start..end].reverse();
                }
            }
        }

        /// Get element index at position (function pointer version for solver).
        /// For usize elements, the element IS the index.
        #[inline]
        pub fn list_get_element_idx_fn(s: &Self, entity_idx: usize, pos: usize) -> usize {
            s.#list_owner_ident
                .get(entity_idx)
                .and_then(|e| e.#list_field_ident.get(pos).copied())
                .unwrap_or(0) as usize
        }

        /// Convert index to element (identity for usize elements).
        #[inline]
        pub fn index_to_element(idx: usize) -> #element_type_ident {
            idx as #element_type_ident
        }

        /// Finalize all problem facts before solving (list variable version).
        #[inline]
        pub fn list_finalize_all(s: &mut Self) {
            #(#finalize_calls)*
        }

        #solve_impl
    }
}

fn generate_basic_variable_operations(
    config: &BasicVariableConfig,
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
    constraints_path: &Option<String>,
    _solution_name: &Ident,
) -> TokenStream {
    // All four fields required for basic variable support
    let (entity_collection, variable_field, variable_type, value_range) = match (
        &config.entity_collection,
        &config.variable_field,
        &config.variable_type,
        &config.value_range,
    ) {
        (Some(ec), Some(vf), Some(vt), Some(vr)) => (ec, vf, vt, vr),
        _ => return TokenStream::new(),
    };

    let entity_collection_ident = Ident::new(entity_collection, proc_macro2::Span::call_site());
    let variable_field_ident = Ident::new(variable_field, proc_macro2::Span::call_site());
    let variable_type_ident = Ident::new(variable_type, proc_macro2::Span::call_site());
    let value_range_ident = Ident::new(value_range, proc_macro2::Span::call_site());
    let variable_field_str = variable_field.as_str();

    // Find descriptor index for the entity collection
    let entity_fields: Vec<_> = fields
        .iter()
        .filter(|f| has_attribute(&f.attrs, "planning_entity_collection"))
        .collect();

    let descriptor_index = entity_fields
        .iter()
        .position(|f| f.ident.as_ref().map(|i| i.to_string()).as_ref() == Some(entity_collection))
        .expect("entity_collection must be a planning_entity_collection field");

    let descriptor_index_lit = syn::LitInt::new(
        &descriptor_index.to_string(),
        proc_macro2::Span::call_site(),
    );

    // Generate finalize calls for all problem_fact_collection fields
    let finalize_calls: Vec<_> = fields
        .iter()
        .filter(|f| has_attribute(&f.attrs, "problem_fact_collection"))
        .filter_map(|f| {
            let field_name = f.ident.as_ref()?;
            Some(quote! {
                for item in &mut s.#field_name {
                    item.finalize();
                }
            })
        })
        .collect();

    // Generate solve_internal() only if constraints path is provided
    let solve_impl = constraints_path.as_ref().map(|path| {
        let constraints_fn: syn::Path =
            syn::parse_str(path).expect("constraints path must be a valid Rust path");

        quote! {
            /// Internal solve implementation called by the Solvable trait.
            fn solve_internal(
                mut self,
                terminate: Option<&std::sync::atomic::AtomicBool>,
                sender: ::tokio::sync::mpsc::UnboundedSender<(Self, <Self as ::solverforge::__internal::PlanningSolution>::Score)>,
            ) -> Self {
                ::solverforge::__internal::init_console();

                Self::finalize_all(&mut self);

                let entity_count = Self::basic_entity_count(&self);
                let value_count = Self::basic_value_count(&self);
                ::solverforge::__internal::tracing::info!(
                    event = "solve_start",
                    entity_count = entity_count,
                    value_count = value_count,
                );

                let config = ::solverforge::__internal::SolverConfig::load("solver.toml").unwrap_or_default();
                let constraints = #constraints_fn();
                let director = ::solverforge::__internal::ScoreDirector::with_descriptor(
                    self,
                    constraints,
                    Self::descriptor(),
                    |s, descriptor_idx| {
                        if descriptor_idx == #descriptor_index_lit {
                            Self::basic_entity_count(s)
                        } else {
                            0
                        }
                    },
                );

                // Direct construction - no factory
                let entity_selector = ::solverforge::__internal::FromSolutionEntitySelector::new(#descriptor_index_lit);
                let value_selector = ::solverforge::__internal::RangeValueSelector::new(Self::basic_value_count);
                let placer = ::solverforge::__internal::QueuedEntityPlacer::new(
                    entity_selector,
                    value_selector,
                    Self::basic_get_variable,
                    Self::basic_set_variable,
                    #descriptor_index_lit,
                    #variable_field_str,
                );
                let construction = ::solverforge::__internal::ConstructionHeuristicPhase::new(
                    placer,
                    ::solverforge::__internal::ConstructionForagerImpl::from_config(
                        ::solverforge::__internal::ConstructionHeuristicType::FirstFit
                    ),
                );

                // Direct construction - MoveSelectorImpl::union of Change + Swap for basic variable local search
                let fn_ptrs = ::solverforge::__internal::BasicVariableFnPtrs {
                    entity_count: Self::basic_entity_count,
                    value_range: |s| (0..Self::basic_value_count(s)).collect(),
                    getter: Self::basic_get_variable,
                    setter: Self::basic_set_variable,
                    variable_name: #variable_field_str,
                    descriptor_index: #descriptor_index_lit,
                };
                let move_selector = ::solverforge::__internal::MoveSelectorImpl::union(vec![
                    ::solverforge::__internal::MoveSelectorImpl::change(fn_ptrs.clone()),
                    ::solverforge::__internal::MoveSelectorImpl::swap(fn_ptrs),
                ]);
                let acceptor = ::solverforge::__internal::AcceptorImpl::late_acceptance();
                let forager = ::solverforge::__internal::LocalSearchForagerImpl::accepted_count(1000);
                let local_search = ::solverforge::__internal::LocalSearchPhase::new(
                    move_selector,
                    acceptor,
                    forager,
                    None,
                    100, // default stagnation threshold
                ).with_sender(sender.clone());

                let time_limit = config.termination.as_ref()
                    .and_then(|t| t.time_limit())
                    .unwrap_or(std::time::Duration::from_secs(30));

                let mut solver = ::solverforge::__internal::Solver::new((construction, local_search))
                    .with_time_limit(time_limit);

                if let Some(flag) = terminate {
                    solver = solver.with_terminate(flag);
                }

                let result = solver.solve(director);

                {
                    use ::solverforge::__internal::PlanningSolution;
                    let final_score = result.score()
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| "N/A".to_string());
                    ::solverforge::__internal::tracing::info!(
                        event = "solve_end",
                        score = %final_score,
                    );

                    // Send final solution through channel
                    if let Some(score) = result.score() {
                        let _ = sender.send((result.clone(), score));
                    }
                }

                result
            }
        }
    });

    quote! {
        /// Get the planning variable value for an entity.
        #[inline]
        pub fn basic_get_variable(s: &Self, entity_idx: usize) -> Option<#variable_type_ident> {
            s.#entity_collection_ident
                .get(entity_idx)
                .and_then(|e| e.#variable_field_ident)
        }

        /// Set the planning variable value for an entity.
        #[inline]
        pub fn basic_set_variable(s: &mut Self, entity_idx: usize, v: Option<#variable_type_ident>) {
            if let Some(e) = s.#entity_collection_ident.get_mut(entity_idx) {
                e.#variable_field_ident = v;
            }
        }

        /// Get valid values for the planning variable.
        #[inline]
        pub fn basic_value_count(s: &Self) -> usize {
            s.#value_range_ident.len()
        }

        /// Get the number of planning entities.
        #[inline]
        pub fn basic_entity_count(s: &Self) -> usize {
            s.#entity_collection_ident.len()
        }

        /// Get the descriptor index for the basic variable entity.
        #[inline]
        pub const fn basic_variable_descriptor_index() -> usize {
            #descriptor_index_lit
        }

        /// Get the variable field name.
        #[inline]
        pub const fn basic_variable_field_name() -> &'static str {
            #variable_field_str
        }

        /// Finalize all problem facts before solving.
        /// Called automatically by solve() to prepare derived fields.
        #[inline]
        pub fn finalize_all(s: &mut Self) {
            #(#finalize_calls)*
        }

        #solve_impl
    }
}

fn generate_solvable_solution(
    shadow_config: &ShadowConfig,
    basic_config: &BasicVariableConfig,
    solution_name: &Ident,
    constraints_path: &Option<String>,
) -> TokenStream {
    // Generate SolvableSolution impl if either list or basic variable config is present
    let has_list_config = shadow_config.list_owner.is_some();
    let has_basic_config = basic_config.entity_collection.is_some();

    if !has_list_config && !has_basic_config {
        return TokenStream::new();
    }

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
                    sender: ::tokio::sync::mpsc::UnboundedSender<(Self, <Self as ::solverforge::__internal::PlanningSolution>::Score)>,
                ) {
                    let _ = #solution_name::solve_internal(self, terminate, sender);
                }
            }

            impl ::solverforge::Analyzable for #solution_name {
                fn analyze(&self) -> ::solverforge::ScoreAnalysis<<Self as ::solverforge::__internal::PlanningSolution>::Score> {
                    use ::solverforge::__internal::{ScoreDirector, ShadowVariableSupport};

                    let mut solution = self.clone();
                    solution.update_all_shadows();

                    let constraints = #constraints_fn();
                    let mut director = ScoreDirector::with_descriptor(
                        solution,
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
    solution_name: &Ident,
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
) -> TokenStream {
    let (list_owner, list_field, element_collection) = match (
        &config.list_owner,
        &config.list_field,
        &config.element_collection,
    ) {
        (Some(lo), Some(lf), Some(ec)) => (lo, lf, ec),
        _ => {
            return quote! {
                impl ::solverforge::__internal::ShadowVariableSupport for #solution_name {
                    #[inline]
                    fn update_element_shadow(&mut self, _entity_idx: usize, _position: usize, _element_idx: usize) {}
                    #[inline]
                    fn retract_element_shadow(&mut self, _entity_idx: usize, _position: usize, _element_idx: usize) {}
                }
            };
        }
    };

    let list_owner_ident = Ident::new(list_owner, proc_macro2::Span::call_site());
    let list_field_ident = Ident::new(list_field, proc_macro2::Span::call_site());
    let element_collection_ident = Ident::new(element_collection, proc_macro2::Span::call_site());

    // O(1) inverse relation update for a single element
    let inverse_update = config.inverse_field.as_ref().map(|field| {
        let field_ident = Ident::new(field, proc_macro2::Span::call_site());
        quote! {
            // Set inverse relation: element.vehicle_idx = Some(entity_idx)
            self.#element_collection_ident[element_idx].#field_ident = Some(entity_idx);
        }
    });

    let inverse_retract = config.inverse_field.as_ref().map(|field| {
        let field_ident = Ident::new(field, proc_macro2::Span::call_site());
        quote! {
            // Clear inverse relation
            self.#element_collection_ident[element_idx].#field_ident = None;
        }
    });

    // O(1) previous pointer update for a single element
    let previous_update = config.previous_field.as_ref().map(|field| {
        let field_ident = Ident::new(field, proc_macro2::Span::call_site());
        quote! {
            // Set previous pointer: look at position-1 if exists
            // Copy indices first to avoid simultaneous borrows
            let prev_element = if position > 0 {
                self.#list_owner_ident[entity_idx].#list_field_ident.get(position - 1).copied()
            } else {
                None
            };
            let next_elem = self.#list_owner_ident[entity_idx].#list_field_ident.get(position + 1).copied();

            self.#element_collection_ident[element_idx].#field_ident = prev_element;

            // Fix next element's previous pointer to point to us
            if let Some(next_elem) = next_elem {
                self.#element_collection_ident[next_elem].#field_ident = Some(element_idx);
            }
        }
    });

    let previous_retract = config.previous_field.as_ref().map(|field| {
        let field_ident = Ident::new(field, proc_macro2::Span::call_site());
        quote! {
            // Copy indices first to avoid simultaneous borrows
            let next_elem = self.#list_owner_ident[entity_idx].#list_field_ident.get(position + 1).copied();
            let prev_element = if position > 0 {
                self.#list_owner_ident[entity_idx].#list_field_ident.get(position - 1).copied()
            } else {
                None
            };

            // Clear previous pointer
            self.#element_collection_ident[element_idx].#field_ident = None;

            // Fix next element's previous pointer to skip us
            if let Some(next_elem) = next_elem {
                self.#element_collection_ident[next_elem].#field_ident = prev_element;
            }
        }
    });

    // O(1) next pointer update for a single element
    let next_update = config.next_field.as_ref().map(|field| {
        let field_ident = Ident::new(field, proc_macro2::Span::call_site());
        quote! {
            // Copy indices first to avoid simultaneous borrows
            let next_element = self.#list_owner_ident[entity_idx].#list_field_ident.get(position + 1).copied();
            let prev_elem = if position > 0 {
                self.#list_owner_ident[entity_idx].#list_field_ident.get(position - 1).copied()
            } else {
                None
            };

            // Set next pointer
            self.#element_collection_ident[element_idx].#field_ident = next_element;

            // Fix previous element's next pointer to point to us
            if let Some(prev_elem) = prev_elem {
                self.#element_collection_ident[prev_elem].#field_ident = Some(element_idx);
            }
        }
    });

    let next_retract = config.next_field.as_ref().map(|field| {
        let field_ident = Ident::new(field, proc_macro2::Span::call_site());
        quote! {
            // Copy indices first to avoid simultaneous borrows
            let next_element = self.#list_owner_ident[entity_idx].#list_field_ident.get(position + 1).copied();
            let prev_elem = if position > 0 {
                self.#list_owner_ident[entity_idx].#list_field_ident.get(position - 1).copied()
            } else {
                None
            };

            // Clear next pointer
            self.#element_collection_ident[element_idx].#field_ident = None;

            // Fix previous element's next pointer to skip us
            if let Some(prev_elem) = prev_elem {
                self.#element_collection_ident[prev_elem].#field_ident = next_element;
            }
        }
    });

    // O(1) cascading update for a single element
    let cascading_update = config.cascading_listener.as_ref().map(|method| {
        let method_ident = Ident::new(method, proc_macro2::Span::call_site());
        quote! {
            self.#method_ident(element_idx);
        }
    });

    // Note: post_update_listener and entity_aggregates/computes are NOT O(1)
    // They require iterating over all elements. For true O(1), these would need
    // to be incrementally updated. For now, we skip them in per-element updates.
    // The full update_all_shadows() method can still use them for initialization.

    // Find the descriptor index for the element collection (e.g., visits)
    // This is used by ShadowAwareScoreDirector to propagate incremental scoring
    // notifications to element-based constraints.
    let entity_fields: Vec<_> = fields
        .iter()
        .filter(|f| has_attribute(&f.attrs, "planning_entity_collection"))
        .collect();

    let element_descriptor_index = entity_fields.iter().position(|f| {
        f.ident.as_ref().map(|i| i.to_string()).as_deref() == Some(element_collection.as_str())
    });

    let element_descriptor_method = match element_descriptor_index {
        Some(idx) => {
            let idx_lit = syn::LitInt::new(&idx.to_string(), proc_macro2::Span::call_site());
            quote! {
                fn element_descriptor_index() -> Option<usize> {
                    Some(#idx_lit)
                }
            }
        }
        None => quote! {},
    };

    // Generate update_all_shadows for initialization (not O(1), but only called once)
    // Note: We collect indices first to avoid borrow conflicts - can't hold &self.vehicles
    // while mutating self.visits
    let full_inverse_update = config.inverse_field.as_ref().map(|field| {
        let field_ident = Ident::new(field, proc_macro2::Span::call_site());
        quote! {
            // Collect (entity_idx, element_indices) pairs to avoid borrow conflicts
            let entity_elements: Vec<(usize, Vec<usize>)> = self.#list_owner_ident
                .iter()
                .enumerate()
                .map(|(entity_idx, entity)| (entity_idx, entity.#list_field_ident.clone()))
                .collect();
            for (entity_idx, element_indices) in entity_elements {
                for element_idx in element_indices {
                    self.#element_collection_ident[element_idx].#field_ident = Some(entity_idx);
                }
            }
        }
    });

    let full_previous_update = config.previous_field.as_ref().map(|field| {
        let field_ident = Ident::new(field, proc_macro2::Span::call_site());
        quote! {
            // Collect element indices per entity to avoid borrow conflicts
            let all_visits: Vec<Vec<usize>> = self.#list_owner_ident
                .iter()
                .map(|entity| entity.#list_field_ident.clone())
                .collect();
            for visits in all_visits {
                let mut prev_idx: Option<usize> = None;
                for element_idx in visits {
                    self.#element_collection_ident[element_idx].#field_ident = prev_idx;
                    prev_idx = Some(element_idx);
                }
            }
        }
    });

    let full_next_update = config.next_field.as_ref().map(|field| {
        let field_ident = Ident::new(field, proc_macro2::Span::call_site());
        quote! {
            // Collect element indices per entity to avoid borrow conflicts
            let all_visits: Vec<Vec<usize>> = self.#list_owner_ident
                .iter()
                .map(|entity| entity.#list_field_ident.clone())
                .collect();
            for visits in all_visits {
                let len = visits.len();
                for (i, element_idx) in visits.iter().enumerate() {
                    let next_idx = if i + 1 < len { Some(visits[i + 1]) } else { None };
                    self.#element_collection_ident[*element_idx].#field_ident = next_idx;
                }
            }
        }
    });

    let full_cascading_update = config.cascading_listener.as_ref().map(|method| {
        let method_ident = Ident::new(method, proc_macro2::Span::call_site());
        quote! {
            // Collect all element indices first to avoid borrow conflicts
            let all_elements: Vec<usize> = self.#list_owner_ident
                .iter()
                .flat_map(|entity| entity.#list_field_ident.clone())
                .collect();
            for element_idx in all_elements {
                self.#method_ident(element_idx);
            }
        }
    });

    let full_post_update = config.post_update_listener.as_ref().map(|method| {
        let method_ident = Ident::new(method, proc_macro2::Span::call_site());
        quote! {
            for entity_idx in 0..self.#list_owner_ident.len() {
                self.#method_ident(entity_idx);
            }
        }
    });

    // Entity aggregates for full update
    let full_aggregate_updates: Vec<_> = config
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
                    for entity_idx in 0..self.#list_owner_ident.len() {
                        // Copy indices first to avoid simultaneous borrows
                        let element_indices: Vec<usize> = self.#list_owner_ident[entity_idx].#list_field_ident.clone();
                        let sum = element_indices
                            .iter()
                            .map(|&idx| self.#element_collection_ident[idx].#source_field)
                            .sum();
                        self.#list_owner_ident[entity_idx].#target_field = sum;
                    }
                }),
                _ => None,
            }
        })
        .collect();

    // Entity deltas for O(1) incremental update
    let delta_insert_updates: Vec<_> = config
        .entity_deltas
        .iter()
        .filter_map(|spec| {
            let parts: Vec<&str> = spec.split(':').collect();
            if parts.len() != 2 {
                return None;
            }
            let target_field = Ident::new(parts[0], proc_macro2::Span::call_site());
            let method_name = Ident::new(parts[1], proc_macro2::Span::call_site());

            Some(quote! {
                self.#list_owner_ident[entity_idx].#target_field += self.#method_name(entity_idx, position, element_idx, true);
            })
        })
        .collect();

    let delta_retract_updates: Vec<_> = config
        .entity_deltas
        .iter()
        .filter_map(|spec| {
            let parts: Vec<&str> = spec.split(':').collect();
            if parts.len() != 2 {
                return None;
            }
            let target_field = Ident::new(parts[0], proc_macro2::Span::call_site());
            let method_name = Ident::new(parts[1], proc_macro2::Span::call_site());

            Some(quote! {
                self.#list_owner_ident[entity_idx].#target_field += self.#method_name(entity_idx, position, element_idx, false);
            })
        })
        .collect();

    // Entity compute incremental: recompute the field after each change
    let compute_incremental_updates: Vec<_> = config
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

    // Full delta initialization: reset to 0 then insert all elements
    let full_delta_updates: Vec<_> = config
        .entity_deltas
        .iter()
        .filter_map(|spec| {
            let parts: Vec<&str> = spec.split(':').collect();
            if parts.len() != 2 {
                return None;
            }
            let target_field = Ident::new(parts[0], proc_macro2::Span::call_site());
            let method_name = Ident::new(parts[1], proc_macro2::Span::call_site());

            Some(quote! {
                for entity_idx in 0..self.#list_owner_ident.len() {
                    self.#list_owner_ident[entity_idx].#target_field = Default::default();
                    for position in 0..self.#list_owner_ident[entity_idx].#list_field_ident.len() {
                        let element_idx = self.#list_owner_ident[entity_idx].#list_field_ident[position];
                        self.#list_owner_ident[entity_idx].#target_field += self.#method_name(entity_idx, position, element_idx, true);
                    }
                }
            })
        })
        .collect();

    quote! {
        impl ::solverforge::__internal::ShadowVariableSupport for #solution_name {
            #[inline]
            fn update_element_shadow(&mut self, entity_idx: usize, position: usize, element_idx: usize) {
                // O(1) shadow update for a single element
                #inverse_update
                #previous_update
                #next_update
                #cascading_update
                #(#delta_insert_updates)*
                #(#compute_incremental_updates)*
            }

            #[inline]
            fn retract_element_shadow(&mut self, entity_idx: usize, position: usize, element_idx: usize) {
                // O(1) shadow retraction for a single element
                #inverse_retract
                #previous_retract
                #next_retract
                #(#delta_retract_updates)*
                #(#compute_incremental_updates)*
            }

            #[inline]
            fn update_all_shadows(&mut self) {
                // Full shadow update for initialization (not O(1))
                #full_inverse_update
                #full_previous_update
                #full_next_update
                #full_cascading_update
                #(#full_aggregate_updates)*
                #(#full_delta_updates)*
                // Compute entity-level shadow fields for all entities
                for entity_idx in 0..self.#list_owner_ident.len() {
                    #(#compute_incremental_updates)*
                }
                #full_post_update
            }

            #element_descriptor_method
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
