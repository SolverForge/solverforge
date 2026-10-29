pub(crate) mod lookup;
mod variable;

pub(crate) use lookup::collect_bindings;
pub(crate) use variable::ResolvedVariableBinding;
