use std::borrow::Cow;
use std::fmt;
use std::marker::PhantomData;

use smallvec::SmallVec;
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use solverforge_scoring::Director;

use crate::builder::context::RuntimeListElement;
use crate::heuristic::r#move::list_kernel::{
    change_do_move, change_is_doable, change_undo_move, k_opt_do_move, k_opt_do_move_into,
    k_opt_is_doable, k_opt_undo_move, multi_swap_do_move, multi_swap_is_doable,
    multi_swap_undo_move, permute_do_move, permute_is_doable, permute_undo_move, reverse_do_move,
    reverse_is_doable, ruin_do_move, ruin_is_doable, ruin_undo_move, sublist_change_do_move,
    sublist_change_is_doable, sublist_change_undo_move, sublist_swap_do_move,
    sublist_swap_is_doable, sublist_swap_undo_move, swap_do_move, swap_is_doable, ListMoveAccess,
};
use crate::heuristic::r#move::{ListUndoJournal, Move, MoveTabuSignature};
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;
use crate::stats::CandidateTraceIdentity;

use super::move_access::RuntimeListMoveAccess;
use super::recipe::{recipe_access, recipe_entity_indices, recipe_family, RuntimeListMoveUndo};
use super::recipe_identity::{recipe_candidate_trace_identity, recipe_tabu_signature};
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

impl<S, V, DM, IDM> RuntimeListMove<S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    /// Whether this move records an arbitrary-length undo journal whose
    /// capacity a later trial can reuse.
    pub(crate) fn reuses_journal(&self) -> bool {
        matches!(self.recipe, RuntimeListRecipe::KOpt { .. })
    }

    /// Applies the move, recording into `retired` when it is a journal shell.
    /// Wrapping carriers take the shell from their own pool, so no layer
    /// builds a scratch pool of this move's undo type.
    pub(crate) fn do_move_reusing<D: Director<S>>(
        &self,
        score_director: &mut D,
        retired: Option<ListUndoJournal<RuntimeListElement<V>>>,
    ) -> RuntimeListMoveUndo<V> {
        let RuntimeListRecipe::KOpt {
            access,
            entity,
            cuts,
            reconnection,
            ..
        } = &self.recipe
        else {
            return Move::do_move(self, score_director);
        };
        let mut journal = retired.unwrap_or_default();
        k_opt_do_move_into(
            access,
            cuts,
            reconnection,
            *entity,
            score_director,
            &mut journal,
        );
        RuntimeListMoveUndo::KOpt(journal)
    }

    /// Reverts the move and returns an empty journal shell for the next
    /// trial when the undo consumed a journal.
    pub(crate) fn undo_move_retiring<D: Director<S>>(
        &self,
        score_director: &mut D,
        undo: RuntimeListMoveUndo<V>,
    ) -> Option<ListUndoJournal<RuntimeListElement<V>>> {
        let RuntimeListMoveUndo::KOpt(values) = undo else {
            Move::undo_move(self, score_director, undo);
            return None;
        };
        let RuntimeListRecipe::KOpt { access, entity, .. } = &self.recipe else {
            panic!("runtime list move undo shape must match recipe");
        };
        // The undo consumes the restored route journal; a fresh shell keeps
        // the inline capacity for the next K-opt trial.
        k_opt_undo_move(access, *entity, values, score_director);
        Some(ListUndoJournal::new())
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

    fn do_move_with_scratch<D: Director<S>>(
        &self,
        score_director: &mut D,
        journal_pool: &mut Vec<Self::Undo>,
    ) -> Self::Undo {
        if !self.reuses_journal() {
            return Move::do_move(self, score_director);
        }
        let retired = journal_pool
            .iter()
            .rposition(|undo| matches!(undo, RuntimeListMoveUndo::KOpt(_)))
            .map(|index| match journal_pool.swap_remove(index) {
                RuntimeListMoveUndo::KOpt(journal) => journal,
                _ => unreachable!("rposition matched the KOpt shape"),
            });
        self.do_move_reusing(score_director, retired)
    }

    fn undo_move_with_scratch<D: Director<S>>(
        &self,
        score_director: &mut D,
        undo: Self::Undo,
        journal_pool: &mut Vec<Self::Undo>,
    ) {
        if let Some(shell) = self.undo_move_retiring(score_director, undo) {
            journal_pool.push(RuntimeListMoveUndo::KOpt(shell));
        }
    }

    fn do_move<D: Director<S>>(&self, score_director: &mut D) -> Self::Undo {
        match &self.recipe {
            RuntimeListRecipe::Change {
                access,
                coordinates,
            } => {
                change_do_move(access, *coordinates, score_director);
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
                inverse_permutation,
            } => {
                permute_do_move(access, *coordinates, permutation, score_director);
                RuntimeListMoveUndo::Permute(inverse_permutation.clone())
            }
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
                RuntimeListMoveUndo::Permute(inverse_permutation),
            ) => permute_undo_move(
                access,
                *coordinates,
                inverse_permutation.as_slice(),
                score_director,
            ),
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

    fn tabu_signature<D: Director<S>>(&self, score_director: &D) -> Cow<'_, MoveTabuSignature> {
        Cow::Owned(recipe_tabu_signature(
            &self.recipe,
            &self.entity_indices,
            score_director,
        ))
    }

    fn candidate_trace_identity(&self) -> Option<CandidateTraceIdentity> {
        recipe_candidate_trace_identity(&self.recipe)
    }
}
