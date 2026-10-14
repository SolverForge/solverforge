//! T08 pins: candidate-backed placement streaming. The placement cursor
//! retains its prevalidated seed store, so repeated placement pulls never
//! re-enumerate the candidate provider, and completed placements are skipped
//! without rebuilding.

use super::*;

use crate::builder::ScalarGroupBinding;
use crate::descriptor::collect_bindings;
use crate::descriptor::ResolvedVariableBinding;
use crate::phase::construction::{EntityPlacer, EntityPlacerCursor};
use std::sync::atomic::{AtomicUsize, Ordering};

static PROVIDER_CALLS: AtomicUsize = AtomicUsize::new(0);

fn counting_provider(
    _solution: &AssignmentPlan,
    _limits: ScalarGroupLimits,
) -> Vec<crate::planning::ScalarCandidate<AssignmentPlan>> {
    PROVIDER_CALLS.fetch_add(1, Ordering::SeqCst);
    let target = crate::planning::ScalarTarget::from_descriptor_index(0, "worker");
    (0..4)
        .map(|entity| {
            crate::planning::ScalarCandidate::new("batch", vec![target.set(entity, Some(entity))])
        })
        .collect()
}

fn candidate_group() -> ScalarGroupBinding<AssignmentPlan> {
    let slot = ScalarVariableSlot::new(
        0,
        0,
        "Task",
        entity_count,
        "worker",
        current_value,
        set_value,
        ValueSource::CountableRange { from: 0, to: 4 },
        true,
    );
    ScalarGroupBinding::bind(
        crate::planning::ScalarGroup::candidates(
            "static_group",
            vec![crate::planning::ScalarTarget::from_descriptor_index(
                0, "worker",
            )],
            counting_provider,
        ),
        &[slot],
    )
}

#[test]
fn candidate_kind_placements_stream_from_a_retained_provider_scan() {
    use crate::heuristic::selector::move_selector::MoveCursor;

    PROVIDER_CALLS.store(0, Ordering::SeqCst);
    let descriptor = descriptor();
    let group = candidate_group();
    let bindings = collect_bindings(&descriptor)
        .into_iter()
        .map(ResolvedVariableBinding::new)
        .collect();
    let plan = AssignmentPlan {
        score: None,
        assignments: vec![None; 4],
        candidates: vec![vec![]; 4],
    };
    let director = ScoreDirector::simple(plan, descriptor, |plan, _| plan.assignments.len());
    let placer = super::super::placer::ScalarGroupPlacer::new(
        0,
        group,
        bindings,
        ScalarGroupLimits::new(),
        ConstructionHeuristicType::FirstFit,
        ConstructionObligation::AssignWhenCandidateExists,
        false,
    );
    let mut cursor = placer.open_cursor(&director);
    let mut placements = 0usize;
    while let Some(mut placement) = cursor.next_placement(&director, |_| false, || false) {
        assert!(
            placement
                .candidates_mut()
                .next_candidate(director.working_solution())
                .is_some(),
            "each streamed placement must expose at least one prevalidated candidate"
        );
        placements += 1;
    }
    assert_eq!(placements, 4);
    assert_eq!(
        PROVIDER_CALLS.load(Ordering::SeqCst),
        1,
        "the provider must be scanned once, not once per placement pull"
    );
}
