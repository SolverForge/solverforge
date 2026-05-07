mod lookup;
mod variable;

pub(crate) use lookup::scalar_work_remaining_with_frontier;
pub(crate) use lookup::{collect_bindings, find_binding, find_resolved_binding};
pub use lookup::{descriptor_has_bindings, scalar_target_matches, scalar_work_remaining};
pub(crate) use variable::{ResolvedVariableBinding, VariableBinding};
