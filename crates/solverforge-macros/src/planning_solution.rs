// #[planning_solution] derive macro implementation

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

    // Aggregate shadow fields on the list owner entity.
    // Format: "field_name:aggregation:source_field" (e.g., "total_demand:sum:demand")
    entity_aggregates: Vec<String>,

    // Computed shadow fields on the list owner entity.
    // Format: "field_name:method_name" (e.g., "total_driving_time:compute_driving_time")
    entity_computes: Vec<String>,
}

/*
Configuration for basic (non-list) planning variables.

Used with `#[basic_variable_config(...)]` attribute to specify:
- Which entity collection contains planning entities
- Which field is the planning variable
- The type of the variable
- Where to get valid values from
*/
#[derive(Default)]
struct BasicVariableConfig {
    // Entity collection field name (e.g., "shifts")
    entity_collection: Option<String>,

    // Planning variable field name (e.g., "employee_idx")
    variable_field: Option<String>,

    // Variable type (e.g., "usize")
    variable_type: Option<String>,

    // Value range source - either a field name or "0..entity_count"
    value_range: Option<String>,
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
        config.list_field = parse_attribute_string(attr, "list_field");
        config.element_collection = parse_attribute_string(attr, "element_collection");
        config.inverse_field = parse_attribute_string(attr, "inverse_field");
        config.previous_field = parse_attribute_string(attr, "previous_field");
        config.next_field = parse_attribute_string(attr, "next_field");
        config.cascading_listener = parse_attribute_string(attr, "cascading_listener");
        config.post_update_listener = parse_attribute_string(attr, "post_update_listener");
        config.element_type = parse_attribute_string(attr, "element_type");
        config.entity_aggregates = parse_attribute_list(attr, "entity_aggregate");
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

    let list_operations = generate_list_operations(&shadow_config, fields);
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

        #[inline]
        pub fn element_count(s: &Self) -> usize {
            s.#element_collection_ident2.len()
        }

        #[inline]
        pub fn assigned_elements(s: &Self) -> Vec<#element_type_ident> {
            s.#list_owner_ident
                .iter()
                .flat_map(|e| e.#list_field_ident.iter().copied())
                .collect()
        }

        #[inline]
        pub fn n_entities(s: &Self) -> usize {
            s.#list_owner_ident.len()
        }

        #[inline]
        pub fn assign_element(s: &mut Self, entity_idx: usize, elem: #element_type_ident) {
            if let Some(e) = s.#list_owner_ident.get_mut(entity_idx) {
                e.#list_field_ident.push(elem);
            }
        }
    }
}

fn generate_basic_variable_operations(
    config: &BasicVariableConfig,
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
    constraints_path: &Option<String>,
    _solution_name: &Ident,
) -> TokenStream {
    // ALL FOUR FIELDS REQUIRED FOR BASIC VARIABLE SUPPORT
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
                self,
                terminate: Option<&std::sync::atomic::AtomicBool>,
                sender: ::tokio::sync::mpsc::UnboundedSender<(Self, <Self as ::solverforge::__internal::PlanningSolution>::Score)>,
            ) -> Self {
                ::solverforge::__internal::init_console();

                ::solverforge::run_solver_with_channel(
                    self,
                    Self::finalize_all,
                    #constraints_fn,
                    Self::basic_get_variable,
                    Self::basic_set_variable,
                    Self::basic_value_count,
                    Self::basic_entity_count,
                    Self::descriptor,
                    Self::entity_count,
                    terminate,
                    sender,
                )
            }
        }
    });

    quote! {
        #[inline]
        pub fn basic_get_variable(s: &Self, entity_idx: usize) -> Option<#variable_type_ident> {
            s.#entity_collection_ident
                .get(entity_idx)
                .and_then(|e| e.#variable_field_ident)
        }

        #[inline]
        pub fn basic_set_variable(s: &mut Self, entity_idx: usize, v: Option<#variable_type_ident>) {
            if let Some(e) = s.#entity_collection_ident.get_mut(entity_idx) {
                e.#variable_field_ident = v;
            }
        }

        #[inline]
        pub fn basic_value_count(s: &Self) -> usize {
            s.#value_range_ident.len()
        }

        #[inline]
        pub fn basic_entity_count(s: &Self) -> usize {
            s.#entity_collection_ident.len()
        }

        #[inline]
        pub const fn basic_variable_descriptor_index() -> usize {
            #descriptor_index_lit
        }

        #[inline]
        pub const fn basic_variable_field_name() -> &'static str {
            #variable_field_str
        }

        // Finalize all problem facts before solving.
        // Called automatically by solve() to prepare derived fields.
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
                    use ::solverforge::__internal::{
                        TypedScoreDirector, ScoreDirector, ShadowAwareScoreDirector,
                    };

                    let constraints = #constraints_fn();
                    let mut director = ShadowAwareScoreDirector::new(
                        TypedScoreDirector::with_descriptor(
                            self.clone(),
                            constraints,
                            Self::descriptor(),
                            Self::entity_count,
                        ),
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

    // Entity aggregates: "target_field:sum:source_field" on list owner
    // Sums a field from each element in the list and stores on the entity
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

    // Entity computes: "target_field:method_name" on list owner
    // Calls a method on self that takes entity_idx and returns the value
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
                #(#aggregate_updates)*
                #(#compute_updates)*
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
