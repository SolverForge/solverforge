use super::*;

#[test]
fn augmenting_assignment_observes_control_during_recursive_search() {
    let descriptor = descriptor();
    let limits = ScalarGroupLimits {
        max_augmenting_depth: Some(4),
        max_rematch_size: Some(8),
        ..ScalarGroupLimits::new()
    };
    let assignment = assignment_binding(&descriptor, limits);
    let solution = AssignmentPlan {
        score: None,
        assignments: vec![Some(0), Some(1), None],
        candidates: vec![vec![0, 2], vec![1, 2], vec![0, 1]],
    };
    let mut state = ScalarAssignmentState::new(&assignment, &solution);
    let mut polls = 0;
    let candidate = assignment_move_for_entity_value(
        &assignment,
        &solution,
        &mut state,
        AssignmentRequest::root(2, 0, limits.max_augmenting_depth.unwrap_or(3)),
        ScalarAssignmentMoveOptions::for_construction(limits),
        AssignmentMoveIntent::required(),
        &mut || {
            polls += 1;
            polls >= 5
        },
    );

    assert!(candidate.is_none());
    assert!(polls >= 5, "control must be polled inside recursion");
    assert_eq!(state.current_value(0), Some(0));
    assert_eq!(state.current_value(1), Some(1));
    assert_eq!(state.current_value(2), None);
}

#[test]
fn interrupted_required_batch_can_resume_without_partial_state() {
    let descriptor = descriptor();
    let limits = ScalarGroupLimits {
        max_augmenting_depth: Some(4),
        max_rematch_size: Some(8),
        ..ScalarGroupLimits::new()
    };
    let assignment = assignment_binding(&descriptor, limits);
    let entity_total = 100;
    let solution = AssignmentPlan {
        score: None,
        assignments: vec![None; entity_total],
        candidates: (0..entity_total).map(|value| vec![value]).collect(),
    };
    let options = ScalarAssignmentMoveOptions::for_construction(limits);
    let mut reference =
        ScalarAssignmentMoveCursor::required_construction(&assignment, &solution, options);
    let mut reference_polls = 0;
    let complete = reference
        .next_move_with_control(&assignment, &solution, &mut || {
            reference_polls += 1;
            false
        })
        .expect("uninterrupted required batch must be generated");
    assert_eq!(complete.edits().len(), entity_total);

    let mut cursor =
        ScalarAssignmentMoveCursor::required_construction(&assignment, &solution, options);
    let mut polls = 0;

    let interrupted = cursor.next_move_with_control(&assignment, &solution, &mut || {
        polls += 1;
        polls >= reference_polls
    });
    assert!(interrupted.is_none());

    let resumed = cursor
        .next_move_with_control(&assignment, &solution, &mut || false)
        .expect("required batch must restart after interrupted generation");
    assert_eq!(resumed.edits().len(), entity_total);
}

static PREPROCESSING_CANDIDATE_READS: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

fn tracked_preprocessing_candidates(plan: &AssignmentPlan, entity: usize, _: usize) -> &[usize] {
    let previous = PREPROCESSING_CANDIDATE_READS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    assert_eq!(
        previous, 0,
        "degree preprocessing read candidates after control requested interruption"
    );
    &plan.candidates[entity]
}

#[test]
fn required_batch_polls_control_between_scarcity_and_degree_preprocessing() {
    PREPROCESSING_CANDIDATE_READS.store(0, std::sync::atomic::Ordering::SeqCst);
    let descriptor = descriptor();
    let limits = ScalarGroupLimits::new();
    let assignment = assignment_binding_with_candidate_values(
        &descriptor,
        limits,
        tracked_preprocessing_candidates,
    );
    let solution = AssignmentPlan {
        score: None,
        assignments: vec![None],
        candidates: vec![vec![0]],
    };
    let options = ScalarAssignmentMoveOptions::for_construction(limits);
    let mut cursor =
        ScalarAssignmentMoveCursor::required_construction(&assignment, &solution, options);
    let interrupted = cursor.next_move_with_control(&assignment, &solution, &mut || {
        PREPROCESSING_CANDIDATE_READS.load(std::sync::atomic::Ordering::SeqCst) >= 1
    });

    assert!(interrupted.is_none());
    assert_eq!(
        PREPROCESSING_CANDIDATE_READS.load(std::sync::atomic::Ordering::SeqCst),
        1
    );
}

#[test]
fn required_value_degrees_bounds_control_polling_for_dense_rows() {
    let descriptor = descriptor();
    let limits = ScalarGroupLimits::new();
    let assignment = assignment_binding(&descriptor, limits);
    let candidate_count = GENERATION_POLL_INTERVAL * 2;
    let solution = AssignmentPlan {
        score: None,
        assignments: vec![None],
        candidates: vec![(0..candidate_count).collect()],
    };
    let mut polls = 0;

    let degrees = required_value_degrees(&assignment, &solution, &[0], None, &mut || {
        polls += 1;
        false
    })
    .expect("uninterrupted degree preprocessing must complete");

    assert_eq!(degrees.len(), candidate_count);
    assert_eq!(polls, 4, "poll once per row, interval, and final boundary");
}
