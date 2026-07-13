//! Declaration-only transfer from typed model authoring into graph compilation.
//!
//! A `RuntimeGraphInput` owns exactly one descriptor-resolved search context
//! and one concrete extension registry. It contains no solve input, source
//! binding, phase instance, cursor, callback view, or cache entry.

use solverforge_core::domain::PlanningSolution;

use crate::builder::SearchContext;

/// One typed or dynamic runtime declaration ready for immutable compilation.
///
/// This replaces the old phase-builder closure as the handoff from model
/// authoring. The compiler consumes it once, freezes structural semantics, and
/// leaves per-solve binding to [`super::CompiledRuntimeExecutor`].
pub(crate) struct RuntimeGraphInput<S, V, DM, IDM, E>
where
    S: PlanningSolution,
{
    context: SearchContext<S, V, DM, IDM>,
    extensions: E,
}

impl<S, V, DM, IDM, E> RuntimeGraphInput<S, V, DM, IDM, E>
where
    S: PlanningSolution,
{
    pub(crate) fn new(context: SearchContext<S, V, DM, IDM>, extensions: E) -> Self {
        Self {
            context,
            extensions,
        }
    }

    pub(crate) fn into_parts(self) -> (SearchContext<S, V, DM, IDM>, E) {
        (self.context, self.extensions)
    }
}
