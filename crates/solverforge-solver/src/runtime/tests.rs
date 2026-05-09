use super::Construction;
use crate::builder::{
    bind_scalar_groups, ListVariableSlot, RuntimeModel, ScalarCandidate, ScalarVariableSlot,
    ValueSource, VariableSlot,
};
use crate::descriptor::{scalar_target_matches, scalar_work_remaining_with_frontier};
use crate::phase::Phase;
use crate::planning::{ScalarGroup, ScalarGroupLimits, ScalarTarget};
use crate::scope::SolverScope;
use crate::DefaultCrossEntityDistanceMeter;
use solverforge_config::{
    ConstructionHeuristicConfig, ConstructionHeuristicType, ConstructionObligation,
    VariableTargetConfig,
};
use solverforge_core::domain::{
    EntityCollectionExtractor, EntityDescriptor, PlanningSolution, ProblemFactDescriptor,
    SolutionDescriptor, VariableDescriptor, VariableType,
};
use solverforge_core::score::{HardSoftScore, SoftScore};
use solverforge_scoring::{Director, ScoreDirector};
use std::any::TypeId;

include!("tests/target_matching.rs");
include!("tests/scalar_runtime.rs");
include!("tests/queue_runtime.rs");
include!("tests/revision_runtime.rs");
include!("tests/multi_owner_runtime.rs");
include!("tests/mixed_target_runtime.rs");
include!("tests/coupled_scalar_runtime.rs");
include!("tests/grouped_scalar_construction.rs");
include!("tests/scalar_assignment_construction.rs");
include!("tests/scalar_assignment_grouped_heuristics.rs");
include!("tests/scalar_assignment_repair.rs");
include!("tests/scalar_assignment_soft.rs");
