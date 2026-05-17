use quote::{format_ident, quote};
use syn::{GenericArgument, PathArguments, ReturnType, Type, TypeParamBound};

use super::ast::{
    ConstraintFunction, ConstraintProgram, SharedGroupedProgram, TailMember, TerminalConstraint,
    TerminalKind,
};

struct EmittedMember<'a> {
    terminal: Option<&'a TerminalConstraint>,
    set: proc_macro2::TokenStream,
    count: proc_macro2::TokenStream,
    metadata_entry_count: proc_macro2::TokenStream,
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
    let return_types = has_other_member(&tail_members)
        .then(|| constraint_set_return_types(&item.sig.output))
        .transpose()?;
    let emitted_prefix_statements = prefix_statements.iter();
    let (emitted_members, emitted_member_preambles) =
        emit_members(&tail_members, &bindings, return_types.as_ref());
    let member_refs = emitted_members.iter().collect::<Vec<_>>();
    let combined = emit_member_set(&member_refs, &bindings, &bindings);

    *item.block = syn::parse_quote!({
        #(#emitted_prefix_statements)*
        #(#emitted_member_preambles)*
        #combined
    });
    Ok(quote! { #item })
}

fn has_other_member(tail_members: &[TailMember]) -> bool {
    tail_members
        .iter()
        .any(|member| matches!(member, TailMember::Other { .. }))
}

fn emit_members<'a>(
    tail_members: &'a [TailMember],
    bindings: &[String],
    return_types: Option<&(Type, Type)>,
) -> (Vec<EmittedMember<'a>>, Vec<proc_macro2::TokenStream>) {
    let mut initialized_shared_bindings = Vec::new();
    let mut emitted_members = Vec::new();
    let mut preambles = Vec::new();

    for (index, member) in tail_members.iter().enumerate() {
        match member {
            TailMember::Terminal(terminal) => {
                let (member, preamble) = emit_terminal_member(
                    index,
                    terminal,
                    bindings,
                    &mut initialized_shared_bindings,
                );
                emitted_members.push(member);
                preambles.push(preamble);
            }
            TailMember::Other { tokens } => {
                let (member, preamble) = emit_other_member(index, tokens, return_types);
                emitted_members.push(member);
                preambles.push(preamble);
            }
        }
    }

    (emitted_members, preambles)
}

fn emit_terminal_member<'a>(
    index: usize,
    terminal: &'a TerminalConstraint,
    bindings: &[String],
    initialized_shared_bindings: &mut Vec<String>,
) -> (EmittedMember<'a>, proc_macro2::TokenStream) {
    let Some(shared_index) = shared_binding_index(bindings, &terminal.source_binding) else {
        let member_ident = format_ident!("__solverforge_member_{index}");
        let terminal_set = emit_terminal(terminal);
        return (
            EmittedMember {
                terminal: Some(terminal),
                set: quote! { #member_ident },
                count: quote! { 1usize },
                metadata_entry_count: quote! { 1usize },
            },
            quote! {
                let #member_ident = #terminal_set;
            },
        );
    };

    let shared_ident = shared_binding_ident(shared_index);
    let chain_source = if initialized_shared_bindings
        .iter()
        .any(|binding| binding == &terminal.source_binding)
    {
        quote! { #shared_ident }
    } else {
        initialized_shared_bindings.push(terminal.source_binding.clone());
        let binding_ident = format_ident!("{}", terminal.source_binding);
        quote! { #binding_ident }
    };
    let terminal_chain = emit_terminal_chain(chain_source, terminal);

    (
        EmittedMember {
            terminal: Some(terminal),
            set: quote! { #shared_ident },
            count: quote! { 1usize },
            metadata_entry_count: quote! { 1usize },
        },
        quote! {
            let #shared_ident = #terminal_chain;
        },
    )
}

fn emit_other_member<'a>(
    index: usize,
    tokens: &proc_macro2::TokenStream,
    return_types: Option<&(Type, Type)>,
) -> (EmittedMember<'a>, proc_macro2::TokenStream) {
    let (solution_type, score_type) =
        return_types.expect("opaque constraint members require parsed return ConstraintSet types");
    let member_ident = format_ident!("__solverforge_member_{index}");
    let count_ident = format_ident!("__solverforge_member_count_{index}");
    let metadata_entry_count_ident =
        format_ident!("__solverforge_member_metadata_entry_count_{index}");
    (
        EmittedMember {
            terminal: None,
            set: quote! { #member_ident },
            count: quote! { #count_ident },
            metadata_entry_count: quote! { #metadata_entry_count_ident },
        },
        quote! {
            let #member_ident = #tokens;
            let #count_ident =
                <_ as ::solverforge::__internal::ConstraintSet<#solution_type, #score_type>>::constraint_count(&#member_ident);
            let #metadata_entry_count_ident =
                <_ as ::solverforge::__internal::ConstraintSet<#solution_type, #score_type>>::constraint_metadata_entries(&#member_ident).len();
        },
    )
}

fn emit_member_set(
    members: &[&EmittedMember<'_>],
    bindings: &[String],
    all_bindings: &[String],
) -> proc_macro2::TokenStream {
    let Some((binding, remaining_bindings)) = bindings.split_first() else {
        let sets = members.iter().map(|member| &member.set).collect::<Vec<_>>();
        return combine_constraint_sets(sets);
    };

    let left = emit_shared_binding_set(binding, all_bindings);
    let mut right_members = Vec::new();
    let mut order = Vec::new();
    for &member in members {
        if member_matches_binding(member, binding) {
            order.push(quote! { ::solverforge::__internal::ConstraintSetSource::Left });
        } else {
            let count = &member.count;
            let metadata_entry_count = &member.metadata_entry_count;
            order.push(quote! {
                ::solverforge::__internal::ConstraintSetSource::Right {
                    constraint_count: #count,
                    metadata_entry_count: #metadata_entry_count,
                }
            });
            right_members.push(member);
        }
    }

    if right_members.is_empty() {
        return left;
    }

    let right = emit_member_set(&right_members, remaining_bindings, all_bindings);
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

fn emit_shared_binding_set(binding: &str, all_bindings: &[String]) -> proc_macro2::TokenStream {
    let shared_index = shared_binding_index(all_bindings, binding)
        .expect("shared binding should be present in the planned binding list");
    let shared_ident = shared_binding_ident(shared_index);
    quote! { #shared_ident }
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

fn shared_binding_index(bindings: &[String], binding: &str) -> Option<usize> {
    bindings.iter().position(|candidate| candidate == binding)
}

fn shared_binding_ident(index: usize) -> proc_macro2::Ident {
    format_ident!("__solverforge_shared_{index}")
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

fn constraint_set_return_types(output: &ReturnType) -> syn::Result<(Type, Type)> {
    let ReturnType::Type(_, return_type) = output else {
        return Err(syn::Error::new_spanned(
            quote! { #output },
            "#[solverforge_constraints] shared functions must return impl ConstraintSet<S, Sc>",
        ));
    };
    let return_type = peel_parens(return_type.as_ref());
    let Type::ImplTrait(impl_trait) = return_type else {
        return Err(syn::Error::new_spanned(
            return_type,
            "#[solverforge_constraints] shared functions must return impl ConstraintSet<S, Sc>",
        ));
    };

    let mut fallback = None;
    for bound in &impl_trait.bounds {
        let Some((types, is_constraint_set)) = constraint_set_bound_types(bound) else {
            continue;
        };
        if is_constraint_set {
            return Ok(types);
        }
        fallback.get_or_insert(types);
    }

    fallback.ok_or_else(|| {
        syn::Error::new_spanned(
            return_type,
            "#[solverforge_constraints] shared functions must return impl ConstraintSet<S, Sc>",
        )
    })
}

fn peel_parens(ty: &Type) -> &Type {
    match ty {
        Type::Paren(paren) => peel_parens(paren.elem.as_ref()),
        _ => ty,
    }
}

fn constraint_set_bound_types(bound: &TypeParamBound) -> Option<((Type, Type), bool)> {
    let TypeParamBound::Trait(trait_bound) = bound else {
        return None;
    };
    let segment = trait_bound.path.segments.last()?;
    let PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return None;
    };

    let mut type_arguments = arguments.args.iter().filter_map(|argument| match argument {
        GenericArgument::Type(ty) => Some(ty.clone()),
        _ => None,
    });
    let solution_type = type_arguments.next()?;
    let score_type = type_arguments.next()?;
    Some((
        (solution_type, score_type),
        segment.ident == "ConstraintSet",
    ))
}
