use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::SoftScore;

use crate::builder::context::{
    ProviderReasonArena, ProviderResolutionError, RawProviderCandidate, RawProviderEdit,
    RuntimeHostCompoundProvider, RuntimeHostProviderErrorBoundary, RuntimeProviderLimits,
    RuntimeProviderRegistry, RuntimeScalarGroupProviderBinding, RuntimeScalarSlot,
    ScalarGroupBinding, ScalarVariableSlot, ValueSource,
};
use crate::heuristic::r#move::{
    reset_runtime_compound_move_clone_count, runtime_compound_move_clone_count,
};
use crate::heuristic::selector::move_selector::MoveStreamContext;
use crate::runtime::compiler::{
    CompiledProviderPlan, ProviderBindingPlan, ProviderBindingPolicy, ProviderCandidateContract,
    ProviderCandidateDeduplication, ProviderMoveKind, ProviderPullTiming, ProviderReasonStorage,
    ProviderSchedule, ProviderTabuIdentity,
};
use crate::runtime::provider_cursor::RuntimeProviderCursor;
use crate::{
    ConflictRepair, RepairCandidate, RepairLimits, ScalarCandidate, ScalarGroup, ScalarGroupLimits,
    ScalarTarget,
};

#[derive(Clone, Debug)]
struct CursorSolution {
    score: Option<SoftScore>,
    values: Vec<Option<usize>>,
}

impl PlanningSolution for CursorSolution {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

fn entity_count(solution: &CursorSolution) -> usize {
    solution.values.len()
}

fn get(solution: &CursorSolution, row: usize, _variable: usize) -> Option<usize> {
    solution.values.get(row).copied().flatten()
}

fn set(solution: &mut CursorSolution, row: usize, _variable: usize, value: Option<usize>) {
    solution.values[row] = value;
}

fn slot() -> RuntimeScalarSlot<CursorSolution> {
    RuntimeScalarSlot::Static(ScalarVariableSlot::new(
        0,
        0,
        "Task",
        entity_count,
        "worker",
        get,
        set,
        ValueSource::CountableRange { from: 0, to: 3 },
        false,
    ))
}

#[derive(Debug)]
struct PanicBoundary;

impl RuntimeHostProviderErrorBoundary for PanicBoundary {
    fn raise(&self, error: ProviderResolutionError) -> ! {
        panic!("unexpected provider normalization error: {error}")
    }
}

struct CountingGroupProvider {
    pulls: Arc<AtomicUsize>,
}

static STATIC_GROUP_PULLS: AtomicUsize = AtomicUsize::new(0);
static STATIC_REPAIR_PULLS: AtomicUsize = AtomicUsize::new(0);

fn static_group_candidates(
    _solution: &CursorSolution,
    limits: ScalarGroupLimits,
) -> Vec<ScalarCandidate<CursorSolution>> {
    STATIC_GROUP_PULLS.fetch_add(1, Ordering::SeqCst);
    assert_eq!(limits.value_candidate_limit, Some(2));
    assert_eq!(limits.max_moves_per_step, Some(2));
    let target = ScalarTarget::from_descriptor_index(0, "worker");
    [1, 2]
        .into_iter()
        .map(|value| ScalarCandidate::new("static_candidate", vec![target.set(0, Some(value))]))
        .collect()
}

fn static_repair_candidates(
    _solution: &CursorSolution,
    limits: RepairLimits,
) -> Vec<RepairCandidate<CursorSolution>> {
    STATIC_REPAIR_PULLS.fetch_add(1, Ordering::SeqCst);
    assert_eq!(limits.max_matches_per_step, 1);
    assert_eq!(limits.max_repairs_per_match, 2);
    assert_eq!(limits.max_moves_per_step, 2);
    let target = ScalarTarget::from_descriptor_index(0, "worker");
    [1, 2]
        .into_iter()
        .map(|value| RepairCandidate::new("static_repair", vec![target.set(0, Some(value))]))
        .collect()
}

impl RuntimeHostCompoundProvider<CursorSolution> for CountingGroupProvider {
    fn pull(
        &self,
        _solution: &CursorSolution,
        _limits: RuntimeProviderLimits,
    ) -> Vec<RawProviderCandidate> {
        self.pulls.fetch_add(1, Ordering::SeqCst);
        [1, 2]
            .into_iter()
            .map(|value| RawProviderCandidate {
                reason: Arc::from("candidate"),
                edits: vec![RawProviderEdit {
                    entity_class: None,
                    variable_name: Arc::from("worker"),
                    entity_index: 0,
                    to_value: Some(value),
                }],
            })
            .collect()
    }
}

fn callback_group_plan(allowed_slot: crate::builder::RuntimeScalarSlotId) -> CompiledProviderPlan {
    callback_group_plan_with_limit(allowed_slot, Some(0))
}

fn callback_group_plan_with_limit(
    allowed_slot: crate::builder::RuntimeScalarSlotId,
    requested_max_moves_per_step: Option<usize>,
) -> CompiledProviderPlan {
    CompiledProviderPlan {
        move_kind: ProviderMoveKind::Grouped,
        schedule: ProviderSchedule::Group {
            value_candidate_limit: None,
            requested_max_moves_per_step,
        },
        bindings: vec![ProviderBindingPlan {
            handle: crate::builder::RuntimeProviderHandle::CallbackGroup(0),
            declared_schema_index: 0,
            allowed_slots: vec![allowed_slot],
            policy: ProviderBindingPolicy::CallbackGroup {
                rotation_seed_salt: 0,
                pull_timing: ProviderPullTiming::FirstReachableNext,
            },
            candidate_contract: provider_candidate_contract(),
        }],
    }
}

fn static_group_plan(allowed_slot: crate::builder::RuntimeScalarSlotId) -> CompiledProviderPlan {
    CompiledProviderPlan {
        move_kind: ProviderMoveKind::Grouped,
        schedule: ProviderSchedule::Group {
            value_candidate_limit: Some(2),
            requested_max_moves_per_step: Some(2),
        },
        bindings: vec![ProviderBindingPlan {
            handle: crate::builder::RuntimeProviderHandle::StaticGroup(0),
            declared_schema_index: 0,
            allowed_slots: vec![allowed_slot],
            policy: ProviderBindingPolicy::StaticGroup {
                rotation_seed_salt: 0,
                declared_max_moves_per_step: None,
                pull_timing: ProviderPullTiming::OpenCursor,
            },
            candidate_contract: provider_candidate_contract(),
        }],
    }
}

fn static_repair_plan(allowed_slot: crate::builder::RuntimeScalarSlotId) -> CompiledProviderPlan {
    CompiledProviderPlan {
        move_kind: ProviderMoveKind::ConflictRepair,
        schedule: ProviderSchedule::Repair {
            constraints: vec!["hard_constraint".to_string()],
            max_matches_per_step: 1,
            max_repairs_per_match: 2,
            max_moves_per_step: 2,
            include_soft_matches: false,
        },
        bindings: vec![ProviderBindingPlan {
            handle: crate::builder::RuntimeProviderHandle::StaticRepair(0),
            declared_schema_index: 0,
            allowed_slots: vec![allowed_slot],
            policy: ProviderBindingPolicy::StaticRepair {
                constraint_rotation_seed_salt: 0,
                provider_rotation_seed_salt: 0,
                spec_rotation_seed_salt: 0,
                pull_timing: ProviderPullTiming::OpenCursor,
            },
            candidate_contract: provider_candidate_contract(),
        }],
    }
}

fn provider_candidate_contract() -> ProviderCandidateContract {
    ProviderCandidateContract {
        reason_storage: ProviderReasonStorage::PerRunInternedId,
        deduplication: ProviderCandidateDeduplication::PerProviderReasonAndOrderedEdits,
        tabu_identity: ProviderTabuIdentity::ProviderKindAndOrderedEdits,
    }
}

#[test]
fn callback_group_is_lazy_and_explicit_zero_clamps_to_one_candidate() {
    let scalar = slot();
    let allowed_slot = scalar.id();
    let pulls = Arc::new(AtomicUsize::new(0));
    let mut registry = RuntimeProviderRegistry::new(
        vec![RuntimeScalarGroupProviderBinding {
            declared_index: 0,
            group_name: Arc::from("callbacks"),
            callback: Arc::new(CountingGroupProvider {
                pulls: Arc::clone(&pulls),
            }),
        }],
        Vec::new(),
        Arc::new(PanicBoundary),
    )
    .unwrap();
    registry.freeze(&[scalar], &[], &[]).unwrap();
    let plan = callback_group_plan(allowed_slot);
    let solution = CursorSolution {
        score: None,
        values: vec![Some(0)],
    };

    let mut reasons = ProviderReasonArena::default();
    reset_runtime_compound_move_clone_count();
    let mut cursor = RuntimeProviderCursor::new(
        plan.clone(),
        solution.clone(),
        MoveStreamContext::default(),
        false,
    );
    assert_eq!(pulls.load(Ordering::SeqCst), 0);

    let id = cursor
        .next_candidate(&registry, &mut reasons)
        .expect("first callback candidate");
    assert_eq!(pulls.load(Ordering::SeqCst), 1);
    let selected = cursor.take_candidate(id);
    let reason_id = selected.reason_id();
    assert!(cursor.next_candidate(&registry, &mut reasons).is_none());
    assert_eq!(pulls.load(Ordering::SeqCst), 1);
    drop(cursor);
    assert_eq!(reasons.label(reason_id), "candidate");
    assert_eq!(reasons.len(), 1);
    assert_eq!(runtime_compound_move_clone_count(), 0);
}

#[test]
fn static_group_stays_lazy_and_normalizes_typed_candidates_directly() {
    STATIC_GROUP_PULLS.store(0, Ordering::SeqCst);
    let scalar = slot();
    let RuntimeScalarSlot::Static(static_scalar) = scalar.clone() else {
        unreachable!("the fixture uses one static scalar slot")
    };
    let group = ScalarGroupBinding::bind(
        ScalarGroup::candidates(
            "static_group",
            vec![ScalarTarget::from_descriptor_index(0, "worker")],
            static_group_candidates,
        ),
        &[static_scalar],
    );
    let plan = static_group_plan(scalar.id());
    let mut registry = RuntimeProviderRegistry::default();
    registry.freeze(&[scalar], &[group], &[]).unwrap();
    let solution = CursorSolution {
        score: None,
        values: vec![Some(0)],
    };
    let mut reasons = ProviderReasonArena::default();
    let mut cursor =
        RuntimeProviderCursor::new(plan, solution, MoveStreamContext::default(), false);
    assert_eq!(STATIC_GROUP_PULLS.load(Ordering::SeqCst), 0);

    let first_id = cursor
        .next_candidate(&registry, &mut reasons)
        .expect("first static candidate");
    let first_reason = cursor.take_candidate(first_id).reason_id();
    let second_id = cursor
        .next_candidate(&registry, &mut reasons)
        .expect("second static candidate");
    let second_reason = cursor.take_candidate(second_id).reason_id();

    assert!(cursor.next_candidate(&registry, &mut reasons).is_none());
    assert_eq!(STATIC_GROUP_PULLS.load(Ordering::SeqCst), 1);
    assert_eq!(first_reason, second_reason);
    assert_eq!(reasons.label(first_reason), "static_candidate");
    assert_eq!(reasons.len(), 1);
}

#[test]
fn static_repair_stays_on_the_typed_candidate_path() {
    STATIC_REPAIR_PULLS.store(0, Ordering::SeqCst);
    let scalar = slot();
    let plan = static_repair_plan(scalar.id());
    let mut registry = RuntimeProviderRegistry::default();
    registry
        .freeze(
            &[scalar],
            &[],
            &[ConflictRepair::new(
                "hard_constraint",
                static_repair_candidates,
            )],
        )
        .unwrap();
    let solution = CursorSolution {
        score: None,
        values: vec![Some(0)],
    };
    let mut reasons = ProviderReasonArena::default();
    let mut cursor =
        RuntimeProviderCursor::new(plan, solution, MoveStreamContext::default(), false);
    assert_eq!(STATIC_REPAIR_PULLS.load(Ordering::SeqCst), 0);

    let first_id = cursor
        .next_candidate(&registry, &mut reasons)
        .expect("first static repair");
    let first_reason = cursor.take_candidate(first_id).reason_id();
    let second_id = cursor
        .next_candidate(&registry, &mut reasons)
        .expect("second static repair");
    let second_reason = cursor.take_candidate(second_id).reason_id();

    assert!(cursor.next_candidate(&registry, &mut reasons).is_none());
    assert_eq!(STATIC_REPAIR_PULLS.load(Ordering::SeqCst), 1);
    assert_eq!(first_reason, second_reason);
    assert_eq!(reasons.label(first_reason), "static_repair");
    assert_eq!(reasons.len(), 1);
}

#[test]
fn provider_reason_arena_reuses_one_id_for_repeated_callback_labels() {
    let scalar = slot();
    let allowed_slot = scalar.id();
    let pulls = Arc::new(AtomicUsize::new(0));
    let mut registry = RuntimeProviderRegistry::new(
        vec![RuntimeScalarGroupProviderBinding {
            declared_index: 0,
            group_name: Arc::from("callbacks"),
            callback: Arc::new(CountingGroupProvider {
                pulls: Arc::clone(&pulls),
            }),
        }],
        Vec::new(),
        Arc::new(PanicBoundary),
    )
    .unwrap();
    registry.freeze(&[scalar], &[], &[]).unwrap();
    let plan = callback_group_plan_with_limit(allowed_slot, Some(2));
    let solution = CursorSolution {
        score: None,
        values: vec![Some(0)],
    };
    let mut reasons = ProviderReasonArena::default();
    let mut cursor = RuntimeProviderCursor::new(
        plan.clone(),
        solution.clone(),
        MoveStreamContext::default(),
        false,
    );

    let first_id = cursor
        .next_candidate(&registry, &mut reasons)
        .expect("first candidate");
    let first = cursor.take_candidate(first_id);
    let second_id = cursor
        .next_candidate(&registry, &mut reasons)
        .expect("second candidate");
    let second = cursor.take_candidate(second_id);
    let first_reason = first.reason_id();
    let second_reason = second.reason_id();
    drop(cursor);

    assert_eq!(pulls.load(Ordering::SeqCst), 1);
    assert_eq!(first_reason, second_reason);
    assert_eq!(reasons.len(), 1);
    assert_eq!(reasons.label(first_reason), "candidate");
}

#[test]
fn concurrent_lazy_cursors_share_the_execution_arena_without_retaining_its_borrow() {
    let scalar = slot();
    let allowed_slot = scalar.id();
    let pulls = Arc::new(AtomicUsize::new(0));
    let mut registry = RuntimeProviderRegistry::new(
        vec![RuntimeScalarGroupProviderBinding {
            declared_index: 0,
            group_name: Arc::from("callbacks"),
            callback: Arc::new(CountingGroupProvider {
                pulls: Arc::clone(&pulls),
            }),
        }],
        Vec::new(),
        Arc::new(PanicBoundary),
    )
    .unwrap();
    registry.freeze(&[scalar], &[], &[]).unwrap();
    let plan = callback_group_plan(allowed_slot);
    let solution = CursorSolution {
        score: None,
        values: vec![Some(0)],
    };
    let mut reasons = ProviderReasonArena::default();

    // Both leaves can remain live. The top-level execution owns the only
    // mutable arena and lends it to whichever lazy leaf reaches a pull.
    let mut first = RuntimeProviderCursor::new(
        plan.clone(),
        solution.clone(),
        MoveStreamContext::default(),
        false,
    );
    let mut second = RuntimeProviderCursor::new(
        plan.clone(),
        solution.clone(),
        MoveStreamContext::default(),
        false,
    );
    assert_eq!(pulls.load(Ordering::SeqCst), 0);

    let first_id = first
        .next_candidate(&registry, &mut reasons)
        .expect("first lazy candidate");
    let first = first.take_candidate(first_id).reason_id();
    let second_id = second
        .next_candidate(&registry, &mut reasons)
        .expect("second lazy candidate");
    let second = second.take_candidate(second_id).reason_id();

    assert_eq!(pulls.load(Ordering::SeqCst), 2);
    assert_eq!(first, second);
    assert_eq!(reasons.len(), 1);
    assert_eq!(reasons.label(first), "candidate");
}
