/* Per-terminal scorers behind a shared operator tree.

Each `OperatorScorer` owns one terminal's identity, weight, and retained
signed row scores; `ScorerChain` composes several scorers over one operator
so compiler-shared grouped nodes keep a single accumulator state.
*/

use solverforge_core::{score::Score, ConstraintRef, ImpactType};

use crate::api::{
    analysis::ConstraintAnalysis,
    constraint_set::{ConstraintMetadata, ConstraintResult},
};
use crate::stream::relational::operator::Operator;
use crate::stream::relational::{HandleMap, RowHandle};

pub trait RowWeight<S: 'static, O: Operator<S>, Sc>: Send + Sync {
    fn score(&self, solution: &S, row: &O::View<'_>) -> Sc;
}

pub trait AdaptWeight<S: 'static, O: Operator<S>, W, Sc>: Send + Sync {
    type RowWeight: RowWeight<S, O, Sc>;

    fn adapt(&self, weight: W) -> Self::RowWeight;
    fn is_hard(&self, weight: &W) -> bool;
}

#[doc(hidden)]
pub struct OperatorScorer<W, Sc> {
    pub(super) constraint_ref: ConstraintRef,
    impact: ImpactType,
    weight: W,
    pub(super) hard: bool,
    scores: HandleMap<Sc>,
}

impl<W, Sc> OperatorScorer<W, Sc> {
    pub(super) fn new(
        constraint_ref: ConstraintRef,
        impact: ImpactType,
        weight: W,
        hard: bool,
    ) -> Self {
        Self {
            constraint_ref,
            impact,
            weight,
            hard,
            scores: HandleMap::new(),
        }
    }
}

impl<W, Sc: Score> OperatorScorer<W, Sc> {
    fn signed(&self, score: Sc) -> Sc {
        match self.impact {
            ImpactType::Penalty => -score,
            ImpactType::Reward => score,
        }
    }
}

#[doc(hidden)]
pub trait OperatorScorerSet<S: 'static, O: Operator<S>, Sc: Score>: Send + Sync {
    fn evaluate(&self, operator: &O, solution: &S) -> Sc;
    fn initialize(&mut self, operator: &O, solution: &S) -> Sc;
    fn apply_changes(
        &mut self,
        operator: &O,
        solution: &S,
        removed: &[RowHandle],
        inserted: &[RowHandle],
    ) -> Sc;
    fn constraint_count(&self) -> usize;
    fn metadata(&self) -> Vec<ConstraintMetadata<'_>>;
    fn evaluate_each<'a>(&'a self, operator: &O, solution: &S) -> Vec<ConstraintResult<'a, Sc>>;
    fn evaluate_detailed<'a>(
        &'a self,
        operator: &O,
        solution: &S,
    ) -> Vec<ConstraintAnalysis<'a, Sc>>;
    fn reset(&mut self);
    fn score_for(&self, _handle: RowHandle) -> Option<Sc> {
        None
    }
}

impl<S, O, Sc> OperatorScorerSet<S, O, Sc> for ()
where
    S: Send + Sync + 'static,
    O: Operator<S> + Send + Sync,
    Sc: Score,
{
    fn evaluate(&self, _operator: &O, _solution: &S) -> Sc {
        Sc::zero()
    }
    fn initialize(&mut self, _operator: &O, _solution: &S) -> Sc {
        Sc::zero()
    }
    fn apply_changes(
        &mut self,
        _operator: &O,
        _solution: &S,
        _removed: &[RowHandle],
        _inserted: &[RowHandle],
    ) -> Sc {
        Sc::zero()
    }
    fn constraint_count(&self) -> usize {
        0
    }
    fn metadata(&self) -> Vec<ConstraintMetadata<'_>> {
        Vec::new()
    }
    fn evaluate_each<'a>(&'a self, _operator: &O, _solution: &S) -> Vec<ConstraintResult<'a, Sc>> {
        Vec::new()
    }
    fn evaluate_detailed<'a>(
        &'a self,
        _operator: &O,
        _solution: &S,
    ) -> Vec<ConstraintAnalysis<'a, Sc>> {
        Vec::new()
    }
    fn reset(&mut self) {}
}

impl<S, O, W, Sc> OperatorScorerSet<S, O, Sc> for OperatorScorer<W, Sc>
where
    S: Send + Sync + 'static,
    O: Operator<S> + Send + Sync,
    W: RowWeight<S, O, Sc>,
    Sc: Score,
{
    fn evaluate(&self, operator: &O, solution: &S) -> Sc {
        let mut total = Sc::zero();
        operator.visit_all(solution, &mut |row| {
            total = total + self.signed(self.weight.score(solution, &row));
        });
        total
    }

    fn initialize(&mut self, operator: &O, solution: &S) -> Sc {
        self.scores.clear();
        let mut total = Sc::zero();
        for handle in operator.handles() {
            let row = operator
                .resolve(solution, handle)
                .expect("live shared terminal row");
            let score = self.signed(self.weight.score(solution, &row));
            self.scores.insert(handle, score);
            total = total + score;
        }
        total
    }

    fn apply_changes(
        &mut self,
        operator: &O,
        solution: &S,
        removed: &[RowHandle],
        inserted: &[RowHandle],
    ) -> Sc {
        let mut delta = Sc::zero();
        for handle in removed.iter().copied() {
            if let Some(score) = self.scores.remove(handle) {
                delta = delta - score;
            }
        }
        for handle in inserted.iter().copied() {
            let row = operator
                .resolve(solution, handle)
                .expect("inserted shared terminal row");
            let score = self.signed(self.weight.score(solution, &row));
            assert!(
                self.scores.insert(handle, score).is_none(),
                "duplicate shared terminal delta"
            );
            delta = delta + score;
        }
        delta
    }

    fn constraint_count(&self) -> usize {
        1
    }

    fn score_for(&self, handle: RowHandle) -> Option<Sc> {
        self.scores.get(handle).copied()
    }

    fn metadata(&self) -> Vec<ConstraintMetadata<'_>> {
        vec![ConstraintMetadata::new(&self.constraint_ref, self.hard)]
    }

    fn evaluate_each<'a>(&'a self, operator: &O, solution: &S) -> Vec<ConstraintResult<'a, Sc>> {
        vec![ConstraintResult {
            name: &self.constraint_ref.name,
            score: self.evaluate(operator, solution),
            match_count: operator.handles().len(),
            is_hard: self.hard,
        }]
    }

    fn evaluate_detailed<'a>(
        &'a self,
        operator: &O,
        solution: &S,
    ) -> Vec<ConstraintAnalysis<'a, Sc>> {
        vec![ConstraintAnalysis::new(
            &self.constraint_ref,
            Sc::zero(),
            self.evaluate(operator, solution),
            Vec::new(),
            self.hard,
        )]
    }

    fn reset(&mut self) {
        self.scores.clear();
    }
}

#[doc(hidden)]
pub struct ScorerChain<T>(pub(super) T);

impl<S, O, Left, Right, Sc> OperatorScorerSet<S, O, Sc> for ScorerChain<(Left, Right)>
where
    S: Send + Sync + 'static,
    O: Operator<S> + Send + Sync,
    Left: OperatorScorerSet<S, O, Sc>,
    Right: OperatorScorerSet<S, O, Sc>,
    Sc: Score,
{
    fn evaluate(&self, operator: &O, solution: &S) -> Sc {
        self.0 .0.evaluate(operator, solution) + self.0 .1.evaluate(operator, solution)
    }
    fn initialize(&mut self, operator: &O, solution: &S) -> Sc {
        self.0 .0.initialize(operator, solution) + self.0 .1.initialize(operator, solution)
    }
    fn apply_changes(
        &mut self,
        operator: &O,
        solution: &S,
        removed: &[RowHandle],
        inserted: &[RowHandle],
    ) -> Sc {
        self.0
             .0
            .apply_changes(operator, solution, removed, inserted)
            + self
                .0
                 .1
                .apply_changes(operator, solution, removed, inserted)
    }
    fn constraint_count(&self) -> usize {
        self.0 .0.constraint_count() + self.0 .1.constraint_count()
    }
    fn metadata(&self) -> Vec<ConstraintMetadata<'_>> {
        let mut values = self.0 .0.metadata();
        values.extend(self.0 .1.metadata());
        values
    }
    fn evaluate_each<'a>(&'a self, operator: &O, solution: &S) -> Vec<ConstraintResult<'a, Sc>> {
        let mut values = self.0 .0.evaluate_each(operator, solution);
        values.extend(self.0 .1.evaluate_each(operator, solution));
        values
    }
    fn evaluate_detailed<'a>(
        &'a self,
        operator: &O,
        solution: &S,
    ) -> Vec<ConstraintAnalysis<'a, Sc>> {
        let mut values = self.0 .0.evaluate_detailed(operator, solution);
        values.extend(self.0 .1.evaluate_detailed(operator, solution));
        values
    }
    fn reset(&mut self) {
        self.0 .0.reset();
        self.0 .1.reset();
    }
}
