use quote::format_ident;

use super::ast::{ConstraintFunction, ConstraintProgram, StreamNode, TerminalSource};
use super::normalize::nodes_by_binding;

pub(crate) fn plan(function: ConstraintFunction) -> ConstraintProgram {
    let Some(first_terminal) = function.terminals.first() else {
        return ConstraintProgram::Passthrough(function.item);
    };
    if function.terminals.len() < 2 {
        return ConstraintProgram::Passthrough(function.item);
    }
    let nodes = nodes_by_binding(&function.stream_nodes);
    let Some(first_key) = terminal_source_key(&first_terminal.source, &nodes) else {
        return ConstraintProgram::Passthrough(function.item);
    };

    if !function
        .terminals
        .iter()
        .all(|terminal| terminal_source_key(&terminal.source, &nodes).as_ref() == Some(&first_key))
    {
        return ConstraintProgram::Passthrough(function.item);
    }

    let (node, materialize_node) = match &first_terminal.source {
        TerminalSource::Binding(binding) => {
            let Some(node) = nodes.get(&binding.to_string()).cloned() else {
                return ConstraintProgram::Passthrough(function.item);
            };
            (node, false)
        }
        TerminalSource::Inline {
            expression,
            fingerprint,
        } => (
            StreamNode {
                id: super::ast::NodeId(function.stream_nodes.len()),
                binding: format_ident!("__solverforge_shared_node_0"),
                expression: expression.clone(),
                fingerprint: fingerprint.clone(),
            },
            true,
        ),
    };

    ConstraintProgram::SharedGrouped {
        item: function.item,
        prefix_statements: function.prefix_statements,
        node,
        terminals: function.terminals,
        materialize_node,
    }
}

fn terminal_source_key(
    source: &TerminalSource,
    nodes: &std::collections::HashMap<String, StreamNode>,
) -> Option<String> {
    match source {
        TerminalSource::Binding(binding) => nodes
            .get(&binding.to_string())
            .map(|node| format!("node:{}", node.fingerprint)),
        TerminalSource::Inline { fingerprint, .. } => Some(format!("inline:{fingerprint}")),
    }
}
