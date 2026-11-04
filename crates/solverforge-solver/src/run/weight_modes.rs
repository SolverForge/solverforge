//! The configured solve across constraint weight modes.
//!
//! A score director's weight mode is a type, chosen here once per solve: a
//! solution whose weights require weighting (`requires_weighted_mode`) starts on
//! a director of mode `W`, every other solution on an `UnweightedMode` director.
//! When an applied problem change needs the other mode, the session stops before
//! rescoring and is moved, with all of its state, onto a director of that mode
//! rebuilt from the changed working solution; then it resumes. For the plain
//! entrypoint `W` is `UnweightedMode`, so both arms below are one instantiation
//! and the weighted solver is never compiled.

use super::*;

use crate::scope::WeightModeControl;
use crate::solver::SolveSession;

type ModeDirector<'d, S, C, M> = ScoreDirector<S, C, ReplicaDescriptor<'d, S, C>, M>;

type ModeSession<'d, S, C, M, Runner> = SolveSession<
    'static,
    Runner,
    Option<ConfiguredSolverTermination<S>>,
    S,
    ModeDirector<'d, S, C, M>,
    ChannelProgressCallback<S>,
>;

/// [`try_run_solver_with_config_and_search`] for a solution type with a
/// `#[constraint_weights]` field: scores on a weighted director that scales
/// each constraint by the solution's bound multipliers.
#[doc(hidden)]
#[allow(clippy::too_many_arguments)]
pub fn try_run_weighted_solver_with_config_and_search<S, C, V, DM, IDM, Declaration, BuildSearch>(
    solution: S,
    constraints: fn(&S) -> C,
    descriptor: SolutionDescriptor,
    entity_count_by_descriptor: fn(&S, usize) -> usize,
    runtime: SolverRuntime<S>,
    config: SolverConfig,
    log_scale: fn(&S),
    qualified_candidate_trace_provenance: Option<QualifiedCandidateTraceRunProvenance>,
    build_search: BuildSearch,
) -> RuntimeBuildResult<S>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score + Copy + Ord + ParseableScore,
    C: ConstraintSet<S, S::Score>,
    V: Clone + Copy + PartialEq + Eq + Hash + Into<usize> + Send + Sync + fmt::Debug + 'static,
    DM: crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter<S>
        + Clone
        + Send
        + Sync
        + fmt::Debug
        + 'static,
    IDM: crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter<S>
        + Clone
        + Send
        + Sync
        + fmt::Debug
        + 'static,
    Declaration: Search<S, V, DM, IDM>,
    Declaration::Extensions: RuntimeExtensionRegistry<S, V, DM, IDM>,
    BuildSearch: FnOnce(&SolverConfig, SolutionDescriptor) -> RuntimeBuildResult<Declaration>,
{
    super::try_run_solver_with_config_and_search_request::<S, C, WeightedMode, V, DM, IDM, _, _>(
        solution,
        constraints,
        descriptor,
        entity_count_by_descriptor,
        runtime,
        config,
        log_scale,
        qualified_candidate_trace_provenance,
        build_search,
    )
}

enum Running<A, B> {
    Unweighted(A),
    Weighted(B),
}

pub(super) struct ModeSolve<'d, 'c, S: PlanningSolution, C> {
    pub(super) solution: S,
    pub(super) constraints: C,
    pub(super) descriptor: ReplicaDescriptor<'d, S, C>,
    pub(super) entity_count_by_descriptor: fn(&S, usize) -> usize,
    pub(super) runtime: SolverRuntime<S>,
    pub(super) config: &'c SolverConfig,
    pub(super) qualified_candidate_trace_provenance: Option<QualifiedCandidateTraceRunProvenance>,
}

pub(super) fn solve_in_weight_modes<'d, S, C, W, Runner>(
    solve: ModeSolve<'d, '_, S, C>,
    runner: Runner,
) -> crate::solver::SolveResult<S>
where
    S: PlanningSolution,
    S::Score: Score + ParseableScore,
    C: ConstraintSet<S, S::Score>,
    W: WeightMode,
    Runner: Phase<S, ModeDirector<'d, S, C, UnweightedMode>, ChannelProgressCallback<S>>
        + Phase<S, ModeDirector<'d, S, C, W>, ChannelProgressCallback<S>>,
{
    let ModeSolve {
        solution,
        constraints,
        descriptor,
        entity_count_by_descriptor,
        runtime,
        config,
        qualified_candidate_trace_provenance,
    } = solve;
    let start_weighted = W::WEIGHTED && requires_weighted_mode(&solution);
    let control = if W::WEIGHTED {
        WeightModeControl::Switchable {
            weighted: start_weighted,
        }
    } else {
        WeightModeControl::UnweightedOnly
    };
    let start = SessionStart {
        runner,
        config,
        runtime,
        qualified_candidate_trace_provenance,
        control,
    };
    let mut running = if start_weighted {
        Running::Weighted(
            start.session::<C, W>(ScoreDirector::with_descriptor_in_mode(
                solution,
                constraints,
                descriptor,
                entity_count_by_descriptor,
            )),
        )
    } else {
        Running::Unweighted(start.session::<C, UnweightedMode>(
            ScoreDirector::with_descriptor_in_mode(
                solution,
                constraints,
                descriptor,
                entity_count_by_descriptor,
            ),
        ))
    };
    loop {
        running = match running {
            Running::Unweighted(mut session) => {
                if !session.run() {
                    return session.finish();
                }
                Running::Weighted(
                    session.switch_director(ScoreDirector::into_weight_mode::<W>, |termination| {
                        termination
                    }),
                )
            }
            Running::Weighted(mut session) => {
                if !session.run() {
                    return session.finish();
                }
                Running::Unweighted(session.switch_director(
                    ScoreDirector::into_weight_mode::<UnweightedMode>,
                    |termination| termination,
                ))
            }
        };
    }
}

/// Everything the first session is configured from, besides its director.
struct SessionStart<'c, S: PlanningSolution, Runner> {
    runner: Runner,
    config: &'c SolverConfig,
    runtime: SolverRuntime<S>,
    qualified_candidate_trace_provenance: Option<QualifiedCandidateTraceRunProvenance>,
    control: WeightModeControl,
}

impl<S, Runner> SessionStart<'_, S, Runner>
where
    S: PlanningSolution,
    S::Score: Score + ParseableScore,
{
    fn session<'d, C, M>(
        self,
        director: ModeDirector<'d, S, C, M>,
    ) -> ModeSession<'d, S, C, M, Runner>
    where
        C: ConstraintSet<S, S::Score>,
        M: WeightMode,
        Runner: Phase<S, ModeDirector<'d, S, C, M>, ChannelProgressCallback<S>>,
    {
        let (termination, time_limit) = build_solver_termination::<S>(self.config);
        let execution_policy = configured_execution_policy::<S>(self.config);
        let mut solver = Solver::new(self.runner)
            .with_config(self.config.clone())
            .with_candidate_trace_execution_policy(execution_policy)
            .with_termination(termination)
            .with_runtime(self.runtime)
            .with_progress_callback(ChannelProgressCallback::new(self.runtime))
            .with_weight_mode_control(self.control);
        if let Some(provenance) = self.qualified_candidate_trace_provenance {
            solver = solver.with_qualified_candidate_trace_run_provenance(provenance);
        }
        if let Some(time_limit) = time_limit {
            solver = solver.with_time_limit(time_limit);
        }
        solver
            .with_terminate(self.runtime.cancel_flag())
            .start_session(director)
    }
}
