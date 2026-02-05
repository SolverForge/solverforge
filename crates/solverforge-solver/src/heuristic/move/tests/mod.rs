//! Tests for the move module.

use super::*;
use solverforge_core::domain::{
    EntityDescriptor, PlanningSolution, SolutionDescriptor, TypedEntityExtractor,
};
use solverforge_core::score::SimpleScore;
use solverforge_scoring::{RecordingScoreDirector, ScoreDirector, SimpleScoreDirector};
use std::any::TypeId;

mod arena;
mod change;
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
