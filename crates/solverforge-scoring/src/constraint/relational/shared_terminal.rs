use std::marker::PhantomData;

use crate::constraint::score_sum::checked;
use solverforge_core::{score::Score, ConstraintRef, ImpactType};

use super::operator_scorer::{
    AdaptWeight, OperatorScorer, OperatorScorerSet, RowWeight, ScorerChain,
};
use crate::api::analysis::EntityRef;
use crate::api::{
    analysis::{ConstraintAnalysis, ConstraintJustification, DetailedConstraintMatch},
    constraint_set::{
        ConstraintMetadata, ConstraintResult, ConstraintSet, IncrementalConstraint,
        SingleConstraint,
    },
};
use crate::stream::relational::operator::Operator;
use crate::stream::relational::RowHandle;

/// Scoring terminal that owns one operator tree and scores it for several named
/// constraints.
///
/// `#[solverforge_constraints]` appends terminals to one grouped node through
/// [`penalize`](Self::penalize)/[`reward`](Self::reward) and
/// [`SharedOperatorBuilder::named`]. Every entity notification advances the
/// operator once, then each scorer in `Scorers` re-scores the changed rows, so
/// terminals share accumulator state but keep their own identity, weight and
/// weight slot. `A` adapts each terminal's weight to the operator's rows.
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

/// Builder holding a terminal's impact and weight until it is named.
///
/// `Scorers` is `()` for the first terminal; [`first_named`](Self::first_named)
/// finalizes it, [`named`](Self::named) appends to an existing chain.
pub struct SharedOperatorBuilder<S, O, A, Scorers, W, Sc> {
    terminal: SharedOperatorTerminal<S, O, A, Scorers, Sc>,
    impact: ImpactType,
    weight: W,
    hard: Option<bool>,
}

impl<S, O, A, W, Sc> SharedOperatorBuilder<S, O, A, (), W, Sc> {
    /// Starts the first terminal over `operator`. Hardness comes from
    /// `adapter.is_hard(&weight)`.
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

    /// Sets the function that lists the entities behind a row, used by
    /// `get_matches`. Without it, `get_matches` returns no matches.
    pub fn with_explainer(mut self, explain: fn(&O, &S, RowHandle, &mut Vec<EntityRef>)) -> Self {
        self.terminal.explain = Some(explain);
        self
    }

    /// Like [`first`](Self::first), with explicit hardness.
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
    /// Names the first terminal, producing a one-scorer terminal.
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
    /// Starts another penalizing terminal over the same operator.
    pub fn penalize<W>(self, weight: W) -> SharedOperatorBuilder<S, O, A, Scorers, W, Sc> {
        SharedOperatorBuilder {
            terminal: self,
            impact: ImpactType::Penalty,
            weight,
            hard: None,
        }
    }
    /// Starts another rewarding terminal over the same operator.
    pub fn reward<W>(self, weight: W) -> SharedOperatorBuilder<S, O, A, Scorers, W, Sc> {
        SharedOperatorBuilder {
            terminal: self,
            impact: ImpactType::Reward,
            weight,
            hard: None,
        }
    }

    /// Sums every scorer over the operator's current rows, without retained state.
    pub fn evaluate(&self, solution: &S) -> Sc
    where
        Scorers: OperatorScorerSet<S, O, Sc>,
    {
        self.scorers.evaluate(&self.operator, solution)
    }
    /// Initializes the operator from `solution` and returns the summed score of
    /// every scorer.
    pub fn initialize(&mut self, solution: &S) -> Sc
    where
        Scorers: OperatorScorerSet<S, O, Sc>,
    {
        self.operator.initialize(solution);
        self.scorers.initialize(&self.operator, solution)
    }
    /// Retracts an entity from the operator and returns the summed score delta.
    ///
    /// # Panics
    ///
    /// Panics if the delta overflows.
    pub fn on_retract(&mut self, solution: &S, index: usize, descriptor: usize) -> Sc
    where
        Scorers: OperatorScorerSet<S, O, Sc>,
    {
        checked(self.update(solution, index, descriptor, false))
    }
    /// Inserts an entity into the operator and returns the summed score delta.
    ///
    /// # Panics
    ///
    /// Panics if the delta overflows.
    pub fn on_insert(&mut self, solution: &S, index: usize, descriptor: usize) -> Sc
    where
        Scorers: OperatorScorerSet<S, O, Sc>,
    {
        checked(self.update(solution, index, descriptor, true))
    }
    /// Number of notifications that changed at least one operator row.
    pub fn update_count(&self) -> usize {
        self.update_count
    }
    /// Count of row-changing notifications; currently incremented together with
    /// [`update_count`](Self::update_count).
    pub fn changed_key_count(&self) -> usize {
        self.changed_key_count
    }

    /// Unchecked delta with its overflow flag; see `ScoreSum`.
    fn update(&mut self, solution: &S, index: usize, descriptor: usize, insert: bool) -> (Sc, bool)
    where
        Scorers: OperatorScorerSet<S, O, Sc>,
    {
        self.advance_operator(solution, index, descriptor, insert);
        self.scorers
            .apply_changes(&self.operator, solution, &self.removed, &self.inserted)
    }

    fn update_weighted(
        &mut self,
        solution: &S,
        index: usize,
        descriptor: usize,
        insert: bool,
        multipliers: &[u32],
    ) -> (Sc, bool)
    where
        Scorers: OperatorScorerSet<S, O, Sc>,
    {
        self.advance_operator(solution, index, descriptor, insert);
        self.scorers.apply_changes_weighted(
            &self.operator,
            solution,
            &self.removed,
            &self.inserted,
            multipliers,
        )
    }

    /// Feeds one entity change through the shared operator, collecting changed rows.
    #[inline(always)]
    fn advance_operator(&mut self, solution: &S, index: usize, descriptor: usize, insert: bool) {
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
    /// Names this terminal and appends it to the scorer chain.
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
    fn on_insert_all_overflowing(
        &mut self,
        solution: &S,
        index: usize,
        descriptor: usize,
    ) -> (Sc, bool) {
        self.update(solution, index, descriptor, true)
    }
    fn on_retract_all_overflowing(
        &mut self,
        solution: &S,
        index: usize,
        descriptor: usize,
    ) -> (Sc, bool) {
        self.update(solution, index, descriptor, false)
    }
    fn reset_all(&mut self) {
        self.operator.clear();
        self.scorers.reset();
        self.removed.clear();
        self.inserted.clear();
    }
    fn constraint_weight_refs<'a>(&'a self, refs: &mut Vec<&'a ConstraintRef>) {
        self.scorers.weight_refs(refs);
    }
    fn evaluate_all_weighted(&self, solution: &S, multipliers: &[u32]) -> Sc {
        self.scorers
            .evaluate_weighted(&self.operator, solution, multipliers)
    }
    fn evaluate_each_weighted<'a>(
        &'a self,
        solution: &S,
        multipliers: &[u32],
    ) -> Vec<ConstraintResult<'a, Sc>> {
        self.scorers
            .evaluate_each_weighted(&self.operator, solution, multipliers)
    }
    fn evaluate_detailed_weighted<'a>(
        &'a self,
        solution: &S,
        multipliers: &[u32],
    ) -> Vec<ConstraintAnalysis<'a, Sc>> {
        self.scorers
            .evaluate_detailed_weighted(&self.operator, solution, multipliers)
    }
    #[inline(always)]
    fn initialize_all_weighted(&mut self, solution: &S, multipliers: &[u32]) -> Sc {
        self.operator.initialize(solution);
        self.scorers
            .initialize_weighted(&self.operator, solution, multipliers)
    }
    #[inline(always)]
    fn on_insert_all_weighted(
        &mut self,
        solution: &S,
        index: usize,
        descriptor: usize,
        multipliers: &[u32],
    ) -> Sc {
        checked(self.update_weighted(solution, index, descriptor, true, multipliers))
    }
    #[inline(always)]
    fn on_insert_all_weighted_overflowing(
        &mut self,
        solution: &S,
        index: usize,
        descriptor: usize,
        multipliers: &[u32],
    ) -> (Sc, bool) {
        self.update_weighted(solution, index, descriptor, true, multipliers)
    }
    #[inline(always)]
    fn on_retract_all_weighted(
        &mut self,
        solution: &S,
        index: usize,
        descriptor: usize,
        multipliers: &[u32],
    ) -> Sc {
        checked(self.update_weighted(solution, index, descriptor, false, multipliers))
    }
    #[inline(always)]
    fn on_retract_all_weighted_overflowing(
        &mut self,
        solution: &S,
        index: usize,
        descriptor: usize,
        multipliers: &[u32],
    ) -> (Sc, bool) {
        self.update_weighted(solution, index, descriptor, false, multipliers)
    }
}

impl<S, O, A, W, Sc> SingleConstraint
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
