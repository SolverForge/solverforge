// Construction heuristic phase implementation.

use std::fmt::Debug;
use std::marker::PhantomData;
use std::time::Instant;

use solverforge_config::ConstructionObligation;
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::move_selector::MoveCursor;
use crate::phase::construction::decision::keep_current_allowed;
use crate::phase::construction::{
    run_construction_phase, ConstructionChoice, ConstructionForager, ConstructionTarget,
    EntityPlacer, EntityPlacerCursor, Placement,
};
use crate::phase::control::{settle_construction_interrupt, StepInterrupt};
use crate::phase::Phase;
use crate::scope::ProgressCallback;
use crate::scope::{SolverScope, StepScope};
use crate::stats::CandidateTraceDisposition;

include!("phase/phase_type.rs");
include!("phase/selection.rs");
