// A zero fixed union weight disables its child.

use super::*;

fn leaf(graph: &mut Graph, entity: usize, len: usize) -> CompositionNodeId {
    graph.push(SelectorCompositionNode::Flat(CustomFlat {
        entity,
        values: (1..=len).map(|value| value as i64).collect(),
    }))
}

fn observe_graph(graph: &Graph, context: MoveStreamContext) -> Vec<(usize, i64)> {
    let mut director = create_nqueens_director(&[0, 0, 0]);
    let state = new_stream_state(graph);
    let mut cursor = open_cursor_with_owned_stream_state(graph, state, &mut (), &director, context);
    observe(&mut cursor, &mut director)
        .into_iter()
        .map(|(_, entity, row)| (entity, row))
        .collect()
}

#[test]
fn a_disabled_child_produces_no_candidates() {
    for seed in [0, 1, 17, 999] {
        let context = MoveStreamContext::new(7, seed, None);
        let mut graph = Graph::new();
        let children = vec![
            leaf(&mut graph, 0, 2),
            leaf(&mut graph, 1, 3),
            leaf(&mut graph, 2, 4),
        ];
        graph.push(SelectorCompositionNode::Union {
            selection_order: UnionSelectionOrder::StratifiedRandom,
            weighting: UnionWeighting::Fixed,
            weights: vec![2, 0, 1],
            children,
        });
        let seen = observe_graph(&graph, context);
        assert_eq!(seen.len(), 2 + 4, "{seen:?}");
        assert!(seen.iter().all(|(entity, _)| *entity != 1), "{seen:?}");
    }
}

#[test]
fn a_disabled_child_is_not_counted_in_the_union_size() {
    // The outer union weighs its children by candidate count. Its inner
    // union disables a three-candidate child, so it must weigh like a union
    // of the two-candidate child alone.
    for seed in [0, 1, 17, 999, 4242] {
        let context = MoveStreamContext::new(3, seed, None);

        let mut with_disabled = Graph::new();
        let inner_children = vec![
            leaf(&mut with_disabled, 0, 2),
            leaf(&mut with_disabled, 1, 3),
        ];
        let inner = with_disabled.push(SelectorCompositionNode::Union {
            selection_order: UnionSelectionOrder::StratifiedRandom,
            weighting: UnionWeighting::Fixed,
            weights: vec![1, 0],
            children: inner_children,
        });
        let other = leaf(&mut with_disabled, 2, 8);
        with_disabled.push(SelectorCompositionNode::Union {
            selection_order: UnionSelectionOrder::StratifiedRandom,
            weighting: UnionWeighting::CandidateCount,
            weights: vec![],
            children: vec![inner, other],
        });

        let mut without = Graph::new();
        let only = leaf(&mut without, 0, 2);
        let inner = without.push(SelectorCompositionNode::Union {
            selection_order: UnionSelectionOrder::StratifiedRandom,
            weighting: UnionWeighting::Fixed,
            weights: vec![1],
            children: vec![only],
        });
        let other = leaf(&mut without, 2, 8);
        without.push(SelectorCompositionNode::Union {
            selection_order: UnionSelectionOrder::StratifiedRandom,
            weighting: UnionWeighting::CandidateCount,
            weights: vec![],
            children: vec![inner, other],
        });

        assert_eq!(
            observe_graph(&with_disabled, context),
            observe_graph(&without, context),
            "seed {seed}"
        );
    }
}
