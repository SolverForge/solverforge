use crate::api::{
    analysis::{ConstraintJustification, DetailedConstraintMatch},
    constraint_set::IncrementalConstraint,
};
use crate::stream::relational::{
    operator::{ExplainRow, Operator},
    HandleMap,
};
use solverforge_core::{score::Score, ConstraintRef, ImpactType};
use std::marker::PhantomData;

/// Scoring terminal over a concrete operator tree, independent of row arity.
/// Weight closures borrow the complete row; retention stores signed scores,
/// never entities or solution references.
pub struct OperatorTerminal<S, O, W, Sc> {
    constraint_ref: ConstraintRef,
    impact: ImpactType,
    operator: O,
    weight: W,
    hard: bool,
    scores: HandleMap<Sc>,
    initialized: bool,
    marker: PhantomData<fn() -> S>,
}

impl<S, O, W, Sc> OperatorTerminal<S, O, W, Sc> {
    pub fn new(
        constraint_ref: ConstraintRef,
        impact: ImpactType,
        operator: O,
        weight: W,
        hard: bool,
    ) -> Self {
        Self {
            constraint_ref,
            impact,
            operator,
            weight,
            hard,
            scores: HandleMap::new(),
            initialized: false,
            marker: PhantomData,
        }
    }
}
impl<S, O, W, Sc: Score> OperatorTerminal<S, O, W, Sc> {
    fn signed(&self, score: Sc) -> Sc {
        match self.impact {
            ImpactType::Penalty => -score,
            ImpactType::Reward => score,
        }
    }
}
impl<S, O, W, Sc> OperatorTerminal<S, O, W, Sc>
where
    S: 'static,
    O: Operator<S>,
    W: for<'a> Fn(&S, &O::View<'a>) -> Sc,
    Sc: Score,
{
    fn apply_changes(
        &mut self,
        solution: &S,
        changes: crate::stream::relational::operator::RowChanges,
    ) -> Sc {
        let mut total = Sc::zero();
        for h in changes.removed {
            if let Some(score) = self.scores.remove(h) {
                total = total + (-score);
            }
        }
        for h in changes.inserted {
            let row = self
                .operator
                .resolve(solution, h)
                .expect("inserted terminal row");
            let score = self.signed((self.weight)(solution, &row));
            assert!(
                self.scores.insert(h, score).is_none(),
                "duplicate terminal delta"
            );
            total = total + score;
        }
        total
    }
}
impl<S, O, W, Sc> IncrementalConstraint<S, Sc> for OperatorTerminal<S, O, W, Sc>
where
    S: Send + Sync + 'static,
    O: Operator<S> + Send + Sync,
    W: for<'a> Fn(&S, &O::View<'a>) -> Sc + Send + Sync,
    Sc: Score,
    for<'a> O::View<'a>: ExplainRow,
{
    fn evaluate(&self, solution: &S) -> Sc {
        let mut score = Sc::zero();
        self.operator.visit_all(solution, &mut |row| {
            score = score + self.signed((self.weight)(solution, &row));
        });
        score
    }
    fn match_count(&self, solution: &S) -> usize {
        let mut count = 0;
        self.operator.visit_all(solution, &mut |_| {
            count += 1;
        });
        count
    }
    fn initialize(&mut self, solution: &S) -> Sc {
        self.scores.clear();
        self.operator.initialize(solution);
        self.initialized = true;
        let mut total = Sc::zero();
        for handle in self.operator.handles() {
            let row = self
                .operator
                .resolve(solution, handle)
                .expect("live terminal row");
            let score = self.signed((self.weight)(solution, &row));
            self.scores.insert(handle, score);
            total = total + score;
        }
        total
    }
    fn on_retract(&mut self, solution: &S, index: usize, descriptor: usize) -> Sc {
        let changes = self.operator.retract(solution, descriptor, index);
        self.apply_changes(solution, changes)
    }
    fn on_insert(&mut self, solution: &S, index: usize, descriptor: usize) -> Sc {
        let changes = self.operator.insert(solution, descriptor, index);
        self.apply_changes(solution, changes)
    }
    fn reset(&mut self) {
        self.scores.clear();
        self.operator.clear();
        self.initialized = false;
    }
    fn name(&self) -> &str {
        &self.constraint_ref.name
    }
    fn constraint_ref(&self) -> &ConstraintRef {
        &self.constraint_ref
    }
    fn is_hard(&self) -> bool {
        self.hard
    }
    fn get_matches<'a>(&'a self, solution: &S) -> Vec<DetailedConstraintMatch<'a, Sc>> {
        let mut matches = Vec::new();
        let mut record = |row: O::View<'_>, score| {
            let mut entities = Vec::new();
            row.explain(&mut entities);
            matches.push(DetailedConstraintMatch::new(
                &self.constraint_ref,
                score,
                ConstraintJustification::new(entities),
            ));
        };
        if self.initialized {
            for handle in self.operator.handles() {
                let row = self
                    .operator
                    .resolve(solution, handle)
                    .expect("live explanation row");
                record(row, *self.scores.get(handle).expect("live terminal score"));
            }
        } else {
            self.operator.visit_all(solution, &mut |row| {
                record(row, self.signed((self.weight)(solution, &row)));
            });
        }
        matches
    }
}
