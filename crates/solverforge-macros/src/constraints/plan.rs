use super::ast::{
    ConstraintFunction, ConstraintProgram, SharedGroupedProgram, StreamNode, TailMember,
};

pub(crate) fn plan(function: ConstraintFunction) -> ConstraintProgram {
    let bindings = repeated_grouped_bindings(&function);
    if bindings.is_empty() {
        return ConstraintProgram::Passthrough(function);
    }

    ConstraintProgram::SharedGrouped(SharedGroupedProgram {
        item: function.item,
        prefix_statements: function.prefix_statements,
        tail_members: function.tail_members,
        bindings,
    })
}

fn repeated_grouped_bindings(function: &ConstraintFunction) -> Vec<String> {
    let mut bindings = Vec::new();
    for member in &function.tail_members {
        let TailMember::Terminal(candidate) = member else {
            continue;
        };
        let binding = &candidate.source_binding;
        if bindings.iter().any(|existing| existing == binding) {
            continue;
        }
        let Some(node) = node_by_binding(&function.stream_nodes, binding) else {
            continue;
        };
        if !node.supports_grouped_sharing {
            continue;
        }
        let count = function
            .tail_members
            .iter()
            .filter(|member| {
                matches!(member, TailMember::Terminal(terminal) if &terminal.source_binding == binding)
            })
            .count();
        if count >= 2 {
            bindings.push(binding.to_string());
        }
    }
    bindings
}

fn node_by_binding<'a>(nodes: &'a [StreamNode], binding: &str) -> Option<&'a StreamNode> {
    nodes.iter().rev().find(|node| node.binding == binding)
}
