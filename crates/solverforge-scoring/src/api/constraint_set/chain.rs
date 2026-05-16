use std::collections::VecDeque;

use solverforge_core::score::Score;

use super::incremental::{
    push_constraint_metadata, ConstraintMetadata, ConstraintResult, ConstraintSet,
};
use crate::api::analysis::ConstraintAnalysis;

pub struct ConstraintSetChain<Left, Right> {
    left: Left,
    right: Right,
}

pub enum ConstraintSetSource {
    Left,
    Right(usize),
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

    fn constraint_metadata(&self) -> Vec<ConstraintMetadata<'_>> {
        let mut metadata = self.left.constraint_metadata();
        for candidate in self.right.constraint_metadata() {
            push_constraint_metadata(&mut metadata, candidate);
        }
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

    fn constraint_metadata(&self) -> Vec<ConstraintMetadata<'_>> {
        let mut left = VecDeque::from(self.left.constraint_metadata());
        let mut right = VecDeque::from(self.right.constraint_metadata());
        let mut metadata = Vec::new();
        for source in &self.order {
            match source {
                ConstraintSetSource::Left => {
                    let Some(candidate) = left.pop_front() else {
                        panic!("ordered constraint set source order does not match metadata");
                    };
                    push_constraint_metadata(&mut metadata, candidate);
                }
                ConstraintSetSource::Right(count) => {
                    for _ in 0..*count {
                        let Some(candidate) = right.pop_front() else {
                            panic!("ordered constraint set source order does not match metadata");
                        };
                        push_constraint_metadata(&mut metadata, candidate);
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
        let mut left = VecDeque::from(self.left.evaluate_each(solution));
        let mut right = VecDeque::from(self.right.evaluate_each(solution));
        let mut results = Vec::with_capacity(self.constraint_count());
        for source in &self.order {
            match source {
                ConstraintSetSource::Left => {
                    let Some(result) = left.pop_front() else {
                        panic!(
                            "ordered constraint set source order does not match constraint results"
                        );
                    };
                    results.push(result);
                }
                ConstraintSetSource::Right(count) => {
                    for _ in 0..*count {
                        let Some(result) = right.pop_front() else {
                            panic!(
                                "ordered constraint set source order does not match constraint results"
                            );
                        };
                        results.push(result);
                    }
                }
            }
        }
        assert!(
            left.is_empty() && right.is_empty(),
            "ordered constraint set source order does not consume every result"
        );
        results
    }

    fn evaluate_detailed<'a>(&'a self, solution: &S) -> Vec<ConstraintAnalysis<'a, Sc>> {
        let mut left = VecDeque::from(self.left.evaluate_detailed(solution));
        let mut right = VecDeque::from(self.right.evaluate_detailed(solution));
        let mut analyses = Vec::with_capacity(self.constraint_count());
        for source in &self.order {
            match source {
                ConstraintSetSource::Left => {
                    let Some(analysis) = left.pop_front() else {
                        panic!("ordered constraint set source order does not match constraint analyses");
                    };
                    analyses.push(analysis);
                }
                ConstraintSetSource::Right(count) => {
                    for _ in 0..*count {
                        let Some(analysis) = right.pop_front() else {
                            panic!(
                                "ordered constraint set source order does not match constraint analyses"
                            );
                        };
                        analyses.push(analysis);
                    }
                }
            }
        }
        assert!(
            left.is_empty() && right.is_empty(),
            "ordered constraint set source order does not consume every analysis"
        );
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
}
