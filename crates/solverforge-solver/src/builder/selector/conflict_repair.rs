use std::collections::HashSet;

use solverforge_config::ConflictRepairMoveSelectorConfig;

use crate::builder::context::{
    ConflictRepairEdit, ConflictRepairLimits, ConflictRepairProviderEntry, ScalarVariableContext,
};
use crate::heuristic::r#move::{ConflictRepairMove, ConflictRepairScalarEdit};
use crate::heuristic::selector::move_selector::CandidateStore;

pub struct ConflictRepairSelector<S> {
    config: ConflictRepairMoveSelectorConfig,
    scalar_variables: Vec<ScalarVariableContext<S>>,
    providers: Vec<ConflictRepairProviderEntry<S>>,
}

impl<S> ConflictRepairSelector<S> {
    pub fn new(
        config: ConflictRepairMoveSelectorConfig,
        scalar_variables: Vec<ScalarVariableContext<S>>,
        providers: Vec<ConflictRepairProviderEntry<S>>,
    ) -> Self {
        Self {
            config,
            scalar_variables,
            providers,
        }
    }

    fn limits(&self) -> ConflictRepairLimits {
        ConflictRepairLimits {
            max_matches_per_step: self.config.max_matches_per_step,
            max_repairs_per_match: self.config.max_repairs_per_match,
            max_moves_per_step: self.config.max_moves_per_step,
        }
    }

    fn variable_for_edit(
        &self,
        edit: &ConflictRepairEdit,
    ) -> Option<ScalarVariableContext<S>> {
        self.scalar_variables.iter().copied().find(|ctx| {
            ctx.descriptor_index == edit.descriptor_index && ctx.variable_name == edit.variable_name
        })
    }
}

impl<S> std::fmt::Debug for ConflictRepairSelector<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConflictRepairSelector")
            .field("constraints", &self.config.constraints)
            .field("max_matches_per_step", &self.config.max_matches_per_step)
            .field("max_repairs_per_match", &self.config.max_repairs_per_match)
            .field("max_moves_per_step", &self.config.max_moves_per_step)
            .finish()
    }
}

pub struct ConflictRepairCursor<S>
where
    S: PlanningSolution + 'static,
{
    store: CandidateStore<S, ScalarMoveUnion<S, usize>>,
    next_index: usize,
}

impl<S> ConflictRepairCursor<S>
where
    S: PlanningSolution + 'static,
{
    fn new(store: CandidateStore<S, ScalarMoveUnion<S, usize>>) -> Self {
        Self {
            store,
            next_index: 0,
        }
    }
}

impl<S> MoveCursor<S, ScalarMoveUnion<S, usize>> for ConflictRepairCursor<S>
where
    S: PlanningSolution + 'static,
{
    fn next_candidate(&mut self) -> Option<CandidateId> {
        if self.next_index >= self.store.len() {
            return None;
        }
        let id = CandidateId::new(self.next_index);
        self.next_index += 1;
        Some(id)
    }

    fn candidate(
        &self,
        id: CandidateId,
    ) -> Option<MoveCandidateRef<'_, S, ScalarMoveUnion<S, usize>>> {
        self.store.candidate(id)
    }

    fn take_candidate(&mut self, id: CandidateId) -> ScalarMoveUnion<S, usize> {
        self.store.take_candidate(id)
    }
}

impl<S> MoveSelector<S, ScalarMoveUnion<S, usize>> for ConflictRepairSelector<S>
where
    S: PlanningSolution + 'static,
{
    type Cursor<'a>
        = ConflictRepairCursor<S>
    where
        Self: 'a;

    fn open_cursor<'a, D: solverforge_scoring::Director<S>>(
        &'a self,
        score_director: &D,
    ) -> Self::Cursor<'a> {
        let solution = score_director.working_solution();
        let limits = self.limits();
        let mut store = CandidateStore::with_capacity(self.config.max_moves_per_step);
        let mut seen = HashSet::new();

        for constraint_name in &self.config.constraints {
            for provider in self
                .providers
                .iter()
                .filter(|provider| provider.constraint_name == constraint_name)
            {
                for spec in (provider.provider)(solution, limits)
                    .into_iter()
                    .take(self.config.max_repairs_per_match)
                {
                    if store.len() >= self.config.max_moves_per_step {
                        return ConflictRepairCursor::new(store);
                    }
                    if spec.edits.is_empty() || !seen.insert(spec.clone()) {
                        continue;
                    }
                    let mut edits = Vec::with_capacity(spec.edits.len());
                    let mut legal = true;
                    for edit in &spec.edits {
                        let Some(ctx) = self.variable_for_edit(edit) else {
                            legal = false;
                            break;
                        };
                        if !ctx.value_is_legal(solution, edit.entity_index, edit.to_value) {
                            legal = false;
                            break;
                        }
                        edits.push(ConflictRepairScalarEdit {
                            descriptor_index: ctx.descriptor_index,
                            entity_index: edit.entity_index,
                            variable_index: ctx.variable_index,
                            variable_name: ctx.variable_name,
                            to_value: edit.to_value,
                            getter: ctx.getter,
                            setter: ctx.setter,
                        });
                    }
                    if legal {
                        store.push(ScalarMoveUnion::ConflictRepair(ConflictRepairMove::new(
                            spec.reason,
                            edits,
                        )));
                    }
                }
            }
        }

        ConflictRepairCursor::new(store)
    }

    fn size<D: solverforge_scoring::Director<S>>(&self, _score_director: &D) -> usize {
        self.config.max_moves_per_step
    }
}
