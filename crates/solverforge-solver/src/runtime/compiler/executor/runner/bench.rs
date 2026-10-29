//! Hands the selector benchmark the retained state a compiled solve runs.
//!
//! Both handoffs consume a runner that `try_new` already lowered and
//! prepared. They move out existing state; nothing here lowers, prepares,
//! or rebuilds a selector or a construction placer.

use std::fmt::Debug;

use solverforge_config::ConstructionHeuristicConfig;
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;

use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;
use crate::phase::construction::{ScalarConstructionSchedule, ScalarOrMixedSlotOrder};
use crate::runtime::compiler::types::CompiledScalarSlot;

use super::super::local_search::{ProviderExecutionResources, RuntimeNeighborhoodState};
use super::super::{PreparedConstruction, PreparedRuntimePhase};
use super::local_search::RuntimeLocalSearch;
use super::{CompiledRuntimePhaseRunner, RunnerPhase};

/// The frozen input of one descriptor-placement scalar construction phase.
pub(crate) struct ScalarConstructionHandoff<S> {
    pub(crate) config: ConstructionHeuristicConfig,
    pub(crate) scalar_slots: Vec<CompiledScalarSlot<S>>,
    pub(crate) slot_order: Vec<ScalarOrMixedSlotOrder>,
}

impl<S, V, DM, IDM, Extension> CompiledRuntimePhaseRunner<S, V, DM, IDM, Extension>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S> + 'static,
    IDM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S> + 'static,
{
    /// Moves out the first phase's lowered acceptor/forager neighborhood and
    /// the solve's provider resources, or `None` when the first phase is not
    /// an explicit acceptor/forager local search.
    pub(crate) fn into_first_local_search(
        self,
    ) -> Option<(
        RuntimeNeighborhoodState<S, V, DM, IDM>,
        ProviderExecutionResources<S>,
    )> {
        match self.phases.into_iter().next()? {
            RunnerPhase::LocalSearch {
                local_search: RuntimeLocalSearch::AcceptorForager { selector, .. },
                ..
            } => Some((selector, self.provider_resources)),
            _ => None,
        }
    }

    /// Moves out the first phase's prepared scalar construction, or `None`
    /// when the first phase is not a descriptor-placement scalar construction.
    pub(crate) fn into_first_scalar_construction(self) -> Option<ScalarConstructionHandoff<S>> {
        match self.prepared_phases.into_iter().next()? {
            PreparedRuntimePhase::Construction(PreparedConstruction::ScalarOrMixed {
                config,
                schedule: ScalarConstructionSchedule::DescriptorPlacement,
                scalar_slots,
                slot_order,
                ..
            }) => Some(ScalarConstructionHandoff {
                config,
                scalar_slots,
                slot_order,
            }),
            _ => None,
        }
    }
}
