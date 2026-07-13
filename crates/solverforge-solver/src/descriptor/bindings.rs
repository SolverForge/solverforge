mod lookup;
mod variable;

pub use lookup::descriptor_has_bindings;
pub(crate) use lookup::{collect_bindings, find_binding};
pub(crate) use variable::{ResolvedVariableBinding, VariableBinding};
