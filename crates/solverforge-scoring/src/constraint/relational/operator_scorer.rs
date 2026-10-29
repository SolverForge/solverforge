/* Per-terminal scorers behind a shared operator tree.

Each `OperatorScorer` owns one terminal's identity, weight, and retained
unsigned row weights; `ScorerChain` composes several scorers over one operator
so compiler-shared grouped nodes keep a single accumulator state.
*/

use crate::constraint::score_sum::ScoreSum;
use solverforge_core::{score::Score, ConstraintRef, ImpactType};

use crate::api::{
    analysis::ConstraintAnalysis,
    constraint_set::{
        scale_contribution, scale_contribution_parts, ConstraintMetadata, ConstraintResult,
    },
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
    // Retained weights are unsigned; this applies the impact for readers.
    fn signed(&self, weight: Sc) -> Sc {
        match self.impact {
            ImpactType::Penalty => -weight,
            ImpactType::Reward => weight,
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
    ) -> (Sc, bool);
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
    ) -> (Sc, bool);
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
    ) -> (Sc, bool) {
        (Sc::zero(), false)
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
    ) -> (Sc, bool) {
        (Sc::zero(), false)
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
        let mut total = ScoreSum::new();
        operator.visit_all(solution, &mut |row| {
            total.add(self.weight.score(solution, &row));
        });
        total.finish_with(self.impact)
    }

    #[inline(always)]
    fn initialize(&mut self, operator: &O, solution: &S) -> Sc {
        self.scores.clear();
        let mut total = ScoreSum::new();
        for handle in operator.handles() {
            let row = operator
                .resolve(solution, handle)
                .expect("live shared terminal row");
            let weight = self.weight.score(solution, &row);
            self.scores.insert(handle, weight);
            total.add(weight);
        }
        total.finish_with(self.impact)
    }

    #[inline(always)]
    fn apply_changes(
        &mut self,
        operator: &O,
        solution: &S,
        removed: &[RowHandle],
        inserted: &[RowHandle],
    ) -> (Sc, bool) {
        let mut delta = ScoreSum::new();
        for handle in removed.iter().copied() {
            if let Some(weight) = self.scores.remove(handle) {
                delta.sub(weight);
            }
        }
        for handle in inserted.iter().copied() {
            let row = operator
                .resolve(solution, handle)
                .expect("inserted shared terminal row");
            let weight = self.weight.score(solution, &row);
            assert!(
                self.scores.insert(handle, weight).is_none(),
                "duplicate shared terminal delta"
            );
            delta.add(weight);
        }
        delta.into_parts_with(self.impact)
    }

    fn constraint_count(&self) -> usize {
        1
    }

    fn score_for(&self, handle: RowHandle) -> Option<Sc> {
        self.scores.get(handle).map(|weight| self.signed(*weight))
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
    ) -> (Sc, bool) {
        scale_contribution_parts(
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
    ) -> (Sc, bool) {
        let mut delta = ScoreSum::new();
        delta.add_parts(
            self.0
                 .0
                .apply_changes(operator, solution, removed, inserted),
        );
        delta.add_parts(
            self.0
                 .1
                .apply_changes(operator, solution, removed, inserted),
        );
        delta.into_parts()
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
    ) -> (Sc, bool) {
        let (left, right) = multipliers.split_at(self.0 .0.constraint_count());
        let mut delta = ScoreSum::new();
        delta.add_parts(
            self.0
                 .0
                .apply_changes_weighted(operator, solution, removed, inserted, left),
        );
        delta.add_parts(
            self.0
                 .1
                .apply_changes_weighted(operator, solution, removed, inserted, right),
        );
        delta.into_parts()
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
