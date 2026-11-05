//! Declaration-only transfer from typed model authoring into graph compilation.
//!
//! A `RuntimeGraphInput` owns exactly one descriptor-resolved search context
//! and one concrete extension registry. It contains no solve input, source
//! binding, phase instance, cursor, callback view, or cache entry.

use solverforge_core::domain::PlanningSolution;

use crate::builder::{RuntimeExtensionRegistry, SearchContext};

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

    pub(crate) fn context(&self) -> &SearchContext<S, V, DM, IDM> {
        &self.context
    }

    pub(crate) fn extensions(&self) -> &E {
        &self.extensions
    }
}

/// How a compiled graph and its runner hold their declaration: the top-level
/// solve owns its [`RuntimeGraphInput`]; a partition's child runner borrows
/// its parent's. Both resolve to the same input type, so a partition nested
/// in a partition compiles to the same runner type as its parent's child.
pub(crate) trait RuntimeSource<S, V, DM, IDM>
where
    S: PlanningSolution,
{
    type Extensions: RuntimeExtensionRegistry<S, V, DM, IDM>;

    fn input(&self) -> &RuntimeGraphInput<S, V, DM, IDM, Self::Extensions>;
}

impl<S, V, DM, IDM, E> RuntimeSource<S, V, DM, IDM> for RuntimeGraphInput<S, V, DM, IDM, E>
where
    S: PlanningSolution,
    E: RuntimeExtensionRegistry<S, V, DM, IDM>,
{
    type Extensions = E;

    fn input(&self) -> &RuntimeGraphInput<S, V, DM, IDM, E> {
        self
    }
}

impl<S, V, DM, IDM, E> RuntimeSource<S, V, DM, IDM> for &RuntimeGraphInput<S, V, DM, IDM, E>
where
    S: PlanningSolution,
    E: RuntimeExtensionRegistry<S, V, DM, IDM>,
{
    type Extensions = E;

    fn input(&self) -> &RuntimeGraphInput<S, V, DM, IDM, E> {
        self
    }
}
