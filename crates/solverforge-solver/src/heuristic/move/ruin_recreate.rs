use std::fmt::{self, Debug};

use smallvec::{smallvec, SmallVec};
use solverforge_config::RecreateHeuristicType;
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use solverforge_scoring::{Director, RecordingDirector};

use super::metadata::{encode_option_usize, encode_usize, hash_str, MoveTabuScope, ScopedEntityTabuToken};
use super::{ChangeMove, Move, MoveTabuSignature};

pub enum ScalarRecreateValueSource<S> {
    Empty,
    CountableRange {
        from: usize,
        to: usize,
    },
    SolutionCount {
        count_fn: fn(&S) -> usize,
    },
    EntitySlice {
        values_for_entity: for<'a> fn(&'a S, usize) -> &'a [usize],
    },
}

impl<S> Clone for ScalarRecreateValueSource<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S> Copy for ScalarRecreateValueSource<S> {}

impl<S> Debug for ScalarRecreateValueSource<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "ScalarRecreateValueSource::Empty"),
            Self::CountableRange { from, to } => {
                write!(f, "ScalarRecreateValueSource::CountableRange({from}..{to})")
            }
            Self::SolutionCount { .. } => {
                write!(f, "ScalarRecreateValueSource::SolutionCount(..)")
            }
            Self::EntitySlice { .. } => write!(f, "ScalarRecreateValueSource::EntitySlice(..)"),
        }
    }
}

impl<S> ScalarRecreateValueSource<S> {
    pub fn values_for_entity(&self, solution: &S, entity_index: usize) -> Vec<usize> {
        match self {
            Self::Empty => Vec::new(),
            Self::CountableRange { from, to } => (*from..*to).collect(),
            Self::SolutionCount { count_fn } => (0..count_fn(solution)).collect(),
            Self::EntitySlice { values_for_entity } => {
                values_for_entity(solution, entity_index).to_vec()
            }
        }
    }
}

pub struct RuinRecreateMove<S> {
    entity_indices: SmallVec<[usize; 8]>,
    getter: fn(&S, usize) -> Option<usize>,
    setter: fn(&mut S, usize, Option<usize>),
    descriptor_index: usize,
    variable_name: &'static str,
    value_source: ScalarRecreateValueSource<S>,
    recreate_heuristic_type: RecreateHeuristicType,
    allows_unassigned: bool,
}

impl<S> Clone for RuinRecreateMove<S> {
    fn clone(&self) -> Self {
        Self {
            entity_indices: self.entity_indices.clone(),
            getter: self.getter,
            setter: self.setter,
            descriptor_index: self.descriptor_index,
            variable_name: self.variable_name,
            value_source: self.value_source,
            recreate_heuristic_type: self.recreate_heuristic_type,
            allows_unassigned: self.allows_unassigned,
        }
    }
}

impl<S> Debug for RuinRecreateMove<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RuinRecreateMove")
            .field("entity_indices", &self.entity_indices)
            .field("descriptor_index", &self.descriptor_index)
            .field("variable_name", &self.variable_name)
            .field("recreate_heuristic_type", &self.recreate_heuristic_type)
            .field("allows_unassigned", &self.allows_unassigned)
            .finish()
    }
}

impl<S> RuinRecreateMove<S>
where
    S: PlanningSolution,
{
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        entity_indices: &[usize],
        getter: fn(&S, usize) -> Option<usize>,
        setter: fn(&mut S, usize, Option<usize>),
        descriptor_index: usize,
        variable_name: &'static str,
        value_source: ScalarRecreateValueSource<S>,
        recreate_heuristic_type: RecreateHeuristicType,
        allows_unassigned: bool,
    ) -> Self {
        Self {
            entity_indices: SmallVec::from_slice(entity_indices),
            getter,
            setter,
            descriptor_index,
            variable_name,
            value_source,
            recreate_heuristic_type,
            allows_unassigned,
        }
    }

    fn apply_value<D: Director<S>>(
        &self,
        score_director: &mut D,
        entity_index: usize,
        value: Option<usize>,
    ) {
        score_director.before_variable_changed(self.descriptor_index, entity_index);
        (self.setter)(score_director.working_solution_mut(), entity_index, value);
        score_director.after_variable_changed(self.descriptor_index, entity_index);
    }

    fn evaluate_candidate<D: Director<S>>(
        &self,
        score_director: &mut D,
        mov: &ChangeMove<S, usize>,
    ) -> S::Score
    where
        S: PlanningSolution,
        S::Score: Score,
    {
        let mut recording = RecordingDirector::new(score_director);
        mov.do_move(&mut recording);
        let score = recording.calculate_score();
        recording.undo_changes();
        score
    }

    fn choose_first_fit<D: Director<S>>(
        &self,
        score_director: &mut D,
        entity_index: usize,
    ) -> Option<usize>
    where
        S: PlanningSolution,
        S::Score: Score,
    {
        let baseline_score = self
            .allows_unassigned
            .then(|| score_director.calculate_score());
        for value in self
            .value_source
            .values_for_entity(score_director.working_solution(), entity_index)
        {
            let mov = ChangeMove::new(
                entity_index,
                Some(value),
                self.getter,
                self.setter,
                self.variable_name,
                self.descriptor_index,
            );
            if !mov.is_doable(score_director) {
                continue;
            }
            let score = self.evaluate_candidate(score_director, &mov);
            if baseline_score.is_none_or(|baseline| score > baseline) {
                return Some(value);
            }
        }
        None
    }

    fn choose_cheapest_insertion<D: Director<S>>(
        &self,
        score_director: &mut D,
        entity_index: usize,
    ) -> Option<usize>
    where
        S: PlanningSolution,
        S::Score: Score,
    {
        let baseline_score = self
            .allows_unassigned
            .then(|| score_director.calculate_score());
        let mut best: Option<(usize, usize, S::Score)> = None;

        for (value_index, value) in self
            .value_source
            .values_for_entity(score_director.working_solution(), entity_index)
            .into_iter()
            .enumerate()
        {
            let mov = ChangeMove::new(
                entity_index,
                Some(value),
                self.getter,
                self.setter,
                self.variable_name,
                self.descriptor_index,
            );
            if !mov.is_doable(score_director) {
                continue;
            }
            let score = self.evaluate_candidate(score_director, &mov);
            let should_replace = match best {
                None => true,
                Some((best_value_index, _, best_score)) => {
                    score > best_score || (score == best_score && value_index < best_value_index)
                }
            };
            if should_replace {
                best = Some((value_index, value, score));
            }
        }

        best.and_then(|(_, value, best_score)| {
            baseline_score
                .is_none_or(|baseline| best_score >= baseline)
                .then_some(value)
        })
    }
}

impl<S> Move<S> for RuinRecreateMove<S>
where
    S: PlanningSolution,
    S::Score: Score,
{
    fn is_doable<D: Director<S>>(&self, score_director: &D) -> bool {
        let solution = score_director.working_solution();
        self.entity_indices
            .iter()
            .any(|&entity_index| (self.getter)(solution, entity_index).is_some())
    }

    fn do_move<D: Director<S>>(&self, score_director: &mut D) {
        let old_values: SmallVec<[(usize, Option<usize>); 8]> = self
            .entity_indices
            .iter()
            .map(|&entity_index| {
                (
                    entity_index,
                    (self.getter)(score_director.working_solution(), entity_index),
                )
            })
            .collect();

        for &entity_index in &self.entity_indices {
            self.apply_value(score_director, entity_index, None);
        }

        for &entity_index in &self.entity_indices {
            if (self.getter)(score_director.working_solution(), entity_index).is_some() {
                continue;
            }

            let selected = match self.recreate_heuristic_type {
                RecreateHeuristicType::FirstFit => {
                    self.choose_first_fit(score_director, entity_index)
                }
                RecreateHeuristicType::CheapestInsertion => {
                    self.choose_cheapest_insertion(score_director, entity_index)
                }
            };

            if let Some(value) = selected {
                self.apply_value(score_director, entity_index, Some(value));
            }
        }

        let setter = self.setter;
        score_director.register_undo(Box::new(move |solution: &mut S| {
            for (entity_index, old_value) in old_values {
                setter(solution, entity_index, old_value);
            }
        }));
    }

    fn descriptor_index(&self) -> usize {
        self.descriptor_index
    }

    fn entity_indices(&self) -> &[usize] {
        &self.entity_indices
    }

    fn variable_name(&self) -> &str {
        self.variable_name
    }

    fn tabu_signature<D: Director<S>>(&self, score_director: &D) -> MoveTabuSignature {
        let variable_id = hash_str(self.variable_name);
        let heuristic_id = match self.recreate_heuristic_type {
            RecreateHeuristicType::FirstFit => hash_str("first_fit"),
            RecreateHeuristicType::CheapestInsertion => hash_str("cheapest_insertion"),
        };
        let scope = MoveTabuScope::new(self.descriptor_index, self.variable_name);
        let entity_ids: SmallVec<[u64; 2]> = self
            .entity_indices
            .iter()
            .map(|&entity_index| encode_usize(entity_index))
            .collect();
        let entity_tokens: SmallVec<[ScopedEntityTabuToken; 2]> = entity_ids
            .iter()
            .copied()
            .map(|entity_id| scope.entity_token(entity_id))
            .collect();
        let mut move_id = smallvec![
            hash_str("ruin_recreate"),
            encode_usize(self.descriptor_index),
            variable_id,
            heuristic_id,
            encode_usize(self.entity_indices.len()),
        ];
        let mut undo_move_id = move_id.clone();
        for &entity_index in &self.entity_indices {
            move_id.push(encode_usize(entity_index));
            undo_move_id.push(encode_usize(entity_index));
            let current = (self.getter)(score_director.working_solution(), entity_index);
            move_id.push(encode_option_usize(current));
            undo_move_id.push(encode_option_usize(current));
        }

        MoveTabuSignature::new(scope, move_id, undo_move_id).with_entity_tokens(entity_tokens)
    }
}
