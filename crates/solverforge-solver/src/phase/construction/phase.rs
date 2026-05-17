// Construction heuristic phase implementation.

use std::fmt::Debug;
use std::marker::PhantomData;
use std::time::Instant;

use solverforge_config::ConstructionObligation;
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;
use tracing::info;

use crate::heuristic::r#move::Move;
use crate::phase::construction::decision::keep_current_allowed;
use crate::phase::construction::{
    ConstructionChoice, ConstructionForager, ConstructionTarget, EntityPlacer, Placement,
};
use crate::phase::control::{settle_construction_interrupt, StepInterrupt};
use crate::phase::Phase;
use crate::scope::ProgressCallback;
use crate::scope::StepControlPolicy;
use crate::scope::{PhaseScope, SolverScope, StepScope};
use crate::stats::{format_duration, whole_units_per_second};

include!("phase/phase_type.rs");
include!("phase/selection.rs");
