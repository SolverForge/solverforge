// Tests for the move module.

use super::*;
use solverforge_core::domain::{
    EntityCollectionExtractor, EntityDescriptor, PlanningSolution, SolutionDescriptor,
};
use solverforge_core::score::SoftScore;
use solverforge_scoring::{Director, RecordingDirector, ScoreDirector};
use std::any::TypeId;

mod arena;
mod change;
mod compound_scalar;
mod conflict_repair;
mod k_opt;
mod list_change;
mod list_reverse;
mod list_ruin;
mod list_swap;
mod pillar_change;
mod pillar_swap;
mod ruin;
mod sublist_change;
mod sublist_swap;
mod swap;
