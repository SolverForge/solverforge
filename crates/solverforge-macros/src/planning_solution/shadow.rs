use proc_macro2::TokenStream;
use quote::quote;
use syn::{Error, Ident};

use super::config::ShadowConfig;

fn shadow_updates_requested(config: &ShadowConfig) -> bool {
    config.inverse_field.is_some()
        || config.index_field.is_some()
        || config.previous_field.is_some()
        || config.next_field.is_some()
        || config.cascading_listener.is_some()
        || config.cascading_field.is_some()
        || config.post_update_listener.is_some()
        || !config.entity_aggregates.is_empty()
        || !config.entity_computes.is_empty()
}

pub(super) fn generate_shadow_support(
    config: &ShadowConfig,
    _fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
    _solution_name: &Ident,
) -> Result<TokenStream, Error> {
    if !shadow_updates_requested(config) {
        return Ok(TokenStream::new());
    }

    if config.list_owner.is_none() {
        return Err(Error::new(
            proc_macro2::Span::call_site(),
            "#[shadow_variable_updates(...)] requires `list_owner = \"entity_collection_field\"` when shadow updates are configured",
        ));
    }

    let cascading = if config.cascading_field.is_some() {
        quote! {
            const HAS_CASCADING_SHADOWS: bool = true;

            #[inline]
            fn cascading_element_count(&self) -> usize {
                <Self as ::solverforge::__internal::PlanningModelSupport>::cascading_element_count(self)
            }

            #[inline]
            fn cascading_assigned(&self, element: usize) -> bool {
                <Self as ::solverforge::__internal::PlanningModelSupport>::cascading_assigned(self, element)
            }

            #[inline]
            fn cascading_next(&self, element: usize) -> ::core::option::Option<usize> {
                <Self as ::solverforge::__internal::PlanningModelSupport>::cascading_next(self, element)
            }

            #[inline]
            fn cascading_dependencies(
                &self,
                element: usize,
                dependencies: &mut ::std::vec::Vec<usize>,
            ) {
                <Self as ::solverforge::__internal::PlanningModelSupport>::cascading_dependencies(
                    self,
                    element,
                    dependencies,
                )
            }

            #[inline]
            fn cascading_unset(&self, element: usize) -> bool {
                <Self as ::solverforge::__internal::PlanningModelSupport>::cascading_unset(self, element)
            }

            #[inline]
            fn update_cascading_shadow<W: ::core::ops::FnOnce(&Self)>(
                &mut self,
                element: usize,
                computable: bool,
                before_write: W,
            ) -> bool {
                <Self as ::solverforge::__internal::PlanningModelSupport>::update_cascading_shadow(
                    self,
                    element,
                    computable,
                    before_write,
                )
            }

            #[inline]
            fn update_list_structure_shadows(&mut self, descriptor_index: usize, entity_idx: usize) {
                let _ = <Self as ::solverforge::__internal::PlanningModelSupport>::update_list_structure_shadows(
                    self,
                    descriptor_index,
                    entity_idx,
                );
            }
        }
    } else {
        TokenStream::new()
    };

    Ok(quote! {
        #cascading

        #[inline]
        fn update_entity_shadows(&mut self, descriptor_index: usize, entity_idx: usize) {
            let _ = <Self as ::solverforge::__internal::PlanningModelSupport>::update_entity_shadows(
                self,
                descriptor_index,
                entity_idx,
            );
        }

        #[inline]
        fn update_all_shadows(&mut self) {
            let _ = <Self as ::solverforge::__internal::PlanningModelSupport>::update_all_shadows(
                self,
            );
        }

        #[inline]
        fn list_shadow_elements(
            &self,
            descriptor_index: usize,
            entity_idx: usize,
        ) -> ::core::option::Option<(usize, &[usize])> {
            <Self as ::solverforge::__internal::PlanningModelSupport>::list_shadow_elements(
                self,
                descriptor_index,
                entity_idx,
            )
        }

        #[inline]
        fn clear_list_shadows(&mut self, element: usize) {
            <Self as ::solverforge::__internal::PlanningModelSupport>::clear_list_shadows(
                self,
                element,
            );
        }

        #[inline]
        fn list_shadow_source(collection: &str) -> ::core::option::Option<usize> {
            <Self as ::solverforge::__internal::PlanningModelSupport>::list_shadow_source(collection)
        }
    })
}
