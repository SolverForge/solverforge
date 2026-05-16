use super::ast::{ConstraintFunction, ConstraintProgram};
use super::normalize::nodes_by_binding;

pub(crate) fn plan(function: ConstraintFunction) -> ConstraintProgram {
    let Some(first_terminal) = function.terminals.first() else {
        return ConstraintProgram::Passthrough(function.item);
    };
    if function.terminals.len() < 2 {
        return ConstraintProgram::Passthrough(function.item);
    }
    if !function
        .terminals
        .iter()
        .all(|terminal| terminal.stream_binding == first_terminal.stream_binding)
    {
        return ConstraintProgram::Passthrough(function.item);
    }

    let nodes = nodes_by_binding(&function.stream_nodes);
    let Some(node) = nodes
        .get(&first_terminal.stream_binding.to_string())
        .cloned()
    else {
        return ConstraintProgram::Passthrough(function.item);
    };

    ConstraintProgram::SharedGrouped {
        item: function.item,
        prefix_statements: function.prefix_statements,
        node,
        terminals: function.terminals,
    }
}
