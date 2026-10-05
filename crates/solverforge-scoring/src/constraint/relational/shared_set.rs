/* Compiler-owned shared operator sets: one producer, many scoring consumers.

A shared set owns one operator tree and a tuple of consumer terminals. The
producer (a `GroupNode`, `JoinNode`, or any other concrete operator) owns all
row state exactly once; each consumer keeps only its retained signed scores
and its own weight closure. One root `on_retract`/`on_insert` routes through
the producer once, and every consumer's book follows the same `RowChanges`,
so grouped accumulations update exactly once per root event regardless of
how many terminals or downstream joins consume them.

No `Rc`/`Arc` and no public share API: the shape is a concrete struct the
macro layer emits for derived bindings used by several terminals. Consumers
are addressed positionally through the `OperatorConsumerSet` tuple trait,
mirroring `GroupedScorerSet` — never through erasure.
*/

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use crate::api::analysis::{ConstraintAnalysis, DetailedConstraintMatch};
use crate::api::constraint_set::{ConstraintMetadata, ConstraintResult, ConstraintSet};
use crate::stream::relational::operator::{ExplainRow, Operator, RowChanges};
use crate::stream::relational::HandleMap;

/// Score-only consumer over one shared operator tree.
///
/// The consumer stores its retained signed scores plus weight and metadata;
/// it never owns rows, so it cannot drift from the producer's state.
pub struct OperatorConsumer<S: 'static, O: Operator<S>, W, Sc: Score> {
    constraint_ref: ConstraintRef,
    impact: ImpactType,
    weight: W,
    hard: bool,
    scores: HandleMap<Sc>,
    marker: std::marker::PhantomData<fn() -> (S, O)>,
}

impl<S: 'static, O: Operator<S>, W, Sc: Score> OperatorConsumer<S, O, W, Sc> {
    pub fn new(constraint_ref: ConstraintRef, impact: ImpactType, weight: W, hard: bool) -> Self {
        Self {
            constraint_ref,
            impact,
            weight,
            hard,
            scores: HandleMap::new(),
            marker: std::marker::PhantomData,
        }
    }
    fn signed(&self, score: Sc) -> Sc {
        match self.impact {
            ImpactType::Penalty => -score,
            ImpactType::Reward => score,
        }
    }
}

impl<S, O, W, Sc> OperatorConsumer<S, O, W, Sc>
where
    S: 'static,
    O: Operator<S>,
    W: for<'a> Fn(&S, &O::View<'a>) -> Sc,
    Sc: Score,
{
    fn apply_changes(&mut self, shared: &O, solution: &S, changes: &RowChanges) -> Sc {
        let mut total = Sc::zero();
        for h in &changes.removed {
            if let Some(score) = self.scores.remove(*h) {
                total = total + (-score);
            }
        }
        for h in &changes.inserted {
            let row = shared.resolve(solution, *h).expect("inserted shared row");
            let score = self.signed((self.weight)(solution, &row));
            assert!(
                self.scores.insert(*h, score).is_none(),
                "duplicate shared consumer delta"
            );
            total = total + score;
        }
        total
    }
    fn evaluate(&self, shared: &O, solution: &S) -> (Sc, usize) {
        let mut total = Sc::zero();
        let mut count = 0usize;
        shared.visit_all(solution, &mut |row| {
            total = total + self.signed((self.weight)(solution, &row));
            count += 1;
        });
        (total, count)
    }
    fn matches<'a>(&'a self, shared: &O, solution: &S) -> Vec<DetailedConstraintMatch<'a, Sc>>
    where
        for<'b> O::View<'b>: ExplainRow,
    {
        let mut out = Vec::new();
        shared.visit_all(solution, &mut |row| {
            let mut entities = Vec::new();
            row.explain(&mut entities);
            out.push(DetailedConstraintMatch::new(
                &self.constraint_ref,
                self.signed((self.weight)(solution, &row)),
                crate::api::analysis::ConstraintJustification::new(entities),
            ));
        });
        out
    }
}

/// A tuple of score consumers over one shared operator tree.
///
/// One root event's `RowChanges` reaches every consumer exactly once;
/// weighting, metadata, and explanations stay independent per consumer.
/// Implemented for a flat tuple of `OperatorConsumer`s (the emitted shape)
/// up to the arity the macro layer writes.
pub trait OperatorConsumerSet<S: 'static, O: Operator<S>, Sc: Score> {
    fn evaluate_all(&self, shared: &O, solution: &S) -> Sc;
    fn initialize_books(&mut self, shared: &O, solution: &S) -> Sc;
    fn apply_changes(&mut self, shared: &O, solution: &S, changes: &RowChanges) -> Sc;
    fn reset_books(&mut self);
    fn constraint_count(&self) -> usize;
    fn constraint_metadata(&self) -> Vec<ConstraintMetadata<'_>>;
    fn evaluate_each<'a>(&'a self, shared: &O, solution: &S) -> Vec<ConstraintResult<'a, Sc>>;
    fn evaluate_detailed<'a>(&'a self, shared: &O, solution: &S) -> Vec<ConstraintAnalysis<'a, Sc>>
    where
        for<'b> O::View<'b>: ExplainRow;
}

impl<S, O, W, Sc> OperatorConsumerSet<S, O, Sc> for OperatorConsumer<S, O, W, Sc>
where
    S: 'static,
    O: Operator<S>,
    W: for<'a> Fn(&S, &O::View<'a>) -> Sc,
    Sc: Score,
{
    fn evaluate_all(&self, shared: &O, solution: &S) -> Sc {
        self.evaluate(shared, solution).0
    }
    fn initialize_books(&mut self, shared: &O, solution: &S) -> Sc {
        self.scores.clear();
        let handles = shared.handles();
        let changes = RowChanges {
            removed: Vec::new(),
            inserted: handles,
        };
        self.apply_changes(shared, solution, &changes)
    }
    fn apply_changes(&mut self, shared: &O, solution: &S, changes: &RowChanges) -> Sc {
        self.apply_changes(shared, solution, changes)
    }
    fn reset_books(&mut self) {
        self.scores.clear();
    }
    fn constraint_count(&self) -> usize {
        1
    }
    fn constraint_metadata(&self) -> Vec<ConstraintMetadata<'_>> {
        vec![ConstraintMetadata::new(&self.constraint_ref, self.hard)]
    }
    fn evaluate_each<'a>(&'a self, shared: &O, solution: &S) -> Vec<ConstraintResult<'a, Sc>> {
        let (score, count) = self.evaluate(shared, solution);
        vec![ConstraintResult {
            name: &self.constraint_ref.name,
            score,
            match_count: count,
            is_hard: self.hard,
        }]
    }
    fn evaluate_detailed<'a>(&'a self, shared: &O, solution: &S) -> Vec<ConstraintAnalysis<'a, Sc>>
    where
        for<'b> O::View<'b>: ExplainRow,
    {
        let (score, _) = self.evaluate(shared, solution);
        vec![ConstraintAnalysis::new(
            &self.constraint_ref,
            Sc::zero(),
            score,
            self.matches(shared, solution),
            self.hard,
        )]
    }
}

macro_rules! impl_operator_consumer_set_for_tuple {
    ($($idx:tt: $T:ident),+) => {
        impl<S, O, Sc, $($T),+> OperatorConsumerSet<S, O, Sc> for ($($T,)+)
        where
            S: 'static,
            O: Operator<S>,
            Sc: Score,
            $($T: OperatorConsumerSet<S, O, Sc>,)+
        {
            fn evaluate_all(&self, shared: &O, solution: &S) -> Sc {
                let mut total = Sc::zero();
                $(total = total + self.$idx.evaluate_all(shared, solution);)+
                total
            }
            fn initialize_books(&mut self, shared: &O, solution: &S) -> Sc {
                let mut total = Sc::zero();
                $(total = total + self.$idx.initialize_books(shared, solution);)+
                total
            }
            fn apply_changes(&mut self, shared: &O, solution: &S, changes: &RowChanges) -> Sc {
                let mut total = Sc::zero();
                $(total = total + self.$idx.apply_changes(shared, solution, changes);)+
                total
            }
            fn reset_books(&mut self) {
                $(self.$idx.reset_books();)+
            }
            fn constraint_count(&self) -> usize {
                let mut total = 0;
                $(total += self.$idx.constraint_count();)+
                total
            }
            fn constraint_metadata(&self) -> Vec<ConstraintMetadata<'_>> {
                let mut metadata = Vec::new();
                $(metadata.extend(self.$idx.constraint_metadata());)+
                metadata
            }
            fn evaluate_each<'a>(&'a self, shared: &O, solution: &S) -> Vec<ConstraintResult<'a, Sc>> {
                let mut results = Vec::new();
                $(results.extend(self.$idx.evaluate_each(shared, solution));)+
                results
            }
            fn evaluate_detailed<'a>(
                &'a self,
                shared: &O,
                solution: &S,
            ) -> Vec<ConstraintAnalysis<'a, Sc>>
            where
                for<'b> O::View<'b>: ExplainRow,
            {
                let mut analyses = Vec::new();
                $(analyses.extend(self.$idx.evaluate_detailed(shared, solution));)+
                analyses
            }
        }
    };
}
impl_operator_consumer_set_for_tuple!(0: C0, 1: C1);
impl_operator_consumer_set_for_tuple!(0: C0, 1: C1, 2: C2);
impl_operator_consumer_set_for_tuple!(0: C0, 1: C1, 2: C2, 3: C3);
impl_operator_consumer_set_for_tuple!(0: C0, 1: C1, 2: C2, 3: C3, 4: C4);
impl_operator_consumer_set_for_tuple!(0: C0, 1: C1, 2: C2, 3: C3, 4: C4, 5: C5);

/// A constraint set over one shared operator tree and N score consumers.
///
/// One root event updates the producer exactly once; each consumer's book
/// follows the published `RowChanges`. The operator view must support cold
/// explanation (`ExplainRow`), which every shipped producer view does.
pub struct SharedOperatorSet<S, O, Consumers, Sc: Score> {
    shared: O,
    consumers: Consumers,
    initialized: bool,
    marker: std::marker::PhantomData<fn() -> (S, Sc)>,
}

impl<S, O, Consumers, Sc: Score> SharedOperatorSet<S, O, Consumers, Sc> {
    pub fn new(shared: O, consumers: Consumers) -> Self {
        Self {
            shared,
            consumers,
            initialized: false,
            marker: std::marker::PhantomData,
        }
    }
}

impl<S, O, Consumers, Sc> ConstraintSet<S, Sc> for SharedOperatorSet<S, O, Consumers, Sc>
where
    S: 'static,
    O: Operator<S> + Send + Sync,
    Consumers: OperatorConsumerSet<S, O, Sc> + Send + Sync,
    Sc: Score,
    for<'a> O::View<'a>: ExplainRow,
{
    fn evaluate_all(&self, solution: &S) -> Sc {
        self.consumers.evaluate_all(&self.shared, solution)
    }

    fn constraint_count(&self) -> usize {
        self.consumers.constraint_count()
    }

    fn constraint_metadata_entries(&self) -> Vec<ConstraintMetadata<'_>> {
        self.consumers.constraint_metadata()
    }

    fn evaluate_each<'a>(&'a self, solution: &S) -> Vec<ConstraintResult<'a, Sc>> {
        self.consumers.evaluate_each(&self.shared, solution)
    }

    fn evaluate_detailed<'a>(&'a self, solution: &S) -> Vec<ConstraintAnalysis<'a, Sc>> {
        self.consumers.evaluate_detailed(&self.shared, solution)
    }

    fn initialize_all(&mut self, solution: &S) -> Sc {
        self.shared.initialize(solution);
        self.initialized = true;
        self.consumers.initialize_books(&self.shared, solution)
    }

    fn on_retract_all(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc {
        let changes = self
            .shared
            .retract(solution, descriptor_index, entity_index);
        self.consumers
            .apply_changes(&self.shared, solution, &changes)
    }

    fn on_insert_all(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc {
        let changes = self.shared.insert(solution, descriptor_index, entity_index);
        self.consumers
            .apply_changes(&self.shared, solution, &changes)
    }

    fn reset_all(&mut self) {
        self.shared.clear();
        self.consumers.reset_books();
        self.initialized = false;
    }
}

impl<S, O, Consumers, Sc: Score> SharedOperatorSet<S, O, Consumers, Sc> {
    /// Whether the producer has been initialized for a solution.
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
}
