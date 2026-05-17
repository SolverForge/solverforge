use quote::quote;
use syn::{Expr, ExprMethodCall, ExprPath, ItemFn, Stmt};

use super::ast::{
    ConstraintFunction, ImpactKind, StreamNode, TailMember, TerminalConstraint, TerminalKind,
};

pub(crate) fn parse_constraint_function(mut item: ItemFn) -> syn::Result<ConstraintFunction> {
    let statements = std::mem::take(&mut item.block.stmts);
    let mut prefix_statements = Vec::new();
    let mut stream_nodes = Vec::new();
    let mut original_tail = None;

    let statement_count = statements.len();
    for (statement_index, statement) in statements.into_iter().enumerate() {
        match statement {
            Stmt::Local(local) => {
                if let Some(node) = stream_let(&local) {
                    stream_nodes.push(node);
                }
                prefix_statements.push(Stmt::Local(local));
            }
            Stmt::Expr(expr, None) if statement_index + 1 == statement_count => {
                original_tail = Some(expr);
            }
            Stmt::Expr(expr, None) => {
                prefix_statements.push(Stmt::Expr(expr, None));
            }
            Stmt::Expr(expr, semicolon) => {
                prefix_statements.push(Stmt::Expr(expr, semicolon));
            }
            Stmt::Item(item) => {
                prefix_statements.push(Stmt::Item(item));
            }
            Stmt::Macro(item) => {
                prefix_statements.push(Stmt::Macro(item));
            }
        }
    }

    let tail_members = original_tail
        .as_ref()
        .map(parse_tail_members)
        .transpose()?
        .unwrap_or_default();

    Ok(ConstraintFunction {
        item,
        prefix_statements,
        tail: original_tail,
        stream_nodes,
        tail_members,
    })
}

fn stream_let(local: &syn::Local) -> Option<StreamNode> {
    let syn::Pat::Ident(binding) = &local.pat else {
        return None;
    };
    let init = local.init.as_ref()?;
    Some(StreamNode {
        binding: binding.ident.to_string(),
        supports_grouped_sharing: source_supports_grouped_sharing(&init.expr),
    })
}

fn parse_tail_members(expr: &Expr) -> syn::Result<Vec<TailMember>> {
    match expr {
        Expr::Tuple(tuple) => {
            let mut members = Vec::new();
            for (order, expr) in tuple.elems.iter().enumerate() {
                if let Some(terminal) = parse_terminal(expr, order)? {
                    members.push(TailMember::Terminal(terminal));
                } else {
                    members.push(other_tail_member(expr));
                }
            }
            Ok(members)
        }
        _ => {
            let members = parse_terminal(expr, 0)?
                .map(TailMember::Terminal)
                .into_iter()
                .collect();
            Ok(members)
        }
    }
}

fn other_tail_member(expr: &Expr) -> TailMember {
    TailMember::Other {
        tokens: quote! { #expr },
    }
}

fn parse_terminal(expr: &Expr, order: usize) -> syn::Result<Option<TerminalConstraint>> {
    let Expr::MethodCall(named) = expr else {
        return Ok(None);
    };
    if named.method != "named" {
        return Ok(None);
    }
    if named.args.len() != 1 {
        return Err(syn::Error::new_spanned(
            named,
            "terminal constraints inside #[solverforge_constraints] must use exactly one .named(\"...\") argument",
        ));
    }
    let name = named
        .args
        .first()
        .expect("named arity was checked before reading the name expression");

    let Expr::MethodCall(score) = named.receiver.as_ref() else {
        return Ok(None);
    };
    let impact = match score.method.to_string().as_str() {
        "penalize" => ImpactKind::Penalty,
        "reward" => ImpactKind::Reward,
        _ => return Ok(None),
    };
    if score.args.len() != 1 {
        return Err(syn::Error::new_spanned(
            score,
            "penalize/reward terminals inside #[solverforge_constraints] must use exactly one weight argument",
        ));
    }
    let Some(weight) = score.args.first() else {
        return Err(syn::Error::new_spanned(
            score,
            "penalize/reward terminal is missing a weight expression",
        ));
    };
    let Some(source_binding) = receiver_binding(score) else {
        return Ok(None);
    };
    let name = quote! { #name };
    let weight = quote! { #weight };

    Ok(Some(TerminalConstraint {
        source_binding,
        impact,
        weight,
        name,
        order,
        kind: TerminalKind::GroupedScore,
    }))
}

fn receiver_binding(method: &ExprMethodCall) -> Option<String> {
    match method.receiver.as_ref() {
        Expr::Path(ExprPath { path, .. }) if path.segments.len() == 1 => {
            Some(path.segments[0].ident.to_string())
        }
        _ => None,
    }
}

fn source_supports_grouped_sharing(expression: &Expr) -> bool {
    let Expr::MethodCall(method) = expression else {
        return false;
    };
    if method.method == "group_by" {
        return true;
    }
    if method.method == "complement" || method.method == "complement_with_key" {
        return complemented_source_supports_grouped_sharing(method.receiver.as_ref());
    }
    false
}

fn complemented_source_supports_grouped_sharing(expression: &Expr) -> bool {
    let mut current = expression;
    loop {
        let Expr::MethodCall(method) = current else {
            return false;
        };
        if method.method == "group_by" {
            return chain_contains_project_or_join(method.receiver.as_ref());
        }
        current = method.receiver.as_ref();
    }
}

fn chain_contains_project_or_join(expression: &Expr) -> bool {
    let mut current = expression;
    loop {
        let Expr::MethodCall(method) = current else {
            return false;
        };
        if method.method == "project" || method.method == "join" {
            return true;
        }
        current = method.receiver.as_ref();
    }
}
