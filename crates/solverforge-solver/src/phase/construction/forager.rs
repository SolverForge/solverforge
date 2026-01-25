//! Foragers for construction heuristic move selection
//!
//! Foragers determine which move to select from the candidates
//! generated for each entity placement.
//!
//! # Zero-Erasure Design
//!
//! Foragers return indices into the placement's move Vec, not cloned moves.
//! The caller takes ownership via the index.

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use solverforge_scoring::api::constraint_set::ConstraintSet;
use solverforge_scoring::ScoreDirector;

use crate::heuristic::r#move::Move;

use super::Placement;

/// Trait for selecting a move during construction.
///
/// Foragers evaluate candidate moves and pick one based on their strategy.
/// Returns the index of the selected move, not a cloned move.
///
/// # Type Parameters
/// * `S` - The planning solution type
/// * `M` - The move type
pub trait ConstructionForager<S, M>: Send + Debug
where
    S: PlanningSolution,
    M: Move<S>,
{
    /// Picks a move index from the placement's candidates.
    ///
    /// Returns None if no suitable move is found.
    fn pick_move_index<C>(
        &self,
        placement: &Placement<S, M>,
        score_director: &mut ScoreDirector<S, C>,
    ) -> Option<usize>
    where
        C: ConstraintSet<S, S::Score>,
        S::Score: Score;
}

/// First Fit forager - picks the first feasible move.
///
/// This is the fastest forager but may not produce optimal results.
/// It simply takes the first move that can be executed.
pub struct FirstFitForager<S, M> {
    _phantom: PhantomData<fn() -> (S, M)>,
}

impl<S, M> Clone for FirstFitForager<S, M> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S, M> Copy for FirstFitForager<S, M> {}

impl<S, M> Default for FirstFitForager<S, M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S, M> Debug for FirstFitForager<S, M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FirstFitForager").finish()
    }
}

impl<S, M> FirstFitForager<S, M> {
    /// Creates a new First Fit forager.
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<S, M> ConstructionForager<S, M> for FirstFitForager<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    fn pick_move_index<C>(
        &self,
        placement: &Placement<S, M>,
        score_director: &mut ScoreDirector<S, C>,
    ) -> Option<usize>
    where
        C: ConstraintSet<S, S::Score>,
        S::Score: Score,
    {
        for (idx, m) in placement.moves.iter().enumerate() {
            if m.is_doable(score_director) {
                return Some(idx);
            }
        }
        None
    }
}

/// Best Fit forager - evaluates all moves and picks the best.
///
/// This forager evaluates each candidate move by executing it,
/// calculating the score, and undoing it. The move with the best
/// score is selected.
pub struct BestFitForager<S, M> {
    _phantom: PhantomData<fn() -> (S, M)>,
}

impl<S, M> Clone for BestFitForager<S, M> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S, M> Copy for BestFitForager<S, M> {}

impl<S, M> Default for BestFitForager<S, M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S, M> Debug for BestFitForager<S, M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BestFitForager").finish()
    }
}

impl<S, M> BestFitForager<S, M> {
    /// Creates a new Best Fit forager.
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<S, M> ConstructionForager<S, M> for BestFitForager<S, M>
where
    S: PlanningSolution + solverforge_scoring::ShadowVariableSupport,
    M: Move<S>,
{
    fn pick_move_index<C>(
        &self,
        placement: &Placement<S, M>,
        score_director: &mut ScoreDirector<S, C>,
    ) -> Option<usize>
    where
        C: ConstraintSet<S, S::Score>,
        S::Score: Score,
    {
        let mut best_idx: Option<usize> = None;
        let mut best_score: Option<S::Score> = None;

        for (idx, m) in placement.moves.iter().enumerate() {
            if !m.is_doable(score_director) {
                continue;
            }

            // Evaluate move: save score, execute, calculate, undo
            score_director.save_score_snapshot();
            m.do_move(score_director);
            let score = score_director.calculate_score();
            score_director.undo_changes();

            // Check if this is the best so far
            let is_better = match &best_score {
                None => true,
                Some(best) => score > *best,
            };

            if is_better {
                best_idx = Some(idx);
                best_score = Some(score);
            }
        }

        best_idx
    }
}

/// First Feasible forager - picks the first move that results in a feasible score.
///
/// This forager evaluates moves until it finds one that produces a feasible
/// (non-negative hard score) solution.
pub struct FirstFeasibleForager<S, M> {
    _phantom: PhantomData<fn() -> (S, M)>,
}

impl<S, M> Clone for FirstFeasibleForager<S, M> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S, M> Copy for FirstFeasibleForager<S, M> {}

impl<S, M> Default for FirstFeasibleForager<S, M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S, M> Debug for FirstFeasibleForager<S, M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FirstFeasibleForager").finish()
    }
}

impl<S, M> FirstFeasibleForager<S, M> {
    /// Creates a new First Feasible forager.
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<S, M> ConstructionForager<S, M> for FirstFeasibleForager<S, M>
where
    S: PlanningSolution + solverforge_scoring::ShadowVariableSupport,
    M: Move<S>,
{
    fn pick_move_index<C>(
        &self,
        placement: &Placement<S, M>,
        score_director: &mut ScoreDirector<S, C>,
    ) -> Option<usize>
    where
        C: ConstraintSet<S, S::Score>,
        S::Score: Score,
    {
        let mut fallback_idx: Option<usize> = None;
        let mut fallback_score: Option<S::Score> = None;

        for (idx, m) in placement.moves.iter().enumerate() {
            if !m.is_doable(score_director) {
                continue;
            }

            // Evaluate move: save score, execute, calculate, undo
            score_director.save_score_snapshot();
            m.do_move(score_director);
            let score = score_director.calculate_score();

            // If feasible, undo and return this move index immediately
            if score.is_feasible() {
                score_director.undo_changes();
                return Some(idx);
            }

            // Undo move
            score_director.undo_changes();

            // Track best infeasible as fallback
            let is_better = match &fallback_score {
                None => true,
                Some(best) => score > *best,
            };

            if is_better {
                fallback_idx = Some(idx);
                fallback_score = Some(score);
            }
        }

        // No feasible move found, return best infeasible
        fallback_idx
    }
}

/// Weakest Fit forager - picks the move with the lowest strength value.
///
/// This forager evaluates each candidate move using a strength function
/// and selects the move with the minimum strength. This is useful for
/// assigning the "weakest" or least constraining values first.
pub struct WeakestFitForager<S, M> {
    /// Function to evaluate strength of a move.
    strength_fn: fn(&M) -> i64,
    _phantom: PhantomData<fn() -> S>,
}

impl<S, M> Clone for WeakestFitForager<S, M> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S, M> Copy for WeakestFitForager<S, M> {}

impl<S, M> Debug for WeakestFitForager<S, M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WeakestFitForager").finish()
    }
}

impl<S, M> WeakestFitForager<S, M> {
    /// Creates a new Weakest Fit forager with the given strength function.
    ///
    /// The strength function evaluates how "strong" a move is. The forager
    /// picks the move with the minimum strength value.
    pub fn new(strength_fn: fn(&M) -> i64) -> Self {
        Self {
            strength_fn,
            _phantom: PhantomData,
        }
    }
}

impl<S, M> ConstructionForager<S, M> for WeakestFitForager<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    fn pick_move_index<C>(
        &self,
        placement: &Placement<S, M>,
        score_director: &mut ScoreDirector<S, C>,
    ) -> Option<usize>
    where
        C: ConstraintSet<S, S::Score>,
        S::Score: Score,
    {
        let mut best_idx: Option<usize> = None;
        let mut min_strength: Option<i64> = None;

        for (idx, m) in placement.moves.iter().enumerate() {
            if !m.is_doable(score_director) {
                continue;
            }

            let strength = (self.strength_fn)(m);

            let is_weaker = match min_strength {
                None => true,
                Some(best) => strength < best,
            };

            if is_weaker {
                best_idx = Some(idx);
                min_strength = Some(strength);
            }
        }

        best_idx
    }
}

/// Strongest Fit forager - picks the move with the highest strength value.
///
/// This forager evaluates each candidate move using a strength function
/// and selects the move with the maximum strength. This is useful for
/// assigning the "strongest" or most constraining values first.
pub struct StrongestFitForager<S, M> {
    /// Function to evaluate strength of a move.
    strength_fn: fn(&M) -> i64,
    _phantom: PhantomData<fn() -> S>,
}

impl<S, M> Clone for StrongestFitForager<S, M> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S, M> Copy for StrongestFitForager<S, M> {}

impl<S, M> Debug for StrongestFitForager<S, M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StrongestFitForager").finish()
    }
}

impl<S, M> StrongestFitForager<S, M> {
    /// Creates a new Strongest Fit forager with the given strength function.
    ///
    /// The strength function evaluates how "strong" a move is. The forager
    /// picks the move with the maximum strength value.
    pub fn new(strength_fn: fn(&M) -> i64) -> Self {
        Self {
            strength_fn,
            _phantom: PhantomData,
        }
    }
}

impl<S, M> ConstructionForager<S, M> for StrongestFitForager<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    fn pick_move_index<C>(
        &self,
        placement: &Placement<S, M>,
        score_director: &mut ScoreDirector<S, C>,
    ) -> Option<usize>
    where
        C: ConstraintSet<S, S::Score>,
        S::Score: Score,
    {
        let mut best_idx: Option<usize> = None;
        let mut max_strength: Option<i64> = None;

        for (idx, m) in placement.moves.iter().enumerate() {
            if !m.is_doable(score_director) {
                continue;
            }

            let strength = (self.strength_fn)(m);

            let is_stronger = match max_strength {
                None => true,
                Some(best) => strength > best,
            };

            if is_stronger {
                best_idx = Some(idx);
                max_strength = Some(strength);
            }
        }

        best_idx
    }
}

/// Cheapest Insertion forager - early-pick on non-worsening score.
///
/// This forager evaluates moves until it finds one with a score >= the last step score
/// (early-pick for greedy improvement). If no such move is found, it falls back to
/// the best-scoring move.
///
/// This is more efficient than BestFit when most moves are non-worsening, as it
/// avoids evaluating all candidates.
pub struct CheapestInsertionForager<S: PlanningSolution, M> {
    last_step_score: Option<S::Score>,
    _phantom: PhantomData<fn() -> (S, M)>,
}

impl<S: PlanningSolution, M> Clone for CheapestInsertionForager<S, M> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S: PlanningSolution, M> Copy for CheapestInsertionForager<S, M> {}

impl<S: PlanningSolution, M> Debug for CheapestInsertionForager<S, M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CheapestInsertionForager").finish()
    }
}

impl<S: PlanningSolution, M> Default for CheapestInsertionForager<S, M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: PlanningSolution, M> CheapestInsertionForager<S, M> {
    /// Creates a new Cheapest Insertion forager.
    pub fn new() -> Self {
        Self {
            last_step_score: None,
            _phantom: PhantomData,
        }
    }

    /// Sets the last step score for early-pick comparison.
    pub fn set_last_step_score(&mut self, score: S::Score) {
        self.last_step_score = Some(score);
    }
}

impl<S, M> ConstructionForager<S, M> for CheapestInsertionForager<S, M>
where
    S: PlanningSolution + solverforge_scoring::ShadowVariableSupport,
    M: Move<S>,
{
    fn pick_move_index<C>(
        &self,
        placement: &Placement<S, M>,
        score_director: &mut ScoreDirector<S, C>,
    ) -> Option<usize>
    where
        C: ConstraintSet<S, S::Score>,
        S::Score: Score,
    {
        // Use current score as baseline if no last step score set
        let baseline = self
            .last_step_score
            .unwrap_or_else(|| score_director.calculate_score());

        let mut best_idx: Option<usize> = None;
        let mut best_score: Option<S::Score> = None;

        for (idx, m) in placement.moves.iter().enumerate() {
            if !m.is_doable(score_director) {
                continue;
            }

            // Evaluate move
            score_director.save_score_snapshot();
            m.do_move(score_director);
            let score = score_director.calculate_score();
            score_director.undo_changes();

            // Early-pick: accept first non-worsening move
            if score >= baseline {
                return Some(idx);
            }

            // Track best as fallback
            let is_better = match &best_score {
                None => true,
                Some(best) => score > *best,
            };
            if is_better {
                best_idx = Some(idx);
                best_score = Some(score);
            }
        }

        // No non-worsening move found, return best
        best_idx
    }
}

/// Regret Insertion forager - picks the move with maximum regret.
///
/// Regret is defined as (second_best_score - best_score) for each entity's possible
/// assignments. High regret means the entity has much more to lose if not assigned
/// its best value now, so it should be prioritized.
///
/// This forager groups moves by entity, finds the best and second-best score for
/// each entity, and selects the move that belongs to the entity with maximum regret.
pub struct RegretInsertionForager<S, M> {
    _phantom: PhantomData<fn() -> (S, M)>,
}

impl<S, M> Clone for RegretInsertionForager<S, M> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S, M> Copy for RegretInsertionForager<S, M> {}

impl<S, M> Debug for RegretInsertionForager<S, M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RegretInsertionForager").finish()
    }
}

impl<S, M> Default for RegretInsertionForager<S, M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S, M> RegretInsertionForager<S, M> {
    /// Creates a new Regret Insertion forager.
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<S, M> ConstructionForager<S, M> for RegretInsertionForager<S, M>
where
    S: PlanningSolution + solverforge_scoring::ShadowVariableSupport,
    M: Move<S>,
{
    fn pick_move_index<C>(
        &self,
        placement: &Placement<S, M>,
        score_director: &mut ScoreDirector<S, C>,
    ) -> Option<usize>
    where
        C: ConstraintSet<S, S::Score>,
        S::Score: Score,
    {
        use std::collections::HashMap;

        // Evaluate all moves and group by entity
        let mut entity_scores: HashMap<usize, Vec<(usize, S::Score)>> = HashMap::new();

        for (idx, m) in placement.moves.iter().enumerate() {
            if !m.is_doable(score_director) {
                continue;
            }

            // Evaluate move
            score_director.save_score_snapshot();
            m.do_move(score_director);
            let score = score_director.calculate_score();
            score_director.undo_changes();

            // Group by entity (use first entity index)
            let entity_idx = m.entity_indices().first().copied().unwrap_or(0);
            entity_scores
                .entry(entity_idx)
                .or_default()
                .push((idx, score));
        }

        // Find move with maximum regret
        let mut max_regret_move: Option<usize> = None;
        let mut max_regret_value: Option<i64> = None;

        for (_entity, mut scores) in entity_scores {
            if scores.is_empty() {
                continue;
            }

            // Sort by score descending (best first)
            scores.sort_by(|a, b| b.1.cmp(&a.1));

            let best_idx = scores[0].0;
            let best_score = &scores[0].1;

            // Calculate regret as sum of level differences (second_best - best)
            // More negative = more regret (more to lose if not assigned best)
            let regret = if scores.len() > 1 {
                let second_best = &scores[1].1;
                let best_levels = best_score.to_level_numbers();
                let second_levels = second_best.to_level_numbers();
                second_levels
                    .iter()
                    .zip(best_levels.iter())
                    .map(|(s, b)| s - b)
                    .sum::<i64>()
            } else {
                // Only one option = infinite regret (must be assigned now)
                i64::MIN
            };

            // We want to maximize the magnitude of regret (most negative)
            let is_more_regret = match max_regret_value {
                None => true,
                Some(r) => regret < r, // More negative = more regret
            };

            if is_more_regret {
                max_regret_move = Some(best_idx);
                max_regret_value = Some(regret);
            }
        }

        max_regret_move
    }
}
