/* Construction heuristic phase

Builds an initial solution by assigning values to uninitialized
planning variables one at a time.
*/

mod config;
mod decision;
mod evaluation;
mod forager;
mod forager_impl;
mod forager_step;
mod frontier;
pub(crate) mod grouped_scalar;
mod phase;
mod placer;
mod runtime_slots;
mod slot;
mod telemetry;

pub use config::{ConstructionHeuristicConfig, ForagerType};
pub use forager::{
    BestFitForager, ConstructionChoice, ConstructionForager, FirstFeasibleForager, FirstFitForager,
    StrongestFitForager, WeakestFitForager,
};
pub(crate) use frontier::ConstructionFrontier;
pub(crate) use grouped_scalar::{
    build_scalar_group_construction, record_scalar_assignment_remaining,
    scalar_group_work_remaining,
};
pub use phase::ConstructionHeuristicPhase;
pub(crate) use placer::ConstructionTarget;
pub use placer::{
    EntityPlacer, EntityPlacerCursor, Placement, QueuedEntityPlacer, SortedEntityPlacer,
};
pub(crate) use runtime_slots::{
    FrozenRuntimeListConstructionSlot, FrozenScalarOrMixedConstruction, ScalarConstructionSchedule,
    ScalarOrMixedSlotOrder,
};
pub(crate) use slot::{
    ConstructionGroupSlotId, ConstructionGroupSlotKey, ConstructionListElementId,
    ConstructionSlotId,
};
pub(crate) use telemetry::run_construction_phase;
