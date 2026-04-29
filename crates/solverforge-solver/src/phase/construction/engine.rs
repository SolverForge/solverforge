use std::fmt::Debug;
use std::hash::Hash;
use std::time::Instant;

use solverforge_config::{
    ConstructionHeuristicConfig, ConstructionHeuristicType, ConstructionObligation,
};
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use solverforge_scoring::{Director, RecordingDirector};
use tracing::info;

use crate::builder::context::{ScalarGetter, ScalarSetter};
use crate::builder::{ListVariableContext, ModelContext, ScalarVariableContext, VariableContext};
use crate::heuristic::r#move::{ChangeMove, Move};
use crate::scope::{PhaseScope, ProgressCallback, SolverScope, StepScope};
use crate::stats::{format_duration, whole_units_per_second};

use super::decision::{
    is_first_fit_improvement, keep_current_allowed, select_best_fit, select_first_fit,
    ScoredChoiceTracker,
};
use super::evaluation::evaluate_trial_move;
use super::ConstructionListElementId;
use super::ConstructionSlotId;

include!("engine/types.rs");
include!("engine/iterations.rs");
include!("engine/first_fit.rs");
include!("engine/best_fit.rs");
include!("engine/commit.rs");
