//! Debug rendering for the frozen runtime list carrier.

use std::fmt;

use super::RuntimeListSlot;

impl<S, V, DM: fmt::Debug, IDM: fmt::Debug> fmt::Debug for RuntimeListSlot<S, V, DM, IDM> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Static {
                slot,
                variable_index,
                route_bindings,
                metadata_bindings,
            } => f
                .debug_struct("RuntimeListSlot::Static")
                .field("slot", slot)
                .field("variable_index", variable_index)
                .field("route_read_policy", &self.route_read_policy().trace_label())
                .field(
                    "route_replace_policy",
                    &self.route_replace_policy().trace_label(),
                )
                .field(
                    "route_feasibility_policy",
                    &self.route_feasibility_policy().trace_label(),
                )
                .field(
                    "savings_metric_class_policy",
                    &route_bindings.metric_class_policy.trace_label(),
                )
                .field(
                    "ownership_policy",
                    &metadata_bindings.ownership_policy.trace_label(),
                )
                .field(
                    "construction_order_policy",
                    &metadata_bindings.construction_order_policy.trace_label(),
                )
                .field(
                    "precedence_policy",
                    &metadata_bindings.precedence_policy.trace_label(),
                )
                .finish(),
            Self::Dynamic(slot) => f
                .debug_struct("RuntimeListSlot::Dynamic")
                .field("slot", slot)
                .field("route_read_policy", &self.route_read_policy().trace_label())
                .field(
                    "route_replace_policy",
                    &self.route_replace_policy().trace_label(),
                )
                .field(
                    "route_feasibility_policy",
                    &self.route_feasibility_policy().trace_label(),
                )
                .field(
                    "savings_metric_class_policy",
                    &self.savings_metric_class_policy().trace_label(),
                )
                .field("ownership_policy", &self.ownership_policy().trace_label())
                .field(
                    "construction_order_policy",
                    &self.construction_order_policy().trace_label(),
                )
                .field("precedence_policy", &self.precedence_policy().trace_label())
                .finish(),
        }
    }
}
