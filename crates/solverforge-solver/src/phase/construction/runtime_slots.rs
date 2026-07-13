//! Frozen runtime-slot construction contracts.
//!
//! The immutable graph records which of the two established scalar
//! construction schedules applies before a solve starts.  The eventual
//! runtime-slot kernel consumes this data directly; it never decides whether
//! a typed scalar-only model should behave like the descriptor placer or a
//! mixed/dynamic model should use the global scan.

mod global;
mod moves;
mod placement;

use std::fmt;

use solverforge_config::ConstructionHeuristicConfig;
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use solverforge_scoring::Director;

use crate::builder::context::{
    RuntimeListElement, RuntimeListSlot, RuntimeListSourceIndex, RuntimeScalarSlot,
};
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;
use crate::scope::{ProgressCallback, SolverScope};

use global::solve_global_runtime_slot_scan;
use placement::solve_descriptor_placement;

/// The construction selection schedule frozen by graph compilation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ScalarConstructionSchedule {
    /// Scalar-only construction uses one placement at a time, with the
    /// configured forager selecting within that placement.
    DescriptorPlacement,
    /// Mixed scalar/list construction uses the global runtime-slot scan.
    /// First-fit and cheapest insertion inspect the declaration-order slot
    /// stream as one selection domain.
    GlobalRuntimeSlotScan,
}

/// One declaration-order entry in a frozen scalar-or-mixed construction
/// node.  The payload vectors retain their compact homogeneous storage; this
/// reference restores the exact original model order without execution-time
/// schema discovery.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ScalarOrMixedSlotOrder {
    Scalar {
        scalar_index: usize,
        construction_slot_index: usize,
    },
    List {
        list_index: usize,
        construction_slot_index: usize,
    },
}

/// One scalar carrier with its already-resolved construction-frontier
/// coordinate. The compiler freezes this coordinate before execution.
#[derive(Clone, Debug)]
pub(crate) struct FrozenScalarConstructionSlot<S> {
    pub(crate) slot: RuntimeScalarSlot<S>,
    pub(crate) construction_slot_index: usize,
}

/// One solve-owned list source paired with its immutable slot binding.
///
/// `source_index` is bound before construction begins, so no candidate path
/// rereads the declared stream or rebuilds a payload-key map. Each iteration
/// resolves current assignments against that frozen index before emitting list
/// candidates.
pub(crate) struct FrozenRuntimeListConstructionSlot<'source, S, V, DM, IDM> {
    pub(crate) slot: RuntimeListSlot<S, V, DM, IDM>,
    pub(crate) source_index: &'source RuntimeListSourceIndex<RuntimeListElement<V>>,
}

impl<S, V, DM, IDM> fmt::Debug for FrozenRuntimeListConstructionSlot<'_, S, V, DM, IDM>
where
    RuntimeListSlot<S, V, DM, IDM>: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FrozenRuntimeListConstructionSlot")
            .field("slot", &self.slot)
            .finish_non_exhaustive()
    }
}

/// The complete frozen input for a canonical scalar-or-mixed construction
/// invocation. Compiled execution constructs this from prepared list-source
/// bindings.
pub(crate) struct FrozenScalarOrMixedConstruction<'source, S, V, DM, IDM> {
    pub(crate) schedule: ScalarConstructionSchedule,
    pub(crate) config: ConstructionHeuristicConfig,
    pub(crate) scalar_slots: Vec<RuntimeScalarSlot<S>>,
    pub(crate) list_slots: Vec<FrozenRuntimeListConstructionSlot<'source, S, V, DM, IDM>>,
    pub(crate) slot_order: Vec<ScalarOrMixedSlotOrder>,
}

impl<'source, S, V, DM, IDM> FrozenScalarOrMixedConstruction<'source, S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    /// Builds a construction invocation from sources already bound by the
    /// caller. The phase borrows each frozen declared-stream index, so a
    /// prepared graph can run repeated construction phases without cloning or
    /// re-enumerating source payloads.
    pub(crate) fn new(
        schedule: ScalarConstructionSchedule,
        config: ConstructionHeuristicConfig,
        scalar_slots: Vec<RuntimeScalarSlot<S>>,
        list_slots: Vec<FrozenRuntimeListConstructionSlot<'source, S, V, DM, IDM>>,
        slot_order: Vec<ScalarOrMixedSlotOrder>,
    ) -> Self {
        Self {
            schedule,
            config,
            scalar_slots,
            list_slots,
            slot_order,
        }
    }

    pub(crate) fn solve<D, ProgressCb>(
        self,
        solver_scope: &mut SolverScope<'_, S, D, ProgressCb>,
    ) -> bool
    where
        S::Score: Score + Copy,
        D: Director<S>,
        ProgressCb: ProgressCallback<S>,
    {
        match self.schedule {
            ScalarConstructionSchedule::DescriptorPlacement => {
                assert!(
                    self.list_slots.is_empty(),
                    "descriptor-placement scalar construction cannot carry list slots"
                );
                solve_descriptor_placement(
                    self.config,
                    self.scalar_slots,
                    self.slot_order,
                    solver_scope,
                )
            }
            ScalarConstructionSchedule::GlobalRuntimeSlotScan => solve_global_runtime_slot_scan(
                self.config,
                self.scalar_slots,
                self.list_slots,
                self.slot_order,
                solver_scope,
            ),
        }
    }
}
