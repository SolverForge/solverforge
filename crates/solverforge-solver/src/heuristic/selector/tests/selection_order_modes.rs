//! Exhaustive finite SelectionOrder semantics over a move selector.
//!
//! For every SelectionOrder mode the cursor's pulled stream must (a) cover the
//! same finite candidate set the canonical stream covers (with-replacement
//! modes may repeat, without-replacement modes may not), (b) stay
//! deterministic and reproduce exactly on replay per (seed, step), and
//! (c) `size()` must equal the covered count without pulling.

use solverforge_core::domain::{
    EntityCollectionExtractor, EntityDescriptor, PlanningSolution, SolutionDescriptor,
};
use solverforge_core::score::SoftScore;
use solverforge_scoring::ScoreDirector;
use std::any::TypeId;

use crate::heuristic::selector::move_selector::{
    ChangeMoveSelector, MoveCursor, MoveSelector, MoveStreamContext,
};
use crate::heuristic::selector::SelectionOrder;

#[derive(Clone, Debug)]
struct OrderTask {
    priority: Option<usize>,
}

#[derive(Clone, Debug)]
struct OrderSolution {
    tasks: Vec<OrderTask>,
    score: Option<SoftScore>,
}

impl PlanningSolution for OrderSolution {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

fn order_get(s: &OrderSolution, idx: usize, _variable_index: usize) -> Option<usize> {
    s.tasks.get(idx).and_then(|task| task.priority)
}

fn order_set(s: &mut OrderSolution, idx: usize, _variable_index: usize, value: Option<usize>) {
    if let Some(task) = s.tasks.get_mut(idx) {
        task.priority = value;
    }
}

fn director(assignments: &[Option<usize>]) -> ScoreDirector<OrderSolution, ()> {
    let extractor = Box::new(EntityCollectionExtractor::new(
        "OrderTask",
        "tasks",
        |s: &OrderSolution| &s.tasks,
        |s: &mut OrderSolution| &mut s.tasks,
    ));
    let entity_desc = EntityDescriptor::new("OrderTask", TypeId::of::<OrderTask>(), "tasks")
        .with_extractor(extractor);
    let descriptor = SolutionDescriptor::new("OrderSolution", TypeId::of::<OrderSolution>())
        .with_entity(entity_desc);
    ScoreDirector::simple(
        OrderSolution {
            tasks: assignments
                .iter()
                .map(|priority| OrderTask {
                    priority: *priority,
                })
                .collect(),
            score: None,
        },
        descriptor,
        |s, _| s.tasks.len(),
    )
}

/// (entity, to_value) pairs in pull order.
fn pull_stream(
    selector: &ChangeMoveSelector<
        OrderSolution,
        usize,
        crate::heuristic::selector::FromSolutionEntitySelector,
        crate::heuristic::selector::StaticValueSelector<OrderSolution, usize>,
    >,
    director: &mut ScoreDirector<OrderSolution, ()>,
    context: MoveStreamContext,
) -> Vec<(usize, Option<usize>)> {
    let mut cursor = selector.open_cursor_with_context(director, context);
    let mut stream = Vec::new();
    while let Some(id) = cursor.next_candidate(director.working_solution()) {
        let mov = cursor
            .candidate(id)
            .expect("finite order candidate must remain live");
        let crate::heuristic::selector::move_selector::MoveCandidateRef::Borrowed(change) = &mov
        else {
            panic!("finite change candidates are borrowed change moves");
        };
        stream.push((change.entity_index(), change.to_value().copied()));
        assert!(cursor.release_candidate(id));
    }
    stream
}
#[test]
fn all_selection_orders_cover_and_replay_the_finite_stream() {
    let assignments = vec![None, Some(10), None, Some(12), None];
    let mut director = director(&assignments);
    let selector =
        ChangeMoveSelector::simple(order_get, order_set, 0, 0, "priority", vec![10, 11, 12]);

    let canonical = pull_stream(
        &selector,
        &mut director,
        MoveStreamContext::new(0, 0x5EED, None),
    );
    // Canonical coverage: the stream is the full entities x values cross
    // product in canonical order; no-op suppression belongs to move
    // doability, not to the order machinery under test.
    let mut expected = Vec::new();
    for (entity, _) in assignments.iter().enumerate() {
        for value in [10usize, 11, 12] {
            expected.push((entity, Some(value)));
        }
    }
    assert_eq!(canonical, expected);

    for order in [
        SelectionOrder::Original,
        SelectionOrder::Random,
        SelectionOrder::Shuffled,
        SelectionOrder::Sorted,
        SelectionOrder::Probabilistic,
    ] {
        let context = MoveStreamContext::new(3, 0x5EED, None).with_selection_order(order);
        let stream = pull_stream(&selector, &mut director, context);

        // Every pull stays inside the finite candidate set.
        let covered: std::collections::BTreeSet<(usize, Option<usize>)> =
            stream.iter().copied().collect();
        assert!(
            covered.is_subset(&expected.iter().copied().collect()),
            "{order:?}: draws must come from the finite candidate set"
        );
        // Shuffled is without-replacement so one cycle covers every
        // coordinate; Random selects with replacement, so coordinates may
        // repeat OR be skipped within a cycle without changing the slot count.
        assert_eq!(stream.len(), expected.len());
        let unique: std::collections::BTreeSet<_> = stream.iter().copied().collect();
        if matches!(order, SelectionOrder::Random) {
            assert!(
                unique.len() <= expected.len(),
                "{order:?}: replacement may skip coordinates"
            );
        } else {
            assert_eq!(
                unique.len(),
                expected.len(),
                "{order:?}: canonical-coverage modes must visit every coordinate exactly once"
            );
        }
        // size() equals the covered count for finite selectors under all
        // modes (streaming never changes the neighborhood's finite size).
        assert_eq!(
            selector.size(&director),
            expected.len(),
            "{order:?}: size() must count the same finite candidates"
        );
        // Deterministic replay.
        let replay = pull_stream(&selector, &mut director, context);
        assert_eq!(
            replay, stream,
            "{order:?}: seeded replay must reproduce the stream"
        );
        // Seeded replay with an identical seed stream must hold for a second
        // step with the same seed too.
        let again = pull_stream(&selector, &mut director, context);
        assert_eq!(again, stream);
    }
}

#[test]
fn random_and_shuffled_orders_differ_by_replacement_shape() {
    let assignments = vec![None; 6];
    let mut director = director(&assignments);
    let selector = ChangeMoveSelector::simple(order_get, order_set, 0, 0, "priority", vec![7, 8]);
    let canonical: std::collections::BTreeSet<(usize, Option<usize>)> = (0..6usize)
        .flat_map(|entity| {
            [Some(7usize), Some(8)]
                .into_iter()
                .map(move |value| (entity, value))
        })
        .collect();
    let random = pull_stream(
        &selector,
        &mut director,
        MoveStreamContext::new(1, 0xABCDEF, None).with_selection_order(SelectionOrder::Random),
    );
    let shuffled = pull_stream(
        &selector,
        &mut director,
        MoveStreamContext::new(1, 0xABCDEF, None).with_selection_order(SelectionOrder::Shuffled),
    );
    // Random draws one coordinate per canonical slot WITH REPLACEMENT: the
    // stream length matches the canonical count, every draw stays in the
    // finite candidate set, and one cycle may legally skip coordinates.
    assert_eq!(random.len(), 12, "Random keeps one draw per canonical slot");
    assert!(
        random
            .iter()
            .all(|coordination| canonical.contains(coordination)),
        "Random draws must come from the finite candidate set"
    );
    // Shuffled is a without-replacement source permutation: every coordinate
    // exactly once per cycle.
    assert_eq!(
        shuffled.len(),
        12,
        "Shuffled visits each source coordinate once"
    );
    let unique_shuffled: std::collections::BTreeSet<_> = shuffled.iter().copied().collect();
    assert_eq!(
        unique_shuffled, canonical,
        "Shuffled must not repeat coordinates"
    );
    // Seeded replay reproduces both draws exactly.
    let random_replay = pull_stream(
        &selector,
        &mut director,
        MoveStreamContext::new(1, 0xABCDEF, None).with_selection_order(SelectionOrder::Random),
    );
    assert_eq!(random_replay, random);
    let shuffled_replay = pull_stream(
        &selector,
        &mut director,
        MoveStreamContext::new(1, 0xABCDEF, None).with_selection_order(SelectionOrder::Shuffled),
    );
    assert_eq!(shuffled_replay, shuffled);
}
