use super::state::{new_stream_state, open_cursor_with_owned_stream_state};
use super::*;
use crate::heuristic::r#move::{CompoundScalarEdit, CompoundScalarMove};
use crate::heuristic::selector::decorator::ResourceVecUnionMoveCursor;
use crate::heuristic::selector::move_selector::{
    ArenaMoveCursor, ResourceMoveCursor, UnitResourceCursor,
};
use crate::heuristic::selector::nearby_list_change::DefaultCrossEntityDistanceMeter;
use crate::phase::localsearch::MoveCursorSource;
use crate::runtime::compiler::executor::local_search::RuntimeNeighborhoodMove;
use crate::test_utils::{create_nqueens_director, get_queen_row, set_queen_row, NQueensSolution};
use solverforge_config::{UnionSelectionOrder, UnionWeighting};

// Deliberately non-Clone: custom concrete leaves and retained state need no copying.
#[derive(Debug)]
struct CustomFlat {
    entity: usize,
    values: Vec<i64>,
}
struct CustomState {
    opens: usize,
}
type TestMove = RuntimeNeighborhoodMove<
    NQueensSolution,
    usize,
    DefaultCrossEntityDistanceMeter,
    DefaultCrossEntityDistanceMeter,
>;
fn get_row(solution: &NQueensSolution, entity: usize, variable: usize) -> Option<usize> {
    get_queen_row(solution, entity, variable).map(|value| value as usize)
}
fn set_row(solution: &mut NQueensSolution, entity: usize, variable: usize, value: Option<usize>) {
    set_queen_row(solution, entity, variable, value.map(|value| value as i64));
}

fn grouped_row_move(entity: usize, row: usize) -> TestMove {
    TestMove::Grouped(CompoundScalarMove::new(
        "test",
        vec![CompoundScalarEdit::static_edit(
            0,
            entity,
            0,
            "row",
            Some(row),
            get_row,
            set_row,
            None,
        )],
    ))
}
type Graph = SelectorComposition<NQueensSolution, TestMove, CustomFlat, CustomState>;

impl StatefulComposedFlat<NQueensSolution, TestMove, CustomState, ()> for CustomFlat {
    type Cursor<'a> = UnitResourceCursor<ArenaMoveCursor<NQueensSolution, TestMove>>;
    fn new_stream_state(&self) -> CustomState {
        CustomState { opens: 0 }
    }
    fn open_cursor_with_stream_state<'a, D: Director<NQueensSolution>>(
        &'a self,
        state: &mut CustomState,
        _: &mut (),
        director: &D,
        _: MoveStreamContext,
    ) -> Self::Cursor<'a> {
        state.opens += 1;
        let mut store = ArenaMoveCursor::new();
        for &value in &self.values {
            let value = if value == -1 {
                get_queen_row(director.working_solution(), 0, 0).unwrap() + 10
            } else {
                value
            };
            store.push(grouped_row_move(self.entity, value as usize));
        }
        UnitResourceCursor::new(store)
    }
    fn size<D: Director<NQueensSolution>>(&self, _: &D) -> usize {
        self.values.len()
    }
    fn validate_cursor<D: Director<NQueensSolution>>(&self, _: &D) {}
}

#[test]
fn arena_declarations_keep_stable_ids_and_concrete_nonclone_leaves() {
    let mut graph = Graph::new();
    let leaf = graph.push(SelectorCompositionNode::Flat(CustomFlat {
        entity: 0,
        values: vec![1, 2],
    }));
    let mut root = leaf;
    for _ in 0..100 {
        root = graph.push(SelectorCompositionNode::Limited {
            selector: root,
            selected_count_limit: 1,
        });
    }
    assert_eq!(leaf.index(), 0);
    assert_eq!(root.index(), 100);
    let mut director = create_nqueens_director(&[0]);
    let mut state = SelectorCompositionState::new(graph);
    let mut source = state.borrowed_source();
    let mut cursor = source.open_cursor(&mut (), &director, MoveStreamContext::default());
    let id = cursor
        .next_candidate_with_resources(&mut director, &mut ())
        .unwrap();
    assert!(cursor.candidate(id).is_some());
    assert!(cursor
        .next_candidate_with_resources(&mut director, &mut ())
        .is_none());
    assert!(cursor.release_candidate(id));
}

#[test]
fn arena_cartesian_defers_right_and_restores_exact_state_after_early_drop() {
    let mut graph = Graph::new();
    let left = graph.push(SelectorCompositionNode::Flat(CustomFlat {
        entity: 0,
        values: vec![1, 2],
    }));
    let right = graph.push(SelectorCompositionNode::Flat(CustomFlat {
        entity: 1,
        values: vec![-1],
    }));
    graph.push(SelectorCompositionNode::Cartesian {
        left,
        right,
        require_hard_improvement: false,
    });
    let mut director = create_nqueens_director(&[0, 0]);
    let mut state = SelectorCompositionState::new(graph);
    {
        let mut source = state.borrowed_source();
        let mut cursor = source.open_cursor(&mut (), &director, MoveStreamContext::default());
        let id = cursor
            .next_candidate_with_resources(&mut director, &mut ())
            .unwrap();
        let candidate = cursor.candidate(id).unwrap();
        let undo = candidate.do_move(&mut director);
        assert_eq!(get_queen_row(director.working_solution(), 1, 0), Some(11));
        candidate.undo_move(&mut director, undo);
        assert_eq!(get_queen_row(director.working_solution(), 0, 0), Some(0));
    }
    let mut source = state.borrowed_source();
    let mut cursor = source.open_cursor(&mut (), &director, MoveStreamContext::default());
    let first = cursor
        .next_candidate_with_resources(&mut director, &mut ())
        .unwrap();
    let second = cursor
        .next_candidate_with_resources(&mut director, &mut ())
        .unwrap();
    assert!(cursor.candidate(first).is_some());
    assert!(cursor.candidate(second).is_some());
    cursor.apply_owned_candidate(first, &mut director);
    assert!(cursor.candidate(first).is_none());
    assert_eq!(get_queen_row(director.working_solution(), 0, 0), Some(1));
    assert_eq!(get_queen_row(director.working_solution(), 1, 0), Some(11));
}

#[test]
fn arena_cartesian_lends_the_left_preview_to_lazy_right_pulls() {
    use crate::heuristic::selector::move_selector::{
        CandidateId, CandidateStore, MoveCandidateRef, MoveCursor,
    };
    #[derive(Debug)]
    struct LazyFlat {
        entity: usize,
        values: Vec<i64>,
    }
    struct LazyCursor<'a> {
        flat: &'a LazyFlat,
        position: usize,
        store: CandidateStore<NQueensSolution, TestMove>,
    }
    impl MoveCursor<NQueensSolution, TestMove> for LazyCursor<'_> {
        fn next_candidate(&mut self, solution: &NQueensSolution) -> Option<CandidateId> {
            let value = *self.flat.values.get(self.position)?;
            self.position += 1;
            let value = if value == -1 {
                get_queen_row(solution, 0, 0).unwrap() + 10
            } else {
                value
            };
            Some(
                self.store
                    .push(grouped_row_move(self.flat.entity, value as usize)),
            )
        }
        fn candidate(
            &self,
            id: CandidateId,
        ) -> Option<MoveCandidateRef<'_, NQueensSolution, TestMove>> {
            self.store.candidate(id)
        }
        fn take_candidate(&mut self, id: CandidateId) -> TestMove {
            self.store.take_candidate(id)
        }
        fn release_candidate(&mut self, id: CandidateId) -> bool {
            self.store.release_candidate(id)
        }
    }
    impl StatefulComposedFlat<NQueensSolution, TestMove, (), ()> for LazyFlat {
        type Cursor<'a> = UnitResourceCursor<LazyCursor<'a>>;
        fn new_stream_state(&self) {}
        fn open_cursor_with_stream_state<'a, D: Director<NQueensSolution>>(
            &'a self,
            _: &mut (),
            _: &mut (),
            _: &D,
            _: MoveStreamContext,
        ) -> Self::Cursor<'a> {
            UnitResourceCursor::new(LazyCursor {
                flat: self,
                position: 0,
                store: CandidateStore::new(),
            })
        }
        fn size<D: Director<NQueensSolution>>(&self, _: &D) -> usize {
            self.values.len()
        }
        fn validate_cursor<D: Director<NQueensSolution>>(&self, _: &D) {}
    }
    let mut graph = SelectorComposition::<NQueensSolution, TestMove, LazyFlat, ()>::new();
    let left = graph.push(SelectorCompositionNode::Flat(LazyFlat {
        entity: 0,
        values: vec![1, 2],
    }));
    let right = graph.push(SelectorCompositionNode::Flat(LazyFlat {
        entity: 1,
        values: vec![-1],
    }));
    graph.push(SelectorCompositionNode::Cartesian {
        left,
        right,
        require_hard_improvement: false,
    });
    let mut state = SelectorCompositionState::new(graph);
    let mut director = create_nqueens_director(&[0, 0]);
    crate::test_utils::reset_nqueens_clone_count();
    let mut source = state.borrowed_source();
    let mut cursor = source.open_cursor(&mut (), &director, MoveStreamContext::default());
    let mut observed = Vec::new();
    while let Some(id) = cursor.next_candidate_with_resources(&mut director, &mut ()) {
        let candidate = cursor.candidate(id).unwrap();
        let undo = candidate.do_move(&mut director);
        observed.push((
            get_queen_row(director.working_solution(), 0, 0),
            get_queen_row(director.working_solution(), 1, 0),
        ));
        candidate.undo_move(&mut director, undo);
        assert!(cursor.release_candidate(id));
        assert_eq!(get_queen_row(director.working_solution(), 0, 0), Some(0));
        assert_eq!(get_queen_row(director.working_solution(), 1, 0), Some(0));
    }
    assert_eq!(observed, vec![(Some(1), Some(11)), (Some(2), Some(12))]);
    assert_eq!(
        crate::test_utils::nqueens_clone_count(),
        0,
        "Cartesian discovery must use the live director"
    );
}

fn observe<C: ResourceMoveCursor<NQueensSolution, TestMove, ()>, D: Director<NQueensSolution>>(
    cursor: &mut C,
    director: &mut D,
) -> Vec<(Option<usize>, usize, i64)> {
    let mut values = Vec::new();
    while let Some(id) = cursor.next_candidate_with_resources(director, &mut ()) {
        let candidate = cursor.candidate(id).unwrap();
        let entity = candidate.entity_indices()[0];
        let undo = candidate.do_move(director);
        values.push((
            cursor.selector_index(id),
            entity,
            get_queen_row(director.working_solution(), entity, 0).unwrap(),
        ));
        candidate.undo_move(director, undo);
        assert!(cursor.release_candidate(id));
        assert!(cursor.candidate(id).is_none());
    }
    values
}

#[test]
fn arena_union_matches_canonical_scheduler_all_orders_weights_and_seeds() {
    for order in [
        UnionSelectionOrder::Sequential,
        UnionSelectionOrder::RoundRobin,
        UnionSelectionOrder::RotatingRoundRobin,
        UnionSelectionOrder::Random,
        UnionSelectionOrder::StratifiedRandom,
    ] {
        for weighting in [
            UnionWeighting::Equal,
            UnionWeighting::Fixed,
            UnionWeighting::CandidateCount,
        ] {
            if weighting != UnionWeighting::Equal
                && !matches!(
                    order,
                    UnionSelectionOrder::Random | UnionSelectionOrder::StratifiedRandom
                )
            {
                continue;
            }
            for seed in [0, 1, 17, 999] {
                let context = MoveStreamContext::new(7, seed, Some(3));
                let mut director = create_nqueens_director(&[0, 0, 0]);
                let mut graph = Graph::new();
                let children = (0..3)
                    .map(|entity| {
                        graph.push(SelectorCompositionNode::Flat(CustomFlat {
                            entity,
                            values: (1..=entity + 2).map(|value| value as i64).collect(),
                        }))
                    })
                    .collect::<Vec<_>>();
                let weights = if weighting == UnionWeighting::Fixed {
                    vec![1, 3, 2]
                } else {
                    vec![]
                };
                let sizes = vec![2, 3, 4];
                let resolved = crate::heuristic::selector::decorator::resolve_union_weights(
                    weighting, &weights, &sizes,
                );
                let cursors = children
                    .iter()
                    .map(|id| {
                        let SelectorCompositionNode::Flat(flat) = &graph.nodes[id.index()] else {
                            unreachable!()
                        };
                        flat.open_cursor_with_stream_state(
                            &mut CustomState { opens: 0 },
                            &mut (),
                            &director,
                            context,
                        )
                    })
                    .collect();
                let mut canonical =
                    ResourceVecUnionMoveCursor::new(cursors, order, context, resolved);
                let expected = observe(&mut canonical, &mut director);
                drop(canonical);
                graph.push(SelectorCompositionNode::Union {
                    selection_order: order,
                    weighting,
                    weights,
                    children,
                });
                let state = new_stream_state(&graph);
                let mut cursor =
                    open_cursor_with_owned_stream_state(&graph, state, &mut (), &director, context);
                assert_eq!(
                    observe(&mut cursor, &mut director),
                    expected,
                    "{order:?}/{weighting:?}/{seed}"
                );
            }
        }
    }
}

#[test]
fn arena_nested_cartesian_returns_retained_leaf_states_without_rebuilding() {
    let mut graph = Graph::new();
    let a = graph.push(SelectorCompositionNode::Flat(CustomFlat {
        entity: 0,
        values: vec![1, 2],
    }));
    let b = graph.push(SelectorCompositionNode::Flat(CustomFlat {
        entity: 1,
        values: vec![-1],
    }));
    let ab = graph.push(SelectorCompositionNode::Cartesian {
        left: a,
        right: b,
        require_hard_improvement: false,
    });
    let c = graph.push(SelectorCompositionNode::Flat(CustomFlat {
        entity: 2,
        values: vec![-1],
    }));
    graph.push(SelectorCompositionNode::Cartesian {
        left: ab,
        right: c,
        require_hard_improvement: true,
    });
    let mut director = create_nqueens_director(&[0, 0, 0]);
    crate::test_utils::reset_nqueens_clone_count();
    let state = new_stream_state(&graph);
    let independent = new_stream_state(&graph);
    let leaves_ptr = state.leaves.as_ptr();
    let mut cursor = open_cursor_with_owned_stream_state(
        &graph,
        state,
        &mut (),
        &director,
        MoveStreamContext::default(),
    );
    let first = cursor
        .next_candidate_with_resources(&mut director, &mut ())
        .unwrap();
    assert_eq!(
        crate::test_utils::nqueens_clone_count(),
        0,
        "nested Cartesian carriers must preserve live preview"
    );
    let candidate = cursor.candidate(first).unwrap();
    assert!(candidate.is_doable_with_working_preview(&mut director));
    assert_eq!(
        crate::test_utils::nqueens_clone_count(),
        0,
        "nested candidate evaluation must preserve live preview"
    );
    assert!(candidate.requires_hard_improvement());
    assert_eq!(candidate.entity_indices(), &[0, 1, 2]);
    let undo = candidate.do_move(&mut director);
    assert_eq!(
        director
            .working_solution()
            .queens
            .iter()
            .map(|q| q.row)
            .collect::<Vec<_>>(),
        vec![Some(1), Some(11), Some(11)]
    );
    candidate.undo_move(&mut director, undo);
    let state = cursor.into_stream_state();
    assert_eq!(state.leaves.as_ptr(), leaves_ptr);
    assert_eq!(state.leaves[a.index()].as_ref().unwrap().opens, 1);
    assert_eq!(state.leaves[b.index()].as_ref().unwrap().opens, 1);
    assert_eq!(state.leaves[c.index()].as_ref().unwrap().opens, 1);
    assert_eq!(independent.leaves[a.index()].as_ref().unwrap().opens, 0);
    let mut reopened = open_cursor_with_owned_stream_state(
        &graph,
        state,
        &mut (),
        &director,
        MoveStreamContext::default(),
    );
    let first = reopened
        .next_candidate_with_resources(&mut director, &mut ())
        .unwrap();
    let second = reopened
        .next_candidate_with_resources(&mut director, &mut ())
        .unwrap();
    assert!(reopened.candidate(first).is_some());
    assert!(reopened.candidate(second).is_some());
    assert!(reopened.release_candidate(second));
    let selected = reopened.take_candidate(first);
    assert!(selected.requires_hard_improvement());
    assert!(reopened.candidate(first).is_none());
    let undo = selected.do_move(&mut director);
    selected.undo_move(&mut director, undo);
    assert_eq!(
        director
            .working_solution()
            .queens
            .iter()
            .map(|q| q.row)
            .collect::<Vec<_>>(),
        vec![Some(0); 3]
    );
    let state = reopened.into_stream_state();
    assert_eq!(state.leaves.as_ptr(), leaves_ptr);
    assert_eq!(state.leaves[a.index()].as_ref().unwrap().opens, 2);
    assert_eq!(state.leaves[b.index()].as_ref().unwrap().opens, 3);
    assert_eq!(state.leaves[c.index()].as_ref().unwrap().opens, 3);
}

#[test]
#[should_panic(expected = "composition leaves cannot share mutable state")]
fn arena_rejects_shared_mutable_child_nodes() {
    let mut graph = Graph::new();
    let leaf = graph.push(SelectorCompositionNode::Flat(CustomFlat {
        entity: 0,
        values: vec![1],
    }));
    graph.push(SelectorCompositionNode::Cartesian {
        left: leaf,
        right: leaf,
        require_hard_improvement: false,
    });
    let _ = SelectorCompositionState::<_, _, _, _, ()>::new(graph);
}
