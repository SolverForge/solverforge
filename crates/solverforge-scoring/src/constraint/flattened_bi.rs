// O(1) flattened bi-constraint for cross-entity joins.
//
// Pre-indexes C items by key for O(1) lookup on entity changes.

use std::collections::HashMap;
use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use crate::api::constraint_set::IncrementalConstraint;

// O(1) flattened bi-constraint.
//
// Given a join between A and B entities by key, this constraint:
// 1. Expands each B into C items via a flatten function
// 2. Pre-indexes C items by (join_key, c_key) for O(1) lookup
// 3. On A entity change, looks up matching C items in O(1) instead of O(|C|)
//
// # Type Parameters
//
// - `S` - Solution type
// - `A` - Entity type A (the planning entity, e.g., Shift)
// - `B` - Entity type B (the joined entity, e.g., Employee)
// - `C` - Flattened item type (e.g., NaiveDate from unavailable dates)
// - `K` - Join key type (e.g., Option<usize> for employee_idx)
// - `CK` - C item key type for indexing (e.g., NaiveDate)
// - `EA` - Extractor for A entities
// - `EB` - Extractor for B entities
// - `KA` - Key extractor for A (join key)
// - `KB` - Key extractor for B (join key)
// - `Flatten` - Function extracting &[C] from &B
// - `CKeyFn` - Function extracting index key from &C
// - `ALookup` - Function extracting lookup key from &A
// - `F` - Filter on (A, C) pairs
// - `W` - Weight function on (A, C) pairs
// - `Sc` - Score type
//
// # Example
//
// ```
// use solverforge_scoring::constraint::flattened_bi::FlattenedBiConstraint;
// use solverforge_scoring::api::constraint_set::IncrementalConstraint;
// use solverforge_core::{ConstraintRef, ImpactType};
// use solverforge_core::score::SimpleScore;
//
// #[derive(Clone)]
// struct Employee {
//     id: usize,
//     unavailable_days: Vec<u32>,
// }
//
// #[derive(Clone)]
// struct Shift {
//     employee_id: Option<usize>,
//     day: u32,
// }
//
// #[derive(Clone)]
// struct Schedule {
//     shifts: Vec<Shift>,
//     employees: Vec<Employee>,
// }
//
// let constraint = FlattenedBiConstraint::new(
//     ConstraintRef::new("", "Unavailable employee"),
//     ImpactType::Penalty,
//     |s: &Schedule| s.shifts.as_slice(),
//     |s: &Schedule| s.employees.as_slice(),
//     |shift: &Shift| shift.employee_id,
//     |emp: &Employee| Some(emp.id),
//     |emp: &Employee| emp.unavailable_days.as_slice(),
//     |day: &u32| *day,           // C → index key
//     |shift: &Shift| shift.day,  // A → lookup key
//     |_s: &Schedule, shift: &Shift, day: &u32| shift.day == *day,
//     |_shift: &Shift, _day: &u32| SimpleScore::of(1),
//     false,
// );
//
// let schedule = Schedule {
//     shifts: vec![
//         Shift { employee_id: Some(0), day: 5 },
//         Shift { employee_id: Some(0), day: 10 },
//     ],
//     employees: vec![
//         Employee { id: 0, unavailable_days: vec![5, 15] },
//     ],
// };
//
// // Day 5 shift conflicts with employee's unavailable day 5 → O(1) lookup!
// assert_eq!(constraint.evaluate(&schedule), SimpleScore::of(-1));
// ```
pub struct FlattenedBiConstraint<
    S,
    A,
    B,
    C,
    K,
    CK,
    EA,
    EB,
    KA,
    KB,
    Flatten,
    CKeyFn,
    ALookup,
    F,
    W,
    Sc,
> where
    Sc: Score,
{
    constraint_ref: ConstraintRef,
    impact_type: ImpactType,
    extractor_a: EA,
    extractor_b: EB,
    key_a: KA,
    key_b: KB,
    flatten: Flatten,
    c_key_fn: CKeyFn,
    a_lookup_fn: ALookup,
    filter: F,
    weight: W,
    is_hard: bool,
    // (join_key, c_key) → list of (b_idx, c_value) for O(1) lookup
    c_index: HashMap<(K, CK), Vec<(usize, C)>>,
    // A index → cached score for this entity's matches
    a_scores: HashMap<usize, Sc>,
    _phantom: PhantomData<(S, A, B)>,
}

impl<S, A, B, C, K, CK, EA, EB, KA, KB, Flatten, CKeyFn, ALookup, F, W, Sc>
    FlattenedBiConstraint<S, A, B, C, K, CK, EA, EB, KA, KB, Flatten, CKeyFn, ALookup, F, W, Sc>
where
    S: 'static,
    A: Clone + 'static,
    B: Clone + 'static,
    C: Clone + 'static,
    K: Eq + Hash + Clone,
    CK: Eq + Hash + Clone,
    EA: Fn(&S) -> &[A],
    EB: Fn(&S) -> &[B],
    KA: Fn(&A) -> K,
    KB: Fn(&B) -> K,
    Flatten: Fn(&B) -> &[C],
    CKeyFn: Fn(&C) -> CK,
    ALookup: Fn(&A) -> CK,
    F: Fn(&S, &A, &C) -> bool,
    W: Fn(&A, &C) -> Sc,
    Sc: Score,
{
    // Creates a new O(1) flattened bi-constraint.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        constraint_ref: ConstraintRef,
        impact_type: ImpactType,
        extractor_a: EA,
        extractor_b: EB,
        key_a: KA,
        key_b: KB,
        flatten: Flatten,
        c_key_fn: CKeyFn,
        a_lookup_fn: ALookup,
        filter: F,
        weight: W,
        is_hard: bool,
    ) -> Self {
        Self {
            constraint_ref,
            impact_type,
            extractor_a,
            extractor_b,
            key_a,
            key_b,
            flatten,
            c_key_fn,
            a_lookup_fn,
            filter,
            weight,
            is_hard,
            c_index: HashMap::new(),
            a_scores: HashMap::new(),
            _phantom: PhantomData,
        }
    }

    #[inline]
    fn compute_score(&self, a: &A, c: &C) -> Sc {
        let base = (self.weight)(a, c);
        match self.impact_type {
            ImpactType::Penalty => -base,
            ImpactType::Reward => base,
        }
    }

    // Build C index: (join_key, c_key) → list of (b_idx, c_value)
    fn build_c_index(&mut self, entities_b: &[B]) {
        self.c_index.clear();
        for (b_idx, b) in entities_b.iter().enumerate() {
            let join_key = (self.key_b)(b);
            for c in (self.flatten)(b) {
                let c_key = (self.c_key_fn)(c);
                self.c_index
                    .entry((join_key.clone(), c_key))
                    .or_default()
                    .push((b_idx, c.clone()));
            }
        }
    }

    // Compute score for entity A using O(1) index lookup.
    fn compute_a_score(&self, solution: &S, a: &A) -> Sc {
        let join_key = (self.key_a)(a);
        let lookup_key = (self.a_lookup_fn)(a);

        // O(1) HashMap lookup instead of O(|C|) iteration!
        let matches = match self.c_index.get(&(join_key, lookup_key)) {
            Some(v) => v.as_slice(),
            None => return Sc::zero(),
        };

        let mut total = Sc::zero();
        for (_b_idx, c) in matches {
            if (self.filter)(solution, a, c) {
                total = total + self.compute_score(a, c);
            }
        }
        total
    }

    fn insert_a(&mut self, solution: &S, entities_a: &[A], a_idx: usize) -> Sc {
        if a_idx >= entities_a.len() {
            return Sc::zero();
        }

        let a = &entities_a[a_idx];
        let score = self.compute_a_score(solution, a);

        if score != Sc::zero() {
            self.a_scores.insert(a_idx, score);
        }
        score
    }

    fn retract_a(&mut self, a_idx: usize) -> Sc {
        match self.a_scores.remove(&a_idx) {
            Some(score) => -score,
            None => Sc::zero(),
        }
    }
}

impl<S, A, B, C, K, CK, EA, EB, KA, KB, Flatten, CKeyFn, ALookup, F, W, Sc>
    IncrementalConstraint<S, Sc>
    for FlattenedBiConstraint<S, A, B, C, K, CK, EA, EB, KA, KB, Flatten, CKeyFn, ALookup, F, W, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    C: Clone + Send + Sync + 'static,
    K: Eq + Hash + Clone + Send + Sync,
    CK: Eq + Hash + Clone + Send + Sync,
    EA: Fn(&S) -> &[A] + Send + Sync,
    EB: Fn(&S) -> &[B] + Send + Sync,
    KA: Fn(&A) -> K + Send + Sync,
    KB: Fn(&B) -> K + Send + Sync,
    Flatten: Fn(&B) -> &[C] + Send + Sync,
    CKeyFn: Fn(&C) -> CK + Send + Sync,
    ALookup: Fn(&A) -> CK + Send + Sync,
    F: Fn(&S, &A, &C) -> bool + Send + Sync,
    W: Fn(&A, &C) -> Sc + Send + Sync,
    Sc: Score,
{
    fn evaluate(&self, solution: &S) -> Sc {
        let entities_a = (self.extractor_a)(solution);
        let entities_b = (self.extractor_b)(solution);
        let mut total = Sc::zero();

        // Build temporary index for standalone evaluation
        let mut temp_index: HashMap<(K, CK), Vec<(usize, C)>> = HashMap::new();
        for (b_idx, b) in entities_b.iter().enumerate() {
            let join_key = (self.key_b)(b);
            for c in (self.flatten)(b) {
                let c_key = (self.c_key_fn)(c);
                temp_index
                    .entry((join_key.clone(), c_key))
                    .or_default()
                    .push((b_idx, c.clone()));
            }
        }

        for a in entities_a {
            let join_key = (self.key_a)(a);
            let lookup_key = (self.a_lookup_fn)(a);

            if let Some(matches) = temp_index.get(&(join_key, lookup_key)) {
                for (_b_idx, c) in matches {
                    if (self.filter)(solution, a, c) {
                        total = total + self.compute_score(a, c);
                    }
                }
            }
        }

        total
    }

    fn match_count(&self, solution: &S) -> usize {
        let entities_a = (self.extractor_a)(solution);
        let entities_b = (self.extractor_b)(solution);
        let mut count = 0;

        // Build temporary index
        let mut temp_index: HashMap<(K, CK), Vec<(usize, C)>> = HashMap::new();
        for (b_idx, b) in entities_b.iter().enumerate() {
            let join_key = (self.key_b)(b);
            for c in (self.flatten)(b) {
                let c_key = (self.c_key_fn)(c);
                temp_index
                    .entry((join_key.clone(), c_key))
                    .or_default()
                    .push((b_idx, c.clone()));
            }
        }

        for a in entities_a {
            let join_key = (self.key_a)(a);
            let lookup_key = (self.a_lookup_fn)(a);

            if let Some(matches) = temp_index.get(&(join_key, lookup_key)) {
                for (_b_idx, c) in matches {
                    if (self.filter)(solution, a, c) {
                        count += 1;
                    }
                }
            }
        }

        count
    }

    fn initialize(&mut self, solution: &S) -> Sc {
        self.reset();

        let entities_a = (self.extractor_a)(solution);
        let entities_b = (self.extractor_b)(solution);

        // Build C index once: O(B × C)
        self.build_c_index(entities_b);

        // Insert all A entities: O(A) with O(1) lookups each
        let mut total = Sc::zero();
        for a_idx in 0..entities_a.len() {
            total = total + self.insert_a(solution, entities_a, a_idx);
        }

        total
    }

    fn on_insert(&mut self, solution: &S, entity_index: usize, _descriptor_index: usize) -> Sc {
        let entities_a = (self.extractor_a)(solution);
        self.insert_a(solution, entities_a, entity_index)
    }

    fn on_retract(&mut self, _solution: &S, entity_index: usize, _descriptor_index: usize) -> Sc {
        self.retract_a(entity_index)
    }

    fn reset(&mut self) {
        self.c_index.clear();
        self.a_scores.clear();
    }

    fn name(&self) -> &str {
        &self.constraint_ref.name
    }

    fn is_hard(&self) -> bool {
        self.is_hard
    }

    fn constraint_ref(&self) -> ConstraintRef {
        self.constraint_ref.clone()
    }
}

impl<S, A, B, C, K, CK, EA, EB, KA, KB, Flatten, CKeyFn, ALookup, F, W, Sc: Score> std::fmt::Debug
    for FlattenedBiConstraint<S, A, B, C, K, CK, EA, EB, KA, KB, Flatten, CKeyFn, ALookup, F, W, Sc>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FlattenedBiConstraint")
            .field("name", &self.constraint_ref.name)
            .field("impact_type", &self.impact_type)
            .field("c_index_size", &self.c_index.len())
            .finish()
    }
}
