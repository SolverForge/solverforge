use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use super::*;

use super::super::completion::complete_routes_by_insertion;
use super::super::owner_assignment::OwnerSlot;
use super::super::route_state::ConstructedRoute;
use crate::builder::context::RuntimeListSourceIndex;
use crate::phase::construction::{run_construction_phase, PendingConstructionMoveTelemetry};
use crate::scope::StepControlPolicy;
use crate::stats::{
    CandidateTraceDisposition, CandidateTraceExecutionPolicy, CandidateTraceHeader,
    CandidateTracePhasePlan, CandidateTraceSource,
};

fn always_feasible(_: &Plan, _: usize, _: &[usize]) -> bool {
    true
}

fn line_distance(_: &Plan, _: usize, left: usize, right: usize) -> i64 {
    (left as i64 - right as i64).abs()
}

static CANCEL_DURING_SAVINGS: AtomicBool = AtomicBool::new(false);
static CANCELLING_DISTANCE_CALLS: AtomicUsize = AtomicUsize::new(0);

fn cancelling_distance(plan: &Plan, owner: usize, left: usize, right: usize) -> i64 {
    if CANCELLING_DISTANCE_CALLS.fetch_add(1, Ordering::SeqCst) == 0 {
        CANCEL_DURING_SAVINGS.store(true, Ordering::SeqCst);
    }
    line_distance(plan, owner, left, right)
}

fn phase() -> ListClarkeWrightPhase<Plan, usize> {
    ListClarkeWrightPhase::new(
        element_count,
        get_assigned,
        entity_count,
        route_len,
        assign_route,
        index_to_element,
        crate::builder::usize_element_source_key,
        depot,
        line_distance,
        always_feasible,
        0,
    )
}

fn director(plan: Plan) -> ScoreDirector<Plan, ()> {
    ScoreDirector::simple(
        plan,
        SolutionDescriptor::new("Plan", TypeId::of::<Plan>()),
        |solution, descriptor_index| {
            if descriptor_index == 0 {
                solution.routes.len()
            } else {
                0
            }
        },
    )
}

fn trace_header() -> CandidateTraceHeader {
    CandidateTraceHeader::new(
        "clarke-wright-interruption".to_string(),
        CandidateTraceExecutionPolicy::known("test", std::iter::empty::<(String, String)>()),
        CandidateTracePhasePlan::known("test", std::iter::empty::<(String, String)>(), Vec::new()),
        None,
    )
}

#[test]
fn interrupted_clarke_wright_savings_preserves_committed_complete_assignment() {
    let customer_count = 64usize;
    let plan = Plan {
        customer_values: (1..=customer_count).collect(),
        routes: vec![Route { visits: Vec::new() }],
        score: None,
    };
    let mut solver_scope = SolverScope::new(director(plan));
    solver_scope.start_solving();
    solver_scope.enable_candidate_trace(trace_header(), 4096);
    let completion_candidate_count = customer_count;
    solver_scope.inphase_move_count_limit = Some(completion_candidate_count as u64 + 1);
    let mut phase = phase();

    phase.solve(&mut solver_scope);

    let mut assigned = solver_scope.working_solution().routes[0].visits.clone();
    assigned.sort_unstable();
    assert_eq!(assigned, (1..=customer_count).collect::<Vec<_>>());
    assert_eq!(solver_scope.stats().moves_accepted, customer_count as u64);
    assert_eq!(solver_scope.stats().moves_applied, customer_count as u64);
    assert_eq!(
        solver_scope.terminal_reason(),
        crate::manager::SolverTerminalReason::TerminatedByConfig
    );
    let trace = solver_scope
        .stats()
        .snapshot()
        .candidate_trace
        .expect("enabled candidate trace");
    assert!(trace.is_complete());
    assert!(trace.pulls.iter().any(|pull| {
        pull.source == CandidateTraceSource::ListClarkeWrightCompletionInsertion
            && pull
                .dispositions
                .contains(&CandidateTraceDisposition::Applied)
    }));
    assert!(trace.pulls.iter().any(|pull| {
        pull.source == CandidateTraceSource::ListClarkeWrightSavings
            && pull
                .dispositions
                .contains(&CandidateTraceDisposition::ForagerIgnored)
    }));
}

#[test]
fn interrupted_clarke_wright_merge_publishes_buffered_merges() {
    let customer_count = 24usize;
    let plan = Plan {
        customer_values: (1..=customer_count).collect(),
        routes: vec![Route { visits: Vec::new() }],
        score: None,
    };
    let mut solver_scope = SolverScope::new(director(plan));
    solver_scope.start_solving();
    solver_scope.enable_candidate_trace(trace_header(), 1024);
    let completion_candidate_count = customer_count;
    let savings_count = customer_count * (customer_count - 1) / 2;
    solver_scope.inphase_move_count_limit =
        Some((completion_candidate_count + savings_count + 1) as u64);
    let mut phase = phase();

    phase.solve(&mut solver_scope);

    let mut assigned = solver_scope.working_solution().routes[0].visits.clone();
    assigned.sort_unstable();
    assert_eq!(assigned, (1..=customer_count).collect::<Vec<_>>());
    assert!(solver_scope.stats().moves_accepted > 0);
    // The buffered merges are a strictly better complete assignment than the
    // balanced seed, so an interrupted phase publishes them rather than
    // dropping them and leaving the seed as the best solution.
    assert!(solver_scope.stats().moves_applied > customer_count as u64);
    assert_eq!(
        solver_scope.stats().moves_applied,
        solver_scope.stats().moves_accepted
    );
    assert_eq!(
        solver_scope.terminal_reason(),
        crate::manager::SolverTerminalReason::TerminatedByConfig
    );
    let trace = solver_scope
        .stats()
        .snapshot()
        .candidate_trace
        .expect("enabled candidate trace");
    assert!(trace.is_complete());
    assert!(trace.pulls.iter().any(|pull| {
        pull.source == CandidateTraceSource::ListClarkeWrightCompletionInsertion
            && pull
                .dispositions
                .contains(&CandidateTraceDisposition::Applied)
    }));
    assert!(trace.pulls.iter().any(|pull| {
        pull.source == CandidateTraceSource::ListClarkeWrightMerge
            && pull
                .dispositions
                .contains(&CandidateTraceDisposition::Applied)
    }));
}

static MERGE_PHASE_DISTANCE_CALLS: AtomicUsize = AtomicUsize::new(0);
static TERMINATE_INSIDE_MERGE: AtomicBool = AtomicBool::new(false);

fn merge_phase_distance(plan: &Plan, owner: usize, left: usize, right: usize) -> i64 {
    MERGE_PHASE_DISTANCE_CALLS.fetch_add(1, Ordering::SeqCst);
    line_distance(plan, owner, left, right)
}

/// Savings generation is the only caller of the distance hook, so once the last
/// savings pair has been priced the phase is inside the merge loop. Flipping the
/// terminate flag from the next feasibility probe therefore interrupts with
/// merges already buffered.
fn merge_phase_feasible(plan: &Plan, owner: usize, route: &[usize]) -> bool {
    let expected = MERGE_PHASE_EXPECTED_DISTANCE_CALLS.load(Ordering::SeqCst);
    if expected > 0 && MERGE_PHASE_DISTANCE_CALLS.load(Ordering::SeqCst) >= expected {
        TERMINATE_INSIDE_MERGE.store(true, Ordering::SeqCst);
    }
    always_feasible(plan, owner, route)
}

static MERGE_PHASE_EXPECTED_DISTANCE_CALLS: AtomicUsize = AtomicUsize::new(0);

#[test]
fn interrupted_clarke_wright_publishes_a_partially_merged_route_set() {
    let element_total = 40usize;
    let plan = Plan {
        customer_values: (1..=element_total).collect(),
        routes: (0..element_total)
            .map(|_| Route { visits: Vec::new() })
            .collect(),
        score: None,
    };
    TERMINATE_INSIDE_MERGE.store(false, Ordering::SeqCst);
    MERGE_PHASE_DISTANCE_CALLS.store(0, Ordering::SeqCst);
    MERGE_PHASE_EXPECTED_DISTANCE_CALLS.store(
        3 * element_total * (element_total - 1) / 2,
        Ordering::SeqCst,
    );
    let mut solver_scope =
        SolverScope::new(director(plan)).with_terminate(Some(&TERMINATE_INSIDE_MERGE));
    solver_scope.start_solving();
    let mut phase = ListClarkeWrightPhase::new(
        element_count,
        get_assigned,
        entity_count,
        route_len,
        assign_route,
        index_to_element,
        crate::builder::usize_element_source_key,
        depot,
        merge_phase_distance,
        merge_phase_feasible,
        0,
    );

    phase.solve(&mut solver_scope);

    let solution = solver_scope.working_solution();
    let mut assigned: Vec<usize> = solution
        .routes
        .iter()
        .flat_map(|route| route.visits.iter().copied())
        .collect();
    assigned.sort_unstable();
    assert_eq!(assigned, (1..=element_total).collect::<Vec<_>>());
    let non_empty_routes = solution
        .routes
        .iter()
        .filter(|route| !route.visits.is_empty())
        .count();
    assert!(
        non_empty_routes < element_total,
        "interrupted construction published {non_empty_routes} routes, expected the buffered merges"
    );
    assert_eq!(
        solver_scope.terminal_reason(),
        crate::manager::SolverTerminalReason::Cancelled
    );
}

#[test]
fn cancellation_after_complete_commit_cannot_return_the_incomplete_prior_best() {
    CANCEL_DURING_SAVINGS.store(false, Ordering::SeqCst);
    CANCELLING_DISTANCE_CALLS.store(0, Ordering::SeqCst);
    let customer_count = 32usize;
    let plan = Plan {
        customer_values: (1..=customer_count).collect(),
        routes: vec![Route { visits: Vec::new() }],
        score: None,
    };
    let mut solver_scope =
        SolverScope::new(director(plan)).with_terminate(Some(&CANCEL_DURING_SAVINGS));
    solver_scope.initialize_working_solution_as_best();
    let mut phase = ListClarkeWrightPhase::new(
        element_count,
        get_assigned,
        entity_count,
        route_len,
        assign_route,
        index_to_element,
        crate::builder::usize_element_source_key,
        depot,
        cancelling_distance,
        always_feasible,
        0,
    );

    phase.solve(&mut solver_scope);

    assert_eq!(
        solver_scope.terminal_reason(),
        crate::manager::SolverTerminalReason::Cancelled
    );
    let mut assigned = solver_scope
        .best_solution()
        .expect("a committed complete assignment replaces the prior best")
        .routes[0]
        .visits
        .clone();
    assigned.sort_unstable();
    assert_eq!(assigned, (1..=customer_count).collect::<Vec<_>>());
}

#[test]
fn termination_before_complete_assignment_keeps_the_working_list_empty() {
    let plan = Plan {
        customer_values: vec![1, 2],
        routes: vec![Route { visits: Vec::new() }],
        score: None,
    };
    let mut solver_scope = SolverScope::new(director(plan));
    solver_scope.start_solving();
    solver_scope.inphase_move_count_limit = Some(0);
    let mut phase = phase();

    phase.solve(&mut solver_scope);

    assert!(solver_scope.working_solution().routes[0].visits.is_empty());
    assert_eq!(solver_scope.stats().moves_accepted, 0);
    assert_eq!(solver_scope.stats().moves_applied, 0);
    assert_eq!(
        solver_scope.terminal_reason(),
        crate::manager::SolverTerminalReason::TerminatedByConfig
    );
}

#[test]
fn interrupted_clarke_wright_completion_discards_local_assignments() {
    let plan = Plan {
        customer_values: vec![1, 2],
        routes: vec![Route { visits: Vec::new() }],
        score: None,
    };
    let access = phase();
    let source_index =
        RuntimeListSourceIndex::bind(&access, &plan).expect("completion source should bind");
    let owner_slots = [OwnerSlot {
        owner_idx: 0,
        metric_class: 0,
    }];
    let routes = [
        ConstructedRoute::singleton(0, true),
        ConstructedRoute::singleton(1, true),
    ];
    let mut solver_scope = SolverScope::new(director(plan));
    solver_scope.start_solving();
    solver_scope.inphase_move_count_limit = Some(1);

    let completed = run_construction_phase(
        &mut solver_scope,
        0,
        "Clarke-Wright Completion Test",
        |phase_scope| {
            let mut pending_move_telemetry = PendingConstructionMoveTelemetry::default();
            let completed = complete_routes_by_insertion(
                phase_scope,
                &access,
                &source_index,
                &owner_slots,
                &routes,
                1,
                super::super::completion::CompletionSelection::Cheapest,
                StepControlPolicy::ObserveConfigLimits,
                &mut pending_move_telemetry,
            );
            pending_move_telemetry.record_discarded(phase_scope);
            completed
        },
    );

    assert!(completed.is_none());
    assert!(solver_scope.working_solution().routes[0].visits.is_empty());
    assert_eq!(solver_scope.stats().moves_accepted, 1);
    assert_eq!(solver_scope.stats().moves_applied, 0);
    assert_eq!(
        solver_scope.terminal_reason(),
        crate::manager::SolverTerminalReason::TerminatedByConfig
    );
}
