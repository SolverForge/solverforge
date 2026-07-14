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

    Ok(quote! {
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
    })
}
