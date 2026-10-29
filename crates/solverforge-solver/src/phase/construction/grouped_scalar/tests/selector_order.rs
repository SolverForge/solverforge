use super::*;

#[test]
fn selector_candidate_dimensions_share_one_rotation_without_cloning() {
    #[derive(Debug, Eq, PartialEq)]
    struct NonCloneCandidate(u8);

    let context = MoveStreamContext::new(17, 23, None).with_selection_order(SelectionOrder::Random);
    let entity_offset = context
        .offset_seed(0xC0A1_E5CE_AAA0_0002)
        .wrapping_add(context.step_index() as usize);
    let options = ScalarAssignmentMoveOptions::for_selector(
        ScalarGroupLimits::new(),
        None,
        256,
        entity_offset,
    );
    let mut short = (0..5).map(NonCloneCandidate).collect::<Vec<_>>();
    let mut long = (0..8).map(NonCloneCandidate).collect::<Vec<_>>();
    let mut expected_short = (0..5).map(NonCloneCandidate).collect::<Vec<_>>();
    let mut expected_long = (0..8).map(NonCloneCandidate).collect::<Vec<_>>();
    let short_len = expected_short.len();
    let long_len = expected_long.len();
    expected_short.rotate_left(options.entity_offset % short_len);
    expected_long.rotate_left(options.entity_offset % long_len);

    order_candidates(&mut short, options);
    order_candidates(&mut long, options);

    assert_eq!(short, expected_short);
    assert_eq!(long, expected_long);
}

#[test]
fn original_selector_order_keeps_assignment_candidates_canonical() {
    let options = ScalarAssignmentMoveOptions::for_selector(ScalarGroupLimits::new(), None, 256, 0);
    let mut candidates = vec![0, 1, 2, 3];

    order_candidates(&mut candidates, options);

    assert_eq!(candidates, [0, 1, 2, 3]);
}
