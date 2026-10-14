//! Sequential-composite doability runs inside an explicit working preview.
//!
//! The working preview applies the left leg on the live director, propagates
//! shadows, checks the right leg, undoes the left leg, and restores score
//! state before it returns. No solution clone services a preview; pre/post
//! state is exact on every path, including a right leg whose legality reads
//! the previewed state.

use super::*;
use crate::heuristic::r#move::compound_scalar::ScalarEditLegality;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug)]
struct PreviewSolution {
    rows: Vec<Option<usize>>,
    clones: usize,
    score: Option<SoftScore>,
}

impl PlanningSolution for PreviewSolution {
    type Score = SoftScore;
    fn score(&self) -> Option<Self::Score> {
        self.score
    }
    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

static PREVIEW_SOLUTION_CLONES: AtomicUsize = AtomicUsize::new(0);

impl Clone for PreviewSolution {
    fn clone(&self) -> Self {
        PREVIEW_SOLUTION_CLONES.fetch_add(1, Ordering::SeqCst);
        Self {
            clones: self.clones + 1,
            rows: self.rows.clone(),
            score: self.score,
        }
    }
}

fn row_get(s: &PreviewSolution, idx: usize, _variable_index: usize) -> Option<usize> {
    s.rows.get(idx).copied().flatten()
}

fn row_set(s: &mut PreviewSolution, idx: usize, _variable_index: usize, v: Option<usize>) {
    if let Some(row) = s.rows.get_mut(idx) {
        *row = v;
    }
}

// Row 1 may hold value 2 only after row 0 vacated value 1.
fn two_needs_vacated_one(
    s: &PreviewSolution,
    _entity_index: usize,
    _variable_index: usize,
    value: Option<usize>,
) -> bool {
    value != Some(2) || s.rows.first().copied().flatten() != Some(1)
}

fn compound_edit(
    entity_index: usize,
    to_value: Option<usize>,
    legality: Option<ScalarEditLegality<PreviewSolution>>,
) -> CompoundScalarEdit<PreviewSolution> {
    CompoundScalarEdit::static_edit(
        0,
        entity_index,
        0,
        "rows",
        to_value,
        row_get,
        row_set,
        legality,
    )
}

fn create_director(rows: Vec<Option<usize>>) -> ScoreDirector<PreviewSolution, ()> {
    let solution = PreviewSolution {
        rows,
        clones: 0,
        score: None,
    };
    let extractor = Box::new(EntityCollectionExtractor::new(
        "Row",
        "rows",
        |s: &PreviewSolution| &s.rows,
        |s: &mut PreviewSolution| &mut s.rows,
    ));
    let entity_desc = EntityDescriptor::new("Row", TypeId::of::<Option<usize>>(), "rows")
        .with_extractor(extractor);
    let descriptor = SolutionDescriptor::new("PreviewSolution", TypeId::of::<PreviewSolution>())
        .with_entity(entity_desc);
    ScoreDirector::simple(solution, descriptor, |s, _| s.rows.len())
}

fn composite(
    first: CompoundScalarMove<PreviewSolution>,
    second: CompoundScalarMove<PreviewSolution>,
) -> SequentialCompositeMove<PreviewSolution, CompoundScalarMove<PreviewSolution>> {
    SequentialCompositeMove::new(
        first,
        second,
        0,
        smallvec::smallvec![0, 1],
        "rows",
        MoveTabuSignature::new(
            crate::heuristic::r#move::metadata::MoveTabuScope::new(0, "rows"),
            smallvec::smallvec![1],
            smallvec::smallvec![1],
        ),
    )
}

#[test]
fn working_preview_doability_needs_no_solution_clone_and_restores_exactly() {
    let mut director = create_director(vec![Some(1), Some(0)]);
    let left = CompoundScalarMove::new("free_one", vec![compound_edit(0, Some(0), None)]);
    let right = CompoundScalarMove::new(
        "claim_two",
        vec![compound_edit(1, Some(2), Some(two_needs_vacated_one))],
    );
    let mov = composite(left, right);

    assert!(
        mov.is_doable(&director),
        "the historic clone-out preview already proves the sequence legal"
    );
    let clones_after_clone_out = PREVIEW_SOLUTION_CLONES.load(Ordering::SeqCst);
    assert!(
        clones_after_clone_out >= 1,
        "RED: the clone-out preview services doability with a solution clone"
    );
    assert_eq!(
        director.working_solution().rows,
        vec![Some(1), Some(0)],
        "the clone-out preview must still restore the exact pre-state"
    );

    // RED anchor: the working preview must reach identical legality without
    // a clone. Trait method arrives next.
    PREVIEW_SOLUTION_CLONES.store(0, Ordering::SeqCst);
    let doable = mov.is_doable_with_working_preview(&mut director);
    assert!(doable);
    assert_eq!(
        PREVIEW_SOLUTION_CLONES.load(Ordering::SeqCst),
        0,
        "the working preview creates NO solution clones"
    );
    assert_eq!(
        director.working_solution().rows,
        vec![Some(1), Some(0)],
        "left undone and score state restored before the preview returns"
    );
}
