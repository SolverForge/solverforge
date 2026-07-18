//! Prepared runtime node types and construction lowering.
//!
//! The graph instantiator and staged-default executor use this one lowering:
//! both resolve list declarations to compact catalog indexes.  The live
//! executor owns source binding at the first reached construction boundary.

use std::collections::HashMap;
use std::fmt;

use solverforge_core::domain::PlanningSolution;

use crate::builder::ScalarGroupBinding;
use crate::descriptor::ResolvedVariableBinding;
use crate::phase::construction::{ScalarConstructionSchedule, ScalarOrMixedSlotOrder};

use super::super::super::defaults::DefaultRuntimeBindings;
use super::super::super::graph::{CompiledConstruction, CompiledLocalSearch, ListConstructionKind};
use super::super::super::types::{CompiledListSlot, CompiledScalarSlot, RuntimeSlotId};
use super::RuntimeInstantiationError;
use crate::builder::context::{
    bind_runtime_list_source, unassigned_from_current_assignment, RuntimeListElement,
    RuntimeListSourceIndex, SourceElement,
};
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;

/// A compact reference into one solve-owned structural list-source catalog.
///
/// `phase_index` is retained for an actionable error should this source be
/// bound for the first time at this reached phase.  The catalog entry itself
/// is immutable; its source index is cached when a reached construction node
/// validates the current assignment before deciding whether work remains.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PreparedListSlot {
    pub(crate) catalog_index: usize,
    pub(crate) phase_index: usize,
}

impl PreparedListSlot {
    pub(super) fn new(phase_index: usize, catalog_index: usize) -> Self {
        Self {
            catalog_index,
            phase_index,
        }
    }
}

/// Explicit construction variants after source binding.
///
/// Clarke-Wright and K-opt remain distinct variants so a caller cannot route
/// savings construction through insertion or infer post-construction K-opt
/// from an initial default-state snapshot.
#[expect(
    clippy::large_enum_variant,
    reason = "prepared construction is a value-owned per-solve graph"
)]
pub(crate) enum PreparedConstruction<S, V, DM, IDM> {
    ScalarOrMixed {
        config: solverforge_config::ConstructionHeuristicConfig,
        schedule: ScalarConstructionSchedule,
        scalar_slots: Vec<CompiledScalarSlot<S>>,
        list_slots: Vec<PreparedListSlot>,
        slot_order: Vec<ScalarOrMixedSlotOrder>,
    },
    RoundRobin {
        config: solverforge_config::ConstructionHeuristicConfig,
        slots: Vec<PreparedListSlot>,
    },
    CheapestInsertion {
        config: solverforge_config::ConstructionHeuristicConfig,
        slots: Vec<PreparedListSlot>,
    },
    RegretInsertion {
        config: solverforge_config::ConstructionHeuristicConfig,
        slots: Vec<PreparedListSlot>,
    },
    ClarkeWright {
        config: solverforge_config::ConstructionHeuristicConfig,
        slots: Vec<PreparedListSlot>,
    },
    KOpt {
        config: solverforge_config::ConstructionHeuristicConfig,
        slots: Vec<CompiledListSlot<S, V, DM, IDM>>,
    },
    GroupedScalar {
        config: solverforge_config::ConstructionHeuristicConfig,
        group_index: usize,
        group: ScalarGroupBinding<S>,
        scalar_bindings: Vec<ResolvedVariableBinding<S>>,
    },
}

/// A default-runtime declaration prepared for one solve but not yet selected.
pub(crate) struct PreparedDefaultRuntime<S, V, DM, IDM> {
    pub(crate) bindings: DefaultRuntimeBindings<S, V, DM, IDM>,
    /// Original graph position used when a staged child resolves a structural
    /// source-catalog entry.  It must not bind a source until that child is
    /// actually reached.
    pub(crate) phase_index: usize,
}

/// One prepared node in the eventual direct/retained runtime runner.
#[expect(
    clippy::large_enum_variant,
    reason = "prepared phases remain value-owned without execution-time indirection"
)]
pub(crate) enum PreparedRuntimePhase<S, V, DM, IDM, Extension> {
    Construction(PreparedConstruction<S, V, DM, IDM>),
    LocalSearch(CompiledLocalSearch<S, V, DM, IDM>),
    Extension(Extension),
    DefaultRuntime(PreparedDefaultRuntime<S, V, DM, IDM>),
}

/// One per-solve graph instantiation. It intentionally has no `PhaseSequence`:
/// the future unified runner consumes these prepared nodes directly.
pub(crate) struct PreparedRuntimeExecution<S, V, DM, IDM, Extension> {
    pub(crate) phases: Vec<PreparedRuntimePhase<S, V, DM, IDM, Extension>>,
    pub(super) list_source_catalog: PreparedListSourceCatalog<S, V, DM, IDM>,
    /// Per-solve binding cache. A source is bound only once, at the first
    /// reached source-backed construction node, then survives pause and resume
    /// without repeated declaration enumeration.
    pub(super) list_source_indices: Vec<Option<RuntimeListSourceIndex<RuntimeListElement<V>>>>,
}

impl<S, V, DM, IDM, Extension> PreparedRuntimeExecution<S, V, DM, IDM, Extension>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    /// Returns the exact current work for one reached list-source boundary.
    ///
    /// First use binds the declaration and returns the binder's already
    /// validated unassigned entries. Later use refreshes only the assignment
    /// against the cached index. This method is intentionally mutable instead
    /// of using `RefCell` or a lock: the compiled runner owns the execution
    /// state for its full solve, including pause/resume.
    pub(crate) fn current_list_source_work(
        &mut self,
        slot: PreparedListSlot,
        solution: &S,
    ) -> Result<Vec<SourceElement<RuntimeListElement<V>>>, RuntimeInstantiationError> {
        let target = self.list_source_catalog.slot(slot.catalog_index).identity();
        if let Some(source_index) = &self.list_source_indices[slot.catalog_index] {
            return unassigned_from_current_assignment(
                self.list_source_catalog.slot(slot.catalog_index),
                source_index,
                solution,
            )
            .map_err(|error| RuntimeInstantiationError {
                phase_index: slot.phase_index,
                kind: super::RuntimeInstantiationErrorKind::SourceRefresh { target, error },
            });
        }
        let binding =
            bind_runtime_list_source(self.list_source_catalog.slot(slot.catalog_index), solution)
                .map_err(|error| RuntimeInstantiationError {
                phase_index: slot.phase_index,
                kind: super::RuntimeInstantiationErrorKind::SourceBinding { target, error },
            })?;
        let (source_index, unassigned) = binding.into_parts();
        self.list_source_indices[slot.catalog_index] = Some(source_index);
        Ok(unassigned)
    }

    pub(crate) fn current_list_unassigned_count(
        &mut self,
        phase_index: usize,
        slot: &CompiledListSlot<S, V, DM, IDM>,
        solution: &S,
    ) -> Result<usize, RuntimeInstantiationError> {
        let target = slot.identity();
        let catalog_index =
            self.list_source_catalog
                .index_for(slot)
                .ok_or(RuntimeInstantiationError {
                    phase_index,
                    kind: super::RuntimeInstantiationErrorKind::MissingRegisteredSource { target },
                })?;
        self.current_list_source_work(PreparedListSlot::new(phase_index, catalog_index), solution)
            .map(|unassigned| unassigned.len())
    }

    /// Borrows an already-bound source without a map lookup or rebinding.
    pub(crate) fn bound_list_source(
        &self,
        slot: PreparedListSlot,
    ) -> (
        &CompiledListSlot<S, V, DM, IDM>,
        &RuntimeListSourceIndex<RuntimeListElement<V>>,
    ) {
        let source_index = self.list_source_indices[slot.catalog_index]
            .as_ref()
            .expect("a reached prepared list source must be bound before execution");
        (
            self.list_source_catalog.slot(slot.catalog_index),
            source_index,
        )
    }

    #[cfg(test)]
    pub(crate) fn bound_list_source_count(&self) -> usize {
        self.list_source_indices
            .iter()
            .filter(|source| source.is_some())
            .count()
    }

    /// Prepares a construction child resolved by a staged default runtime.
    ///
    /// The default graph can select a child only after earlier construction
    /// has changed the working solution. This conversion resolves its frozen
    /// structural catalog entries only; the reached child binds and validates
    /// a source later, before deciding whether current work remains.
    pub(crate) fn prepare_resolved_construction(
        &self,
        phase_index: usize,
        construction: &CompiledConstruction<S, V, DM, IDM>,
    ) -> Result<PreparedConstruction<S, V, DM, IDM>, RuntimeInstantiationError> {
        prepare_construction(construction, |slots| {
            slots
                .iter()
                .map(|slot| {
                    self.list_source_catalog
                        .index_for(slot)
                        .map(|catalog_index| PreparedListSlot::new(phase_index, catalog_index))
                        .ok_or_else(|| RuntimeInstantiationError {
                            phase_index,
                            kind: super::RuntimeInstantiationErrorKind::MissingRegisteredSource {
                                target: slot.identity(),
                            },
                        })
                })
                .collect()
        })
    }
}

/// Immutable declaration catalog paired with a mutable per-solve binding
/// cache in [`PreparedRuntimeExecution`].  Registration never calls an
/// element source or validates current assignments.
pub(crate) struct PreparedListSourceCatalog<S, V, DM, IDM> {
    by_identity: HashMap<RuntimeSlotId, usize>,
    slots: Vec<CompiledListSlot<S, V, DM, IDM>>,
}

impl<S, V, DM, IDM> Default for PreparedListSourceCatalog<S, V, DM, IDM> {
    fn default() -> Self {
        Self {
            by_identity: HashMap::new(),
            slots: Vec::new(),
        }
    }
}

impl<S, V, DM, IDM> PreparedListSourceCatalog<S, V, DM, IDM>
where
    S: Clone,
    V: Clone,
    DM: Clone,
    IDM: Clone,
{
    pub(super) fn register(&mut self, slot: &CompiledListSlot<S, V, DM, IDM>) -> usize {
        let target = slot.identity();
        if let Some(&catalog_index) = self.by_identity.get(&target) {
            return catalog_index;
        }
        let catalog_index = self.slots.len();
        self.slots.push(slot.clone());
        self.by_identity.insert(target, catalog_index);
        catalog_index
    }

    pub(super) fn index_for(&self, slot: &CompiledListSlot<S, V, DM, IDM>) -> Option<usize> {
        self.by_identity.get(&slot.identity()).copied()
    }

    fn slot(&self, catalog_index: usize) -> &CompiledListSlot<S, V, DM, IDM> {
        &self.slots[catalog_index]
    }

    pub(super) fn binding_slots(&self) -> usize {
        self.slots.len()
    }
}

/// Converts an immutable construction declaration into one prepared node.
///
/// This is intentionally the sole lowering for configured and staged-default
/// construction. The caller supplies only the source-slot resolver because
/// graph instantiation and staged execution own the same source registry at
/// different lifecycle boundaries.
pub(super) fn prepare_construction<S, V, DM, IDM, ResolveSlots>(
    construction: &CompiledConstruction<S, V, DM, IDM>,
    mut resolve_slots: ResolveSlots,
) -> Result<PreparedConstruction<S, V, DM, IDM>, RuntimeInstantiationError>
where
    S: Clone,
    V: Clone,
    DM: Clone,
    IDM: Clone,
    ResolveSlots: FnMut(
        &[CompiledListSlot<S, V, DM, IDM>],
    ) -> Result<Vec<PreparedListSlot>, RuntimeInstantiationError>,
{
    Ok(match construction {
        CompiledConstruction::ScalarOrMixed {
            config,
            schedule,
            scalar_slots,
            list_slots,
            slot_order,
        } => PreparedConstruction::ScalarOrMixed {
            config: config.clone(),
            schedule: *schedule,
            scalar_slots: scalar_slots.clone(),
            list_slots: resolve_slots(list_slots)?,
            slot_order: slot_order.clone(),
        },
        CompiledConstruction::List {
            kind,
            config,
            slots,
        } => match kind {
            ListConstructionKind::RoundRobin => PreparedConstruction::RoundRobin {
                config: config.clone(),
                slots: resolve_slots(slots)?,
            },
            ListConstructionKind::CheapestInsertion => PreparedConstruction::CheapestInsertion {
                config: config.clone(),
                slots: resolve_slots(slots)?,
            },
            ListConstructionKind::RegretInsertion => PreparedConstruction::RegretInsertion {
                config: config.clone(),
                slots: resolve_slots(slots)?,
            },
            ListConstructionKind::ClarkeWright => PreparedConstruction::ClarkeWright {
                config: config.clone(),
                slots: resolve_slots(slots)?,
            },
            ListConstructionKind::KOpt => PreparedConstruction::KOpt {
                config: config.clone(),
                slots: slots.clone(),
            },
        },
        CompiledConstruction::GroupedScalar {
            config,
            group_index,
            group,
            scalar_bindings,
        } => PreparedConstruction::GroupedScalar {
            config: config.clone(),
            group_index: *group_index,
            group: group.clone(),
            scalar_bindings: scalar_bindings.clone(),
        },
    })
}
