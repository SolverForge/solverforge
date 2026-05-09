/* Zero-erasure balance constraint stream for load distribution patterns.

A `BalanceConstraintStream` is created from `UniConstraintStream::balance()`
and provides fluent finalization into a `BalanceConstraint`.
*/

use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use super::collection_extract::CollectionExtract;
use super::filter::UniFilter;
use super::weighting_support::fixed_weight_is_hard;
use crate::constraint::balance::BalanceConstraint;

/* Zero-erasure stream for building balance constraints.

Created by `UniConstraintStream::balance()`. Provides `penalize()` and
`reward()` methods to finalize the constraint.

# Type Parameters

- `S` - Solution type
- `A` - Entity type
- `K` - Group key type
- `E` - Extractor function for entities
- `F` - Filter type
- `KF` - Key function (returns Option<K> to skip unassigned entities)
- `Sc` - Score type

# Example

```
use solverforge_scoring::stream::ConstraintFactory;
use solverforge_scoring::api::constraint_set::IncrementalConstraint;
use solverforge_core::score::SoftScore;

#[derive(Clone)]
struct Shift { employee_id: Option<usize> }

#[derive(Clone)]
struct Solution { shifts: Vec<Shift> }

let constraint = ConstraintFactory::<Solution, SoftScore>::new()
.for_each(|s: &Solution| &s.shifts)
.balance(|shift: &Shift| shift.employee_id)
.penalize(SoftScore::of(1000))
.named("Balance workload");

let solution = Solution {
shifts: vec![
Shift { employee_id: Some(0) },
Shift { employee_id: Some(0) },
Shift { employee_id: Some(0) },
Shift { employee_id: Some(1) },
],
};

// std_dev = 1.0, penalty = -1000
assert_eq!(constraint.evaluate(&solution), SoftScore::of(-1000));
```
*/
pub struct BalanceConstraintStream<S, A, K, E, F, KF, Sc>
where
    Sc: Score,
{
    extractor: E,
    filter: F,
    key_fn: KF,
    _phantom: PhantomData<(fn() -> S, fn() -> A, fn() -> K, fn() -> Sc)>,
}

impl<S, A, K, E, F, KF, Sc> BalanceConstraintStream<S, A, K, E, F, KF, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    K: Clone + Eq + Hash + Send + Sync + 'static,
    E: CollectionExtract<S, Item = A>,
    F: UniFilter<S, A>,
    KF: Fn(&A) -> Option<K> + Send + Sync,
    Sc: Score + 'static,
{
    fn into_builder(
        self,
        impact_type: ImpactType,
        base_score: Sc,
    ) -> BalanceConstraintBuilder<S, A, K, E, F, KF, Sc> {
        BalanceConstraintBuilder {
            extractor: self.extractor,
            filter: self.filter,
            key_fn: self.key_fn,
            impact_type,
            base_score,
            is_hard: fixed_weight_is_hard(base_score),
            _phantom: PhantomData,
        }
    }

    // Creates a new balance constraint stream.
    pub(crate) fn new(extractor: E, filter: F, key_fn: KF) -> Self {
        Self {
            extractor,
            filter,
            key_fn,
            _phantom: PhantomData,
        }
    }

    /* Penalizes imbalanced distribution with the given base score per unit std_dev.

    The final score is `base_score.multiply(std_dev)`, negated for penalty.
    */
    pub fn penalize(self, base_score: Sc) -> BalanceConstraintBuilder<S, A, K, E, F, KF, Sc> {
        self.into_builder(ImpactType::Penalty, base_score)
    }

    /* Rewards imbalanced distribution with the given base score per unit std_dev.

    The final score is `base_score.multiply(std_dev)`.
    */
    pub fn reward(self, base_score: Sc) -> BalanceConstraintBuilder<S, A, K, E, F, KF, Sc> {
        self.into_builder(ImpactType::Reward, base_score)
    }
}

impl<S, A, K, E, F, KF, Sc: Score> std::fmt::Debug
    for BalanceConstraintStream<S, A, K, E, F, KF, Sc>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BalanceConstraintStream").finish()
    }
}

// Zero-erasure builder for finalizing a balance constraint.
pub struct BalanceConstraintBuilder<S, A, K, E, F, KF, Sc>
where
    Sc: Score,
{
    extractor: E,
    filter: F,
    key_fn: KF,
    impact_type: ImpactType,
    base_score: Sc,
    is_hard: bool,
    _phantom: PhantomData<(fn() -> S, fn() -> A, fn() -> K)>,
}

impl<S, A, K, E, F, KF, Sc> BalanceConstraintBuilder<S, A, K, E, F, KF, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    K: Clone + Eq + Hash + Send + Sync + 'static,
    E: CollectionExtract<S, Item = A>,
    F: UniFilter<S, A>,
    KF: Fn(&A) -> Option<K> + Send + Sync,
    Sc: Score + 'static,
{
    pub fn named(self, name: &str) -> BalanceConstraint<S, A, K, E, F, KF, Sc> {
        BalanceConstraint::new(
            ConstraintRef::new("", name),
            self.impact_type,
            self.extractor,
            self.filter,
            self.key_fn,
            self.base_score,
            self.is_hard,
        )
    }

    // Finalizes the builder into a zero-erasure `BalanceConstraint`.
}

impl<S, A, K, E, F, KF, Sc: Score> std::fmt::Debug
    for BalanceConstraintBuilder<S, A, K, E, F, KF, Sc>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BalanceConstraintBuilder")
            .field("impact_type", &self.impact_type)
            .finish()
    }
}
