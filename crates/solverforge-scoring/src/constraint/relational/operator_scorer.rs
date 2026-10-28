/* Per-terminal scorers behind a shared operator tree.

Each `OperatorScorer` owns one terminal's identity, weight, and retained
signed row scores; `ScorerChain` composes several scorers over one operator
so compiler-shared grouped nodes keep a single accumulator state.
*/

use solverforge_core::{score::Score, ConstraintRef, ImpactType};

use crate::api::{
    analysis::ConstraintAnalysis,
    constraint_set::{scale_contribution, ConstraintMetadata, ConstraintResult},
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

    /* Weighted counterparts: one multiplier per scorer, in chain order. Each
    scorer scales its own contribution, so terminals sharing one operator
    keep independent multipliers while the operator state stays shared. */
    fn weight_refs<'a>(&'a self, refs: &mut Vec<&'a ConstraintRef>);
    fn evaluate_weighted(&self, operator: &O, solution: &S, multipliers: &[u32]) -> Sc;
    fn initialize_weighted(&mut self, operator: &O, solution: &S, multipliers: &[u32]) -> Sc;
    fn apply_changes_weighted(
        &mut self,
        operator: &O,
        solution: &S,
        removed: &[RowHandle],
        inserted: &[RowHandle],
        multipliers: &[u32],
    ) -> Sc;
    fn evaluate_each_weighted<'a>(
        &'a self,
        operator: &O,
        solution: &S,
        multipliers: &[u32],
    ) -> Vec<ConstraintResult<'a, Sc>>;
    fn evaluate_detailed_weighted<'a>(
        &'a self,
        operator: &O,
        solution: &S,
        multipliers: &[u32],
    ) -> Vec<ConstraintAnalysis<'a, Sc>>;
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
    fn weight_refs<'a>(&'a self, _refs: &mut Vec<&'a ConstraintRef>) {}
    fn evaluate_weighted(&self, _operator: &O, _solution: &S, _multipliers: &[u32]) -> Sc {
        Sc::zero()
    }
    #[inline(always)]
    fn initialize_weighted(&mut self, _operator: &O, _solution: &S, _multipliers: &[u32]) -> Sc {
        Sc::zero()
    }
    #[inline(always)]
    fn apply_changes_weighted(
        &mut self,
        _operator: &O,
        _solution: &S,
        _removed: &[RowHandle],
        _inserted: &[RowHandle],
        _multipliers: &[u32],
    ) -> Sc {
        Sc::zero()
    }
    fn evaluate_each_weighted<'a>(
        &'a self,
        _operator: &O,
        _solution: &S,
        _multipliers: &[u32],
    ) -> Vec<ConstraintResult<'a, Sc>> {
        Vec::new()
    }
    fn evaluate_detailed_weighted<'a>(
        &'a self,
        _operator: &O,
        _solution: &S,
        _multipliers: &[u32],
    ) -> Vec<ConstraintAnalysis<'a, Sc>> {
        Vec::new()
    }
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

    #[inline(always)]
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

    #[inline(always)]
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
            weight_multiplier: 1,
        }]
    }

    fn evaluate_detailed<'a>(
        &'a self,
        operator: &O,
        solution: &S,
    ) -> Vec<ConstraintAnalysis<'a, Sc>> {
        vec![ConstraintAnalysis::new(
            &self.constraint_ref,
            1,
            self.evaluate(operator, solution),
            Vec::new(),
            self.hard,
        )]
    }

    fn reset(&mut self) {
        self.scores.clear();
    }

    fn weight_refs<'a>(&'a self, refs: &mut Vec<&'a ConstraintRef>) {
        refs.push(&self.constraint_ref);
    }

    fn evaluate_weighted(&self, operator: &O, solution: &S, multipliers: &[u32]) -> Sc {
        scale_contribution(self.evaluate(operator, solution), multipliers[0])
    }

    #[inline(always)]
    fn initialize_weighted(&mut self, operator: &O, solution: &S, multipliers: &[u32]) -> Sc {
        scale_contribution(self.initialize(operator, solution), multipliers[0])
    }

    #[inline(always)]
    fn apply_changes_weighted(
        &mut self,
        operator: &O,
        solution: &S,
        removed: &[RowHandle],
        inserted: &[RowHandle],
        multipliers: &[u32],
    ) -> Sc {
        scale_contribution(
            self.apply_changes(operator, solution, removed, inserted),
            multipliers[0],
        )
    }

    fn evaluate_each_weighted<'a>(
        &'a self,
        operator: &O,
        solution: &S,
        multipliers: &[u32],
    ) -> Vec<ConstraintResult<'a, Sc>> {
        vec![ConstraintResult {
            name: &self.constraint_ref.name,
            score: self.evaluate_weighted(operator, solution, multipliers),
            match_count: operator.handles().len(),
            is_hard: self.hard,
            weight_multiplier: multipliers[0],
        }]
    }

    fn evaluate_detailed_weighted<'a>(
        &'a self,
        operator: &O,
        solution: &S,
        multipliers: &[u32],
    ) -> Vec<ConstraintAnalysis<'a, Sc>> {
        vec![ConstraintAnalysis::new(
            &self.constraint_ref,
            multipliers[0],
            self.evaluate_weighted(operator, solution, multipliers),
            Vec::new(),
            self.hard,
        )]
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
    #[inline(always)]
    fn initialize(&mut self, operator: &O, solution: &S) -> Sc {
        self.0 .0.initialize(operator, solution) + self.0 .1.initialize(operator, solution)
    }
    #[inline(always)]
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
    fn weight_refs<'a>(&'a self, refs: &mut Vec<&'a ConstraintRef>) {
        self.0 .0.weight_refs(refs);
        self.0 .1.weight_refs(refs);
    }
    fn evaluate_weighted(&self, operator: &O, solution: &S, multipliers: &[u32]) -> Sc {
        let (left, right) = multipliers.split_at(self.0 .0.constraint_count());
        self.0 .0.evaluate_weighted(operator, solution, left)
            + self.0 .1.evaluate_weighted(operator, solution, right)
    }
    #[inline(always)]
    fn initialize_weighted(&mut self, operator: &O, solution: &S, multipliers: &[u32]) -> Sc {
        let (left, right) = multipliers.split_at(self.0 .0.constraint_count());
        self.0 .0.initialize_weighted(operator, solution, left)
            + self.0 .1.initialize_weighted(operator, solution, right)
    }
    #[inline(always)]
    fn apply_changes_weighted(
        &mut self,
        operator: &O,
        solution: &S,
        removed: &[RowHandle],
        inserted: &[RowHandle],
        multipliers: &[u32],
    ) -> Sc {
        let (left, right) = multipliers.split_at(self.0 .0.constraint_count());
        self.0
             .0
            .apply_changes_weighted(operator, solution, removed, inserted, left)
            + self
                .0
                 .1
                .apply_changes_weighted(operator, solution, removed, inserted, right)
    }
    fn evaluate_each_weighted<'a>(
        &'a self,
        operator: &O,
        solution: &S,
        multipliers: &[u32],
    ) -> Vec<ConstraintResult<'a, Sc>> {
        let (left, right) = multipliers.split_at(self.0 .0.constraint_count());
        let mut values = self.0 .0.evaluate_each_weighted(operator, solution, left);
        values.extend(self.0 .1.evaluate_each_weighted(operator, solution, right));
        values
    }
    fn evaluate_detailed_weighted<'a>(
        &'a self,
        operator: &O,
        solution: &S,
        multipliers: &[u32],
    ) -> Vec<ConstraintAnalysis<'a, Sc>> {
        let (left, right) = multipliers.split_at(self.0 .0.constraint_count());
        let mut values = self
            .0
             .0
            .evaluate_detailed_weighted(operator, solution, left);
        values.extend(
            self.0
                 .1
                .evaluate_detailed_weighted(operator, solution, right),
        );
        values
    }
}
