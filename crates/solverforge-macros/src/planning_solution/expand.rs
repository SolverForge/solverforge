use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Data, DeriveInput, Error, Fields};

use crate::attr_parse::has_attribute;
use crate::scalar_registry::lookup_scalar_entity_metadata;

use super::config::{
    parse_config_path, parse_constraints_path, parse_shadow_config, parse_solver_toml_path,
};
use super::list_operations::generate_list_operations;
use super::runtime::{
    generate_runtime_phase_support, generate_runtime_solve_internal, generate_solvable_solution,
};
use super::shadow::generate_shadow_support;
use super::stream_extensions::generate_constraint_stream_extensions;
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
                    Box::new(::solverforge::__internal::EntityCollectionExtractor::new(
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
                    Box::new(::solverforge::__internal::EntityCollectionExtractor::new(
                        stringify!(#element_type),
                        #field_name_str,
                        |s: &#name| &s.#field_name,
                        |s: &mut #name| &mut s.#field_name,
                    ))
                ))
            })
        })
        .collect();

    let scalar_descriptor_fields: Vec<_> = fields
        .iter()
        .filter(|f| has_attribute(&f.attrs, "planning_entity_collection"))
        .enumerate()
        .filter_map(|(idx, field)| {
            let field_name = field.ident.as_ref()?;
            let field_type = extract_collection_inner_type(&field.ty)?;
            let syn::Type::Path(type_path) = field_type else {
                return None;
            };
            let type_name = type_path.path.segments.last()?.ident.to_string();
            let metadata = lookup_scalar_entity_metadata(&type_name)?;
            if metadata.variables.is_empty() {
                return None;
            }
            Some((idx, field_name, metadata))
        })
        .collect();

    let scalar_descriptor_meter_helpers: Vec<_> = scalar_descriptor_fields
        .iter()
        .flat_map(|(_, field_name, metadata)| {
            metadata.variables.iter().flat_map(move |variable| {
                let mut helpers = Vec::new();
                if let Some(meter_name) = &variable.nearby_value_distance_meter {
                    let meter_ident = format_ident!("{meter_name}");
                    let helper_ident = format_ident!(
                        "__solverforge_descriptor_nearby_value_distance_{}_{}",
                        field_name,
                        variable.field_name
                    );
                    helpers.push(quote! {
                        fn #helper_ident(
                            solution: &dyn ::std::any::Any,
                            entity_index: usize,
                            value: usize,
                        ) -> f64 {
                            let solution = solution
                                .downcast_ref::<Self>()
                                .expect("solution type mismatch for nearby value distance meter");
                            let entity = &solution.#field_name[entity_index];
                            #meter_ident(solution, entity, value)
                        }
                    });
                }
                if let Some(meter_name) = &variable.nearby_entity_distance_meter {
                    let meter_ident = format_ident!("{meter_name}");
                    let helper_ident = format_ident!(
                        "__solverforge_descriptor_nearby_entity_distance_{}_{}",
                        field_name,
                        variable.field_name
                    );
                    helpers.push(quote! {
                        fn #helper_ident(
                            solution: &dyn ::std::any::Any,
                            left_entity_index: usize,
                            right_entity_index: usize,
                        ) -> f64 {
                            let solution = solution
                                .downcast_ref::<Self>()
                                .expect("solution type mismatch for nearby entity distance meter");
                            let left = &solution.#field_name[left_entity_index];
                            let right = &solution.#field_name[right_entity_index];
                            #meter_ident(solution, left, right)
                        }
                    });
                }
                helpers
            })
        })
        .collect();

    let scalar_descriptor_meter_attachments: Vec<_> = scalar_descriptor_fields
        .iter()
        .flat_map(|(descriptor_index, field_name, metadata)| {
            metadata.variables.iter().flat_map(move |variable| {
                let variable_name = &variable.field_name;
                let mut attachments = Vec::new();
                if variable.nearby_value_distance_meter.is_some() {
                    let helper_ident = format_ident!(
                        "__solverforge_descriptor_nearby_value_distance_{}_{}",
                        field_name,
                        variable.field_name
                    );
                    attachments.push(quote! {
                        {
                            let entity_descriptor = descriptor
                                .entity_descriptors
                                .get_mut(#descriptor_index)
                                .expect("entity descriptor missing for nearby value distance meter");
                            let variable_descriptor = entity_descriptor
                                .variable_descriptors
                                .iter_mut()
                                .find(|variable| variable.name == #variable_name)
                                .expect("variable descriptor missing for nearby value distance meter");
                            variable_descriptor.nearby_value_distance_meter =
                                ::core::option::Option::Some(Self::#helper_ident);
                        }
                    });
                }
                if variable.nearby_entity_distance_meter.is_some() {
                    let helper_ident = format_ident!(
                        "__solverforge_descriptor_nearby_entity_distance_{}_{}",
                        field_name,
                        variable.field_name
                    );
                    attachments.push(quote! {
                        {
                            let entity_descriptor = descriptor
                                .entity_descriptors
                                .get_mut(#descriptor_index)
                                .expect("entity descriptor missing for nearby entity distance meter");
                            let variable_descriptor = entity_descriptor
                                .variable_descriptors
                                .iter_mut()
                                .find(|variable| variable.name == #variable_name)
                                .expect("variable descriptor missing for nearby entity distance meter");
                            variable_descriptor.nearby_entity_distance_meter =
                                ::core::option::Option::Some(Self::#helper_ident);
                        }
                    });
                }
                attachments
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
    let entity_count_arms: Vec<_> = fields
        .iter()
        .filter(|f| has_attribute(&f.attrs, "planning_entity_collection"))
        .enumerate()
        .map(|(idx, f)| {
            let field_name = f.ident.as_ref().unwrap();
            quote! { #idx => this.#field_name.len(), }
        })
        .collect();

    let list_operations = generate_list_operations(fields);
    let runtime_phase_support = generate_runtime_phase_support(fields, &constraints_path, name);
    let runtime_solve_internal =
        generate_runtime_solve_internal(&constraints_path, &config_path, &solver_toml_path);
    let solvable_solution_impl = generate_solvable_solution(name, &constraints_path);

    let stream_extensions = generate_constraint_stream_extensions(fields, name);

    let expanded = quote! {
        impl #impl_generics ::solverforge::__internal::PlanningSolution for #name #ty_generics #where_clause {
            type Score = #score_type;
            fn score(&self) -> Option<Self::Score> { self.#score_field_name.clone() }
            fn set_score(&mut self, score: Option<Self::Score>) { self.#score_field_name = score; }
            #shadow_support_impl
        }

        impl #impl_generics #name #ty_generics #where_clause {
            #(#scalar_descriptor_meter_helpers)*

            pub fn descriptor() -> ::solverforge::__internal::SolutionDescriptor {
                let mut descriptor = ::solverforge::__internal::SolutionDescriptor::new(
                    #name_str,
                    ::std::any::TypeId::of::<Self>(),
                )
                .with_score_field(#score_field_str)
                #(#entity_descriptors)*
                #(#fact_descriptors)*;
                #(#scalar_descriptor_meter_attachments)*
                descriptor
            }

            #[inline]
            pub fn entity_count(this: &Self, descriptor_index: usize) -> usize {
                match descriptor_index {
                    #(#entity_count_arms)*
                    _ => 0,
                }
            }

            #list_operations
            #runtime_solve_internal
        }

        #runtime_phase_support
        #solvable_solution_impl

        #stream_extensions
    };

    Ok(expanded)
}
