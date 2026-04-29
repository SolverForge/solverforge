/* Zero-erasure complemented group constraint.

Evaluates grouped results plus complement entities with default values.
Provides true incremental scoring by tracking per-key accumulators.
*/

use std::collections::HashMap;
use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use crate::stream::collection_extract::ChangeSource;
use crate::stream::collector::{Accumulator, UniCollector};

/* Zero-erasure constraint for complemented grouped results.

Groups A entities by key, then iterates over B entities (complement source),
using grouped values where they exist and default values otherwise.

The key function for A returns `Option<K>`, allowing entities to be skipped
when they don't have a valid key (e.g., unassigned shifts).

# Type Parameters

- `S` - Solution type
- `A` - Entity type being grouped (e.g., Shift)
- `B` - Complement entity type (e.g., Employee)
- `K` - Group key type
- `EA` - Extractor for A entities
- `EB` - Extractor for B entities
- `KA` - Key function for A (returns `Option<K>` to allow skipping)
- `KB` - Key function for B
- `C` - Collector type
- `D` - Default value function
- `W` - Weight function
- `Sc` - Score type

# Example

```
use solverforge_scoring::constraint::complemented::ComplementedGroupConstraint;
use solverforge_scoring::stream::collector::count;
use solverforge_scoring::api::constraint_set::IncrementalConstraint;
use solverforge_core::{ConstraintRef, ImpactType};
use solverforge_core::score::SoftScore;

#[derive(Clone, Hash, PartialEq, Eq)]
struct Employee { id: usize }

#[derive(Clone)]
struct Shift { employee_id: Option<usize> }

#[derive(Clone)]
struct Schedule {
employees: Vec<Employee>,
shifts: Vec<Shift>,
}

let constraint = ComplementedGroupConstraint::new(
ConstraintRef::new("", "Shift count"),
ImpactType::Penalty,
|s: &Schedule| s.shifts.as_slice(),
|s: &Schedule| s.employees.as_slice(),
|shift: &Shift| shift.employee_id,  // Returns Option<usize>
|emp: &Employee| emp.id,
count(),
|_emp: &Employee| 0usize,
|count: &usize| SoftScore::of(*count as i64),
false,
);

let schedule = Schedule {
employees: vec![Employee { id: 0 }, Employee { id: 1 }],
shifts: vec![
Shift { employee_id: Some(0) },
Shift { employee_id: Some(0) },
Shift { employee_id: None },  // Skipped - no key
],
};

// Employee 0: 2 shifts, Employee 1: 0 shifts → Total: -2
// Unassigned shift is skipped
assert_eq!(constraint.evaluate(&schedule), SoftScore::of(-2));
```
*/
pub struct ComplementedGroupConstraint<S, A, B, K, EA, EB, KA, KB, C, D, W, Sc>
where
    C: UniCollector<A>,
    Sc: Score,
{
    pub(super) constraint_ref: ConstraintRef,
    pub(super) impact_type: ImpactType,
    pub(super) extractor_a: EA,
    pub(super) extractor_b: EB,
    pub(super) key_a: KA,
    pub(super) key_b: KB,
    pub(super) collector: C,
    pub(super) default_fn: D,
    pub(super) weight_fn: W,
    pub(super) is_hard: bool,
    pub(super) a_source: ChangeSource,
    pub(super) b_source: ChangeSource,
    // Group key -> accumulator for incremental scoring
    pub(super) groups: HashMap<K, C::Accumulator>,
    // A entity index -> group key (for tracking which group each entity belongs to)
    pub(super) entity_groups: HashMap<usize, K>,
    // A entity index -> extracted value (for correct retraction after entity mutation)
    pub(super) entity_values: HashMap<usize, C::Value>,
    // B key -> B entity index (for looking up B entities by key)
    pub(super) b_by_key: HashMap<K, usize>,
    // B entity index -> B key (for localized B retraction)
    pub(super) b_index_to_key: HashMap<usize, K>,
    pub(super) _phantom: PhantomData<(fn() -> S, fn() -> A, fn() -> B, fn() -> Sc)>,
}

impl<S, A, B, K, EA, EB, KA, KB, C, D, W, Sc>
    ComplementedGroupConstraint<S, A, B, K, EA, EB, KA, KB, C, D, W, Sc>
where
    S: 'static,
    A: Clone + 'static,
    B: Clone + 'static,
    K: Clone + Eq + Hash,
    EA: crate::stream::collection_extract::CollectionExtract<S, Item = A>,
    EB: crate::stream::collection_extract::CollectionExtract<S, Item = B>,
    KA: Fn(&A) -> Option<K>,
    KB: Fn(&B) -> K,
    C: UniCollector<A>,
    C::Result: Clone,
    D: Fn(&B) -> C::Result,
    W: Fn(&C::Result) -> Sc,
    Sc: Score,
{
    // Creates a new complemented group constraint.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        constraint_ref: ConstraintRef,
        impact_type: ImpactType,
        extractor_a: EA,
        extractor_b: EB,
        key_a: KA,
        key_b: KB,
        collector: C,
        default_fn: D,
        weight_fn: W,
        is_hard: bool,
    ) -> Self {
        let a_source = extractor_a.change_source();
        let b_source = extractor_b.change_source();
        Self {
            constraint_ref,
            impact_type,
            extractor_a,
            extractor_b,
            key_a,
            key_b,
            collector,
            default_fn,
            weight_fn,
            is_hard,
            a_source,
            b_source,
            groups: HashMap::new(),
            entity_groups: HashMap::new(),
            entity_values: HashMap::new(),
            b_by_key: HashMap::new(),
            b_index_to_key: HashMap::new(),
            _phantom: PhantomData,
        }
    }

    #[inline]
    pub(super) fn compute_score(&self, result: &C::Result) -> Sc {
        let base = (self.weight_fn)(result);
        match self.impact_type {
            ImpactType::Penalty => -base,
            ImpactType::Reward => base,
        }
    }

    // Build grouped results from A entities.
    pub(super) fn build_groups(&self, entities_a: &[A]) -> HashMap<K, C::Result> {
        let mut accumulators: HashMap<K, C::Accumulator> = HashMap::new();

        for a in entities_a {
            // Skip entities with no key (e.g., unassigned shifts)
            let Some(key) = (self.key_a)(a) else {
                continue;
            };
            let value = self.collector.extract(a);
            accumulators
                .entry(key)
                .or_insert_with(|| self.collector.create_accumulator())
                .accumulate(&value);
        }

        accumulators
            .into_iter()
            .map(|(k, acc)| (k, acc.finish()))
            .collect()
    }
}
