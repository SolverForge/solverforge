// Construction heuristic phase implementation.

use std::any::Any;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::time::Instant;

use solverforge_config::ConstructionObligation;
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use solverforge_scoring::Director;
use tracing::info;

use crate::heuristic::r#move::Move;
use crate::phase::construction::decision::{
    is_first_fit_improvement, keep_current_allowed, select_best_fit, select_first_feasible,
    select_first_fit, ScoredChoiceTracker,
};
use crate::phase::construction::evaluation::evaluate_trial_move;
use crate::phase::construction::{
    BestFitForager, ConstructionChoice, ConstructionForager, EntityPlacer, FirstFeasibleForager,
    FirstFitForager, Placement, StrongestFitForager, WeakestFitForager,
};
use crate::phase::control::{
    settle_construction_interrupt, should_interrupt_evaluation, StepInterrupt,
};
use crate::phase::Phase;
use crate::scope::ProgressCallback;
use crate::scope::{PhaseScope, SolverScope, StepScope};
use crate::stats::{format_duration, whole_units_per_second};

include!("phase/phase_type.rs");
include!("phase/selection.rs");
