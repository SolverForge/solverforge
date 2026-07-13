//! Canonical route-local 2-opt execution.
//!
//! The public function-pointer facade and compiled `RuntimeListSlot` both
//! implement [`ListKOptAccess`].  This module owns the only candidate order,
//! acceptance, trace, and committed-mutation loop for the family.

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use super::super::distance_arithmetic::sum_two;
use crate::scope::{PhaseScope, ProgressCallback, SolverScope, StepControlPolicy, StepScope};
use crate::stats::{
    CandidateTraceConstructionTarget, CandidateTraceDisposition, CandidateTraceSource,
};

/// The declaration-resolved route operations used by canonical list K-opt.
///
/// Implementations provide only data access and mutation.  The kernel owns
/// candidate enumeration, tie/order behavior, feasibility handling, tracing,
/// and score lifecycle so static and compiled runtime paths cannot drift.
pub(crate) trait ListKOptAccess<S> {
    fn descriptor_index(&self) -> usize;
    fn entity_count(&self, solution: &S) -> usize;
    fn route_values(&self, solution: &S, entity_index: usize) -> Vec<usize>;
    fn replace_route(&self, solution: &mut S, entity_index: usize, route: Vec<usize>);
    fn route_depot(&self, solution: &S, entity_index: usize) -> usize;
    fn route_distance(&self, solution: &S, entity_index: usize, from: usize, to: usize) -> i64;
    fn route_feasible(&self, solution: &S, entity_index: usize, route: &[usize]) -> bool;
}

/// Runs the one canonical list K-opt kernel.
///
/// The public phase keeps its historic `k` contract: only `k = 2` is
/// implemented, and every other value is a scored no-op.  The caller supplies
/// a fully resolved access object; source binding is deliberately absent
/// because K-opt never consumes an assignment source stream.
pub(crate) fn run_list_k_opt<S, A, D, BestCb>(
    access: &A,
    k: usize,
    control_policy: StepControlPolicy,
    solver_scope: &mut SolverScope<'_, S, D, BestCb>,
) where
    S: PlanningSolution,
    A: ListKOptAccess<S>,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
{
    if k != 2 {
        tracing::warn!(
            k,
            "ListKOptPhase: only k=2 is implemented; skipping k-opt polishing"
        );
        let mut phase_scope = PhaseScope::with_phase_type(solver_scope, 0, "List K-Opt");
        let _score = phase_scope.score_director_mut().calculate_score();
        phase_scope.update_best_solution();
        return;
    }

    let mut phase_scope = PhaseScope::with_phase_type(solver_scope, 0, "List K-Opt");

    let n_entities = {
        let solution = phase_scope.score_director().working_solution();
        access.entity_count(solution)
    };

    if n_entities == 0 {
        let _score = phase_scope.score_director_mut().calculate_score();
        phase_scope.update_best_solution();
        return;
    }

    let descriptor_index = access.descriptor_index();

    'entities: for entity_idx in 0..n_entities {
        if control_policy.should_terminate_construction(phase_scope.solver_scope_mut()) {
            break;
        }
        let (depot, mut route) = {
            let solution = phase_scope.score_director().working_solution();
            let depot = access.route_depot(solution, entity_idx);
            let route = access.route_values(solution, entity_idx);
            (depot, route)
        };

        let n = route.len();
        if n < 4 {
            continue;
        }

        let mut changed = false;
        let mut interrupted = false;

        // 2-opt: try all (i, j) segment reversals.
        loop {
            let mut improved = false;
            for i in 0..n - 1 {
                let a = if i == 0 { depot } else { route[i - 1] };
                let b = route[i];
                for j in i + 1..n {
                    if control_policy.should_terminate_construction(phase_scope.solver_scope_mut())
                    {
                        interrupted = true;
                        break;
                    }
                    let c = route[j];
                    let e = if j + 1 < n { route[j + 1] } else { depot };
                    let trace_token = phase_scope.record_candidate_operation(
                        CandidateTraceSource::ListKOptReconnection,
                        None,
                        j,
                        Some(CandidateTraceConstructionTarget {
                            descriptor_index,
                            entity_index: entity_idx,
                        }),
                        descriptor_index,
                        "list_k_opt_reconnection",
                        [
                            entity_idx as u64,
                            i as u64,
                            j as u64,
                            a as u64,
                            b as u64,
                            c as u64,
                            e as u64,
                        ],
                    );
                    // Accept if reversing [i..=j] reduces distance.
                    let solution = phase_scope.score_director().working_solution();
                    let proposed_distance = sum_two(
                        access.route_distance(solution, entity_idx, a, c),
                        access.route_distance(solution, entity_idx, b, e),
                    );
                    let current_distance = sum_two(
                        access.route_distance(solution, entity_idx, a, b),
                        access.route_distance(solution, entity_idx, c, e),
                    );
                    if let Some(token) = trace_token {
                        phase_scope.record_candidate_trace_disposition(
                            token,
                            CandidateTraceDisposition::Evaluated,
                        );
                    }
                    if proposed_distance < current_distance {
                        route[i..=j].reverse();
                        if !access.route_feasible(
                            phase_scope.score_director().working_solution(),
                            entity_idx,
                            &route,
                        ) {
                            route[i..=j].reverse();
                            if let Some(token) = trace_token {
                                phase_scope.record_candidate_trace_disposition(
                                    token,
                                    CandidateTraceDisposition::ForagerIgnored,
                                );
                            }
                            continue;
                        }
                        if let Some(token) = trace_token {
                            phase_scope.record_candidate_trace_disposition(
                                token,
                                CandidateTraceDisposition::Selected,
                            );
                        }
                        improved = true;
                        changed = true;
                        if let Some(token) = trace_token {
                            phase_scope.record_candidate_trace_disposition(
                                token,
                                CandidateTraceDisposition::Applied,
                            );
                        }
                    } else if let Some(token) = trace_token {
                        phase_scope.record_candidate_trace_disposition(
                            token,
                            CandidateTraceDisposition::ForagerIgnored,
                        );
                    }
                }
                if interrupted {
                    break;
                }
            }
            if interrupted || !improved {
                break;
            }
        }

        if interrupted {
            break 'entities;
        }

        if changed {
            let mut step_scope =
                StepScope::new_with_control_policy(&mut phase_scope, control_policy);
            step_scope.apply_committed_change(|sd| {
                sd.before_variable_changed(descriptor_index, entity_idx);
                access.replace_route(sd.working_solution_mut(), entity_idx, route);
                sd.after_variable_changed(descriptor_index, entity_idx);
            });
            let step_score = step_scope.calculate_score();
            step_scope.set_step_score(step_score);
            step_scope.complete();
        }
    }

    phase_scope.update_best_solution();
}
