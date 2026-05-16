use quote::{format_ident, quote};

use super::ast::{
    ConstraintFunction, ConstraintProgram, OtherMemberKind, SharedGroupedProgram, TailMember,
    TerminalConstraint, TerminalKind,
};

struct EmittedMember<'a> {
    terminal: Option<&'a TerminalConstraint>,
    set: proc_macro2::TokenStream,
    count: proc_macro2::TokenStream,
    preamble: proc_macro2::TokenStream,
}

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
    let emitted_members = tail_members
        .iter()
        .enumerate()
        .map(|(index, member)| emit_member(index, member))
        .collect::<Vec<_>>();
    let emitted_member_preambles = emitted_members.iter().map(|member| &member.preamble);
    let member_refs = emitted_members.iter().collect::<Vec<_>>();
    let combined = emit_member_set(&member_refs, &bindings);

    *item.block = syn::parse_quote!({
        #(#emitted_prefix_statements)*
        #(#emitted_member_preambles)*
        #combined
    });
    Ok(quote! { #item })
}

fn emit_member(index: usize, member: &TailMember) -> EmittedMember<'_> {
    match member {
        TailMember::Terminal(terminal) => EmittedMember {
            terminal: Some(terminal),
            set: emit_terminal(terminal),
            count: quote! { 1usize },
            preamble: quote! {},
        },
        TailMember::Other { tokens, kind } => {
            let member_ident = format_ident!("__solverforge_member_{index}");
            let count_ident = format_ident!("__solverforge_member_count_{index}");
            let set = match kind {
                OtherMemberKind::SingleConstraint => quote! { (#member_ident,) },
                OtherMemberKind::ConstraintSet => quote! { #member_ident },
            };
            let count = match kind {
                OtherMemberKind::SingleConstraint => quote! { 1usize },
                OtherMemberKind::ConstraintSet => quote! { #count_ident },
            };
            let count_statement = match kind {
                OtherMemberKind::SingleConstraint => quote! {},
                OtherMemberKind::ConstraintSet => {
                    quote! {
                        let #count_ident =
                            ::solverforge::__internal::ConstraintSet::constraint_count(&#member_ident);
                    }
                }
            };
            EmittedMember {
                terminal: None,
                set,
                count,
                preamble: quote! {
                    let #member_ident = #tokens;
                    #count_statement
                },
            }
        }
    }
}

fn emit_member_set(
    members: &[&EmittedMember<'_>],
    bindings: &[String],
) -> proc_macro2::TokenStream {
    let Some((binding, remaining_bindings)) = bindings.split_first() else {
        let sets = members.iter().map(|member| &member.set).collect::<Vec<_>>();
        return combine_constraint_sets(sets);
    };

    let left = emit_shared_binding_chain(members, binding);
    let mut right_members = Vec::new();
    let mut order = Vec::new();
    for &member in members {
        if member_matches_binding(member, binding) {
            order.push(quote! { ::solverforge::__internal::ConstraintSetSource::Left });
        } else {
            let count = &member.count;
            order.push(quote! { ::solverforge::__internal::ConstraintSetSource::Right(#count) });
            right_members.push(member);
        }
    }

    if right_members.is_empty() {
        return left;
    }

    let right = emit_member_set(&right_members, remaining_bindings);
    quote! {{
        let mut __solverforge_order = ::std::vec::Vec::new();
        #(__solverforge_order.push(#order);)*
        ::solverforge::__internal::OrderedConstraintSetChain::new(
            #left,
            #right,
            __solverforge_order,
        )
    }}
}

fn emit_shared_binding_chain(
    members: &[&EmittedMember<'_>],
    binding: &str,
) -> proc_macro2::TokenStream {
    let binding_ident = format_ident!("{binding}");
    members
        .iter()
        .filter_map(|member| member.terminal)
        .filter(|terminal| terminal.source_binding.as_str() == binding)
        .fold(quote! { #binding_ident }, |chain, terminal| {
            emit_terminal_chain(chain, terminal)
        })
}

fn member_matches_binding(member: &EmittedMember<'_>, binding: &str) -> bool {
    matches!(member.terminal, Some(terminal) if terminal.source_binding.as_str() == binding)
}

fn combine_constraint_sets(sets: Vec<&proc_macro2::TokenStream>) -> proc_macro2::TokenStream {
    let mut sets = sets.into_iter();
    let Some(combined) = sets.next() else {
        return quote! { () };
    };
    let mut combined = quote! { #combined };
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
