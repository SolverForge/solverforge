use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Data, DeriveInput, Error, Fields};

use crate::attr_parse::{get_attribute, has_attribute};
use crate::attr_validation::{
    validate_list_element_collection_attribute, validate_no_attribute_args,
    validate_shadow_updates_attribute,
};

use super::config::{
    parse_config_path, parse_conflict_repairs_path, parse_constraints_path,
    parse_coverage_groups_path, parse_scalar_groups_path, parse_shadow_config,
    parse_solver_toml_path,
};
use super::list_operations::generate_list_operations;
use super::runtime::{
    generate_runtime_phase_support, generate_runtime_solve_internal, generate_solvable_solution,
};
use super::shadow::generate_shadow_support;
use super::stream_extensions::generate_collection_source_methods;
use super::type_helpers::{extract_collection_inner_type, extract_option_inner_type};

pub(crate) fn expand_derive(input: DeriveInput) -> Result<TokenStream, Error> {
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

    validate_solution_attributes(&input, fields)?;

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
                .with_entity({
                    let __solverforge_entity_descriptor =
                        #element_type::entity_descriptor(#field_name_str);
                    let __solverforge_entity_type_name =
                        __solverforge_entity_descriptor.type_name;
                    __solverforge_entity_descriptor.with_extractor(
                        Box::new(::solverforge::__internal::EntityCollectionExtractor::new(
                            __solverforge_entity_type_name,
                            #field_name_str,
                            |s: &#name| &s.#field_name,
                            |s: &mut #name| &mut s.#field_name,
                        ))
                    )
                })
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
                .with_problem_fact({
                    let __solverforge_fact_descriptor =
                        #element_type::problem_fact_descriptor(#field_name_str);
                    let __solverforge_fact_type_name =
                        __solverforge_fact_descriptor.type_name;
                    __solverforge_fact_descriptor.with_extractor(
                        Box::new(::solverforge::__internal::EntityCollectionExtractor::new(
                            __solverforge_fact_type_name,
                            #field_name_str,
                            |s: &#name| &s.#field_name,
                            |s: &mut #name| &mut s.#field_name,
                        ))
                    )
                })
            })
        })
        .collect();

    let name_str = name.to_string();
    let score_field_str = score_field_name.to_string();

    let shadow_config = parse_shadow_config(&input.attrs);
    let shadow_support_impl = generate_shadow_support(&shadow_config, fields, name)?;
    let constraints_path = parse_constraints_path(&input.attrs);
    let config_path = parse_config_path(&input.attrs);
    let solver_toml_path = parse_solver_toml_path(&input.attrs);
    let conflict_repairs_path = parse_conflict_repairs_path(&input.attrs);
    let scalar_groups_path = parse_scalar_groups_path(&input.attrs);
    let coverage_groups_path = parse_coverage_groups_path(&input.attrs);
    let entity_count_arms: Vec<_> = fields
        .iter()
        .filter(|f| has_attribute(&f.attrs, "planning_entity_collection"))
        .enumerate()
        .map(|(idx, f)| {
            let field_name = f.ident.as_ref().unwrap();
            quote! { #idx => this.#field_name.len(), }
        })
        .collect();
    let collection_accessors: Vec<_> = fields
        .iter()
        .filter(|f| {
            has_attribute(&f.attrs, "planning_entity_collection")
                || has_attribute(&f.attrs, "problem_fact_collection")
        })
        .filter_map(|f| {
            let field_name = f.ident.as_ref()?;
            let element_type = extract_collection_inner_type(&f.ty)?;
            let collection_ident = format_ident!("__solverforge_collection_{}", field_name);
            let collection_mut_ident = format_ident!("__solverforge_collection_{}_mut", field_name);
            let entity_ident = format_ident!("__solverforge_entity_{}", field_name);
            Some(quote! {
                #[doc(hidden)]
                #[allow(dead_code, private_interfaces)]
                pub fn #collection_ident(this: &Self) -> &[#element_type] {
                    &this.#field_name
                }

                #[doc(hidden)]
                #[allow(dead_code, private_interfaces)]
                pub fn #collection_mut_ident(this: &mut Self) -> &mut [#element_type] {
                    &mut this.#field_name
                }

                #[doc(hidden)]
                #[allow(dead_code, private_interfaces)]
                pub fn #entity_ident(this: &Self, entity_index: usize) -> &#element_type {
                    &this.#field_name[entity_index]
                }
            })
        })
        .collect();

    let list_operations = generate_list_operations(fields);
    let runtime_phase_support = generate_runtime_phase_support(
        fields,
        &constraints_path,
        &conflict_repairs_path,
        &scalar_groups_path,
        &coverage_groups_path,
        name,
    );
    let runtime_solve_internal =
        generate_runtime_solve_internal(&constraints_path, &config_path, &solver_toml_path);
    let solvable_solution_impl = generate_solvable_solution(name, &constraints_path);

    let collection_source_methods = generate_collection_source_methods(fields);

    let expanded = quote! {
        impl #impl_generics ::solverforge::__internal::PlanningSolution for #name #ty_generics #where_clause {
            type Score = #score_type;
            fn score(&self) -> Option<Self::Score> { self.#score_field_name.clone() }
            fn set_score(&mut self, score: Option<Self::Score>) { self.#score_field_name = score; }
            #shadow_support_impl
        }

        impl #impl_generics #name #ty_generics #where_clause {
            pub fn descriptor() -> ::solverforge::__internal::SolutionDescriptor {
                let mut descriptor = ::solverforge::__internal::SolutionDescriptor::new(
                    #name_str,
                    ::std::any::TypeId::of::<Self>(),
                )
                .with_score_field(#score_field_str)
                #(#entity_descriptors)*
                #(#fact_descriptors)*;
                <Self as ::solverforge::__internal::PlanningModelSupport>::attach_descriptor_hooks(
                    &mut descriptor,
                );
                <Self as ::solverforge::__internal::PlanningModelSupport>::validate_model(
                    &descriptor,
                );
                descriptor
            }

            #[inline]
            pub fn entity_count(this: &Self, descriptor_index: usize) -> usize {
                match descriptor_index {
                    #(#entity_count_arms)*
                    _ => 0,
                }
            }

            #(#collection_accessors)*
            #collection_source_methods

            #list_operations
            #runtime_solve_internal
        }

        #runtime_phase_support
        #solvable_solution_impl
    };

    Ok(expanded)
}

fn validate_solution_attributes(
    input: &DeriveInput,
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
) -> Result<(), Error> {
    if let Some(attr) = get_attribute(&input.attrs, "shadow_variable_updates") {
        validate_shadow_updates_attribute(attr)?;
    }

    for field in fields {
        if let Some(attr) = get_attribute(&field.attrs, "planning_entity_collection") {
            validate_no_attribute_args(attr, "planning_entity_collection")?;
        }
        if let Some(attr) = get_attribute(&field.attrs, "problem_fact_collection") {
            validate_no_attribute_args(attr, "problem_fact_collection")?;
        }
        if let Some(attr) = get_attribute(&field.attrs, "planning_score") {
            validate_no_attribute_args(attr, "planning_score")?;
        }
        if let Some(attr) = get_attribute(&field.attrs, "value_range_provider") {
            validate_no_attribute_args(attr, "value_range_provider")?;
        }
        if let Some(attr) = get_attribute(&field.attrs, "planning_list_element_collection") {
            validate_list_element_collection_attribute(attr)?;
        }
    }

    Ok(())
}
