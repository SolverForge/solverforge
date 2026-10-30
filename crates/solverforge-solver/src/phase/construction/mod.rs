//! Construction heuristic phase
//!
//! Builds an initial solution by assigning values to uninitialized
//! planning variables one at a time.

mod decision;
mod evaluation;
pub(crate) mod forager;
mod forager_impl;
mod forager_step;
mod frontier;
pub(crate) mod grouped_scalar;
pub(crate) mod phase;
pub(crate) mod placer;
mod runtime_slots;
mod slot;
mod telemetry;

pub(crate) use forager::{
    BestFitForager, ConstructionChoice, ConstructionForager, FirstFitForager, StrongestFitForager,
    WeakestFitForager,
};
pub(crate) use frontier::ConstructionFrontier;
pub(crate) use grouped_scalar::{
    build_scalar_group_construction, record_scalar_assignment_remaining,
    scalar_group_work_remaining,
};
pub(crate) use phase::ConstructionHeuristicPhase;
pub(crate) use placer::ConstructionTarget;
pub(crate) use placer::{EntityPlacer, EntityPlacerCursor, Placement};
pub(crate) use runtime_slots::{
    descriptor_placer, FrozenRuntimeListConstructionSlot, FrozenScalarOrMixedConstruction,
    RuntimeScalarCandidateCursor, RuntimeScalarConstructionCursor, RuntimeScalarConstructionMove,
    RuntimeScalarConstructionPlacer, ScalarConstructionSchedule, ScalarOrMixedSlotOrder,
};
pub(crate) use slot::{
    ConstructionGroupSlotId, ConstructionGroupSlotKey, ConstructionListElementId,
    ConstructionSlotId,
};
pub(crate) use telemetry::{
    record_construction_candidate, report_construction_progress_if_due, run_construction_phase,
    PendingConstructionMoveTelemetry,
};
