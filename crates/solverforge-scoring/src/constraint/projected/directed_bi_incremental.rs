use std::hash::Hash;

use solverforge_core::score::Score;
use solverforge_core::ConstraintRef;

use crate::api::constraint_set::IncrementalConstraint;
use crate::stream::filter::{BiFilter, UniFilter};
use crate::stream::projected::Source;

use super::directed_bi::DirectedBi;

impl<S, Out, K, Src, F, KL, KR, PF, W, Sc> IncrementalConstraint<S, Sc>
    for DirectedBi<S, Out, K, Src, F, KL, KR, PF, W, Sc>
where
    S: Send + Sync + 'static,
    Out: Send + Sync + 'static,
    K: Eq + Hash + Send + Sync + 'static,
    Src: Source<S, Out>,
    F: UniFilter<S, Out>,
    KL: Fn(&Out) -> K + Send + Sync,
    KR: Fn(&Out) -> K + Send + Sync,
    PF: BiFilter<S, Out, Out>,
    W: Fn(&Out, &Out) -> Sc + Send + Sync,
    Sc: Score + 'static,
{
    fn evaluate(&self, solution: &S) -> Sc {
        let rows = self.evaluate_rows(solution);
        let right_index = self.right_index_for(&rows);

        let mut total = Sc::zero();
        for (left_index, left) in rows.iter().enumerate() {
            let key = (self.left_key_fn)(&left.output);
            let Some(right_indices) = right_index.get(&key) else {
                continue;
            };
            for &right_index in right_indices {
                if right_index == left_index {
                    continue;
                }
                total = total + self.score_evaluation_pair(solution, left, &rows[right_index]);
            }
        }
        total
    }

    fn match_count(&self, solution: &S) -> usize {
        let rows = self.evaluate_rows(solution);
        let right_index = self.right_index_for(&rows);

        let mut count = 0;
        for (left_index, left) in rows.iter().enumerate() {
            let key = (self.left_key_fn)(&left.output);
            let Some(right_indices) = right_index.get(&key) else {
                continue;
            };
            for &right_index in right_indices {
                if right_index != left_index
                    && self.evaluation_pair_matches(solution, left, &rows[right_index])
                {
                    count += 1;
                }
            }
        }
        count
    }

    fn initialize(&mut self, solution: &S) -> Sc {
        self.reset();
        let state = self.source.build_state(solution);
        let mut rows = Vec::new();
        self.source
            .collect_all(solution, &state, |coordinate, output| {
                if self.filter.test(solution, &output) {
                    rows.push((coordinate, output));
                }
            });
        self.source_state = Some(state);

        rows.into_iter()
            .fold(Sc::zero(), |total, (coordinate, output)| {
                total + self.insert_row(solution, coordinate, output)
            })
    }

    fn on_insert(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc {
        let owners = self.localized_owners(descriptor_index, entity_index);
        self.ensure_source_state(solution);
        {
            let state = self.source_state.as_mut().expect("projected source state");
            for owner in &owners {
                self.source.insert_entity_state(
                    solution,
                    state,
                    owner.source_slot,
                    owner.entity_index,
                );
            }
        }
        let mut rows = Vec::new();
        let state = self.source_state.as_ref().expect("projected source state");
        for owner in &owners {
            self.source.collect_entity(
                solution,
                state,
                owner.source_slot,
                owner.entity_index,
                |coordinate, output| {
                    if self.filter.test(solution, &output) {
                        rows.push((coordinate, output));
                    }
                },
            );
        }
        let mut total = Sc::zero();
        for (coordinate, output) in rows {
            total = total + self.insert_row(solution, coordinate, output);
        }
        total
    }

    fn on_retract(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc {
        let owners = self.localized_owners(descriptor_index, entity_index);
        let mut total = Sc::zero();
        for row_id in self.row_ids_for_owners(&owners) {
            total = total + self.retract_row(solution, row_id);
        }
        if let Some(state) = self.source_state.as_mut() {
            for owner in &owners {
                self.source.retract_entity_state(
                    solution,
                    state,
                    owner.source_slot,
                    owner.entity_index,
                );
            }
        }
        total
    }

    fn reset(&mut self) {
        self.source_state = None;
        self.rows.clear();
        self.free_row_ids.clear();
        self.rows_by_owner.clear();
        self.row_ids_by_coordinate.clear();
        self.rows_by_left_key.clear();
        self.rows_by_right_key.clear();
    }

    fn name(&self) -> &str {
        &self.constraint_ref.name
    }

    fn constraint_ref(&self) -> &ConstraintRef {
        &self.constraint_ref
    }

    fn is_hard(&self) -> bool {
        self.is_hard
    }
}
