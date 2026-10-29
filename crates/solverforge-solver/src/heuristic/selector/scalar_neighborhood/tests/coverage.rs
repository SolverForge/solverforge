use super::*;
use crate::heuristic::selector::scalar_neighborhood::spec::ScalarNeighborhoodSpec;

#[test]
fn scalar_change_coverage_across_all_selection_orders_matches_drained_size() {
    use super::super::ScalarNeighborhoodLeaf;
    use crate::builder::RuntimeScalarSlot;
    use crate::heuristic::selector::move_selector::{MoveCursor, MoveSelector, MoveStreamContext};
    use solverforge_config::SelectionOrder;

    let order_modes = [
        SelectionOrder::Original,
        SelectionOrder::Random,
        SelectionOrder::Shuffled,
        SelectionOrder::Sorted,
        SelectionOrder::Probabilistic,
    ];
    for (mode_index, mode) in order_modes.iter().enumerate() {
        let director = director(Plan {
            values: vec![Some(0), None, Some(1), Some(1)],
            candidates: vec![0, 1, 2],
            score: None,
        });
        let leaf = ScalarNeighborhoodLeaf::from_spec(
            ScalarNeighborhoodSpec::Change {
                value_candidate_limit: None,
            },
            RuntimeScalarSlot::Dynamic(slot()),
            Some(0x50 + mode_index as u64),
        )
        .expect("test slot supplies the change capability");
        let expected_size = MoveSelector::size(&leaf, &director);
        let mut state = leaf.new_stream_state();
        let mut cursor = leaf.open_cursor_with_stream_state(
            &mut state,
            &director,
            MoveStreamContext::default().with_selection_order(*mode),
        );
        let mut drained = 0usize;
        while cursor
            .next_owned_candidate(director.working_solution())
            .is_some()
        {
            drained += 1;
        }
        assert_eq!(
            drained, expected_size,
            "order {mode:?}: the drained stream must match the finite size() count without a \
             second eager collector",
        );
    }
}

#[test]
fn in_place_usize_order_gather_matches_canonical_rotation() {
    use crate::heuristic::selector::move_selector::MoveStreamContext;
    use solverforge_config::SelectionOrder;

    let contexts = [
        MoveStreamContext::default().with_selection_order(SelectionOrder::Random),
        MoveStreamContext::default().with_selection_order(SelectionOrder::Shuffled),
    ];
    let mut reference = 0u64;
    for (index, context) in contexts.iter().enumerate() {
        for len in [2usize, 3, 8, 17, 64] {
            let canonical: Vec<usize> = (0..len)
                .map(|offset| 100 + offset * 7 % (len + 3))
                .collect();
            let mut expected = canonical.clone();
            context.apply_selection_order(&mut expected, 0xAB ^ len as u64);
            let mut actual = canonical.clone();
            context.apply_selection_order_usize(&mut actual, 0xAB ^ len as u64);
            assert_eq!(
                actual, expected,
                "in-place gather must equal the canonical rotation (order {index}, len {len})"
            );
            let _ = &mut reference;
        }
    }
}
