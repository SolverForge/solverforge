//! Solver and SolverFactory implementations

use std::marker::PhantomData;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use solverforge_scoring::ScoreDirector;
use solverforge_config::SolverConfig;
use solverforge_core::domain::PlanningSolution;
use solverforge_core::SolverForgeError;

use crate::phase::Phase;
use crate::scope::SolverScope;
use crate::termination::Termination;

/// Factory for creating Solver instances.
pub struct SolverFactory<S: PlanningSolution> {
    config: SolverConfig,
    _phantom: PhantomData<S>,
}

impl<S: PlanningSolution> SolverFactory<S> {
    /// Creates a new SolverFactory from configuration.
    pub fn create(config: SolverConfig) -> Result<Self, SolverForgeError> {
        Ok(SolverFactory {
            config,
            _phantom: PhantomData,
        })
    }

    /// Creates a SolverFactory from a TOML configuration file.
    pub fn from_toml_file(path: impl AsRef<std::path::Path>) -> Result<Self, SolverForgeError> {
        let config = SolverConfig::from_toml_file(path)
            .map_err(|e| SolverForgeError::Config(e.to_string()))?;
        Self::create(config)
    }

    /// Creates a SolverFactory from a YAML configuration file.
    pub fn from_yaml_file(path: impl AsRef<std::path::Path>) -> Result<Self, SolverForgeError> {
        let config = SolverConfig::from_yaml_file(path)
            .map_err(|e| SolverForgeError::Config(e.to_string()))?;
        Self::create(config)
    }

    /// Builds a new Solver instance.
    pub fn build_solver(&self) -> Solver<S> {
        Solver::from_config(self.config.clone())
    }

    /// Returns a reference to the configuration.
    pub fn config(&self) -> &SolverConfig {
        &self.config
    }
}

/// The main solver that optimizes planning solutions.
///
/// The solver executes phases in sequence, checking termination conditions
/// between phases and potentially within phases.
pub struct Solver<S: PlanningSolution> {
    /// Phases to execute in order.
    phases: Vec<Box<dyn Phase<S>>>,
    /// Global termination condition.
    termination: Option<Box<dyn Termination<S>>>,
    /// Flag for early termination requests.
    terminate_early_flag: Arc<AtomicBool>,
    /// Whether solver is currently running.
    solving: Arc<AtomicBool>,
    /// Optional configuration.
    config: Option<SolverConfig>,
}

impl<S: PlanningSolution> Solver<S> {
    /// Creates a new solver with the given phases.
    pub fn new(phases: Vec<Box<dyn Phase<S>>>) -> Self {
        Solver {
            phases,
            termination: None,
            terminate_early_flag: Arc::new(AtomicBool::new(false)),
            solving: Arc::new(AtomicBool::new(false)),
            config: None,
        }
    }

    /// Creates a solver from configuration (phases must be added separately).
    pub fn from_config(config: SolverConfig) -> Self {
        Solver {
            phases: Vec::new(),
            termination: None,
            terminate_early_flag: Arc::new(AtomicBool::new(false)),
            solving: Arc::new(AtomicBool::new(false)),
            config: Some(config),
        }
    }

    /// Adds a phase to the solver.
    pub fn with_phase(mut self, phase: Box<dyn Phase<S>>) -> Self {
        self.phases.push(phase);
        self
    }

    /// Adds phases to the solver.
    pub fn with_phases(mut self, phases: Vec<Box<dyn Phase<S>>>) -> Self {
        self.phases.extend(phases);
        self
    }

    /// Sets the termination condition.
    pub fn with_termination(mut self, termination: Box<dyn Termination<S>>) -> Self {
        self.termination = Some(termination);
        self
    }

    /// Solves using the provided score director.
    ///
    /// This is the main solving method that executes all phases.
    pub fn solve_with_director(
        &mut self,
        score_director: Box<dyn ScoreDirector<S>>,
    ) -> S {
        self.solving.store(true, Ordering::SeqCst);
        self.terminate_early_flag.store(false, Ordering::SeqCst);

        let mut solver_scope = SolverScope::new(score_director);
        solver_scope.set_terminate_early_flag(self.terminate_early_flag.clone());
        solver_scope.start_solving();

        // Note: We don't set the initial solution as "best" here because
        // construction heuristic will create a fully assigned solution
        // which may have a worse score than the unassigned initial state.
        // Phases are responsible for updating best solution appropriately.

        // Execute phases
        let mut phase_index = 0;
        while phase_index < self.phases.len() {
            // Check termination before phase
            if self.check_termination(&solver_scope) {
                tracing::debug!(
                    "Terminating before phase {} ({})",
                    phase_index,
                    self.phases[phase_index].phase_type_name()
                );
                break;
            }

            tracing::debug!(
                "Starting phase {} ({})",
                phase_index,
                self.phases[phase_index].phase_type_name()
            );

            self.phases[phase_index].solve(&mut solver_scope);

            tracing::debug!(
                "Finished phase {} ({}) with score {:?}",
                phase_index,
                self.phases[phase_index].phase_type_name(),
                solver_scope.best_score()
            );

            phase_index += 1;
        }

        self.solving.store(false, Ordering::SeqCst);

        // Return the best solution if set, otherwise the working solution
        // (This handles the case where construction heuristic creates an assigned
        // solution but local search didn't find any improvements)
        solver_scope.take_best_or_working_solution()
    }

    /// Checks if solving should terminate.
    fn check_termination(&self, solver_scope: &SolverScope<S>) -> bool {
        // Check early termination request
        if self.terminate_early_flag.load(Ordering::SeqCst) {
            return true;
        }

        // Check termination condition
        if let Some(ref termination) = self.termination {
            if termination.is_terminated(solver_scope) {
                return true;
            }
        }

        false
    }

    /// Requests early termination of the solving process.
    ///
    /// This method is thread-safe and can be called from another thread.
    pub fn terminate_early(&self) -> bool {
        if self.solving.load(Ordering::SeqCst) {
            self.terminate_early_flag.store(true, Ordering::SeqCst);
            true
        } else {
            false
        }
    }

    /// Returns true if the solver is currently solving.
    pub fn is_solving(&self) -> bool {
        self.solving.load(Ordering::SeqCst)
    }

    /// Returns the configuration if set.
    pub fn config(&self) -> Option<&SolverConfig> {
        self.config.as_ref()
    }
}

/// Solver status enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolverStatus {
    /// Solver is not currently solving.
    NotSolving,
    /// Solver is initializing.
    SolvingScheduled,
    /// Solver is actively solving.
    SolvingActive,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::heuristic::r#move::ChangeMove;
    use crate::heuristic::selector::ChangeMoveSelector;
    use crate::manager::SolverPhaseFactory;
    use crate::termination::StepCountTermination;
    use solverforge_scoring::SimpleScoreDirector;
    use solverforge_core::domain::{
        EntityDescriptor, SolutionDescriptor, TypedEntityExtractor,
    };
    use solverforge_core::score::SimpleScore;
    use std::any::TypeId;

    #[derive(Clone, Debug)]
    struct Queen {
        column: i32,
        row: Option<i32>,
    }

    #[derive(Clone, Debug)]
    struct NQueensSolution {
        n: i32,
        queens: Vec<Queen>,
        score: Option<SimpleScore>,
    }

    type NQueensMove = ChangeMove<NQueensSolution, i32>;

    impl PlanningSolution for NQueensSolution {
        type Score = SimpleScore;

        fn score(&self) -> Option<Self::Score> {
            self.score
        }

        fn set_score(&mut self, score: Option<Self::Score>) {
            self.score = score;
        }
    }

    fn get_queens(s: &NQueensSolution) -> &Vec<Queen> {
        &s.queens
    }

    fn get_queens_mut(s: &mut NQueensSolution) -> &mut Vec<Queen> {
        &mut s.queens
    }

    // Zero-erasure typed getter/setter for solution-level access
    fn get_queen_row(s: &NQueensSolution, idx: usize) -> Option<i32> {
        s.queens.get(idx).and_then(|q| q.row)
    }

    fn set_queen_row(s: &mut NQueensSolution, idx: usize, v: Option<i32>) {
        if let Some(queen) = s.queens.get_mut(idx) {
            queen.row = v;
        }
    }

    fn calculate_conflicts(solution: &NQueensSolution) -> SimpleScore {
        let mut conflicts = 0i64;

        for (i, q1) in solution.queens.iter().enumerate() {
            if let Some(row1) = q1.row {
                for q2 in solution.queens.iter().skip(i + 1) {
                    if let Some(row2) = q2.row {
                        // Same row conflict
                        if row1 == row2 {
                            conflicts += 1;
                        }
                        // Diagonal conflict
                        let col_diff = (q2.column - q1.column).abs();
                        let row_diff = (row2 - row1).abs();
                        if col_diff == row_diff {
                            conflicts += 1;
                        }
                    }
                }
            }
        }

        SimpleScore::of(-conflicts)
    }

    fn create_test_director(solution: NQueensSolution) -> SimpleScoreDirector<NQueensSolution, impl Fn(&NQueensSolution) -> SimpleScore> {
        let extractor = Box::new(TypedEntityExtractor::new(
            "Queen",
            "queens",
            get_queens,
            get_queens_mut,
        ));let entity_desc = EntityDescriptor::new("Queen", TypeId::of::<Queen>(), "queens")
            .with_extractor(extractor);

        let descriptor =
            SolutionDescriptor::new("NQueensSolution", TypeId::of::<NQueensSolution>())
                .with_entity(entity_desc);

        SimpleScoreDirector::with_calculator(solution, descriptor, calculate_conflicts)
    }

    #[test]
    fn test_solver_new() {
        let solver: Solver<NQueensSolution> = Solver::new(vec![]);
        assert!(!solver.is_solving());
    }

    #[test]
    fn test_solver_with_phases() {
        use crate::manager::LocalSearchPhaseFactory;

        let values: Vec<i32> = (0..4).collect();
        let factory = LocalSearchPhaseFactory::<NQueensSolution, NQueensMove, _>::hill_climbing(
            move || Box::new(ChangeMoveSelector::<NQueensSolution, i32>::simple(
                get_queen_row, set_queen_row, 0, "row", values.clone()
            )),
        ).with_step_limit(10);
        let local_search = factory.create_phase();

        let solver: Solver<NQueensSolution> =
            Solver::new(vec![]).with_phase(local_search);

        assert!(!solver.is_solving());
    }

    #[test]
    fn test_solver_local_search_only() {
        use crate::manager::LocalSearchPhaseFactory;

        // Start with an already-assigned solution
        let n = 4;
        let queens = (0..n)
            .map(|col| Queen {
                column: col,
                row: Some(0), // All in row 0 - lots of conflicts
            })
            .collect();

        let solution = NQueensSolution {
            n,
            queens,
            score: None,
        };

        let director = create_test_director(solution);

        // Calculate initial conflicts
        let initial_score = calculate_conflicts(director.working_solution());
        assert!(initial_score < SimpleScore::of(0)); // Should have conflicts

        let values: Vec<i32> = (0..n).collect();
        let factory = LocalSearchPhaseFactory::<NQueensSolution, NQueensMove, _>::hill_climbing(
            move || Box::new(ChangeMoveSelector::<NQueensSolution, i32>::simple(
                get_queen_row, set_queen_row, 0, "row", values.clone()
            )),
        ).with_step_limit(50);
        let local_search = factory.create_phase();

        let mut solver = Solver::new(vec![local_search]);

        let result = solver.solve_with_director(Box::new(director));

        // Should have improved or stayed the same
        let final_score = calculate_conflicts(&result);
        assert!(final_score >= initial_score);
    }

    #[test]
    fn test_solver_with_termination() {
        use crate::manager::LocalSearchPhaseFactory;

        let n = 4;
        let queens = (0..n)
            .map(|col| Queen {
                column: col,
                row: Some(0),
            })
            .collect();

        let solution = NQueensSolution {
            n,
            queens,
            score: None,
        };

        let director = create_test_director(solution);

        let values: Vec<i32> = (0..n).collect();
        let factory = LocalSearchPhaseFactory::<NQueensSolution, NQueensMove, _>::hill_climbing(
            move || Box::new(ChangeMoveSelector::<NQueensSolution, i32>::simple(
                get_queen_row, set_queen_row, 0, "row", values.clone()
            )),
        ).with_step_limit(1000);
        let local_search = factory.create_phase();

        // Terminate after just 5 steps
        let termination = StepCountTermination::new(5);

        let mut solver = Solver::new(vec![local_search])
            .with_termination(Box::new(termination));

        let result = solver.solve_with_director(Box::new(director));

        // Should complete without error
        assert!(result.score().is_some() || result.queens.iter().any(|q| q.row.is_some()));
    }

    #[test]
    fn test_solver_status() {
        let solver: Solver<NQueensSolution> = Solver::new(vec![]);

        assert!(!solver.is_solving());
        assert!(!solver.terminate_early()); // Can't terminate when not solving
    }

    #[test]
    fn test_solver_multiple_phases() {
        use crate::manager::LocalSearchPhaseFactory;

        let n = 4;
        let values: Vec<i32> = (0..n).collect();

        // First local search phase with limited steps
        let values1 = values.clone();
        let factory1 = LocalSearchPhaseFactory::<NQueensSolution, NQueensMove, _>::hill_climbing(
            move || Box::new(ChangeMoveSelector::<NQueensSolution, i32>::simple(
                get_queen_row, set_queen_row, 0, "row", values1.clone()
            )),
        ).with_step_limit(10);
        let phase1 = factory1.create_phase();

        // Second local search phase
        let values2 = values.clone();
        let factory2 = LocalSearchPhaseFactory::<NQueensSolution, NQueensMove, _>::hill_climbing(
            move || Box::new(ChangeMoveSelector::<NQueensSolution, i32>::simple(
                get_queen_row, set_queen_row, 0, "row", values2.clone()
            )),
        ).with_step_limit(10);
        let phase2 = factory2.create_phase();

        let mut solver = Solver::new(vec![phase1, phase2]);

        // Need to start with assigned values for local search
        let n = 4;
        let queens = (0..n)
            .map(|col| Queen {
                column: col,
                row: Some(col % n), // Assign different rows
            })
            .collect();

        let solution = NQueensSolution {
            n,
            queens,
            score: None,
        };
        assert_eq!(solution.n, n);

        let director = create_test_director(solution);
        let result = solver.solve_with_director(Box::new(director));

        // Should complete both phases
        assert!(result.queens.iter().all(|q| q.row.is_some()));
    }
}
