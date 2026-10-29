//! A prebuilt composite signature is lent through every wrapper layer.
//!
//! Each assertion checks that the returned `Cow` borrows the composite's own
//! stored signature, so no layer rebuilds or copies it.

use std::borrow::Cow;

use crate::heuristic::r#move::metadata::MoveTabuScope;
use crate::heuristic::r#move::{
    ChangeMove, CompoundScalarEdit, CompoundScalarMove, Move, MoveTabuSignature,
    SequentialCompositeMove, SequentialCompositeMoveRef,
};
use crate::heuristic::selector::move_selector::MoveCandidateRef;
use crate::heuristic::selector::nearby_list_change::DefaultCrossEntityDistanceMeter;
use crate::runtime::compiler::executor::local_search::RuntimeNeighborhoodMove;
use crate::test_utils::{create_nqueens_director, get_queen_row, set_queen_row, NQueensSolution};

type QueenMove = ChangeMove<NQueensSolution, i64>;
type RuntimeMove = RuntimeNeighborhoodMove<
    NQueensSolution,
    usize,
    DefaultCrossEntityDistanceMeter,
    DefaultCrossEntityDistanceMeter,
>;

fn queen_move(entity: usize, row: i64) -> QueenMove {
    ChangeMove::new(entity, Some(row), get_queen_row, set_queen_row, 0, "row", 0)
}

fn get_row(solution: &NQueensSolution, entity: usize, variable: usize) -> Option<usize> {
    get_queen_row(solution, entity, variable).map(|value| value as usize)
}

fn set_row(solution: &mut NQueensSolution, entity: usize, variable: usize, value: Option<usize>) {
    set_queen_row(solution, entity, variable, value.map(|value| value as i64));
}

fn runtime_move(entity: usize, row: usize) -> RuntimeMove {
    RuntimeMove::Grouped(CompoundScalarMove::new(
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

fn composite_signature() -> MoveTabuSignature {
    MoveTabuSignature::new(
        MoveTabuScope::new(0, "row"),
        smallvec::smallvec![7],
        smallvec::smallvec![8],
    )
}

fn assert_lends(signature: Cow<'_, MoveTabuSignature>, stored: &MoveTabuSignature) {
    match signature {
        Cow::Borrowed(lent) => assert!(
            std::ptr::eq(lent, stored),
            "the lent signature must be the composite's stored signature"
        ),
        Cow::Owned(_) => panic!("a prebuilt composite signature must be lent, not rebuilt"),
    }
}

#[test]
fn owned_composite_lends_through_reference_and_candidate_wrappers() {
    let director = create_nqueens_director(&[0, 1]);
    let composite = SequentialCompositeMove::new(
        queen_move(0, 2),
        queen_move(1, 3),
        0,
        smallvec::smallvec![0, 1],
        "row",
        composite_signature(),
    );
    let stored = match composite.tabu_signature(&director) {
        Cow::Borrowed(stored) => stored,
        Cow::Owned(_) => panic!("an owned composite must lend its stored signature"),
    };

    let by_reference = &composite;
    assert_lends(Move::tabu_signature(&by_reference, &director), stored);
    let candidate = MoveCandidateRef::Borrowed(&composite);
    assert_lends(candidate.tabu_signature(&director), stored);
}

#[test]
fn borrowed_sequential_candidate_lends_the_cursor_signature() {
    let director = create_nqueens_director(&[0, 1]);
    let left = queen_move(0, 2);
    let right = queen_move(1, 3);
    let stored = composite_signature();
    let candidate = MoveCandidateRef::Sequential(SequentialCompositeMoveRef::new(
        &left,
        &right,
        0,
        &[0, 1],
        "row",
        &stored,
        false,
    ));

    assert_lends(candidate.tabu_signature(&director), &stored);
}

#[test]
fn compiled_runtime_sequential_move_lends_its_signature() {
    let director = create_nqueens_director(&[0, 1]);
    let selected = RuntimeMove::Sequential(SequentialCompositeMove::new(
        runtime_move(0, 2),
        runtime_move(1, 3),
        0,
        smallvec::smallvec![0, 1],
        "row",
        composite_signature(),
    ));
    let RuntimeMove::Sequential(composite) = &selected else {
        unreachable!("constructed as a sequential carrier")
    };
    let stored = match composite.tabu_signature(&director) {
        Cow::Borrowed(stored) => stored,
        Cow::Owned(_) => panic!("an owned composite must lend its stored signature"),
    };

    assert_lends(selected.tabu_signature(&director), stored);
}

#[test]
fn leaf_moves_build_an_owned_signature() {
    let director = create_nqueens_director(&[0]);

    assert!(matches!(
        queen_move(0, 2).tabu_signature(&director),
        Cow::Owned(_)
    ));
}
