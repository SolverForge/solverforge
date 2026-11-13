use solverforge_config::ConstructionHeuristicType;

use super::execution_record::{
    DefaultConstructionStageExecutionRecord, DefaultRuntimeConstructionExecution,
    ResolvedConstructionExecutionOutcome, ResolvedConstructionExecutionStep,
};
use super::trace::default_construction_plan;
use crate::runtime::compiler::{
    DefaultConstructionReason, DefaultConstructionStage, DefaultConstructionStepKind,
    DefaultPreconstructionStage,
};

fn executed_stage(
    stage: DefaultConstructionStage,
    kind: DefaultConstructionStepKind,
    reason: DefaultConstructionReason,
) -> DefaultConstructionStageExecutionRecord {
    DefaultConstructionStageExecutionRecord {
        stage,
        outcome: ResolvedConstructionExecutionOutcome::Executed,
        steps: vec![ResolvedConstructionExecutionStep {
            kind,
            reason,
            required_only: false,
            target: None,
            list_policies: None,
            outcome: ResolvedConstructionExecutionOutcome::Executed,
        }],
    }
}

#[test]
fn default_terminal_trace_keeps_clarke_wright_then_k_opt_exactly() {
    let execution = DefaultRuntimeConstructionExecution {
        ran_child_phase: true,
        stages: vec![
            executed_stage(
                DefaultConstructionStage::Preconstruction(
                    DefaultPreconstructionStage::ListConstruction,
                ),
                DefaultConstructionStepKind::ListConstruction(
                    ConstructionHeuristicType::ListClarkeWright,
                ),
                DefaultConstructionReason::SavingsDeclared,
            ),
            executed_stage(
                DefaultConstructionStage::PostConstructionKOpt,
                DefaultConstructionStepKind::ListKOpt,
                DefaultConstructionReason::RouteWithContent,
            ),
        ],
    };

    let plan = default_construction_plan(&execution);
    let executed_kinds = plan
        .children
        .iter()
        .flat_map(|stage| stage.children.iter())
        .map(|step| step.kind.as_str())
        .collect::<Vec<_>>();

    assert_eq!(
        executed_kinds,
        [
            "solverforge.runtime.construction.clarke_wright",
            "solverforge.runtime.construction.k_opt",
        ]
    );
    let reasons = plan
        .children
        .iter()
        .flat_map(|stage| stage.children.iter())
        .map(|step| {
            step.attributes
                .iter()
                .find(|attribute| attribute.key == "reason")
                .map(|attribute| attribute.value.as_str())
        })
        .collect::<Vec<_>>();
    assert_eq!(
        reasons,
        [
            Some("savings_hooks_declared"),
            Some("route_hooks_declared_and_route_non_empty"),
        ]
    );
    assert!(
        plan.is_complete(),
        "terminal construction trace must not be opaque"
    );
}
