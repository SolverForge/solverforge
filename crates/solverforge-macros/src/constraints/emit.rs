use quote::quote;

use super::ast::{ConstraintProgram, TerminalKind};

pub(crate) fn emit(program: ConstraintProgram) -> syn::Result<proc_macro2::TokenStream> {
    match program {
        ConstraintProgram::Passthrough(item) => Ok(quote! { #item }),
        ConstraintProgram::SharedGrouped {
            mut item,
            prefix_statements,
            node,
            terminals,
        } => {
            let binding = &node.binding;
            let _node_id = &node.id;
            let _node_expression = &node.expression;
            let _node_fingerprint = &node.fingerprint;
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
            item.block = Box::new(syn::parse_quote!({
                #(#prefix_statements)*
                #binding.into_shared_constraint_set(
                    stringify!(#binding),
                    (#(#scorers,)*)
                )
            }));
            Ok(quote! { #item })
        }
    }
}
