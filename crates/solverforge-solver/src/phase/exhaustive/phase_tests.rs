use super::*;
use crate::manager::SolverTerminalReason;
use crate::phase::exhaustive::decider::SimpleDecider;
use crate::phase::exhaustive::ExplorationType;
use crate::phase::Phase;
use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};
use solverforge_core::score::SoftScore;
use solverforge_scoring::{ConstraintMetadata, Director};
use std::any::TypeId;
use std::sync::atomic::AtomicBool;

#[derive(Clone, Debug)]
struct TestSolution {
    values: Vec<Option<i32>>,
    score: Option<SoftScore>,
}

impl PlanningSolution for TestSolution {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

fn set_row(s: &mut TestSolution, idx: usize, v: Option<i32>) {
    if let Some(slot) = s.values.get_mut(idx) {
        *slot = v;
    }
}

#[derive(Clone, Debug)]
struct ExhaustiveTestDirector {
    solution: TestSolution,
    descriptor: SolutionDescriptor,
}

impl ExhaustiveTestDirector {
    fn new(values: Vec<Option<i32>>) -> Self {
        Self {
            solution: TestSolution {
                values,
                score: None,
            },
            descriptor: SolutionDescriptor::new("TestSolution", TypeId::of::<TestSolution>()),
        }
    }
}

impl Director<TestSolution> for ExhaustiveTestDirector {
    fn working_solution(&self) -> &TestSolution {
        &self.solution
    }

    fn working_solution_mut(&mut self) -> &mut TestSolution {
        &mut self.solution
    }

    fn calculate_score(&mut self) -> SoftScore {
        let mut total = 0;
        for (index, value) in self.solution.values.iter().enumerate() {
            let target = index as i32 + 2;
            total -= value.map_or(100, |actual| (actual - target).abs() as i64);
        }
        let score = SoftScore::of(total);
        self.solution.set_score(Some(score));
        score
    }

    fn solution_descriptor(&self) -> &SolutionDescriptor {
        &self.descriptor
    }

    fn clone_working_solution(&self) -> TestSolution {
        self.solution.clone()
    }

    fn before_variable_changed(&mut self, _descriptor_index: usize, _entity_index: usize) {}

    fn after_variable_changed(&mut self, _descriptor_index: usize, _entity_index: usize) {}

    fn entity_count(&self, _descriptor_index: usize) -> Option<usize> {
        Some(self.solution.values.len())
    }

    fn total_entity_count(&self) -> Option<usize> {
        Some(self.solution.values.len())
    }

    fn constraint_metadata(&self) -> Vec<ConstraintMetadata<'_>> {
        Vec::new()
    }
}

#[test]
fn test_exploration_type_display() {
    assert_eq!(format!("{}", ExplorationType::DepthFirst), "DepthFirst");
    assert_eq!(format!("{}", ExplorationType::BreadthFirst), "BreadthFirst");
    assert_eq!(format!("{}", ExplorationType::ScoreFirst), "ScoreFirst");
    assert_eq!(
        format!("{}", ExplorationType::OptimisticBoundFirst),
        "OptimisticBoundFirst"
    );
}

#[test]
fn test_exploration_type_default() {
    assert_eq!(ExplorationType::default(), ExplorationType::DepthFirst);
}

#[test]
fn test_config_default() {
    let config = ExhaustiveSearchConfig::default();
    assert_eq!(config.exploration_type, ExplorationType::DepthFirst);
    assert_eq!(config.node_limit, Some(10_000));
    assert!(config.depth_limit.is_none());
    assert!(config.enable_pruning);
}

#[test]
fn test_phase_type_name() {
    let decider: SimpleDecider<TestSolution, i32> =
        SimpleDecider::new(0, "row", vec![0, 1, 2, 3], set_row);
    let phase = ExhaustiveSearchPhase::depth_first(decider);

    assert_eq!(phase.phase_type_name(), "ExhaustiveSearch");
}

#[test]
fn test_phase_debug() {
    let decider: SimpleDecider<TestSolution, i32> =
        SimpleDecider::new(0, "row", vec![0, 1, 2, 3], set_row);
    let phase = ExhaustiveSearchPhase::depth_first(decider);

    let debug = format!("{:?}", phase);
    assert!(debug.contains("ExhaustiveSearchPhase"));
    assert!(debug.contains("DepthFirst"));
}

#[test]
fn solve_publishes_the_actual_best_leaf_solution() {
    let decider: SimpleDecider<TestSolution, i32> =
        SimpleDecider::new(0, "row", vec![1, 2, 3], set_row);
    let mut phase = ExhaustiveSearchPhase::depth_first(decider);
    let director = ExhaustiveTestDirector::new(vec![None, None]);
    let mut solver_scope = SolverScope::new(director);

    phase.solve(&mut solver_scope);

    let best = solver_scope
        .best_solution()
        .expect("exhaustive search should publish a leaf solution");
    assert_eq!(best.values, vec![Some(2), Some(3)]);
    assert_eq!(best.score, Some(SoftScore::of(0)));
    assert_eq!(solver_scope.best_score().copied(), Some(SoftScore::of(0)));
}

#[test]
fn solve_stops_before_work_when_cancelled() {
    let decider: SimpleDecider<TestSolution, i32> =
        SimpleDecider::new(0, "row", vec![1, 2, 3], set_row);
    let mut phase = ExhaustiveSearchPhase::depth_first(decider);
    let director = ExhaustiveTestDirector::new(vec![None, None]);
    let cancel = AtomicBool::new(true);
    let mut solver_scope = SolverScope::new(director).with_terminate(Some(&cancel));

    phase.solve(&mut solver_scope);

    assert_eq!(solver_scope.total_step_count(), 0);
    assert!(solver_scope.best_solution().is_none());
    assert_eq!(
        solver_scope.terminal_reason(),
        SolverTerminalReason::Cancelled
    );
}

#[test]
fn solve_honors_inphase_step_limit() {
    let decider: SimpleDecider<TestSolution, i32> =
        SimpleDecider::new(0, "row", vec![1, 2, 3], set_row);
    let mut phase = ExhaustiveSearchPhase::depth_first(decider);
    let director = ExhaustiveTestDirector::new(vec![None, None, None]);
    let mut solver_scope = SolverScope::new(director);
    solver_scope.inphase_step_count_limit = Some(2);

    phase.solve(&mut solver_scope);

    assert_eq!(solver_scope.total_step_count(), 2);
    assert_eq!(
        solver_scope.terminal_reason(),
        SolverTerminalReason::TerminatedByConfig
    );
}

#[test]
fn solve_keeps_last_complete_leaf_when_step_limit_interrupts_frontier() {
    let decider: SimpleDecider<TestSolution, i32> =
        SimpleDecider::new(0, "row", vec![1, 2], set_row);
    let mut phase = ExhaustiveSearchPhase::depth_first(decider);
    let director = ExhaustiveTestDirector::new(vec![None]);
    let mut solver_scope = SolverScope::new(director);
    solver_scope.inphase_step_count_limit = Some(2);

    phase.solve(&mut solver_scope);

    assert_eq!(solver_scope.total_step_count(), 2);
    assert!(solver_scope.best_solution().is_some());
    assert_eq!(
        solver_scope.terminal_reason(),
        SolverTerminalReason::TerminatedByConfig
    );
}

#[test]
fn solve_marks_node_limit_cut_as_config_termination() {
    let decider: SimpleDecider<TestSolution, i32> =
        SimpleDecider::new(0, "row", vec![1, 2, 3], set_row);
    let mut phase = ExhaustiveSearchPhase::new(
        decider,
        ExhaustiveSearchConfig {
            node_limit: Some(1),
            ..ExhaustiveSearchConfig::default()
        },
    );
    let director = ExhaustiveTestDirector::new(vec![None, None]);
    let mut solver_scope = SolverScope::new(director);

    phase.solve(&mut solver_scope);

    assert_eq!(solver_scope.total_step_count(), 1);
    assert_eq!(
        solver_scope.terminal_reason(),
        SolverTerminalReason::TerminatedByConfig
    );
}

#[test]
fn solve_marks_depth_limit_before_leaf_as_config_termination() {
    let decider: SimpleDecider<TestSolution, i32> =
        SimpleDecider::new(0, "row", vec![1, 2, 3], set_row);
    let mut phase = ExhaustiveSearchPhase::new(
        decider,
        ExhaustiveSearchConfig {
            depth_limit: Some(1),
            ..ExhaustiveSearchConfig::default()
        },
    );
    let director = ExhaustiveTestDirector::new(vec![None, None]);
    let mut solver_scope = SolverScope::new(director);

    phase.solve(&mut solver_scope);

    assert!(solver_scope.best_solution().is_none());
    assert_eq!(
        solver_scope.terminal_reason(),
        SolverTerminalReason::TerminatedByConfig
    );
}

#[test]
fn solve_reports_completed_when_frontier_is_exhausted() {
    let decider: SimpleDecider<TestSolution, i32> =
        SimpleDecider::new(0, "row", vec![1, 2], set_row);
    let mut phase = ExhaustiveSearchPhase::depth_first(decider);
    let director = ExhaustiveTestDirector::new(vec![None]);
    let mut solver_scope = SolverScope::new(director);

    phase.solve(&mut solver_scope);

    assert!(solver_scope.best_solution().is_some());
    assert_eq!(
        solver_scope.terminal_reason(),
        SolverTerminalReason::Completed
    );
}
