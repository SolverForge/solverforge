//! #[planning_solution] derive macro implementation

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Error, Fields, Ident, Lit, Meta};

use crate::{get_attribute, has_attribute, parse_attribute_string};

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
                .with_entity(::solverforge::__internal::EntityDescriptor::new(
                    stringify!(#element_type),
                    ::std::any::TypeId::of::<#element_type>(),
                    #field_name_str,
                ).with_extractor(Box::new(::solverforge::__internal::TypedEntityExtractor::new(
                    stringify!(#element_type),
                    #field_name_str,
                    |s: &#name| &s.#field_name,
                    |s: &mut #name| &mut s.#field_name,
                ))))
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
                .with_problem_fact(::solverforge::__internal::ProblemFactDescriptor::new(
                    stringify!(#element_type),
                    ::std::any::TypeId::of::<#element_type>(),
                    #field_name_str,
                ).with_extractor(Box::new(::solverforge::__internal::TypedEntityExtractor::new(
                    stringify!(#element_type),
                    #field_name_str,
                    |s: &#name| &s.#field_name,
                    |s: &mut #name| &mut s.#field_name,
                ))))
            })
        })
        .collect();

    let name_str = name.to_string();
    let score_field_str = score_field_name.to_string();

    let shadow_config = parse_shadow_config(&input.attrs);
    let shadow_support_impl = generate_shadow_support(&shadow_config, name);
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

    let list_operations = generate_list_operations(&shadow_config, fields, &constraints_path);
    let solvable_solution_impl = generate_solvable_solution(&shadow_config, name);

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

    // Generate solve() only if constraints path is provided
    let list_field_str = list_field;
    let solve_impl = constraints_path.as_ref().map(|path| {
        let constraints_fn: syn::Path = syn::parse_str(path)
            .expect("constraints path must be a valid Rust path");

        quote! {
            /// Solve with external termination flag.
            pub fn solve_with_terminate(
                self,
                terminate: Option<::std::sync::Arc<::std::sync::atomic::AtomicBool>>,
            ) -> Self {
                Self::solve_internal(self, terminate)
            }

            /// Solve with zero parameters - constraints embedded at compile time.
            pub fn solve(self) -> Self {
                Self::solve_internal(self, None)
            }

            fn solve_internal(
                solution: Self,
                _terminate: Option<::std::sync::Arc<::std::sync::atomic::AtomicBool>>,
            ) -> Self {
                use ::solverforge::__internal::{
                    SolverManager, SolverPhaseFactory,
                    KOptPhaseBuilder, ListConstructionPhaseBuilder,
                    FromSolutionEntitySelector, DefaultDistanceMeter,
                    ShadowAwareScoreDirector, TypedScoreDirector, ScoreDirector,
                    SolverConfig,
                };

                // Load config
                let config = SolverConfig::load("solver.toml").unwrap_or_default();

                // Constraints embedded at compile time
                let constraints = #constraints_fn();

                // Build SolverManager with constraint-based scoring
                let descriptor_index = Self::list_variable_descriptor_index();

                // Construction phase
                let construction = ListConstructionPhaseBuilder::<Self, usize>::new(
                    Self::element_count,
                    Self::assigned_elements,
                    Self::n_entities,
                    Self::assign_element,
                    |i| i,
                    #list_field_str,
                    descriptor_index,
                );

                // Local search phase
                let local_search = KOptPhaseBuilder::<Self, usize, _, _>::new(
                    DefaultDistanceMeter,
                    move || Box::new(FromSolutionEntitySelector::new(descriptor_index)),
                    Self::list_len,
                    Self::sublist_remove,
                    Self::sublist_insert,
                    #list_field_str,
                    descriptor_index,
                );

                let manager = SolverManager::<Self>::builder(move |solution: &Self| {
                    let constraints_clone = #constraints_fn();
                    let mut director = ShadowAwareScoreDirector::new(
                        TypedScoreDirector::with_descriptor(
                            solution.clone(),
                            constraints_clone,
                            Self::descriptor(),
                            Self::entity_count,
                        ),
                    );
                    director.calculate_score()
                })
                .with_phase_factory(construction)
                .with_phase_factory(local_search)
                .with_config(config)
                .build()
                .expect("Failed to build solver");

                // Create director for solving
                let director = ShadowAwareScoreDirector::new(
                    TypedScoreDirector::with_descriptor(
                        solution,
                        constraints,
                        Self::descriptor(),
                        Self::entity_count,
                    ),
                );

                // Solve
                let mut solver = manager.create_solver();
                solver.solve_with_director(Box::new(director))
            }
        }
    });

    let element_collection_ident2 = Ident::new(element_collection, proc_macro2::Span::call_site());

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

        #solve_impl
    }
}

fn generate_solvable_solution(config: &ShadowConfig, solution_name: &Ident) -> TokenStream {
    if config.list_owner.is_none() {
        return TokenStream::new();
    }

    quote! {
        impl ::solverforge::__internal::SolvableSolution for #solution_name {
            fn descriptor() -> ::solverforge::__internal::SolutionDescriptor {
                #solution_name::descriptor()
            }

            fn entity_count(solution: &Self, descriptor_index: usize) -> usize {
                #solution_name::entity_count(solution, descriptor_index)
            }
        }
    }
}

fn generate_shadow_support(config: &ShadowConfig, solution_name: &Ident) -> TokenStream {
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
                    fn update_entity_shadows(&mut self, _entity_idx: usize) {}
                }
            };
        }
    };

    let list_owner_ident = Ident::new(list_owner, proc_macro2::Span::call_site());
    let list_field_ident = Ident::new(list_field, proc_macro2::Span::call_site());
    let element_collection_ident = Ident::new(element_collection, proc_macro2::Span::call_site());

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

    quote! {
        impl ::solverforge::__internal::ShadowVariableSupport for #solution_name {
            #[inline]
            fn update_entity_shadows(&mut self, entity_idx: usize) {
                let element_indices: Vec<usize> = self.#list_owner_ident[entity_idx]
                    .#list_field_ident
                    .clone();

                #inverse_update
                #previous_update
                #next_update
                #cascading_update
                #post_update
            }
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
