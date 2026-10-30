//! A configured conflict-repair solve reports every repair outcome the
//! provider cursor and the evaluation decide.

use std::any::TypeId;

use solverforge_config::{
    AcceptorConfig, ConflictRepairMoveSelectorConfig, ForagerConfig, LocalSearchConfig,
    MoveSelectorConfig, PhaseConfig, SelectionOrder, SolverConfig, TerminationConfig,
};
use solverforge_core::domain::{
    EntityClassId, EntityCollectionExtractor, EntityDescriptor, PlanningSolution,
    SolutionDescriptor, ValueRangeType, VariableDescriptor, VariableId,
};
use solverforge_core::score::HardSoftScore;
use solverforge_scoring::{ConstraintMetadata, Director};

use crate::__bench::CompiledRuntimeRunner;
use crate::builder::{RuntimeModel, ScalarVariableSlot, SearchContext, ValueSource, VariableSlot};
use crate::heuristic::selector::nearby_list_change::DefaultCrossEntityDistanceMeter;
use crate::phase::Phase;
use crate::planning::scalar::target::ScalarTarget;
use crate::scope::SolverScope;
use crate::{ConflictRepair, RepairCandidate, RepairLimits};

type Meter = DefaultCrossEntityDistanceMeter;

#[derive(Clone, Debug)]
struct Shifts {
    score: Option<HardSoftScore>,
    workers: Vec<Option<usize>>,
    candidates: Vec<usize>,
}

impl PlanningSolution for Shifts {
    type Score = HardSoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

/// One hard point lost per pair of shifts sharing a worker.
fn hard_score(plan: &Shifts) -> HardSoftScore {
    let mut clashes = 0;
    for (index, worker) in plan.workers.iter().enumerate() {
        clashes += plan.workers[index + 1..]
            .iter()
            .filter(|other| worker.is_some() && *other == worker)
            .count() as i64;
    }
    HardSoftScore::of(-clashes, 0)
}

/// Six raw repairs for a `[0, 0, 1]` roster, one per counted outcome:
/// a hard-improving repair, its exact repeat, an empty repair, a repair
/// editing one shift twice, a repair that changes nothing, and a repair
/// that adds clashes.
fn repairs(_: &Shifts, _: RepairLimits) -> Vec<RepairCandidate<Shifts>> {
    let worker = ScalarTarget::from_descriptor_index(0, "worker");
    vec![
        RepairCandidate::new("move", vec![worker.set(0, Some(2))]),
        RepairCandidate::new("move", vec![worker.set(0, Some(2))]),
        RepairCandidate::new("empty", Vec::new()),
        RepairCandidate::new(
            "twice",
            vec![worker.set(1, Some(1)), worker.set(1, Some(2))],
        ),
        RepairCandidate::new("keep", vec![worker.set(1, Some(0))]),
        RepairCandidate::new("pile", vec![worker.set(2, Some(0))]),
    ]
}

fn descriptor() -> SolutionDescriptor {
    SolutionDescriptor::new("Shifts", TypeId::of::<Shifts>()).with_entity(
        EntityDescriptor::new("Shift", TypeId::of::<Option<usize>>(), "workers")
            .with_logical_id(EntityClassId(0))
            .with_extractor(Box::new(EntityCollectionExtractor::new(
                "Shift",
                "workers",
                |plan: &Shifts| &plan.workers,
                |plan: &mut Shifts| &mut plan.workers,
            )))
            .with_variable(
                VariableDescriptor::genuine("worker")
                    .with_logical_id(VariableId(0))
                    .with_allows_unassigned(true)
                    .with_value_range_type(ValueRangeType::EntityDependent),
            ),
    )
}

fn context() -> SearchContext<Shifts, usize, Meter, Meter> {
    let slot = ScalarVariableSlot::new(
        0,
        0,
        "Shift",
        |plan: &Shifts| plan.workers.len(),
        "worker",
        |plan: &Shifts, entity, _| plan.workers[entity],
        |plan: &mut Shifts, entity, _, value| plan.workers[entity] = value,
        ValueSource::EntitySlice {
            values_for_entity: |plan: &Shifts, _, _| &plan.candidates,
        },
        true,
    );
    let model = RuntimeModel::new(vec![VariableSlot::Scalar(slot)])
        .with_conflict_repairs(vec![ConflictRepair::new("clash", repairs)]);
    SearchContext::new(descriptor(), model, Some(7)).expect("the test model binds")
}

fn config() -> SolverConfig {
    SolverConfig {
        random_seed: Some(7),
        phases: vec![PhaseConfig::LocalSearch(LocalSearchConfig {
            acceptor: Some(AcceptorConfig::HillClimbing {}),
            forager: Some(ForagerConfig::BestScore {}),
            move_selector: Some(MoveSelectorConfig::ConflictRepairMoveSelector(
                ConflictRepairMoveSelectorConfig {
                    selection_order: Some(SelectionOrder::Original),
                    constraints: vec!["clash".to_string()],
                    ..ConflictRepairMoveSelectorConfig::default()
                },
            )),
            termination: Some(TerminationConfig {
                step_count_limit: Some(1),
                ..TerminationConfig::default()
            }),
            ..LocalSearchConfig::default()
        })],
        ..SolverConfig::default()
    }
}

#[test]
fn conflict_repair_solve_counts_each_outcome_where_it_is_decided() {
    let mut runner =
        CompiledRuntimeRunner::compile(&config(), context()).expect("the configuration compiles");
    let mut scope = SolverScope::new(ShiftDirector::new(vec![Some(0), Some(0), Some(1)]));
    scope.start_solving();
    runner.solve(&mut scope);

    assert_eq!(
        scope.working_solution().workers,
        [Some(2), Some(0), Some(1)]
    );
    let stats = scope.stats();
    assert_eq!(stats.step_count(), 1);
    assert_eq!(stats.conflict_repair_provider_generated(), 6);
    assert_eq!(stats.conflict_repair_duplicate_filtered(), 1);
    assert_eq!(stats.conflict_repair_illegal_filtered(), 2);
    assert_eq!(stats.conflict_repair_not_doable_filtered(), 1);
    assert_eq!(stats.conflict_repair_exposed(), 2);
    assert_eq!(stats.conflict_repair_hard_improving(), 1);
    assert_eq!(stats.moves_generated(), 2);

    let telemetry = stats.snapshot();
    assert_eq!(telemetry.conflict_repair_provider_generated, 6);
    assert_eq!(telemetry.conflict_repair_duplicate_filtered, 1);
    assert_eq!(telemetry.conflict_repair_illegal_filtered, 2);
    assert_eq!(telemetry.conflict_repair_not_doable_filtered, 1);
    assert_eq!(telemetry.conflict_repair_exposed, 2);
    assert_eq!(telemetry.conflict_repair_hard_improving, 1);
}

#[derive(Clone, Debug)]
struct ShiftDirector {
    working_solution: Shifts,
    descriptor: SolutionDescriptor,
}

impl ShiftDirector {
    fn new(workers: Vec<Option<usize>>) -> Self {
        Self {
            working_solution: Shifts {
                score: None,
                workers,
                candidates: vec![0, 1, 2],
            },
            descriptor: descriptor(),
        }
    }
}

impl Director<Shifts> for ShiftDirector {
    fn working_solution(&self) -> &Shifts {
        &self.working_solution
    }

    fn working_solution_mut(&mut self) -> &mut Shifts {
        &mut self.working_solution
    }

    fn calculate_score(&mut self) -> HardSoftScore {
        let score = hard_score(&self.working_solution);
        self.working_solution.set_score(Some(score));
        score
    }

    fn solution_descriptor(&self) -> &SolutionDescriptor {
        &self.descriptor
    }

    fn clone_working_solution(&self) -> Shifts {
        self.working_solution.clone()
    }

    fn before_variable_changed(&mut self, _descriptor_index: usize, _entity_index: usize) {}

    fn after_variable_changed(&mut self, _descriptor_index: usize, _entity_index: usize) {}

    fn entity_count(&self, descriptor_index: usize) -> Option<usize> {
        (descriptor_index == 0).then_some(self.working_solution.workers.len())
    }

    fn total_entity_count(&self) -> Option<usize> {
        Some(self.working_solution.workers.len())
    }

    fn constraint_metadata(&self) -> Vec<ConstraintMetadata<'_>> {
        Vec::new()
    }
}
