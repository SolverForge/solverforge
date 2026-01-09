//! SolverForge - A Constraint Solver in Rust
//!
//! Zero-wiring API: Just annotate your domain and call `solution.solve()`.

pub use solverforge_macros::{
    planning_entity, planning_solution, problem_fact,
    PlanningEntityImpl, PlanningSolutionImpl, ProblemFactImpl,
};

pub use solverforge_core::score::{
    BendableScore, HardMediumSoftScore, HardSoftDecimalScore, HardSoftScore, Score, SimpleScore,
};

pub use solverforge_scoring::stream;

// Re-export traits needed by macro-generated code
pub use solverforge_scoring::{
    ConstraintSet, ScoreDirector, ShadowAwareScoreDirector, ShadowVariableSupport,
    SolvableSolution, TypedScoreDirector,
};
pub use solverforge_core::domain::{
    EntityDescriptor, PlanningEntity, PlanningId, PlanningSolution, ProblemFactDescriptor,
    ShadowVariableKind, SolutionDescriptor, TypedEntityExtractor, VariableDescriptor,
};

// Re-export PlanningEntity as PlanningEntityTrait for macro compatibility
pub use PlanningEntity as PlanningEntityTrait;

// Re-export for k-opt phase distance meter
pub use solverforge_solver::ListPositionDistanceMeter;

pub mod prelude {
    pub use super::{
        planning_entity, planning_solution, problem_fact,
        BendableScore, HardMediumSoftScore, HardSoftDecimalScore, HardSoftScore, Score, SimpleScore,
        ConstraintSet,
    };
    pub use super::stream::{joiner, ConstraintFactory};
}
