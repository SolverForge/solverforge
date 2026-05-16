use std::collections::HashMap;

use super::ast::{ConstraintFunction, StreamNode};

pub(crate) fn normalize(function: ConstraintFunction) -> ConstraintFunction {
    function
}

pub(crate) fn nodes_by_binding(nodes: &[StreamNode]) -> HashMap<String, StreamNode> {
    nodes
        .iter()
        .map(|node| (node.binding.to_string(), node.clone()))
        .collect()
}
