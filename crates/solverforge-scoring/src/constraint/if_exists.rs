//! Zero-erasure if_exists/if_not_exists uni-constraint.
//!
//! Filters A entities based on whether a matching B entity exists in another collection.
//! The result is still a uni-constraint over A, not a bi-constraint.

use std::collections::HashSet;
use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use crate::api::constraint_set::IncrementalConstraint;

/// Whether to include A entities that have or don't have matching B entities.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExistenceMode {
    /// Include A if at least one matching B exists.
    Exists,
    /// Include A if no matching B exists.
    NotExists,
}

/// Zero-erasure uni-constraint with existence filtering.
///
/// Scores A entities based on whether a matching B entity exists (or doesn't exist)
/// in another collection. Unlike join, this produces a uni-constraint over A.
///
/// # Type Parameters
///
/// - `S` - Solution type
/// - `A` - Primary entity type (scored)
/// - `B` - Secondary entity type (checked for existence)
/// - `K` - Join key type
/// - `EA` - Extractor for A entities
/// - `EB` - Extractor for B entities
/// - `KA` - Key extractor for A
/// - `KB` - Key extractor for B
/// - `FA` - Filter on A entities
/// - `W` - Weight function for A entities
/// - `Sc` - Score type
///
/// # Example
///
/// ```
/// use solverforge_scoring::constraint::if_exists::{IfExistsUniConstraint, ExistenceMode};
/// use solverforge_scoring::api::constraint_set::IncrementalConstraint;
/// use solverforge_core::{ConstraintRef, ImpactType};
/// use solverforge_core::score::SimpleScore;
///
/// #[derive(Clone)]
/// struct Shift { id: usize, employee_idx: Option<usize> }
///
/// #[derive(Clone)]
/// struct Employee { id: usize, on_vacation: bool }
///
/// #[derive(Clone)]
/// struct Schedule { shifts: Vec<Shift>, employees: Vec<Employee> }
///
/// // Penalize shifts assigned to employees who are on vacation
/// let constraint = IfExistsUniConstraint::new(
///     ConstraintRef::new("", "Vacation conflict"),
///     ImpactType::Penalty,
///     ExistenceMode::Exists,
///     |s: &Schedule| s.shifts.as_slice(),
///     |s: &Schedule| s.employees.iter().filter(|e| e.on_vacation).cloned().collect::<Vec<_>>(),
///     |shift: &Shift| shift.employee_idx,
///     |emp: &Employee| Some(emp.id),
///     |shift: &Shift| shift.employee_idx.is_some(),
///     |_shift: &Shift| SimpleScore::of(1),
///     false,
/// );
///
/// let schedule = Schedule {
///     shifts: vec![
///         Shift { id: 0, employee_idx: Some(0) },  // assigned to vacationing emp
///         Shift { id: 1, employee_idx: Some(1) },  // assigned to working emp
///         Shift { id: 2, employee_idx: None },     // unassigned
///     ],
///     employees: vec![
///         Employee { id: 0, on_vacation: true },
///         Employee { id: 1, on_vacation: false },
///     ],
/// };
///
/// // Only shift 0 matches (assigned to employee on vacation)
/// assert_eq!(constraint.evaluate(&schedule), SimpleScore::of(-1));
/// ```
pub struct IfExistsUniConstraint<S, A, B, K, EA, EB, KA, KB, FA, W, Sc>
where
    Sc: Score,
{
    constraint_ref: ConstraintRef,
    impact_type: ImpactType,
    mode: ExistenceMode,
    extractor_a: EA,
    extractor_b: EB,
    key_a: KA,
    key_b: KB,
    filter_a: FA,
    weight: W,
    is_hard: bool,
    _phantom: PhantomData<(S, A, B, K, Sc)>,
}

impl<S, A, B, K, EA, EB, KA, KB, FA, W, Sc> IfExistsUniConstraint<S, A, B, K, EA, EB, KA, KB, FA, W, Sc>
where
    S: 'static,
    A: Clone + 'static,
    B: Clone + 'static,
    K: Eq + Hash + Clone,
    EA: Fn(&S) -> &[A],
    EB: Fn(&S) -> Vec<B>,
    KA: Fn(&A) -> K,
    KB: Fn(&B) -> K,
    FA: Fn(&A) -> bool,
    W: Fn(&A) -> Sc,
    Sc: Score,
{
    /// Creates a new if_exists/if_not_exists constraint.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        constraint_ref: ConstraintRef,
        impact_type: ImpactType,
        mode: ExistenceMode,
        extractor_a: EA,
        extractor_b: EB,
        key_a: KA,
        key_b: KB,
        filter_a: FA,
        weight: W,
        is_hard: bool,
    ) -> Self {
        Self {
            constraint_ref,
            impact_type,
            mode,
            extractor_a,
            extractor_b,
            key_a,
            key_b,
            filter_a,
            weight,
            is_hard,
            _phantom: PhantomData,
        }
    }

    #[inline]
    fn compute_score(&self, a: &A) -> Sc {
        let base = (self.weight)(a);
        match self.impact_type {
            ImpactType::Penalty => -base,
            ImpactType::Reward => base,
        }
    }

    fn build_b_keys(&self, solution: &S) -> HashSet<K> {
        let entities_b = (self.extractor_b)(solution);
        entities_b.iter().map(|b| (self.key_b)(b)).collect()
    }

    fn matches_existence(&self, a: &A, b_keys: &HashSet<K>) -> bool {
        let key = (self.key_a)(a);
        let exists = b_keys.contains(&key);
        match self.mode {
            ExistenceMode::Exists => exists,
            ExistenceMode::NotExists => !exists,
        }
    }
}

impl<S, A, B, K, EA, EB, KA, KB, FA, W, Sc> IncrementalConstraint<S, Sc>
    for IfExistsUniConstraint<S, A, B, K, EA, EB, KA, KB, FA, W, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    K: Eq + Hash + Clone + Send + Sync,
    EA: Fn(&S) -> &[A] + Send + Sync,
    EB: Fn(&S) -> Vec<B> + Send + Sync,
    KA: Fn(&A) -> K + Send + Sync,
    KB: Fn(&B) -> K + Send + Sync,
    FA: Fn(&A) -> bool + Send + Sync,
    W: Fn(&A) -> Sc + Send + Sync,
    Sc: Score,
{
    fn evaluate(&self, solution: &S) -> Sc {
        let entities_a = (self.extractor_a)(solution);
        let b_keys = self.build_b_keys(solution);

        let mut total = Sc::zero();
        for a in entities_a {
            if (self.filter_a)(a) && self.matches_existence(a, &b_keys) {
                total = total + self.compute_score(a);
            }
        }
        total
    }

    fn match_count(&self, solution: &S) -> usize {
        let entities_a = (self.extractor_a)(solution);
        let b_keys = self.build_b_keys(solution);

        entities_a
            .iter()
            .filter(|a| (self.filter_a)(a) && self.matches_existence(a, &b_keys))
            .count()
    }

    fn initialize(&mut self, solution: &S) -> Sc {
        self.evaluate(solution)
    }

    fn on_insert(&mut self, solution: &S, entity_index: usize) -> Sc {
        let entities_a = (self.extractor_a)(solution);
        if entity_index >= entities_a.len() {
            return Sc::zero();
        }

        let a = &entities_a[entity_index];
        if !(self.filter_a)(a) {
            return Sc::zero();
        }

        let b_keys = self.build_b_keys(solution);
        if self.matches_existence(a, &b_keys) {
            self.compute_score(a)
        } else {
            Sc::zero()
        }
    }

    fn on_retract(&mut self, solution: &S, entity_index: usize) -> Sc {
        let entities_a = (self.extractor_a)(solution);
        if entity_index >= entities_a.len() {
            return Sc::zero();
        }

        let a = &entities_a[entity_index];
        if !(self.filter_a)(a) {
            return Sc::zero();
        }

        let b_keys = self.build_b_keys(solution);
        if self.matches_existence(a, &b_keys) {
            -self.compute_score(a)
        } else {
            Sc::zero()
        }
    }

    fn reset(&mut self) {
        // No cached state to clear - we rebuild b_keys on each evaluation
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

impl<S, A, B, K, EA, EB, KA, KB, FA, W, Sc: Score> std::fmt::Debug
    for IfExistsUniConstraint<S, A, B, K, EA, EB, KA, KB, FA, W, Sc>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IfExistsUniConstraint")
            .field("name", &self.constraint_ref.name)
            .field("impact_type", &self.impact_type)
            .field("mode", &self.mode)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solverforge_core::score::SimpleScore;

    #[derive(Clone)]
    struct Task {
        _id: usize,
        assignee: Option<usize>,
    }

    #[derive(Clone)]
    struct Worker {
        id: usize,
        available: bool,
    }

    #[derive(Clone)]
    struct Schedule {
        tasks: Vec<Task>,
        workers: Vec<Worker>,
    }

    #[test]
    fn test_if_exists_penalizes_assigned_to_unavailable() {
        // Penalize tasks assigned to unavailable workers
        let constraint = IfExistsUniConstraint::new(
            ConstraintRef::new("", "Unavailable worker"),
            ImpactType::Penalty,
            ExistenceMode::Exists,
            |s: &Schedule| s.tasks.as_slice(),
            |s: &Schedule| {
                s.workers
                    .iter()
                    .filter(|w| !w.available)
                    .cloned()
                    .collect()
            },
            |t: &Task| t.assignee,
            |w: &Worker| Some(w.id),
            |t: &Task| t.assignee.is_some(),
            |_t: &Task| SimpleScore::of(1),
            false,
        );

        let schedule = Schedule {
            tasks: vec![
                Task { _id: 0, assignee: Some(0) }, // assigned to unavailable
                Task { _id: 1, assignee: Some(1) }, // assigned to available
                Task { _id: 2, assignee: None },    // unassigned
            ],
            workers: vec![
                Worker { id: 0, available: false },
                Worker { id: 1, available: true },
            ],
        };

        // Only task 0 matches
        assert_eq!(constraint.evaluate(&schedule), SimpleScore::of(-1));
        assert_eq!(constraint.match_count(&schedule), 1);
    }

    #[test]
    fn test_if_not_exists_penalizes_unassigned() {
        // Penalize tasks not assigned to any available worker
        let constraint = IfExistsUniConstraint::new(
            ConstraintRef::new("", "No available worker"),
            ImpactType::Penalty,
            ExistenceMode::NotExists,
            |s: &Schedule| s.tasks.as_slice(),
            |s: &Schedule| {
                s.workers
                    .iter()
                    .filter(|w| w.available)
                    .cloned()
                    .collect()
            },
            |t: &Task| t.assignee,
            |w: &Worker| Some(w.id),
            |t: &Task| t.assignee.is_some(),
            |_t: &Task| SimpleScore::of(1),
            false,
        );

        let schedule = Schedule {
            tasks: vec![
                Task { _id: 0, assignee: Some(0) }, // assigned to unavailable - no match in available
                Task { _id: 1, assignee: Some(1) }, // assigned to available
                Task { _id: 2, assignee: None },    // unassigned - filtered out by filter_a
            ],
            workers: vec![
                Worker { id: 0, available: false },
                Worker { id: 1, available: true },
            ],
        };

        // Task 0 is assigned but worker 0 is not available
        assert_eq!(constraint.evaluate(&schedule), SimpleScore::of(-1));
        assert_eq!(constraint.match_count(&schedule), 1);
    }
}
