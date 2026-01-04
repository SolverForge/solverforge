//! Typed solver using full Solver infrastructure with incremental scoring.
//!
//! This module demonstrates using `TypedScoreDirector` with the complete
//! Solver infrastructure (phases, move selectors, foragers, termination).
//!
//! Key difference from manual solver loop (`solver.rs`):
//! - Uses `Solver<S>` with `Box<dyn ScoreDirector<S>>`
//! - Uses `ConstructionPhaseFactory` and `LocalSearchPhaseFactory`
//! - Uses `ChangeMoveSelector` with typed value selectors
//!
//! Preserves zero-erasure:
//! - `TypedScoreDirector<S, C>` for O(1) incremental scoring
//! - `ChangeMove<S, V>` with `fn` pointer getters/setters
//! - Only boxing at `Box<dyn ScoreDirector<S>>` boundary

use std::time::Duration;

use solverforge::{
    ChangeMove, ChangeMoveSelector, ConstructionPhaseFactory, FromSolutionEntitySelector,
    LocalSearchPhaseFactory, QueuedEntityPlacer, Solver, SolverPhaseFactory,
    StaticTypedValueSelector, TimeTermination, TypedScoreDirector,
};

use crate::constraints::{create_fluent_constraints, create_solution_descriptor};
use crate::domain::EmployeeSchedule;

/// Move type for employee scheduling: assigns an employee index to a shift.
pub type ShiftMove = ChangeMove<EmployeeSchedule, usize>;

// Zero-erasure getters/setters for shift's employee_idx variable
fn get_employee_idx(s: &EmployeeSchedule, shift_idx: usize) -> Option<usize> {
    s.shifts.get(shift_idx).and_then(|shift| shift.employee_idx)
}

fn set_employee_idx(s: &mut EmployeeSchedule, shift_idx: usize, v: Option<usize>) {
    if let Some(shift) = s.shifts.get_mut(shift_idx) {
        shift.employee_idx = v;
    }
}

/// Configuration for the typed solver.
#[derive(Clone)]
pub struct TypedSolverConfig {
    /// Time limit for solving.
    pub time_limit: Duration,
    /// Late acceptance queue size.
    pub late_acceptance_size: usize,
}

impl Default for TypedSolverConfig {
    fn default() -> Self {
        Self {
            time_limit: Duration::from_secs(30),
            late_acceptance_size: 400,
        }
    }
}

/// Solves employee scheduling using full Solver infrastructure with TypedScoreDirector.
///
/// Returns the optimized schedule with the best score found.
pub fn solve(schedule: EmployeeSchedule, config: TypedSolverConfig) -> EmployeeSchedule {
    let n_employees = schedule.employees.len();
    let values: Vec<usize> = (0..n_employees).collect();
    let values_for_ch = values.clone();
    let values_for_ls = values.clone();

    // Create constraints and descriptor
    let constraints = create_fluent_constraints();
    let descriptor = create_solution_descriptor();

    // Create TypedScoreDirector with descriptor (enables ScoreDirector trait)
    let director = TypedScoreDirector::with_descriptor(schedule, constraints, descriptor);

    // Construction phase: FirstFit with QueuedEntityPlacer
    let ch_factory = ConstructionPhaseFactory::<EmployeeSchedule, ShiftMove, _>::first_fit(
        move || {
            let entity_sel = Box::new(FromSolutionEntitySelector::new(0));
            let value_sel = Box::new(StaticTypedValueSelector::new(values_for_ch.clone()));
            Box::new(QueuedEntityPlacer::new(
                entity_sel,
                value_sel,
                get_employee_idx,
                set_employee_idx,
                0,
                "employee_idx",
            ))
        },
    );

    // Local search phase: Late Acceptance
    let late_size = config.late_acceptance_size;
    let ls_factory = LocalSearchPhaseFactory::<EmployeeSchedule, ShiftMove, _>::late_acceptance(
        late_size,
        move || {
            Box::new(ChangeMoveSelector::simple(
                get_employee_idx,
                set_employee_idx,
                0,
                "employee_idx",
                values_for_ls.clone(),
            ))
        },
    );

    // Build solver with termination
    let termination = TimeTermination::new(config.time_limit);
    let mut solver = Solver::new(vec![])
        .with_phase(ch_factory.create_phase())
        .with_phase(ls_factory.create_phase())
        .with_termination(Box::new(termination));

    // Solve! TypedScoreDirector is boxed as Box<dyn ScoreDirector<S>>
    solver.solve_with_director(Box::new(director))
}

/// Solves with default configuration (30 seconds, late acceptance size 400).
pub fn solve_default(schedule: EmployeeSchedule) -> EmployeeSchedule {
    solve(schedule, TypedSolverConfig::default())
}
