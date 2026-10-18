use std::marker::PhantomData;

use solverforge_core::{score::Score, ConstraintRef, ImpactType};

use crate::api::analysis::EntityRef;
use crate::api::{
    analysis::{ConstraintAnalysis, ConstraintJustification, DetailedConstraintMatch},
    constraint_set::{
        ConstraintMetadata, ConstraintResult, ConstraintSet, IncrementalConstraint,
        IncrementalConstraintSealed,
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
    constraint_ref: ConstraintRef,
    impact: ImpactType,
    weight: W,
    hard: bool,
    scores: HandleMap<Sc>,
}

impl<W, Sc> OperatorScorer<W, Sc> {
    fn new(constraint_ref: ConstraintRef, impact: ImpactType, weight: W, hard: bool) -> Self {
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
pub struct ScorerChain<T>(T);

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

pub struct SharedOperatorTerminal<S, O, A, Scorers, Sc> {
    operator: O,
    adapter: A,
    scorers: Scorers,
    removed: Vec<RowHandle>,
    inserted: Vec<RowHandle>,
    update_count: usize,
    changed_key_count: usize,
    explain: Option<fn(&O, &S, RowHandle, &mut Vec<EntityRef>)>,
    marker: PhantomData<fn() -> (S, Sc)>,
}

pub struct SharedOperatorBuilder<S, O, A, Scorers, W, Sc> {
    terminal: SharedOperatorTerminal<S, O, A, Scorers, Sc>,
    impact: ImpactType,
    weight: W,
    hard: Option<bool>,
}

impl<S, O, A, W, Sc> SharedOperatorBuilder<S, O, A, (), W, Sc> {
    pub fn first(operator: O, adapter: A, impact: ImpactType, weight: W) -> Self {
        Self {
            terminal: SharedOperatorTerminal {
                operator,
                adapter,
                scorers: (),
                removed: Vec::new(),
                inserted: Vec::new(),
                update_count: 0,
                changed_key_count: 0,
                explain: None,
                marker: PhantomData,
            },
            impact,
            weight,
            hard: None,
        }
    }

    pub fn with_explainer(mut self, explain: fn(&O, &S, RowHandle, &mut Vec<EntityRef>)) -> Self {
        self.terminal.explain = Some(explain);
        self
    }

    pub fn first_with_hard(
        operator: O,
        adapter: A,
        impact: ImpactType,
        weight: W,
        hard: bool,
    ) -> Self {
        let mut builder = Self::first(operator, adapter, impact, weight);
        builder.hard = Some(hard);
        builder
    }
}

impl<S, O, A, W, Sc> SharedOperatorBuilder<S, O, A, (), W, Sc>
where
    S: Send + Sync + 'static,
    O: Operator<S> + Send + Sync,
    A: AdaptWeight<S, O, W, Sc>,
    Sc: Score,
{
    pub fn first_named(
        self,
        name: &str,
    ) -> SharedOperatorTerminal<S, O, A, OperatorScorer<A::RowWeight, Sc>, Sc> {
        let hard = self
            .hard
            .unwrap_or_else(|| self.terminal.adapter.is_hard(&self.weight));
        let row_weight = self.terminal.adapter.adapt(self.weight);
        SharedOperatorTerminal {
            operator: self.terminal.operator,
            adapter: self.terminal.adapter,
            scorers: OperatorScorer::new(
                ConstraintRef::new("", name),
                self.impact,
                row_weight,
                hard,
            ),
            removed: self.terminal.removed,
            inserted: self.terminal.inserted,
            update_count: self.terminal.update_count,
            changed_key_count: self.terminal.changed_key_count,
            explain: self.terminal.explain,
            marker: PhantomData,
        }
    }
}

impl<S, O, A, Scorers, Sc> SharedOperatorTerminal<S, O, A, Scorers, Sc>
where
    S: Send + Sync + 'static,
    O: Operator<S> + Send + Sync,
    Sc: Score,
{
    pub fn penalize<W>(self, weight: W) -> SharedOperatorBuilder<S, O, A, Scorers, W, Sc> {
        SharedOperatorBuilder {
            terminal: self,
            impact: ImpactType::Penalty,
            weight,
            hard: None,
        }
    }
    pub fn reward<W>(self, weight: W) -> SharedOperatorBuilder<S, O, A, Scorers, W, Sc> {
        SharedOperatorBuilder {
            terminal: self,
            impact: ImpactType::Reward,
            weight,
            hard: None,
        }
    }

    pub fn evaluate(&self, solution: &S) -> Sc
    where
        Scorers: OperatorScorerSet<S, O, Sc>,
    {
        self.scorers.evaluate(&self.operator, solution)
    }
    pub fn initialize(&mut self, solution: &S) -> Sc
    where
        Scorers: OperatorScorerSet<S, O, Sc>,
    {
        self.operator.initialize(solution);
        self.scorers.initialize(&self.operator, solution)
    }
    pub fn on_retract(&mut self, solution: &S, index: usize, descriptor: usize) -> Sc
    where
        Scorers: OperatorScorerSet<S, O, Sc>,
    {
        self.update(solution, index, descriptor, false)
    }
    pub fn on_insert(&mut self, solution: &S, index: usize, descriptor: usize) -> Sc
    where
        Scorers: OperatorScorerSet<S, O, Sc>,
    {
        self.update(solution, index, descriptor, true)
    }
    pub fn update_count(&self) -> usize {
        self.update_count
    }
    pub fn changed_key_count(&self) -> usize {
        self.changed_key_count
    }

    fn update(&mut self, solution: &S, index: usize, descriptor: usize, insert: bool) -> Sc
    where
        Scorers: OperatorScorerSet<S, O, Sc>,
    {
        self.removed.clear();
        self.inserted.clear();
        if insert {
            self.operator.insert(
                solution,
                descriptor,
                index,
                &mut self.removed,
                &mut self.inserted,
            );
        } else {
            self.operator.retract(
                solution,
                descriptor,
                index,
                &mut self.removed,
                &mut self.inserted,
            );
        }
        if !self.removed.is_empty() || !self.inserted.is_empty() {
            self.update_count += 1;
            self.changed_key_count += 1;
        }
        self.scorers
            .apply_changes(&self.operator, solution, &self.removed, &self.inserted)
    }
}

impl<S, O, A, Scorers, W, Sc> SharedOperatorBuilder<S, O, A, Scorers, W, Sc>
where
    S: Send + Sync + 'static,
    O: Operator<S> + Send + Sync,
    A: AdaptWeight<S, O, W, Sc>,
    Scorers: OperatorScorerSet<S, O, Sc>,
    Sc: Score,
{
    pub fn named(
        self,
        name: &str,
    ) -> SharedOperatorTerminal<S, O, A, ScorerChain<(Scorers, OperatorScorer<A::RowWeight, Sc>)>, Sc>
    {
        let hard = self
            .hard
            .unwrap_or_else(|| self.terminal.adapter.is_hard(&self.weight));
        let scorer = OperatorScorer::new(
            ConstraintRef::new("", name),
            self.impact,
            self.terminal.adapter.adapt(self.weight),
            hard,
        );
        SharedOperatorTerminal {
            operator: self.terminal.operator,
            adapter: self.terminal.adapter,
            scorers: ScorerChain((self.terminal.scorers, scorer)),
            removed: self.terminal.removed,
            inserted: self.terminal.inserted,
            update_count: self.terminal.update_count,
            changed_key_count: self.terminal.changed_key_count,
            explain: self.terminal.explain,
            marker: PhantomData,
        }
    }
}

impl<S, O, A, Left, Right, Sc> ConstraintSet<S, Sc>
    for SharedOperatorTerminal<S, O, A, ScorerChain<(Left, Right)>, Sc>
where
    S: Send + Sync + 'static,
    O: Operator<S> + Send + Sync,
    A: Send + Sync,
    Left: OperatorScorerSet<S, O, Sc>,
    Right: OperatorScorerSet<S, O, Sc>,
    Sc: Score,
{
    fn evaluate_all(&self, solution: &S) -> Sc {
        self.scorers.evaluate(&self.operator, solution)
    }
    fn constraint_count(&self) -> usize {
        self.scorers.constraint_count()
    }
    fn constraint_metadata_entries(&self) -> Vec<ConstraintMetadata<'_>> {
        self.scorers.metadata()
    }
    fn evaluate_each<'a>(&'a self, solution: &S) -> Vec<ConstraintResult<'a, Sc>> {
        self.scorers.evaluate_each(&self.operator, solution)
    }
    fn evaluate_detailed<'a>(&'a self, solution: &S) -> Vec<ConstraintAnalysis<'a, Sc>> {
        self.scorers.evaluate_detailed(&self.operator, solution)
    }
    fn initialize_all(&mut self, solution: &S) -> Sc {
        self.initialize(solution)
    }
    fn on_insert_all(&mut self, solution: &S, index: usize, descriptor: usize) -> Sc {
        self.on_insert(solution, index, descriptor)
    }
    fn on_retract_all(&mut self, solution: &S, index: usize, descriptor: usize) -> Sc {
        self.on_retract(solution, index, descriptor)
    }
    fn reset_all(&mut self) {
        self.operator.clear();
        self.scorers.reset();
        self.removed.clear();
        self.inserted.clear();
    }
}

impl<S, O, A, W, Sc> IncrementalConstraintSealed
    for SharedOperatorTerminal<S, O, A, OperatorScorer<W, Sc>, Sc>
{
}

impl<S, O, A, W, Sc> IncrementalConstraint<S, Sc>
    for SharedOperatorTerminal<S, O, A, OperatorScorer<W, Sc>, Sc>
where
    S: Send + Sync + 'static,
    O: Operator<S> + Send + Sync,
    A: Send + Sync,
    W: RowWeight<S, O, Sc>,
    Sc: Score,
{
    fn evaluate(&self, solution: &S) -> Sc {
        self.scorers.evaluate(&self.operator, solution)
    }
    fn match_count(&self, solution: &S) -> usize {
        if self.operator.handles().is_empty() {
            let mut count = 0;
            self.operator.visit_all(solution, &mut |_| count += 1);
            count
        } else {
            self.operator.handles().len()
        }
    }
    fn initialize(&mut self, solution: &S) -> Sc {
        SharedOperatorTerminal::initialize(self, solution)
    }
    fn on_insert(&mut self, solution: &S, index: usize, descriptor: usize) -> Sc {
        SharedOperatorTerminal::on_insert(self, solution, index, descriptor)
    }
    fn on_retract(&mut self, solution: &S, index: usize, descriptor: usize) -> Sc {
        SharedOperatorTerminal::on_retract(self, solution, index, descriptor)
    }
    fn reset(&mut self) {
        self.operator.clear();
        self.scorers.reset();
        self.removed.clear();
        self.inserted.clear();
    }
    fn constraint_ref(&self) -> &ConstraintRef {
        &self.scorers.constraint_ref
    }
    fn is_hard(&self) -> bool {
        self.scorers.hard
    }
    fn get_matches<'a>(&'a self, solution: &S) -> Vec<DetailedConstraintMatch<'a, Sc>> {
        let Some(explain) = self.explain else {
            return Vec::new();
        };
        let mut matches = Vec::new();
        let handles = self.operator.handles();
        for handle in handles {
            let mut entities = Vec::new();
            explain(&self.operator, solution, handle, &mut entities);
            matches.push(DetailedConstraintMatch::new(
                &self.scorers.constraint_ref,
                self.scorers.score_for(handle).unwrap_or_else(Sc::zero),
                ConstraintJustification::new(entities),
            ));
        }
        matches
    }
}
