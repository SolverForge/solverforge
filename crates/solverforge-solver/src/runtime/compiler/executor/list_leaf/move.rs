use std::fmt;
use std::marker::PhantomData;

use smallvec::SmallVec;
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use solverforge_scoring::Director;

use crate::heuristic::r#move::list_kernel::{
    change_candidate_trace_identity, change_do_move, change_is_doable, change_tabu_signature,
    change_undo_move, k_opt_do_move, k_opt_is_doable, k_opt_tabu_signature, k_opt_undo_move,
    multi_swap_do_move, multi_swap_is_doable, multi_swap_tabu_signature, multi_swap_undo_move,
    permute_candidate_trace_identity, permute_do_move, permute_is_doable, permute_tabu_signature,
    permute_undo_move, reverse_candidate_trace_identity, reverse_do_move, reverse_is_doable,
    reverse_tabu_signature, ruin_do_move, ruin_is_doable, ruin_tabu_signature, ruin_undo_move,
    sublist_change_candidate_trace_identity, sublist_change_do_move, sublist_change_is_doable,
    sublist_change_tabu_signature, sublist_change_undo_move, sublist_swap_candidate_trace_identity,
    sublist_swap_do_move, sublist_swap_is_doable, sublist_swap_tabu_signature,
    sublist_swap_undo_move, swap_candidate_trace_identity, swap_do_move, swap_is_doable,
    swap_tabu_signature, ChangeValueTransfer, ListMoveAccess, RuinValueTransfer,
};
use crate::heuristic::r#move::{Move, MoveTabuSignature};
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;
use crate::stats::CandidateTraceIdentity;

use super::move_access::RuntimeListMoveAccess;
use super::recipe::{recipe_access, recipe_entity_indices, recipe_family, RuntimeListMoveUndo};
use super::spec::RuntimeListRecipe;

#[derive(Clone)]
pub(crate) struct RuntimeListMove<S, V, DM, IDM> {
    recipe: RuntimeListRecipe<S, V>,
    entity_indices: SmallVec<[usize; 8]>,
    _phantom: PhantomData<fn() -> (DM, IDM)>,
}

impl<S, V, DM, IDM> RuntimeListMove<S, V, DM, IDM> {
    pub(super) fn new(recipe: RuntimeListRecipe<S, V>) -> Self {
        let entity_indices = recipe_entity_indices(&recipe);
        Self {
            recipe,
            entity_indices,
            _phantom: PhantomData,
        }
    }

    #[cfg(test)]
    pub(super) fn into_recipe(self) -> RuntimeListRecipe<S, V> {
        self.recipe
    }

    fn access(&self) -> &RuntimeListMoveAccess<S, V> {
        recipe_access(&self.recipe)
    }
}

impl<S, V, DM, IDM> fmt::Debug for RuntimeListMove<S, V, DM, IDM> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RuntimeListMove")
            .field("family", &recipe_family(&self.recipe))
            .field("access", self.access())
            .field("entity_indices", &self.entity_indices)
            .finish()
    }
}

impl<S, V, DM, IDM> Move<S> for RuntimeListMove<S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    type Undo = RuntimeListMoveUndo<V>;

    fn is_doable<D: Director<S>>(&self, score_director: &D) -> bool {
        match &self.recipe {
            RuntimeListRecipe::Change {
                access,
                coordinates,
            } => change_is_doable(access, *coordinates, score_director),
            RuntimeListRecipe::Swap {
                access,
                coordinates,
            } => swap_is_doable(access, *coordinates, score_director),
            RuntimeListRecipe::Permute {
                access,
                coordinates,
                permutation,
                ..
            } => permute_is_doable(access, *coordinates, permutation, score_director),
            RuntimeListRecipe::Reverse {
                access,
                coordinates,
            } => reverse_is_doable(access, *coordinates, score_director),
            RuntimeListRecipe::SublistChange {
                access,
                coordinates,
            } => sublist_change_is_doable(access, *coordinates, score_director),
            RuntimeListRecipe::SublistSwap {
                access,
                coordinates,
            } => sublist_swap_is_doable(access, *coordinates, score_director),
            RuntimeListRecipe::KOpt {
                access,
                entity,
                cuts,
                reconnection,
                ..
            } => k_opt_is_doable(access, cuts, reconnection, *entity, score_director),
            RuntimeListRecipe::Ruin {
                access, sources, ..
            } => ruin_is_doable(access, sources, score_director),
            RuntimeListRecipe::MultiSwap {
                access,
                coordinates,
                ..
            } => multi_swap_is_doable(access, coordinates, score_director),
        }
    }

    fn do_move<D: Director<S>>(&self, score_director: &mut D) -> Self::Undo {
        match &self.recipe {
            RuntimeListRecipe::Change {
                access,
                coordinates,
            } => {
                change_do_move(
                    access,
                    *coordinates,
                    ChangeValueTransfer::MoveIntoInsert,
                    score_director,
                );
                RuntimeListMoveUndo::None
            }
            RuntimeListRecipe::Swap {
                access,
                coordinates,
            } => {
                swap_do_move(access, *coordinates, score_director);
                RuntimeListMoveUndo::None
            }
            RuntimeListRecipe::Permute {
                access,
                coordinates,
                permutation,
                ..
            } => RuntimeListMoveUndo::Permute(permute_do_move(
                access,
                *coordinates,
                permutation,
                score_director,
            )),
            RuntimeListRecipe::Reverse {
                access,
                coordinates,
            } => {
                reverse_do_move(access, *coordinates, score_director);
                RuntimeListMoveUndo::None
            }
            RuntimeListRecipe::SublistChange {
                access,
                coordinates,
            } => {
                sublist_change_do_move(access, *coordinates, score_director);
                RuntimeListMoveUndo::None
            }
            RuntimeListRecipe::SublistSwap {
                access,
                coordinates,
            } => {
                sublist_swap_do_move(access, *coordinates, score_director);
                RuntimeListMoveUndo::None
            }
            RuntimeListRecipe::KOpt {
                access,
                entity,
                cuts,
                reconnection,
                ..
            } => RuntimeListMoveUndo::KOpt(k_opt_do_move(
                access,
                cuts,
                reconnection,
                *entity,
                score_director,
            )),
            RuntimeListRecipe::Ruin {
                access,
                sources,
                skip_empty_destinations,
            } => RuntimeListMoveUndo::Ruin(ruin_do_move(
                access,
                sources,
                *skip_empty_destinations,
                RuinValueTransfer::MoveIntoInsert,
                score_director,
            )),
            RuntimeListRecipe::MultiSwap {
                access,
                coordinates,
                ..
            } => {
                multi_swap_do_move(access, coordinates, &self.entity_indices, score_director);
                RuntimeListMoveUndo::None
            }
        }
    }

    fn undo_move<D: Director<S>>(&self, score_director: &mut D, undo: Self::Undo) {
        match (&self.recipe, undo) {
            (
                RuntimeListRecipe::Change {
                    access,
                    coordinates,
                },
                RuntimeListMoveUndo::None,
            ) => change_undo_move(access, *coordinates, score_director),
            (
                RuntimeListRecipe::Swap {
                    access,
                    coordinates,
                },
                RuntimeListMoveUndo::None,
            ) => swap_do_move(access, *coordinates, score_director),
            (
                RuntimeListRecipe::Permute {
                    access,
                    coordinates,
                    ..
                },
                RuntimeListMoveUndo::Permute(values),
            ) => permute_undo_move(access, *coordinates, values, score_director),
            (
                RuntimeListRecipe::Reverse {
                    access,
                    coordinates,
                },
                RuntimeListMoveUndo::None,
            ) => reverse_do_move(access, *coordinates, score_director),
            (
                RuntimeListRecipe::SublistChange {
                    access,
                    coordinates,
                },
                RuntimeListMoveUndo::None,
            ) => sublist_change_undo_move(access, *coordinates, score_director),
            (
                RuntimeListRecipe::SublistSwap {
                    access,
                    coordinates,
                },
                RuntimeListMoveUndo::None,
            ) => sublist_swap_undo_move(access, *coordinates, score_director),
            (RuntimeListRecipe::KOpt { access, entity, .. }, RuntimeListMoveUndo::KOpt(values)) => {
                k_opt_undo_move(access, *entity, values, score_director)
            }
            (
                RuntimeListRecipe::Ruin {
                    access, sources, ..
                },
                RuntimeListMoveUndo::Ruin(undo),
            ) => ruin_undo_move(access, sources, undo, score_director),
            (
                RuntimeListRecipe::MultiSwap {
                    access,
                    coordinates,
                    ..
                },
                RuntimeListMoveUndo::None,
            ) => multi_swap_undo_move(access, coordinates, &self.entity_indices, score_director),
            _ => panic!("runtime list move undo shape must match recipe"),
        }
    }

    fn descriptor_index(&self) -> usize {
        ListMoveAccess::descriptor_index(self.access())
    }

    fn entity_indices(&self) -> &[usize] {
        &self.entity_indices
    }

    fn variable_name(&self) -> &str {
        ListMoveAccess::variable_name(self.access())
    }

    fn telemetry_label(&self) -> &'static str {
        match &self.recipe {
            RuntimeListRecipe::Change { .. } => "runtime_list_change",
            RuntimeListRecipe::Swap { .. } => "runtime_list_swap",
            RuntimeListRecipe::Permute { .. } => "runtime_list_permute",
            RuntimeListRecipe::Reverse { .. } => "runtime_list_reverse",
            RuntimeListRecipe::SublistChange { .. } => "runtime_sublist_change",
            RuntimeListRecipe::SublistSwap { .. } => "runtime_sublist_swap",
            RuntimeListRecipe::KOpt { .. } => "runtime_k_opt",
            RuntimeListRecipe::Ruin { .. } => "runtime_list_ruin",
            RuntimeListRecipe::MultiSwap { .. } => "runtime_list_multi_swap",
        }
    }

    fn requires_score_improvement(&self) -> bool {
        matches!(
            self.recipe,
            RuntimeListRecipe::MultiSwap {
                require_score_improvement: true,
                ..
            }
        )
    }

    fn tabu_signature<D: Director<S>>(&self, score_director: &D) -> MoveTabuSignature {
        match &self.recipe {
            RuntimeListRecipe::Change {
                access,
                coordinates,
            } => change_tabu_signature(access, *coordinates, score_director),
            RuntimeListRecipe::Swap {
                access,
                coordinates,
            } => swap_tabu_signature(access, *coordinates, score_director),
            RuntimeListRecipe::Permute {
                access,
                coordinates,
                permutation,
                inverse_permutation,
            } => permute_tabu_signature(
                access,
                *coordinates,
                permutation,
                inverse_permutation,
                score_director,
            ),
            RuntimeListRecipe::Reverse {
                access,
                coordinates,
            } => reverse_tabu_signature(access, *coordinates, score_director),
            RuntimeListRecipe::SublistChange {
                access,
                coordinates,
            } => sublist_change_tabu_signature(access, *coordinates, score_director),
            RuntimeListRecipe::SublistSwap {
                access,
                coordinates,
            } => sublist_swap_tabu_signature(access, *coordinates, score_director),
            RuntimeListRecipe::KOpt {
                access,
                entity,
                cuts,
                reconnection,
                variable_id,
            } => k_opt_tabu_signature(
                access,
                cuts,
                reconnection,
                *variable_id,
                *entity,
                score_director,
            ),
            RuntimeListRecipe::Ruin {
                access, sources, ..
            } => ruin_tabu_signature(access, sources, &self.entity_indices, score_director),
            RuntimeListRecipe::MultiSwap {
                access,
                coordinates,
                ..
            } => {
                multi_swap_tabu_signature(access, coordinates, &self.entity_indices, score_director)
            }
        }
    }

    fn candidate_trace_identity(&self) -> Option<CandidateTraceIdentity> {
        let identity = match &self.recipe {
            RuntimeListRecipe::Change {
                access,
                coordinates,
            } => change_candidate_trace_identity(access, *coordinates),
            RuntimeListRecipe::Swap {
                access,
                coordinates,
            } => swap_candidate_trace_identity(access, *coordinates),
            RuntimeListRecipe::Permute {
                access,
                coordinates,
                permutation,
                ..
            } => permute_candidate_trace_identity(access, *coordinates, permutation),
            RuntimeListRecipe::Reverse {
                access,
                coordinates,
            } => reverse_candidate_trace_identity(access, *coordinates),
            RuntimeListRecipe::SublistChange {
                access,
                coordinates,
            } => sublist_change_candidate_trace_identity(access, *coordinates),
            RuntimeListRecipe::SublistSwap {
                access,
                coordinates,
            } => sublist_swap_candidate_trace_identity(access, *coordinates),
            RuntimeListRecipe::KOpt {
                access,
                entity,
                cuts,
                reconnection,
                ..
            } => CandidateTraceIdentity::logical_move(
                ListMoveAccess::descriptor_index(access),
                ListMoveAccess::variable_name(access),
                "k_opt",
                cuts.iter().flat_map(|cut| [*entity, cut.position()]).chain(
                    reconnection
                        .segment_order()
                        .iter()
                        .copied()
                        .map(usize::from),
                ),
            ),
            RuntimeListRecipe::Ruin {
                access, sources, ..
            } => CandidateTraceIdentity::logical_move(
                ListMoveAccess::descriptor_index(access),
                ListMoveAccess::variable_name(access),
                "list_ruin",
                sources.iter().flat_map(|(entity, indices)| {
                    std::iter::once(*entity)
                        .chain(std::iter::once(indices.len()))
                        .chain(indices.iter().copied())
                }),
            ),
            RuntimeListRecipe::MultiSwap {
                access,
                coordinates,
                ..
            } => CandidateTraceIdentity::logical_move(
                ListMoveAccess::descriptor_index(access),
                ListMoveAccess::variable_name(access),
                "list_multi_swap",
                coordinates
                    .iter()
                    .flat_map(|&(entity, first, second)| [entity, first, second]),
            ),
        };
        Some(identity)
    }
}
