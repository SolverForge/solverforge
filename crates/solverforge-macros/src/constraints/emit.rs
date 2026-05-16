use quote::{format_ident, quote};

use super::ast::{
    ConstraintFunction, ConstraintProgram, SharedGroupedProgram, TailMember, TerminalConstraint,
    TerminalKind,
};

pub(crate) fn emit(program: ConstraintProgram) -> syn::Result<proc_macro2::TokenStream> {
    match program {
        ConstraintProgram::Passthrough(function) => emit_passthrough(function),
        ConstraintProgram::SharedGrouped(program) => emit_shared_grouped(program),
    }
}

fn emit_passthrough(function: ConstraintFunction) -> syn::Result<proc_macro2::TokenStream> {
    let ConstraintFunction {
        mut item,
        prefix_statements,
        tail,
        ..
    } = function;
    let emitted_prefix_statements = prefix_statements.iter();
    if let Some(tail) = tail.as_ref() {
        *item.block = syn::parse_quote!({
            #(#emitted_prefix_statements)*
            #tail
        });
    } else {
        *item.block = syn::parse_quote!({
            #(#emitted_prefix_statements)*
        });
    }
    Ok(quote! { #item })
}

fn emit_shared_grouped(program: SharedGroupedProgram) -> syn::Result<proc_macro2::TokenStream> {
    let SharedGroupedProgram {
        mut item,
        prefix_statements,
        bindings,
        tail_members,
    } = program;
    let emitted_prefix_statements = prefix_statements.iter();
    let member_refs = tail_members.iter().collect::<Vec<_>>();
    let combined = emit_member_set(&member_refs, &bindings);

    *item.block = syn::parse_quote!({
        #(#emitted_prefix_statements)*
        #combined
    });
    Ok(quote! { #item })
}

fn emit_member_set(members: &[&TailMember], bindings: &[String]) -> proc_macro2::TokenStream {
    let Some((binding, remaining_bindings)) = bindings.split_first() else {
        let mut sets = Vec::new();
        for &member in members {
            match member {
                TailMember::Terminal(terminal) => {
                    let terminal = emit_terminal(terminal);
                    sets.push(quote! { (#terminal,) });
                }
                TailMember::Other(tokens) => {
                    sets.push(quote! { (#tokens,) });
                }
            }
        }
        return combine_constraint_sets(sets);
    };

    let left = emit_shared_binding_chain(members, binding);
    let mut right_members = Vec::new();
    let mut order = Vec::new();
    for &member in members {
        if member_matches_binding(member, binding) {
            order.push(quote! { ::solverforge::__internal::ConstraintSetSource::Left });
        } else {
            order.push(quote! { ::solverforge::__internal::ConstraintSetSource::Right });
            right_members.push(member);
        }
    }

    if right_members.is_empty() {
        return left;
    }

    let right = emit_member_set(&right_members, remaining_bindings);
    quote! {
        ::solverforge::__internal::OrderedConstraintSetChain::new(
            #left,
            #right,
            &[#(#order),*],
        )
    }
}

fn emit_shared_binding_chain(members: &[&TailMember], binding: &str) -> proc_macro2::TokenStream {
    let binding_ident = format_ident!("{binding}");
    members
        .iter()
        .filter_map(|&member| match member {
            TailMember::Terminal(terminal) if terminal.source_binding.as_str() == binding => {
                Some(terminal)
            }
            _ => None,
        })
        .fold(quote! { #binding_ident }, |chain, terminal| {
            emit_terminal_chain(chain, terminal)
        })
}

fn member_matches_binding(member: &TailMember, binding: &str) -> bool {
    matches!(member, TailMember::Terminal(terminal) if terminal.source_binding.as_str() == binding)
}

fn combine_constraint_sets(sets: Vec<proc_macro2::TokenStream>) -> proc_macro2::TokenStream {
    let mut sets = sets.into_iter();
    let Some(mut combined) = sets.next() else {
        return quote! { () };
    };
    for set in sets {
        combined = quote! {
            ::solverforge::__internal::ConstraintSetChain::new(#combined, #set)
        };
    }
    combined
}

fn emit_terminal(terminal: &TerminalConstraint) -> proc_macro2::TokenStream {
    let binding = format_ident!("{}", terminal.source_binding);
    emit_terminal_chain(quote! { #binding }, terminal)
}

fn emit_terminal_chain(
    chain: proc_macro2::TokenStream,
    terminal: &TerminalConstraint,
) -> proc_macro2::TokenStream {
    debug_assert_eq!(terminal.kind, TerminalKind::GroupedScore);
    let method = terminal.impact.method_ident();
    let weight = &terminal.weight;
    let name = &terminal.name;
    let _order = terminal.order;
    quote! {
        #chain.#method(#weight).named(#name)
    }
}
