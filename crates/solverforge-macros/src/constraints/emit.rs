use quote::quote;
use std::collections::HashSet;

use super::ast::{ConstraintProgram, SharedGroupedProgram, TerminalKind, TerminalSource};

pub(crate) fn emit(program: ConstraintProgram) -> syn::Result<proc_macro2::TokenStream> {
    match program {
        ConstraintProgram::Passthrough(item) => {
            let item = *item;
            Ok(quote! { #item })
        }
        ConstraintProgram::SharedGrouped(program) => {
            let SharedGroupedProgram {
                item,
                prefix_statements,
                node,
                terminals,
                materialize_node,
            } = *program;
            let mut item = *item;
            let binding = &node.binding;
            let _node_id = &node.id;
            let node_expression = &node.expression;
            let _node_fingerprint = &node.fingerprint;
            let skip_bindings = redundant_terminal_bindings(&terminals, binding, materialize_node);
            let emitted_prefix_statements = prefix_statements
                .iter()
                .filter(|statement| !is_skipped_local(statement, &skip_bindings));
            let node_materialization = materialize_node.then(|| {
                quote! {
                    let #binding = #node_expression;
                }
            });
            let scorers = terminals.iter().map(|terminal| {
                debug_assert_eq!(terminal.kind, TerminalKind::GroupedScore);
                let helper = terminal.impact.helper_path();
                let weight = &terminal.weight;
                let name = &terminal.name;
                let _order = terminal.order;
                quote! {
                    #helper(#name, #weight)
                }
            });
            *item.block = syn::parse_quote!({
                #(#emitted_prefix_statements)*
                #node_materialization
                #binding.into_shared_constraint_set(
                    stringify!(#binding),
                    (#(#scorers,)*)
                )
            });
            Ok(quote! { #item })
        }
    }
}

fn redundant_terminal_bindings(
    terminals: &[super::ast::TerminalConstraint],
    selected: &syn::Ident,
    materialize_node: bool,
) -> HashSet<String> {
    if materialize_node {
        return HashSet::new();
    }
    terminals
        .iter()
        .filter_map(|terminal| match &terminal.source {
            TerminalSource::Binding(binding) if binding != selected => Some(binding.to_string()),
            _ => None,
        })
        .collect()
}

fn is_skipped_local(statement: &syn::Stmt, skip_bindings: &HashSet<String>) -> bool {
    let syn::Stmt::Local(local) = statement else {
        return false;
    };
    let syn::Pat::Ident(binding) = &local.pat else {
        return false;
    };
    skip_bindings.contains(&binding.ident.to_string())
}
