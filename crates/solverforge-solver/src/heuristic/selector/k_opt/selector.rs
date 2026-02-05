//! K-opt move selector for tour optimization.

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use crate::heuristic::r#move::k_opt_reconnection::{
    enumerate_reconnections, KOptReconnection, THREE_OPT_RECONNECTIONS,
};
use crate::heuristic::r#move::KOptMove;

use super::super::entity::EntitySelector;
use super::super::typed_move_selector::MoveSelector;
use super::config::KOptConfig;
use super::iterators::count_cut_combinations;
use super::iterators::CutCombinationIterator;

/// A move selector that generates k-opt moves.
///
/// Enumerates all valid cut point combinations for each selected entity
/// and generates moves for each reconnection pattern.
pub struct KOptMoveSelector<S, V, ES> {
    /// Selects entities (routes) to apply k-opt to.
    entity_selector: ES,
    /// K-opt configuration.
    config: KOptConfig,
    /// Reconnection patterns to use.
    patterns: Vec<&'static KOptReconnection>,
    /// Get list length for an entity.
    list_len: fn(&S, usize) -> usize,
    /// Remove sublist [start, end).
    sublist_remove: fn(&mut S, usize, usize, usize) -> Vec<V>,
    /// Insert elements at position.
    sublist_insert: fn(&mut S, usize, usize, Vec<V>),
    /// Variable name.
    variable_name: &'static str,
    /// Descriptor index.
    descriptor_index: usize,
    _phantom: PhantomData<(S, V)>,
}

impl<S, V: Debug, ES: Debug> Debug for KOptMoveSelector<S, V, ES> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KOptMoveSelector")
            .field("entity_selector", &self.entity_selector)
            .field("config", &self.config)
            .field("pattern_count", &self.patterns.len())
            .field("variable_name", &self.variable_name)
            .finish()
    }
}

impl<S: PlanningSolution, V, ES> KOptMoveSelector<S, V, ES> {
    /// Creates a new k-opt move selector.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        entity_selector: ES,
        config: KOptConfig,
        list_len: fn(&S, usize) -> usize,
        sublist_remove: fn(&mut S, usize, usize, usize) -> Vec<V>,
        sublist_insert: fn(&mut S, usize, usize, Vec<V>),
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        // Get static patterns for k=3, generate for others
        let patterns: Vec<&'static KOptReconnection> = if config.k == 3 {
            THREE_OPT_RECONNECTIONS.iter().collect()
        } else {
            // For other k values, we need to leak the patterns to get 'static lifetime
            // This is a one-time allocation per selector creation
            let generated = enumerate_reconnections(config.k);
            let leaked: &'static [KOptReconnection] = Box::leak(generated.into_boxed_slice());
            leaked.iter().collect()
        };

        Self {
            entity_selector,
            config,
            patterns,
            list_len,
            sublist_remove,
            sublist_insert,
            variable_name,
            descriptor_index,
            _phantom: PhantomData,
        }
    }
}

impl<S, V, ES> MoveSelector<S, KOptMove<S, V>> for KOptMoveSelector<S, V, ES>
where
    S: PlanningSolution,
    ES: EntitySelector<S>,
    V: Clone + Send + Sync + Debug + 'static,
{
    fn iter_moves<'a, D: ScoreDirector<S>>(
        &'a self,
        score_director: &'a D,
    ) -> Box<dyn Iterator<Item = KOptMove<S, V>> + 'a> {
        let k = self.config.k;
        let min_seg = self.config.min_segment_len;
        let patterns = &self.patterns;
        let list_len = self.list_len;
        let sublist_remove = self.sublist_remove;
        let sublist_insert = self.sublist_insert;
        let variable_name = self.variable_name;
        let descriptor_index = self.descriptor_index;

        let iter = self
            .entity_selector
            .iter(score_director)
            .flat_map(move |entity_ref| {
                let entity_idx = entity_ref.entity_index;
                let solution = score_director.working_solution();
                let len = list_len(solution, entity_idx);

                // Generate all valid cut combinations
                let cuts_iter = CutCombinationIterator::new(k, len, min_seg, entity_idx);

                cuts_iter.flat_map(move |cuts| {
                    // For each cut combination, generate moves for each pattern
                    patterns.iter().map(move |&pattern| {
                        KOptMove::new(
                            &cuts,
                            pattern,
                            list_len,
                            sublist_remove,
                            sublist_insert,
                            variable_name,
                            descriptor_index,
                        )
                    })
                })
            });

        Box::new(iter)
    }

    fn size<D: ScoreDirector<S>>(&self, score_director: &D) -> usize {
        let k = self.config.k;
        let min_seg = self.config.min_segment_len;
        let pattern_count = self.patterns.len();

        self.entity_selector
            .iter(score_director)
            .map(|entity_ref| {
                let solution = score_director.working_solution();
                let len = (self.list_len)(solution, entity_ref.entity_index);
                count_cut_combinations(k, len, min_seg) * pattern_count
            })
            .sum()
    }
}
