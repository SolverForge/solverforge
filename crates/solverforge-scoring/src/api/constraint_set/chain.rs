use std::collections::VecDeque;

use solverforge_core::score::Score;
use solverforge_core::ConstraintRef;

use super::incremental::{ConstraintMetadata, ConstraintResult, ConstraintSet};
use crate::api::analysis::ConstraintAnalysis;

pub struct ConstraintSetChain<Left, Right> {
    left: Left,
    right: Right,
}

pub enum ConstraintSetSource {
    Left,
    Right {
        constraint_count: usize,
        metadata_entry_count: usize,
    },
}

pub struct OrderedConstraintSetChain<Left, Right> {
    left: Left,
    right: Right,
    order: Vec<ConstraintSetSource>,
}

impl<Left, Right> ConstraintSetChain<Left, Right> {
    pub fn new(left: Left, right: Right) -> Self {
        Self { left, right }
    }
}

impl<Left, Right> OrderedConstraintSetChain<Left, Right> {
    pub fn new(left: Left, right: Right, order: Vec<ConstraintSetSource>) -> Self {
        Self { left, right, order }
    }

    // Restores authored order across per-constraint left and right outputs.
    fn interleave<T>(&self, left: Vec<T>, right: Vec<T>, what: &str) -> Vec<T> {
        let mut left = VecDeque::from(left);
        let mut right = VecDeque::from(right);
        let mut values = Vec::with_capacity(left.len() + right.len());
        for source in &self.order {
            match source {
                ConstraintSetSource::Left => {
                    let Some(value) = left.pop_front() else {
                        panic!("ordered constraint set source order does not match {what}");
                    };
                    values.push(value);
                }
                ConstraintSetSource::Right {
                    constraint_count, ..
                } => {
                    for _ in 0..*constraint_count {
                        let Some(value) = right.pop_front() else {
                            panic!("ordered constraint set source order does not match {what}");
                        };
                        values.push(value);
                    }
                }
            }
        }
        assert!(
            left.is_empty() && right.is_empty(),
            "ordered constraint set source order does not consume every one of the {what}"
        );
        values
    }
}

/* Weighted delta methods shared by both chains: the left child owns the first
`left.constraint_count()` weight slots and the right child the rest. */
macro_rules! chain_weighted_deltas {
    () => {
        fn constraint_weight_refs<'a>(&'a self, refs: &mut Vec<&'a ConstraintRef>) {
            self.left.constraint_weight_refs(refs);
            self.right.constraint_weight_refs(refs);
        }

        #[inline]
        fn evaluate_all_weighted(&self, solution: &S, multipliers: &[u32]) -> Sc {
            let (left, right) = multipliers.split_at(self.left.constraint_count());
            self.left.evaluate_all_weighted(solution, left)
                + self.right.evaluate_all_weighted(solution, right)
        }

        #[inline]
        fn initialize_all_weighted(&mut self, solution: &S, multipliers: &[u32]) -> Sc {
            let (left, right) = multipliers.split_at(self.left.constraint_count());
            self.left.initialize_all_weighted(solution, left)
                + self.right.initialize_all_weighted(solution, right)
        }

        #[inline]
        fn on_insert_all_weighted(
            &mut self,
            solution: &S,
            entity_index: usize,
            descriptor_index: usize,
            multipliers: &[u32],
        ) -> Sc {
            let (left, right) = multipliers.split_at(self.left.constraint_count());
            self.left
                .on_insert_all_weighted(solution, entity_index, descriptor_index, left)
                + self
                    .right
                    .on_insert_all_weighted(solution, entity_index, descriptor_index, right)
        }

        #[inline]
        fn on_retract_all_weighted(
            &mut self,
            solution: &S,
            entity_index: usize,
            descriptor_index: usize,
            multipliers: &[u32],
        ) -> Sc {
            let (left, right) = multipliers.split_at(self.left.constraint_count());
            self.left
                .on_retract_all_weighted(solution, entity_index, descriptor_index, left)
                + self.right.on_retract_all_weighted(
                    solution,
                    entity_index,
                    descriptor_index,
                    right,
                )
        }
    };
}

impl<S, Sc, Left, Right> ConstraintSet<S, Sc> for ConstraintSetChain<Left, Right>
where
    S: Send + Sync,
    Sc: Score,
    Left: ConstraintSet<S, Sc>,
    Right: ConstraintSet<S, Sc>,
{
    #[inline]
    fn evaluate_all(&self, solution: &S) -> Sc {
        self.left.evaluate_all(solution) + self.right.evaluate_all(solution)
    }

    #[inline]
    fn constraint_count(&self) -> usize {
        self.left.constraint_count() + self.right.constraint_count()
    }

    fn constraint_metadata_entries(&self) -> Vec<ConstraintMetadata<'_>> {
        let mut metadata = self.left.constraint_metadata_entries();
        metadata.extend(self.right.constraint_metadata_entries());
        metadata
    }

    fn evaluate_each<'a>(&'a self, solution: &S) -> Vec<ConstraintResult<'a, Sc>> {
        let mut results = self.left.evaluate_each(solution);
        results.extend(self.right.evaluate_each(solution));
        results
    }

    fn evaluate_detailed<'a>(&'a self, solution: &S) -> Vec<ConstraintAnalysis<'a, Sc>> {
        let mut analyses = self.left.evaluate_detailed(solution);
        analyses.extend(self.right.evaluate_detailed(solution));
        analyses
    }

    #[inline]
    fn initialize_all(&mut self, solution: &S) -> Sc {
        self.left.initialize_all(solution) + self.right.initialize_all(solution)
    }

    #[inline]
    fn on_insert_all(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc {
        self.left
            .on_insert_all(solution, entity_index, descriptor_index)
            + self
                .right
                .on_insert_all(solution, entity_index, descriptor_index)
    }

    #[inline]
    fn on_retract_all(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc {
        self.left
            .on_retract_all(solution, entity_index, descriptor_index)
            + self
                .right
                .on_retract_all(solution, entity_index, descriptor_index)
    }

    #[inline]
    fn reset_all(&mut self) {
        self.left.reset_all();
        self.right.reset_all();
    }

    chain_weighted_deltas!();

    fn evaluate_each_weighted<'a>(
        &'a self,
        solution: &S,
        multipliers: &[u32],
    ) -> Vec<ConstraintResult<'a, Sc>> {
        let (left, right) = multipliers.split_at(self.left.constraint_count());
        let mut results = self.left.evaluate_each_weighted(solution, left);
        results.extend(self.right.evaluate_each_weighted(solution, right));
        results
    }

    fn evaluate_detailed_weighted<'a>(
        &'a self,
        solution: &S,
        multipliers: &[u32],
    ) -> Vec<ConstraintAnalysis<'a, Sc>> {
        let (left, right) = multipliers.split_at(self.left.constraint_count());
        let mut analyses = self.left.evaluate_detailed_weighted(solution, left);
        analyses.extend(self.right.evaluate_detailed_weighted(solution, right));
        analyses
    }
}

impl<S, Sc, Left, Right> ConstraintSet<S, Sc> for OrderedConstraintSetChain<Left, Right>
where
    S: Send + Sync,
    Sc: Score,
    Left: ConstraintSet<S, Sc>,
    Right: ConstraintSet<S, Sc>,
{
    #[inline]
    fn evaluate_all(&self, solution: &S) -> Sc {
        self.left.evaluate_all(solution) + self.right.evaluate_all(solution)
    }

    #[inline]
    fn constraint_count(&self) -> usize {
        self.left.constraint_count() + self.right.constraint_count()
    }

    fn constraint_metadata_entries(&self) -> Vec<ConstraintMetadata<'_>> {
        let mut left = VecDeque::from(self.left.constraint_metadata_entries());
        let mut right = VecDeque::from(self.right.constraint_metadata_entries());
        let mut metadata = Vec::new();
        for source in &self.order {
            match source {
                ConstraintSetSource::Left => {
                    let Some(candidate) = left.pop_front() else {
                        panic!("ordered constraint set source order does not match metadata");
                    };
                    metadata.push(candidate);
                }
                ConstraintSetSource::Right {
                    metadata_entry_count,
                    ..
                } => {
                    for _ in 0..*metadata_entry_count {
                        let Some(candidate) = right.pop_front() else {
                            panic!("ordered constraint set source order does not match metadata");
                        };
                        metadata.push(candidate);
                    }
                }
            }
        }
        assert!(
            left.is_empty() && right.is_empty(),
            "ordered constraint set source order does not consume every metadata entry"
        );
        metadata
    }

    fn evaluate_each<'a>(&'a self, solution: &S) -> Vec<ConstraintResult<'a, Sc>> {
        self.interleave(
            self.left.evaluate_each(solution),
            self.right.evaluate_each(solution),
            "constraint results",
        )
    }

    fn evaluate_detailed<'a>(&'a self, solution: &S) -> Vec<ConstraintAnalysis<'a, Sc>> {
        self.interleave(
            self.left.evaluate_detailed(solution),
            self.right.evaluate_detailed(solution),
            "constraint analyses",
        )
    }

    #[inline]
    fn initialize_all(&mut self, solution: &S) -> Sc {
        self.left.initialize_all(solution) + self.right.initialize_all(solution)
    }

    #[inline]
    fn on_insert_all(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc {
        self.left
            .on_insert_all(solution, entity_index, descriptor_index)
            + self
                .right
                .on_insert_all(solution, entity_index, descriptor_index)
    }

    #[inline]
    fn on_retract_all(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc {
        self.left
            .on_retract_all(solution, entity_index, descriptor_index)
            + self
                .right
                .on_retract_all(solution, entity_index, descriptor_index)
    }

    #[inline]
    fn reset_all(&mut self) {
        self.left.reset_all();
        self.right.reset_all();
    }

    chain_weighted_deltas!();

    fn evaluate_each_weighted<'a>(
        &'a self,
        solution: &S,
        multipliers: &[u32],
    ) -> Vec<ConstraintResult<'a, Sc>> {
        let (left, right) = multipliers.split_at(self.left.constraint_count());
        self.interleave(
            self.left.evaluate_each_weighted(solution, left),
            self.right.evaluate_each_weighted(solution, right),
            "constraint results",
        )
    }

    fn evaluate_detailed_weighted<'a>(
        &'a self,
        solution: &S,
        multipliers: &[u32],
    ) -> Vec<ConstraintAnalysis<'a, Sc>> {
        let (left, right) = multipliers.split_at(self.left.constraint_count());
        self.interleave(
            self.left.evaluate_detailed_weighted(solution, left),
            self.right.evaluate_detailed_weighted(solution, right),
            "constraint analyses",
        )
    }
}
