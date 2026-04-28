/* O(1) flattened bi-constraint for cross-entity joins.

Pre-indexes C items by key for O(1) lookup on entity changes.
*/

use std::collections::HashMap;
use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use crate::api::constraint_set::IncrementalConstraint;
use crate::stream::collection_extract::ChangeSource;

#[derive(Clone)]
struct BucketEntry<C> {
    b_idx: usize,
    b_entry_pos: usize,
    value: C,
}

#[derive(Clone, Copy)]
struct ASlot {
    bucket: usize,
    pos: usize,
}

#[derive(Clone, Copy)]
struct BEntry {
    bucket: usize,
    pos: usize,
}
/* O(1) flattened bi-constraint.

Given a join between A and B entities by key, this constraint:
1. Expands each B into C items via a flatten function
2. Pre-indexes C items by (join_key, c_key) for O(1) lookup
3. On A entity change, looks up matching C items in O(1) instead of O(|C|)

# Type Parameters

- `S` - Solution type
- `A` - Entity type A (the planning entity, e.g., Shift)
- `B` - Entity type B (the joined entity, e.g., Employee)
- `C` - Flattened item type (e.g., NaiveDate from unavailable dates)
- `K` - Join key type (e.g., Option<usize> for employee_idx)
- `CK` - C item key type for indexing (e.g., NaiveDate)
- `EA` - Extractor for A entities
- `EB` - Extractor for B entities
- `KA` - Key extractor for A (join key)
- `KB` - Key extractor for B (join key)
- `Flatten` - Function extracting `&[C]` from `&B`
- `CKeyFn` - Function extracting index key from &C
- `ALookup` - Function extracting lookup key from &A
- `F` - Filter on (A, C) pairs
- `W` - Weight function on (A, C) pairs
- `Sc` - Score type

# Example

```
use solverforge_scoring::constraint::flattened_bi::FlattenedBiConstraint;
use solverforge_scoring::api::constraint_set::IncrementalConstraint;
use solverforge_core::{ConstraintRef, ImpactType};
use solverforge_core::score::SoftScore;

#[derive(Clone)]
struct Employee {
id: usize,
unavailable_days: Vec<u32>,
}

#[derive(Clone)]
struct Shift {
employee_id: Option<usize>,
day: u32,
}

#[derive(Clone)]
struct Schedule {
shifts: Vec<Shift>,
employees: Vec<Employee>,
}

let constraint = FlattenedBiConstraint::new(
ConstraintRef::new("", "Unavailable employee"),
ImpactType::Penalty,
|s: &Schedule| s.shifts.as_slice(),
|s: &Schedule| s.employees.as_slice(),
|shift: &Shift| shift.employee_id,
|emp: &Employee| Some(emp.id),
|emp: &Employee| emp.unavailable_days.as_slice(),
|day: &u32| *day,           // C → index key
|shift: &Shift| shift.day,  // A → lookup key
|_s: &Schedule, shift: &Shift, day: &u32| shift.day == *day,
|_shift: &Shift, _day: &u32| SoftScore::of(1),
false,
);

let schedule = Schedule {
shifts: vec![
Shift { employee_id: Some(0), day: 5 },
Shift { employee_id: Some(0), day: 10 },
],
employees: vec![
Employee { id: 0, unavailable_days: vec![5, 15] },
],
};

// Day 5 shift conflicts with employee's unavailable day 5 → O(1) lookup!
assert_eq!(constraint.evaluate(&schedule), SoftScore::of(-1));
```
*/
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
    a_source: ChangeSource,
    b_source: ChangeSource,
    // (join_key, c_key) → bucket id for O(1) lookup
    bucket_by_key: HashMap<(K, CK), usize>,
    // bucket id → flattened entries with back-pointers into `b_entries`
    c_index: Vec<Vec<BucketEntry<C>>>,
    // A index → cached score for this entity's matches
    a_scores: HashMap<usize, Sc>,
    // A index → bucket id and position for reverse localized updates
    a_index_to_bucket: HashMap<usize, ASlot>,
    // bucket id → A indices affected by that flattened bucket
    a_by_bucket: HashMap<usize, Vec<usize>>,
    // B index → flattened bucket entries owned by that B entity
    b_entries: HashMap<usize, Vec<BEntry>>,
    _phantom: PhantomData<(fn() -> S, fn() -> A, fn() -> B)>,
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
    EA: crate::stream::collection_extract::CollectionExtract<S, Item = A>,
    EB: crate::stream::collection_extract::CollectionExtract<S, Item = B>,
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
        let a_source = extractor_a.change_source();
        let b_source = extractor_b.change_source();
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
            a_source,
            b_source,
            bucket_by_key: HashMap::new(),
            c_index: Vec::new(),
            a_scores: HashMap::new(),
            a_index_to_bucket: HashMap::new(),
            a_by_bucket: HashMap::new(),
            b_entries: HashMap::new(),
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
        self.bucket_by_key.clear();
        self.c_index.clear();
        self.b_entries.clear();
        for (b_idx, b) in entities_b.iter().enumerate() {
            let join_key = (self.key_b)(b);
            let mut entries = Vec::new();
            for c in (self.flatten)(b) {
                let c_key = (self.c_key_fn)(c);
                let bucket = self.bucket_for_key(join_key.clone(), c_key);
                let b_entry_pos = entries.len();
                let pos = self.push_bucket_entry(bucket, b_idx, b_entry_pos, c.clone());
                entries.push(BEntry { bucket, pos });
            }
            self.b_entries.insert(b_idx, entries);
        }
    }

    fn bucket_for_key(&mut self, join_key: K, c_key: CK) -> usize {
        if let Some(bucket) = self.bucket_by_key.get(&(join_key.clone(), c_key.clone())) {
            return *bucket;
        }
        let bucket = self.c_index.len();
        self.bucket_by_key.insert((join_key, c_key), bucket);
        self.c_index.push(Vec::new());
        bucket
    }

    fn bucket_lookup(&self, join_key: K, c_key: CK) -> Option<usize> {
        self.bucket_by_key.get(&(join_key, c_key)).copied()
    }

    // Compute score for entity A using O(1) index lookup.
    fn compute_a_score(&self, solution: &S, a: &A) -> Sc {
        let join_key = (self.key_a)(a);
        let lookup_key = (self.a_lookup_fn)(a);

        // O(1) HashMap lookup instead of O(|C|) iteration!
        let matches = match self
            .bucket_lookup(join_key, lookup_key)
            .and_then(|bucket| self.c_index.get(bucket))
        {
            Some(v) => v.as_slice(),
            None => return Sc::zero(),
        };

        let mut total = Sc::zero();
        for entry in matches {
            if (self.filter)(solution, a, &entry.value) {
                total = total + self.compute_score(a, &entry.value);
            }
        }
        total
    }

    fn push_bucket_entry(
        &mut self,
        bucket: usize,
        b_idx: usize,
        b_entry_pos: usize,
        value: C,
    ) -> usize {
        let entries = &mut self.c_index[bucket];
        let pos = entries.len();
        entries.push(BucketEntry {
            b_idx,
            b_entry_pos,
            value,
        });
        pos
    }

    fn remove_bucket_entry(&mut self, bucket: usize, pos: usize) -> Option<(usize, usize, usize)> {
        let entries = &mut self.c_index[bucket];
        let last = entries.len() - 1;
        entries.swap_remove(pos);
        if pos != last {
            let moved_b_idx = entries[pos].b_idx;
            let moved_b_entry_pos = entries[pos].b_entry_pos;
            if let Some(b_entries) = self.b_entries.get_mut(&moved_b_idx) {
                b_entries[moved_b_entry_pos].pos = pos;
            }
            return Some((moved_b_idx, moved_b_entry_pos, pos));
        }
        None
    }

    fn insert_a(&mut self, solution: &S, entities_a: &[A], a_idx: usize) -> Sc {
        if a_idx >= entities_a.len() {
            return Sc::zero();
        }

        let a = &entities_a[a_idx];
        let bucket = self.bucket_for_key((self.key_a)(a), (self.a_lookup_fn)(a));
        let indices = self.a_by_bucket.entry(bucket).or_default();
        let pos = indices.len();
        indices.push(a_idx);
        self.a_index_to_bucket.insert(a_idx, ASlot { bucket, pos });
        let score = self.compute_a_score(solution, a);

        if score != Sc::zero() {
            self.a_scores.insert(a_idx, score);
        }
        score
    }

    fn retract_a(&mut self, a_idx: usize) -> Sc {
        if let Some(slot) = self.a_index_to_bucket.remove(&a_idx) {
            self.remove_a_from_bucket(a_idx, slot);
        }
        match self.a_scores.remove(&a_idx) {
            Some(score) => -score,
            None => Sc::zero(),
        }
    }

    fn remove_a_from_bucket(&mut self, a_idx: usize, slot: ASlot) {
        let mut remove_bucket = false;
        if let Some(indices) = self.a_by_bucket.get_mut(&slot.bucket) {
            let last = indices.len() - 1;
            let moved = indices.swap_remove(slot.pos);
            debug_assert_eq!(moved, a_idx);
            if slot.pos != last {
                let moved_a_idx = indices[slot.pos];
                if let Some(moved_slot) = self.a_index_to_bucket.get_mut(&moved_a_idx) {
                    moved_slot.pos = slot.pos;
                }
            }
            remove_bucket = indices.is_empty();
        }
        if remove_bucket {
            self.a_by_bucket.remove(&slot.bucket);
        }
    }

    fn replace_a_score(&mut self, solution: &S, entities_a: &[A], a_idx: usize) -> Sc {
        if a_idx >= entities_a.len() {
            return Sc::zero();
        }
        let old_score = self.a_scores.remove(&a_idx).unwrap_or_else(Sc::zero);
        let new_score = self.compute_a_score(solution, &entities_a[a_idx]);
        if new_score != Sc::zero() {
            self.a_scores.insert(a_idx, new_score);
        }
        new_score - old_score
    }

    fn insert_b(&mut self, solution: &S, entities_a: &[A], entities_b: &[B], b_idx: usize) -> Sc {
        if b_idx >= entities_b.len() {
            return Sc::zero();
        }
        let b = &entities_b[b_idx];
        let join_key = (self.key_b)(b);
        let mut entries = Vec::new();
        let mut total = Sc::zero();
        for c in (self.flatten)(b) {
            let c_key = (self.c_key_fn)(c);
            let bucket = self.bucket_for_key(join_key.clone(), c_key);
            let b_entry_pos = entries.len();
            let pos = self.push_bucket_entry(bucket, b_idx, b_entry_pos, c.clone());
            entries.push(BEntry { bucket, pos });
            total = total + self.replace_bucket_a_scores(solution, entities_a, bucket);
        }
        self.b_entries.insert(b_idx, entries);
        total
    }

    fn retract_b(&mut self, solution: &S, entities_a: &[A], b_idx: usize) -> Sc {
        let mut entries = self.b_entries.remove(&b_idx).unwrap_or_default();
        let mut total = Sc::zero();
        while let Some(entry) = entries.pop() {
            if let Some((moved_b_idx, moved_b_entry_pos, moved_pos)) =
                self.remove_bucket_entry(entry.bucket, entry.pos)
            {
                if moved_b_idx == b_idx && moved_b_entry_pos < entries.len() {
                    entries[moved_b_entry_pos].pos = moved_pos;
                }
            }
            total = total + self.replace_bucket_a_scores(solution, entities_a, entry.bucket);
        }
        total
    }

    fn replace_bucket_a_scores(&mut self, solution: &S, entities_a: &[A], bucket: usize) -> Sc {
        let Some(len) = self.a_by_bucket.get(&bucket).map(Vec::len) else {
            return Sc::zero();
        };
        let mut total = Sc::zero();
        for pos in 0..len {
            let Some(a_idx) = self
                .a_by_bucket
                .get(&bucket)
                .and_then(|indices| indices.get(pos))
                .copied()
            else {
                continue;
            };
            total = total + self.replace_a_score(solution, entities_a, a_idx);
        }
        total
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
    EA: crate::stream::collection_extract::CollectionExtract<S, Item = A>,
    EB: crate::stream::collection_extract::CollectionExtract<S, Item = B>,
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
        let entities_a = self.extractor_a.extract(solution);
        let entities_b = self.extractor_b.extract(solution);
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
        let entities_a = self.extractor_a.extract(solution);
        let entities_b = self.extractor_b.extract(solution);
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

        let entities_a = self.extractor_a.extract(solution);
        let entities_b = self.extractor_b.extract(solution);

        // Build C index once: O(B × C)
        self.build_c_index(entities_b);

        // Insert all A entities: O(A) with O(1) lookups each
        let mut total = Sc::zero();
        for a_idx in 0..entities_a.len() {
            total = total + self.insert_a(solution, entities_a, a_idx);
        }

        total
    }

    fn on_insert(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc {
        let a_changed = self
            .a_source
            .assert_localizes(descriptor_index, &self.constraint_ref.name);
        let b_changed = self
            .b_source
            .assert_localizes(descriptor_index, &self.constraint_ref.name);
        let entities_a = self.extractor_a.extract(solution);
        let entities_b = self.extractor_b.extract(solution);
        let mut total = Sc::zero();
        if a_changed {
            total = total + self.insert_a(solution, entities_a, entity_index);
        }
        if b_changed {
            total = total + self.insert_b(solution, entities_a, entities_b, entity_index);
        }
        total
    }

    fn on_retract(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc {
        let a_changed = self
            .a_source
            .assert_localizes(descriptor_index, &self.constraint_ref.name);
        let b_changed = self
            .b_source
            .assert_localizes(descriptor_index, &self.constraint_ref.name);
        let entities_a = self.extractor_a.extract(solution);
        let mut total = Sc::zero();
        if a_changed {
            total = total + self.retract_a(entity_index);
        }
        if b_changed {
            total = total + self.retract_b(solution, entities_a, entity_index);
        }
        total
    }

    fn reset(&mut self) {
        self.bucket_by_key.clear();
        self.c_index.clear();
        self.a_scores.clear();
        self.a_index_to_bucket.clear();
        self.a_by_bucket.clear();
        self.b_entries.clear();
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
