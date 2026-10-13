use super::{ValueCycleCoordinates, ValuePairCoordinates, ValuePairKind};

#[test]
fn lazy_cycles_match_both_eager_orientations_through_rotation_wrap_and_exhaustion() {
    for size in [0, 1, 2, 3, 4, 9, 64] {
        for reverse in [false, true] {
            let mut values: Vec<_> = (0..size).map(|i| i * 13 + 5).collect();
            if reverse {
                values.reverse();
            }
            for offset in [0, 1, 2, 3, 17, 4096, usize::MAX] {
                let mut expected = Vec::new();
                for i in 0..size {
                    for j in i + 1..size {
                        for k in j + 1..size {
                            expected.push((values[i], values[j], values[k]));
                            expected.push((values[i], values[k], values[j]));
                        }
                    }
                }
                order_candidates(&mut expected, options(offset));
                let mut actual = ValueCycleCoordinates::new(values.to_vec(), options(offset));
                for triple in expected {
                    assert_eq!(actual.current(), Some(triple));
                    assert_eq!(actual.current(), Some(triple));
                    actual.advance();
                }
                for _ in 0..3 {
                    assert_eq!(actual.current(), None);
                    actual.advance();
                }
            }
        }
    }
}
use crate::phase::construction::grouped_scalar::assignment_candidate::{
    order_candidates, ScalarAssignmentMoveOptions,
};
use crate::planning::ScalarGroupLimits;

#[derive(Clone, Debug)]
struct ValuePlan {
    assignments: Vec<Option<usize>>,
    sequences: Vec<usize>,
    score: Option<solverforge_core::score::SoftScore>,
}

impl solverforge_core::domain::PlanningSolution for ValuePlan {
    type Score = solverforge_core::score::SoftScore;
    fn score(&self) -> Option<Self::Score> {
        self.score
    }
    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

fn binding() -> crate::builder::ScalarAssignmentBinding<ValuePlan> {
    use crate::builder::{
        bind_scalar_groups, ScalarGroupBindingKind, ScalarVariableSlot, ValueSource,
    };
    use crate::planning::{ScalarGroup, ScalarTarget};
    let slot = ScalarVariableSlot::new(
        0,
        0,
        "ValueRow",
        |s: &ValuePlan| s.assignments.len(),
        "value",
        |s, e, _| s.assignments[e],
        |s, e, _, v| s.assignments[e] = v,
        ValueSource::EntitySlice {
            values_for_entity: |_, _, _| &[0, 1, 2],
        },
        true,
    );
    let mut groups = bind_scalar_groups(
        vec![
            ScalarGroup::assignment("values", ScalarTarget::from_descriptor_index(0, "value"))
                .with_sequence_key(|s: &ValuePlan, e, _| Some(s.sequences[e])),
        ],
        &[slot],
    );
    match groups.remove(0).kind {
        ScalarGroupBindingKind::Assignment(binding) => binding,
        _ => panic!("expected assignment binding"),
    }
}

#[test]
fn real_value_cursors_preserve_complete_candidate_order() {
    use crate::phase::construction::grouped_scalar::{
        assignment_block::{
            ValueBlockReassignmentCursor, ValueLongWindowCursor, ValueWindowCursor,
        },
        assignment_state::ScalarAssignmentState,
        assignment_value_cycle::ValueWindowCycleCursor,
        assignment_value_index::assigned_value_sequence_index,
    };
    let binding = binding();
    let plan = ValuePlan {
        assignments: vec![Some(0), Some(1), Some(2), Some(0), Some(1), Some(2)],
        sequences: vec![0, 0, 0, 1, 1, 1],
        score: None,
    };
    for offset in [0, 1, 2, 5, usize::MAX] {
        let mut opts = options(offset);
        opts.max_depth = 2;
        opts.max_rematch_size = 2;
        let mut state = ScalarAssignmentState::new(&binding, &plan);
        let mut values = vec![0, 1, 2];
        order_candidates(&mut values, opts);
        let mut pairs = Vec::new();
        let mut directed = Vec::new();
        for i in 0..3 {
            for j in 0..3 {
                if i < j {
                    pairs.push((values[i], values[j]));
                }
                if i != j {
                    directed.push((values[i], values[j]));
                }
            }
        }
        order_candidates(&mut pairs, opts);
        order_candidates(&mut directed, opts);
        let mut window = ValueWindowCursor::new(
            opts,
            assigned_value_sequence_index(&binding, &plan, &state, opts),
        );
        let mut long = ValueLongWindowCursor::new(
            opts,
            assigned_value_sequence_index(&binding, &plan, &state, opts),
        );
        for (left, right) in pairs {
            for candidate in [
                window.next(&binding, &plan, &mut state),
                long.next(&binding, &plan, &mut state),
            ] {
                let candidate = candidate.expect("each pair swaps both sequences");
                assert_eq!(candidate.edits().len(), 4);
                for edit in candidate.edits() {
                    let from = plan.assignments[edit.entity_index].unwrap();
                    assert_eq!(edit.to_value, Some(if from == left { right } else { left }));
                }
            }
        }
        assert!(window.next(&binding, &plan, &mut state).is_none());
        assert!(long.next(&binding, &plan, &mut state).is_none());
        let mut block = ValueBlockReassignmentCursor::new(
            opts,
            assigned_value_sequence_index(&binding, &plan, &state, opts),
        );
        for (source, target) in directed {
            let candidate = block
                .next(&binding, &plan, &mut state)
                .expect("each directed pair reassigns a two-row block");
            assert_eq!(candidate.edits().len(), 2);
            for edit in candidate.edits() {
                assert_eq!(plan.assignments[edit.entity_index], Some(source));
                assert_eq!(edit.to_value, Some(target));
            }
        }
        assert!(block.next(&binding, &plan, &mut state).is_none());
        let mut triples = vec![
            (values[0], values[1], values[2]),
            (values[0], values[2], values[1]),
        ];
        order_candidates(&mut triples, opts);
        let mut cycle = ValueWindowCycleCursor::new(
            opts,
            assigned_value_sequence_index(&binding, &plan, &state, opts),
        );
        for (a, b, c) in triples {
            let candidate = cycle
                .next(&binding, &plan, &mut state)
                .expect("both cycle orientations cover both sequences");
            assert_eq!(candidate.edits().len(), 6);
            for edit in candidate.edits() {
                let from = plan.assignments[edit.entity_index].unwrap();
                assert_eq!(
                    edit.to_value,
                    Some(if from == a {
                        b
                    } else if from == b {
                        c
                    } else {
                        a
                    })
                );
            }
        }
        assert!(cycle.next(&binding, &plan, &mut state).is_none());
        assert_eq!(state.current_value(0), Some(0));
    }
}

#[test]
fn real_gap_swap_releases_an_island_after_filling_the_gap() {
    use crate::phase::construction::grouped_scalar::{
        assignment_state::ScalarAssignmentState,
        assignment_value_index::assigned_value_sequence_index,
        assignment_value_run::ValueRunGapSwapCursor,
    };
    let binding = binding();
    let plan = ValuePlan {
        assignments: vec![Some(0), Some(1), Some(0), Some(0)],
        sequences: vec![0, 1, 2, 4],
        score: None,
    };
    let mut state = ScalarAssignmentState::new(&binding, &plan);
    let mut cursor = ValueRunGapSwapCursor::new(
        options(0),
        assigned_value_sequence_index(&binding, &plan, &state, options(0)),
    );
    for released_entity in [0, 2, 3] {
        let candidate = cursor
            .next(&binding, &plan, &mut state)
            .expect("each isolated target row can be released after filling the gap");
        assert_eq!(
            candidate
                .edits()
                .iter()
                .map(|e| (e.entity_index, e.to_value))
                .collect::<Vec<_>>(),
            vec![(1, Some(0)), (released_entity, Some(1))]
        );
    }
    assert!(cursor.next(&binding, &plan, &mut state).is_none());
    assert!(cursor.next(&binding, &plan, &mut state).is_none());
}

#[test]
#[ignore = "release-mode coordinate setup/traversal benchmark"]
fn bench_value_coordinates() {
    use std::{hint::black_box, time::Instant};
    fn median(mut run: impl FnMut(usize) -> usize, repeats: usize) -> f64 {
        let mut samples = Vec::new();
        for _ in 0..9 {
            let start = Instant::now();
            for offset in 0..repeats {
                black_box(run(black_box(offset * 37)));
            }
            samples.push(start.elapsed().as_nanos() as f64 / repeats as f64);
        }
        samples.sort_by(f64::total_cmp);
        samples[4]
    }
    for n in [16, 128] {
        for kind in [ValuePairKind::Unordered, ValuePairKind::Directed] {
            let repeats = if n == 16 { 20000 } else { 500 };
            let before = median(
                |offset| {
                    let values: Vec<_> = (0..black_box(n)).map(|i| i * 7 + 3).collect();
                    let mut tuples = Vec::new();
                    for i in 0..n {
                        let start = if kind == ValuePairKind::Unordered {
                            i + 1
                        } else {
                            0
                        };
                        for j in start..n {
                            if i != j {
                                tuples.push((values[i], values[j]));
                            }
                        }
                    }
                    order_candidates(&mut tuples, options(offset));
                    tuples
                        .into_iter()
                        .fold(0usize, |sum, tuple| sum.wrapping_add(black_box(tuple).0))
                },
                repeats,
            );
            let after = median(
                |offset| {
                    let values: Vec<_> = (0..black_box(n)).map(|i| i * 7 + 3).collect();
                    let mut cursor = ValuePairCoordinates::new(values, kind, options(offset));
                    let mut sum = 0usize;
                    while let Some(tuple) = cursor.current() {
                        sum = sum.wrapping_add(black_box(tuple).0);
                        cursor.advance();
                    }
                    sum
                },
                repeats,
            );
            println!(
                "pair directed={} n={n} before_ns={before:.3} after_ns={after:.3} ratio={:.3}",
                kind == ValuePairKind::Directed,
                after / before
            );
        }
    }
    for n in [16, 64] {
        let repeats = if n == 16 { 5000 } else { 80 };
        let before = median(
            |offset| {
                let values: Vec<_> = (0..black_box(n)).map(|i| i * 7 + 3).collect();
                let mut tuples = Vec::new();
                for i in 0..n {
                    for j in i + 1..n {
                        for k in j + 1..n {
                            tuples.push((values[i], values[j], values[k]));
                            tuples.push((values[i], values[k], values[j]));
                        }
                    }
                }
                order_candidates(&mut tuples, options(offset));
                tuples
                    .into_iter()
                    .fold(0usize, |sum, tuple| sum.wrapping_add(black_box(tuple).0))
            },
            repeats,
        );
        let after = median(
            |offset| {
                let values: Vec<_> = (0..black_box(n)).map(|i| i * 7 + 3).collect();
                let mut cursor = ValueCycleCoordinates::new(values, options(offset));
                let mut sum = 0usize;
                while let Some(tuple) = cursor.current() {
                    sum = sum.wrapping_add(black_box(tuple).0);
                    cursor.advance();
                }
                sum
            },
            repeats,
        );
        println!(
            "cycle n={n} before_ns={before:.3} after_ns={after:.3} ratio={:.3}",
            after / before
        );
    }
}

fn options(offset: usize) -> ScalarAssignmentMoveOptions {
    ScalarAssignmentMoveOptions::for_selector(ScalarGroupLimits::new(), Some(1), 1, offset)
}

#[test]
fn lazy_cycle_count_can_exceed_machine_word_without_truncating() {
    let mut cursor = ValueCycleCoordinates::new((0..4_000_000).collect(), options(usize::MAX));
    for _ in 0..8 {
        let (first, second, third) = cursor.current().expect("the full cycle space remains live");
        assert!(first < second && first < third);
        assert_ne!(second, third);
        assert!(second < 4_000_000 && third < 4_000_000);
        cursor.advance();
    }
}

#[test]
fn lazy_pairs_match_eager_order_through_rotation_wrap_and_exhaustion() {
    for size in [0, 1, 2, 3, 7, 64, 129] {
        for reverse in [false, true] {
            let mut values: Vec<_> = (0..size).map(|i| i * 11 + 7).collect();
            if reverse {
                values.reverse();
            }
            for kind in [ValuePairKind::Unordered, ValuePairKind::Directed] {
                for offset in [0, 1, 2, 7, 63, 1024, usize::MAX] {
                    let mut expected = Vec::new();
                    for (i, &left) in values.iter().enumerate() {
                        for (j, &right) in values.iter().enumerate() {
                            if (kind == ValuePairKind::Unordered && i < j)
                                || (kind == ValuePairKind::Directed && left != right)
                            {
                                expected.push((left, right));
                            }
                        }
                    }
                    order_candidates(&mut expected, options(offset));
                    let mut actual =
                        ValuePairCoordinates::new(values.to_vec(), kind, options(offset));
                    for pair in expected {
                        assert_eq!(actual.current(), Some(pair));
                        assert_eq!(actual.current(), Some(pair), "peek must not advance");
                        actual.advance();
                    }
                    for _ in 0..3 {
                        assert_eq!(actual.current(), None);
                        actual.advance();
                    }
                }
            }
        }
    }
}
