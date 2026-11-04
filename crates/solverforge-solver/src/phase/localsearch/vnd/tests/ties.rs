// `score_tie_break` among equally scored best improving VND moves.

use std::any::TypeId;
use std::borrow::Cow;

use solverforge_config::ScoreTieBreak;
use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};
use solverforge_core::score::SoftScore;
use solverforge_scoring::Director;

use super::super::{solve_vnd_with_resources, TieBreak};
use crate::heuristic::r#move::metadata::{hash_str, MoveTabuScope};
use crate::heuristic::r#move::{Move, MoveTabuSignature};
use crate::heuristic::selector::move_selector::ArenaMoveCursor;
use crate::heuristic::selector::MoveSelector;
use crate::phase::localsearch::SelectorCursorSource;
use crate::scope::SolverScope;

#[derive(Clone, Debug)]
struct TiePlan {
    value: i64,
    tag: usize,
    score: Option<SoftScore>,
}

impl PlanningSolution for TiePlan {
    type Score = SoftScore;

    fn score(&self) -> Option<SoftScore> {
        self.score
    }

    fn set_score(&mut self, score: Option<SoftScore>) {
        self.score = score;
    }
}

struct TieDirector {
    plan: TiePlan,
    descriptor: SolutionDescriptor,
}

impl Director<TiePlan> for TieDirector {
    fn working_solution(&self) -> &TiePlan {
        &self.plan
    }

    fn working_solution_mut(&mut self) -> &mut TiePlan {
        &mut self.plan
    }

    fn calculate_score(&mut self) -> SoftScore {
        let score = SoftScore::of(self.plan.value);
        self.plan.set_score(Some(score));
        score
    }

    fn solution_descriptor(&self) -> &SolutionDescriptor {
        &self.descriptor
    }

    fn clone_working_solution(&self) -> TiePlan {
        self.plan.clone()
    }

    fn before_variable_changed(&mut self, _descriptor_index: usize, _entity_index: usize) {}

    fn after_variable_changed(&mut self, _descriptor_index: usize, _entity_index: usize) {}

    fn entity_count(&self, _descriptor_index: usize) -> Option<usize> {
        Some(0)
    }

    fn total_entity_count(&self) -> Option<usize> {
        Some(0)
    }

    fn constraint_metadata(&self) -> Vec<solverforge_scoring::ConstraintMetadata<'_>> {
        Vec::new()
    }
}

/// Sets the plan's value (its score) and tags the plan with the move.
#[derive(Clone, Debug)]
struct TieMove {
    value: i64,
    tag: usize,
}

impl Move<TiePlan> for TieMove {
    type Undo = (i64, usize);

    fn is_doable<D: Director<TiePlan>>(&self, _score_director: &D) -> bool {
        true
    }

    fn do_move<D: Director<TiePlan>>(&self, score_director: &mut D) -> Self::Undo {
        let plan = score_director.working_solution_mut();
        let previous = (plan.value, plan.tag);
        plan.value = self.value;
        plan.tag = self.tag;
        previous
    }

    fn undo_move<D: Director<TiePlan>>(&self, score_director: &mut D, undo: Self::Undo) {
        let plan = score_director.working_solution_mut();
        (plan.value, plan.tag) = undo;
    }

    fn descriptor_index(&self) -> usize {
        0
    }

    fn entity_indices(&self) -> &[usize] {
        &[]
    }

    fn variable_name(&self) -> &str {
        "tie_move"
    }

    fn tabu_signature<D: Director<TiePlan>>(
        &self,
        _score_director: &D,
    ) -> Cow<'_, MoveTabuSignature> {
        Cow::Owned(MoveTabuSignature::new(
            MoveTabuScope::new(0, "tie_move"),
            &[hash_str("vnd_tie_move")],
            &[hash_str("vnd_tie_move")],
        ))
    }
}

#[derive(Clone, Debug)]
struct TieSelector {
    moves: Vec<TieMove>,
}

impl MoveSelector<TiePlan, TieMove> for TieSelector {
    type Cursor<'a>
        = ArenaMoveCursor<TiePlan, TieMove>
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<TiePlan>>(&'a self, _score_director: &D) -> Self::Cursor<'a> {
        ArenaMoveCursor::from_moves(self.moves.iter().cloned())
    }

    fn size<D: Director<TiePlan>>(&self, _score_director: &D) -> usize {
        self.moves.len()
    }
}

/// One VND step over eight moves that all improve to the same score, with
/// `seed`; returns the tag of the applied move.
fn applied_tag(score_tie_break: ScoreTieBreak, seed: u64) -> usize {
    let director = TieDirector {
        plan: TiePlan {
            value: 0,
            tag: usize::MAX,
            score: None,
        },
        descriptor: SolutionDescriptor::new("TiePlan", TypeId::of::<TiePlan>()),
    };
    let mut solver_scope = SolverScope::new(director).with_seed(seed);
    let selector = TieSelector {
        moves: (0..8).map(|tag| TieMove { value: 5, tag }).collect(),
    };
    let mut neighborhoods = vec![SelectorCursorSource::new(selector)];
    solve_vnd_with_resources::<_, _, _, TieMove, _>(
        &mut neighborhoods,
        &mut (),
        Some(1),
        score_tie_break,
        &mut solver_scope,
    );
    assert_eq!(solver_scope.working_solution().value, 5);
    solver_scope.working_solution().tag
}

#[test]
fn first_keeps_the_first_tied_move() {
    for seed in 0..32 {
        assert_eq!(applied_tag(ScoreTieBreak::First, seed), 0);
    }
}

#[test]
fn random_picks_among_tied_moves_reproducibly() {
    let tags: Vec<usize> = (0..32)
        .map(|seed| applied_tag(ScoreTieBreak::Random, seed))
        .collect();
    assert!(
        tags.iter().any(|&tag| tag != 0),
        "random tie-breaking never left the first move: {tags:?}"
    );
    let distinct: std::collections::BTreeSet<_> = tags.iter().collect();
    assert!(distinct.len() > 2, "{tags:?}");
    for seed in 0..32 {
        assert_eq!(
            applied_tag(ScoreTieBreak::Random, seed),
            tags[seed as usize]
        );
    }
}

#[test]
fn a_better_move_always_wins_over_ties() {
    let mut ties = TieBreak::new(ScoreTieBreak::Random, 7);
    ties.first();
    assert!(ties.replaces(std::cmp::Ordering::Greater));
    assert!(!ties.replaces(std::cmp::Ordering::Less));
    let mut first = TieBreak::new(ScoreTieBreak::First, 7);
    first.first();
    for _ in 0..16 {
        assert!(!first.replaces(std::cmp::Ordering::Equal));
    }
}
