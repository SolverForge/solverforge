/* Basic variable solver for simple assignment problems.

This module provides `BasicSpec` for problems using `#[basic_variable_config]`,
where each entity has a single planning variable that can be assigned from a
fixed value range.

Logging levels:
- **INFO**: Solver start/end, phase summaries, problem scale
- **DEBUG**: Individual steps with timing and scores
- **TRACE**: Move evaluation details
*/

use std::sync::atomic::AtomicBool;
use std::time::Duration;

use solverforge_config::{PhaseConfig, SolverConfig};
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::{ParseableScore, Score};
use solverforge_scoring::{ConstraintSet, ScoreDirector};
use tracing::info;

use crate::builder::basic_selector::BasicLeafSelector;
use crate::builder::{
    AcceptorBuilder, AnyAcceptor, BasicContext, BasicMoveSelectorBuilder, ForagerBuilder,
};
use crate::heuristic::r#move::EitherMove;
use crate::heuristic::selector::decorator::UnionMoveSelector;
use crate::heuristic::selector::decorator::VecUnionSelector;
use crate::heuristic::selector::{
    EitherChangeMoveSelector, EitherSwapMoveSelector, FromSolutionEntitySelector,
    StaticTypedValueSelector,
};
use crate::phase::construction::{BestFitForager, ConstructionHeuristicPhase, QueuedEntityPlacer};
use crate::phase::localsearch::{LocalSearchPhase, SimulatedAnnealingAcceptor};
use crate::problem_spec::ProblemSpec;
use crate::run::AnyTermination;
use crate::solver::{SolveResult, Solver};

// Type alias for the config-driven local search phase
type ConfigLocalSearch<S> = LocalSearchPhase<
    S,
    EitherMove<S, usize>,
    VecUnionSelector<S, EitherMove<S, usize>, BasicLeafSelector<S>>,
    AnyAcceptor<S>,
    crate::builder::AnyForager<S>,
>;

// Type alias for the default local search phase (SA + UnionMoveSelector)
type DefaultLocalSearch<S> = LocalSearchPhase<
    S,
    EitherMove<S, usize>,
    UnionMoveSelector<
        S,
        EitherMove<S, usize>,
        EitherChangeMoveSelector<
            S,
            usize,
            FromSolutionEntitySelector,
            StaticTypedValueSelector<S, usize>,
        >,
        EitherSwapMoveSelector<S, usize, FromSolutionEntitySelector, FromSolutionEntitySelector>,
    >,
    SimulatedAnnealingAcceptor,
    crate::phase::localsearch::AcceptedCountForager<S>,
>;

// Monomorphized phase enum for config-driven basic solver.
enum BasicLocalSearch<S: PlanningSolution>
where
    S::Score: Score,
{
    Default(DefaultLocalSearch<S>),
    Config(ConfigLocalSearch<S>),
}

/// Problem specification for basic variable problems.
///
/// Passed to `run_solver` to provide problem-specific construction and local
/// search phases for solutions using `#[basic_variable_config]`.
pub struct BasicSpec<S> {
    pub get_variable: fn(&S, usize) -> Option<usize>,
    pub set_variable: fn(&mut S, usize, Option<usize>),
    pub value_count: fn(&S) -> usize,
    pub entity_count_fn: fn(&S) -> usize,
    pub variable_field: &'static str,
    pub descriptor_index: usize,
}

impl<S, C> ProblemSpec<S, C> for BasicSpec<S>
where
    S: PlanningSolution,
    S::Score: Score + ParseableScore,
    C: ConstraintSet<S, S::Score>,
{
    fn is_trivial(&self, solution: &S) -> bool {
        (self.entity_count_fn)(solution) == 0 || (self.value_count)(solution) == 0
    }

    fn default_time_limit_secs(&self) -> u64 {
        30
    }

    fn log_scale(&self, solution: &S) {
        info!(
            event = "solve_start",
            entity_count = (self.entity_count_fn)(solution),
            value_count = (self.value_count)(solution),
        );
    }

    fn build_and_solve(
        self,
        director: ScoreDirector<S, C>,
        config: &SolverConfig,
        time_limit: Duration,
        termination: AnyTermination<S, ScoreDirector<S, C>>,
        terminate: Option<&AtomicBool>,
        callback: impl Fn(&S) + Send + Sync,
    ) -> SolveResult<S> {
        let n_values = (self.value_count)(director.working_solution());
        let values: Vec<usize> = (0..n_values).collect();
        let entity_selector = FromSolutionEntitySelector::new(0);
        let value_selector = StaticTypedValueSelector::new(values.clone());
        let placer = QueuedEntityPlacer::new(
            entity_selector,
            value_selector,
            self.get_variable,
            self.set_variable,
            0,
            self.variable_field,
        );
        let construction = ConstructionHeuristicPhase::new(placer, BestFitForager::new());

        let local_search = build_local_search::<S>(
            config,
            self.get_variable,
            self.set_variable,
            values,
            self.variable_field,
            self.descriptor_index,
        );

        match local_search {
            BasicLocalSearch::Default(ls) => {
                let solver = Solver::new(((), construction, ls))
                    .with_termination(termination)
                    .with_time_limit(time_limit)
                    .with_best_solution_callback(callback);
                if let Some(flag) = terminate {
                    solver.with_terminate(flag).solve(director)
                } else {
                    solver.solve(director)
                }
            }
            BasicLocalSearch::Config(ls) => {
                let solver = Solver::new(((), construction, ls))
                    .with_termination(termination)
                    .with_time_limit(time_limit)
                    .with_best_solution_callback(callback);
                if let Some(flag) = terminate {
                    solver.with_terminate(flag).solve(director)
                } else {
                    solver.solve(director)
                }
            }
        }
    }
}

// Builds the local search phase from config or falls back to defaults.
fn build_local_search<S>(
    config: &SolverConfig,
    get_variable: fn(&S, usize) -> Option<usize>,
    set_variable: fn(&mut S, usize, Option<usize>),
    values: Vec<usize>,
    variable_field: &'static str,
    descriptor_index: usize,
) -> BasicLocalSearch<S>
where
    S: PlanningSolution,
    S::Score: Score,
{
    // Find first local search phase config
    let ls_config = config.phases.iter().find_map(|p| {
        if let PhaseConfig::LocalSearch(ls) = p {
            Some(ls)
        } else {
            None
        }
    });

    let Some(ls) = ls_config else {
        // No phases configured — use default SA + union(Change, Swap)
        let change_selector = EitherChangeMoveSelector::simple(
            get_variable,
            set_variable,
            descriptor_index,
            variable_field,
            values,
        );
        let swap_selector = EitherSwapMoveSelector::simple(
            get_variable,
            set_variable,
            descriptor_index,
            variable_field,
        );
        let move_selector = UnionMoveSelector::new(change_selector, swap_selector);
        let acceptor = SimulatedAnnealingAcceptor::default();
        let forager = crate::phase::localsearch::AcceptedCountForager::new(1);
        return BasicLocalSearch::Default(LocalSearchPhase::new(
            move_selector,
            acceptor,
            forager,
            None,
        ));
    };

    // Config-driven: build acceptor, forager, move selector from config
    let acceptor = ls
        .acceptor
        .as_ref()
        .map(|ac| AcceptorBuilder::build::<S>(ac))
        .unwrap_or_else(|| AnyAcceptor::SimulatedAnnealing(SimulatedAnnealingAcceptor::default()));

    let forager = ForagerBuilder::build::<S>(ls.forager.as_ref());

    let ctx = BasicContext {
        get_variable,
        set_variable,
        values,
        descriptor_index,
        variable_field,
    };

    let move_selector = BasicMoveSelectorBuilder::build(ls.move_selector.as_ref(), &ctx);

    BasicLocalSearch::Config(LocalSearchPhase::new(
        move_selector,
        acceptor,
        forager,
        None,
    ))
}
