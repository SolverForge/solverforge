//! Fallible per-solve instantiation of one immutable runtime graph.
//!
//! This layer has no phase assembly closure. It registers immutable list
//! declarations without enumerating them, eagerly creates explicitly selected
//! typed extensions once, and retains default runtime work as an unresolved
//! staged transition until construction has actually run.

use std::fmt;

use solverforge_core::domain::PlanningSolution;

use crate::builder::RuntimeExtensionRegistry;
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;

use super::super::graph::{
    CompiledConstruction, CompiledRuntimeExtension, CompiledRuntimeGraph, CompiledRuntimePhase,
    ListConstructionKind,
};
use super::super::types::{CompiledListSlot, RuntimeSlotId};
use crate::builder::context::ListConstructionKernelError;

mod types;

use types::prepare_construction;
pub(crate) use types::{
    PreparedConstruction, PreparedDefaultRuntime, PreparedListSlot, PreparedListSourceCatalog,
    PreparedRuntimeExecution, PreparedRuntimePhase,
};

/// One immutable graph plus the concrete extension registry that will be
/// instantiated once for each solve. It owns no solution or phase instance.
pub(crate) struct CompiledRuntimeExecutor<S, V, DM, IDM, E>
where
    S: PlanningSolution,
{
    graph: CompiledRuntimeGraph<S, V, DM, IDM, E>,
}

impl<S, V, DM, IDM, E> CompiledRuntimeExecutor<S, V, DM, IDM, E>
where
    S: PlanningSolution,
{
    pub(crate) fn new(graph: CompiledRuntimeGraph<S, V, DM, IDM, E>) -> Self {
        Self { graph }
    }

    pub(crate) fn graph(&self) -> &CompiledRuntimeGraph<S, V, DM, IDM, E> {
        &self.graph
    }
}

impl<S, V, DM, IDM, E> CompiledRuntimeExecutor<S, V, DM, IDM, E>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    E: RuntimeExtensionRegistry<S, V, DM, IDM>,
{
    /// Registers one solve's structural source catalog and eagerly
    /// instantiates explicit extensions.
    ///
    /// A `DefaultRuntime` node stays unresolved: its child stages depend on
    /// construction results, so selecting K-opt from the initial state would
    /// be semantically wrong. The eventual executor records each reached,
    /// skipped, or not-reached stage in terminal trace provenance.
    pub(crate) fn instantiate(
        &self,
    ) -> Result<PreparedRuntimeExecution<S, V, DM, IDM, E::Phase>, RuntimeInstantiationError> {
        let mut sources = PreparedListSourceRegistry::default();
        // Registration is structural only. It deliberately does not enumerate
        // a declared stream, inspect current assignments, or invoke a host
        // callback. Typed extension builders retain their established eager
        // preparation timing below.
        for (phase_index, phase) in self.graph.phases().iter().enumerate() {
            self.register_phase_sources(phase_index, phase, &mut sources)?;
        }
        let mut phases = Vec::with_capacity(self.graph.phases().len());
        for (phase_index, phase) in self.graph.phases().iter().enumerate() {
            phases.push(self.instantiate_phase(phase_index, phase, &sources)?);
        }
        let list_source_catalog = sources.into_catalog();
        let list_source_indices = (0..list_source_catalog.binding_slots())
            .map(|_| None)
            .collect();
        Ok(PreparedRuntimeExecution {
            phases,
            list_source_catalog,
            list_source_indices,
        })
    }

    fn instantiate_phase(
        &self,
        phase_index: usize,
        phase: &CompiledRuntimePhase<S, V, DM, IDM>,
        sources: &PreparedListSourceRegistry<S, V, DM, IDM>,
    ) -> Result<PreparedRuntimePhase<S, V, DM, IDM, E::Phase>, RuntimeInstantiationError> {
        match phase {
            CompiledRuntimePhase::Construction(construction) => {
                Ok(PreparedRuntimePhase::Construction(
                    self.instantiate_construction(phase_index, construction, sources)?,
                ))
            }
            CompiledRuntimePhase::LocalSearch(local_search) => {
                Ok(PreparedRuntimePhase::LocalSearch(local_search.clone()))
            }
            CompiledRuntimePhase::Extension(extension) => Ok(PreparedRuntimePhase::Extension(
                self.instantiate_extension(phase_index, extension)?,
            )),
            CompiledRuntimePhase::DefaultRuntime => Ok(PreparedRuntimePhase::DefaultRuntime(
                PreparedDefaultRuntime {
                    bindings: self.graph.default_bindings().clone(),
                    phase_index,
                },
            )),
        }
    }

    fn register_phase_sources(
        &self,
        phase_index: usize,
        phase: &CompiledRuntimePhase<S, V, DM, IDM>,
        sources: &mut PreparedListSourceRegistry<S, V, DM, IDM>,
    ) -> Result<(), RuntimeInstantiationError> {
        match phase {
            CompiledRuntimePhase::Construction(construction) => {
                self.register_construction_sources(phase_index, construction, sources)
            }
            CompiledRuntimePhase::DefaultRuntime => self.register_list_slots(
                phase_index,
                &self.graph.default_bindings().list_slots,
                sources,
            ),
            CompiledRuntimePhase::LocalSearch(_) | CompiledRuntimePhase::Extension(_) => Ok(()),
        }
    }

    fn register_construction_sources(
        &self,
        phase_index: usize,
        construction: &CompiledConstruction<S, V, DM, IDM>,
        sources: &mut PreparedListSourceRegistry<S, V, DM, IDM>,
    ) -> Result<(), RuntimeInstantiationError> {
        match construction {
            CompiledConstruction::ScalarOrMixed { list_slots, .. } => {
                self.register_list_slots(phase_index, list_slots, sources)
            }
            CompiledConstruction::List { kind, slots, .. }
                if !matches!(kind, ListConstructionKind::KOpt) =>
            {
                self.register_list_slots(phase_index, slots, sources)
            }
            CompiledConstruction::List { .. } | CompiledConstruction::GroupedScalar { .. } => {
                Ok(())
            }
        }
    }

    fn register_list_slots(
        &self,
        _phase_index: usize,
        slots: &[CompiledListSlot<S, V, DM, IDM>],
        sources: &mut PreparedListSourceRegistry<S, V, DM, IDM>,
    ) -> Result<(), RuntimeInstantiationError> {
        slots.iter().for_each(|slot| {
            sources.register(slot);
        });
        Ok(())
    }

    fn instantiate_construction(
        &self,
        phase_index: usize,
        construction: &CompiledConstruction<S, V, DM, IDM>,
        sources: &PreparedListSourceRegistry<S, V, DM, IDM>,
    ) -> Result<PreparedConstruction<S, V, DM, IDM>, RuntimeInstantiationError> {
        prepare_construction(construction, |slots| {
            self.prepared_list_slots(phase_index, slots, sources)
        })
    }

    fn prepared_list_slots(
        &self,
        phase_index: usize,
        slots: &[CompiledListSlot<S, V, DM, IDM>],
        sources: &PreparedListSourceRegistry<S, V, DM, IDM>,
    ) -> Result<Vec<PreparedListSlot>, RuntimeInstantiationError> {
        slots
            .iter()
            .map(|slot| {
                sources
                    .index_for(phase_index, slot)
                    .map(|catalog_index| PreparedListSlot::new(phase_index, catalog_index))
            })
            .collect()
    }

    fn instantiate_extension(
        &self,
        phase_index: usize,
        extension: &CompiledRuntimeExtension,
    ) -> Result<E::Phase, RuntimeInstantiationError> {
        match extension {
            CompiledRuntimeExtension::Custom { name } => self
                .graph
                .extensions()
                .instantiate_custom(name, self.graph.context())
                .ok_or_else(|| RuntimeInstantiationError {
                    phase_index,
                    kind: RuntimeInstantiationErrorKind::MissingInstantiatedCustomExtension {
                        name: name.clone(),
                    },
                }),
            CompiledRuntimeExtension::Partitioned { name, config } => self
                .graph
                .extensions()
                .instantiate_partitioned(name, config, self.graph.context())
                .ok_or_else(|| RuntimeInstantiationError {
                    phase_index,
                    kind: RuntimeInstantiationErrorKind::MissingInstantiatedPartitioner {
                        name: name.clone(),
                    },
                }),
        }
    }
}

/// Structural registration owned only while one compiled graph instantiates.
///
/// It intentionally has no mutable source-index cache. The returned
/// `PreparedRuntimeExecution` owns that per-solve cache and binds only when a
/// construction stage is actually reached.
struct PreparedListSourceRegistry<S, V, DM, IDM> {
    catalog: PreparedListSourceCatalog<S, V, DM, IDM>,
}

impl<S, V, DM, IDM> Default for PreparedListSourceRegistry<S, V, DM, IDM> {
    fn default() -> Self {
        Self {
            catalog: PreparedListSourceCatalog::default(),
        }
    }
}

impl<S, V, DM, IDM> PreparedListSourceRegistry<S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    fn register(&mut self, slot: &CompiledListSlot<S, V, DM, IDM>) -> usize {
        self.catalog.register(slot)
    }

    fn index_for(
        &self,
        phase_index: usize,
        slot: &CompiledListSlot<S, V, DM, IDM>,
    ) -> Result<usize, RuntimeInstantiationError> {
        self.catalog
            .index_for(slot)
            .ok_or(RuntimeInstantiationError {
                phase_index,
                kind: RuntimeInstantiationErrorKind::MissingRegisteredSource {
                    target: slot.identity(),
                },
            })
    }

    fn into_catalog(self) -> PreparedListSourceCatalog<S, V, DM, IDM> {
        self.catalog
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RuntimeInstantiationError {
    pub(crate) phase_index: usize,
    pub(crate) kind: RuntimeInstantiationErrorKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum RuntimeInstantiationErrorKind {
    SourceBinding {
        target: RuntimeSlotId,
        error: ListConstructionKernelError,
    },
    /// A reached source-backed construction kernel could not reconcile the
    /// current assignment with its already-bound declaration stream. This is
    /// execution-time work, never an eager graph-preparation failure.
    SourceRefresh {
        target: RuntimeSlotId,
        error: ListConstructionKernelError,
    },
    MissingRegisteredSource {
        target: RuntimeSlotId,
    },
    MissingInstantiatedCustomExtension {
        name: String,
    },
    MissingInstantiatedPartitioner {
        name: String,
    },
}

impl fmt::Display for RuntimeInstantiationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "runtime graph instantiation at phase {}: ",
            self.phase_index
        )?;
        match &self.kind {
            RuntimeInstantiationErrorKind::SourceBinding { target, error } => {
                write!(f, "{target} source binding failed: {error:?}")
            }
            RuntimeInstantiationErrorKind::SourceRefresh { target, error } => {
                write!(f, "{target} source assignment refresh failed: {error:?}")
            }
            RuntimeInstantiationErrorKind::MissingRegisteredSource { target } => {
                write!(f, "{target} had no registered source declaration")
            }
            RuntimeInstantiationErrorKind::MissingInstantiatedCustomExtension { name } => {
                write!(
                    f,
                    "compiled custom extension `{name}` was no longer instantiable"
                )
            }
            RuntimeInstantiationErrorKind::MissingInstantiatedPartitioner { name } => {
                write!(
                    f,
                    "compiled partitioned extension `{name}` was no longer instantiable"
                )
            }
        }
    }
}

impl std::error::Error for RuntimeInstantiationError {}

#[cfg(test)]
mod tests;
