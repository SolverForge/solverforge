//! A solve on the compiled runtime reports the policy it actually ran in its
//! final telemetry, whether or not candidate tracing is enabled.

use solverforge::{CandidateTracePhasePlan, SolverEvent, SolverManager, SolverTelemetry};

#[path = "list_cvrp_k_opt_time_window/domain/mod.rs"]
mod domain;

use domain::{build_plan, Plan};

const DEFAULT_POLICY_SOLVER_TOML: &str = "\
environment_mode = \"reproducible\"
random_seed = 7

[termination]
step_count_limit = 20
";

fn completed_telemetry(solver_toml: &'static str) -> SolverTelemetry {
    static MANAGER: SolverManager<Plan> = SolverManager::new();

    let mut plan = build_plan();
    plan.routes[0].visits.clear();
    plan.solver_toml = solver_toml;
    let (job_id, mut receiver) = MANAGER.solve(plan).expect("solve should start");
    let telemetry = loop {
        match receiver
            .blocking_recv()
            .expect("event stream should reach a terminal event")
        {
            SolverEvent::Completed { metadata, .. } => break metadata.telemetry,
            SolverEvent::Failed { error, .. } => panic!("solve unexpectedly failed: {error}"),
            SolverEvent::Cancelled { metadata } => {
                panic!("solve was cancelled: {:?}", metadata.terminal_reason)
            }
            _ => {}
        }
    };
    MANAGER
        .delete(job_id)
        .expect("completed job should delete cleanly");
    telemetry
}

fn find<'a>(plan: &'a CandidateTracePhasePlan, kind: &str) -> Option<&'a CandidateTracePhasePlan> {
    if plan.kind == kind {
        return Some(plan);
    }
    plan.children.iter().find_map(|child| find(child, kind))
}

fn attribute<'a>(plan: &'a CandidateTracePhasePlan, key: &str) -> &'a str {
    plan.attributes
        .iter()
        .find(|attribute| attribute.key == key)
        .unwrap_or_else(|| panic!("{} has no {key} attribute", plan.kind))
        .value
        .as_str()
}

#[test]
fn final_telemetry_names_the_default_construction_and_why() {
    let telemetry = completed_telemetry(DEFAULT_POLICY_SOLVER_TOML);
    assert!(
        telemetry.candidate_trace.is_none(),
        "the policy must not depend on candidate tracing"
    );
    let plan = telemetry
        .resolved_phase_plan
        .expect("a completed compiled solve reports its resolved plan");
    assert!(plan.is_complete(), "{plan:#?}");

    let construction = find(&plan, "solverforge.runtime.construction.clarke_wright")
        .unwrap_or_else(|| panic!("default construction ran Clarke-Wright: {plan:#?}"));
    assert_eq!(attribute(construction, "reason"), "savings_hooks_declared");

    let local_search = find(&plan, "solverforge.runtime.default_local_search")
        .unwrap_or_else(|| panic!("default local search is reported: {plan:#?}"));
    assert_eq!(attribute(local_search, "acceptor"), "late_acceptance");
    assert_eq!(
        attribute(local_search, "forager_accepted_count_limit"),
        "256"
    );
    assert_eq!(attribute(local_search, "weighting"), "equal");
    assert_eq!(
        attribute(local_search, "selection_order"),
        "stratified_random"
    );
}

const EXPLICIT_PREFIX: &str = "\
environment_mode = \"reproducible\"
random_seed = 7

[termination]
step_count_limit = 20

[[phases]]
type = \"construction_heuristic\"
construction_heuristic_type = \"list_cheapest_insertion\"

[[phases]]
type = \"local_search\"
";

fn explicit_local_search(forager: &str) -> &'static str {
    Box::leak(format!("{EXPLICIT_PREFIX}{forager}").into_boxed_str())
}

fn explicit_local_search_plan(forager: &str) -> CandidateTracePhasePlan {
    let plan = completed_telemetry(explicit_local_search(forager))
        .resolved_phase_plan
        .expect("a completed compiled solve reports its resolved plan");
    find(&plan, "solverforge.runtime.local_search.acceptor_forager")
        .unwrap_or_else(|| panic!("explicit local search is reported: {plan:#?}"))
        .clone()
}

#[test]
fn an_omitted_forager_inherits_the_default_policy() {
    let local_search = explicit_local_search_plan("");
    assert_eq!(
        attribute(&local_search, "forager"),
        "default:AcceptedCount { limit: 256 }"
    );
    assert_eq!(
        attribute(&local_search, "forager_accepted_count_limit"),
        "256"
    );
}

#[test]
fn an_explicit_accepted_count_forager_without_a_limit_takes_one() {
    let local_search =
        explicit_local_search_plan("\n[phases.forager]\ntype = \"accepted_count\"\n");
    assert!(
        attribute(&local_search, "forager").starts_with("configured:AcceptedCount"),
        "{local_search:#?}"
    );
    assert_eq!(
        attribute(&local_search, "forager_accepted_count_limit"),
        "1"
    );
}

#[test]
fn an_implicit_tabu_forager_has_no_accepted_count_horizon() {
    let local_search = explicit_local_search_plan(
        "\n[phases.acceptor]\ntype = \"tabu_search\"\nentity_tabu_size = 3\n",
    );
    assert_eq!(
        attribute(&local_search, "forager_accepted_count_limit"),
        "none"
    );
}

#[test]
fn variable_neighborhood_descent_has_no_accepted_count_horizon() {
    let local_search = explicit_local_search_plan(
        "local_search_type = \"variable_neighborhood_descent\"\n\n\
         [[phases.neighborhoods]]\ntype = \"list_change_move_selector\"\n",
    );
    assert_eq!(
        attribute(&local_search, "forager_accepted_count_limit"),
        "none"
    );
}
