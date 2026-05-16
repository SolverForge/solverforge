use syn::{Expr, ExprMethodCall, ExprPath, Ident, ItemFn, Stmt};

use super::ast::{
    ConstraintFunction, ImpactKind, NodeId, StreamNode, TerminalConstraint, TerminalKind,
    TerminalSource,
};
use super::fingerprint::expression_fingerprint;

pub(crate) fn parse_constraint_function(item: ItemFn) -> syn::Result<ConstraintFunction> {
    let mut prefix_statements = Vec::new();
    let mut stream_nodes = Vec::new();
    let mut original_tail = None;

    for statement in &item.block.stmts {
        match statement {
            Stmt::Local(local) => {
                if let Some((ident, expression)) = stream_let(local) {
                    let id = NodeId(stream_nodes.len());
                    stream_nodes.push(StreamNode {
                        id,
                        binding: ident.clone(),
                        expression: expression.clone(),
                        fingerprint: expression_fingerprint(expression),
                    });
                }
                prefix_statements.push(statement.clone());
            }
            Stmt::Expr(expr, None) => {
                original_tail = Some(expr.clone());
            }
            Stmt::Expr(_, Some(_)) | Stmt::Item(_) | Stmt::Macro(_) => {
                prefix_statements.push(statement.clone());
            }
        }
    }

    let terminals = original_tail
        .as_ref()
        .map(parse_tail_terminals)
        .transpose()?
        .unwrap_or_default();

    Ok(ConstraintFunction {
        item,
        prefix_statements,
        stream_nodes,
        terminals,
    })
}

fn stream_let(local: &syn::Local) -> Option<(Ident, &Expr)> {
    let syn::Pat::Ident(binding) = &local.pat else {
        return None;
    };
    let init = local.init.as_ref()?;
    Some((binding.ident.clone(), &init.expr))
}

fn parse_tail_terminals(expr: &Expr) -> syn::Result<Vec<TerminalConstraint>> {
    match expr {
        Expr::Tuple(tuple) => tuple
            .elems
            .iter()
            .enumerate()
            .filter_map(|(order, expr)| parse_terminal(expr, order).transpose())
            .collect(),
        _ => parse_terminal(expr, 0).map(|terminal| terminal.into_iter().collect()),
    }
}

fn parse_terminal(expr: &Expr, order: usize) -> syn::Result<Option<TerminalConstraint>> {
    let Expr::MethodCall(named) = expr else {
        return Ok(None);
    };
    if named.method != "named" {
        return Ok(None);
    }
    let Some(name) = named.args.first().and_then(lit_str_arg) else {
        return Err(syn::Error::new_spanned(
            named,
            "terminal constraints inside #[solverforge_constraints] must use .named(\"...\")",
        ));
    };

    let Expr::MethodCall(score) = named.receiver.as_ref() else {
        return Ok(None);
    };
    let impact = match score.method.to_string().as_str() {
        "penalize" => ImpactKind::Penalty,
        "reward" => ImpactKind::Reward,
        _ => return Ok(None),
    };
    let Some(weight) = score.args.first().cloned() else {
        return Err(syn::Error::new_spanned(
            score,
            "penalize/reward terminal is missing a weight expression",
        ));
    };
    let source = receiver_source(score);

    Ok(Some(TerminalConstraint {
        source,
        impact,
        weight,
        name,
        order,
        kind: TerminalKind::GroupedScore,
    }))
}

fn receiver_source(method: &ExprMethodCall) -> TerminalSource {
    match method.receiver.as_ref() {
        Expr::Path(ExprPath { path, .. }) if path.segments.len() == 1 => {
            TerminalSource::Binding(path.segments[0].ident.clone())
        }
        expression => TerminalSource::Inline {
            expression: expression.clone(),
            fingerprint: expression_fingerprint(expression),
        },
    }
}

fn lit_str_arg(expr: &Expr) -> Option<syn::LitStr> {
    let Expr::Lit(expr_lit) = expr else {
        return None;
    };
    let syn::Lit::Str(lit) = &expr_lit.lit else {
        return None;
    };
    Some(lit.clone())
}
