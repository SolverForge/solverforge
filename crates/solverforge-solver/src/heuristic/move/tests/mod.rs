// Tests for the move module.

use super::*;
use snapshot_director::SnapshotDirector;
use solverforge_core::domain::{
    EntityCollectionExtractor, EntityDescriptor, PlanningSolution, SolutionDescriptor,
};
use solverforge_core::score::SoftScore;
use solverforge_scoring::{Director, ScoreDirector};
use std::any::TypeId;

mod change;
mod composite_preview;
mod compound_scalar;
mod k_opt;
mod list_change;
mod list_multi_swap;
mod list_permute;
mod list_reverse;
mod list_ruin;
mod list_swap;
mod list_telemetry;
mod ruin;
mod snapshot_director;
mod sublist_change;
mod sublist_swap;
mod tabu_signature_lending;
mod transfer_payloads;
