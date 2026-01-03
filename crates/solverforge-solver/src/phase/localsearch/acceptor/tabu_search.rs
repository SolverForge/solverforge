//! Tabu search acceptor.

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;

use super::Acceptor;

/// Tabu search acceptor - maintains a tabu list of recently visited solutions.
///
/// Tabu search prevents revisiting recently explored solutions by maintaining
/// a tabu list. This helps escape local optima and prevents cycling.
///
/// This implementation tracks recent scores to identify solutions that should
/// be forbidden (tabu). A more sophisticated implementation would track the
/// actual moves or entity changes.
///
/// # Example
///
/// ```
/// use solverforge_solver::phase::localsearch::TabuSearchAcceptor;
/// use solverforge_core::score::SimpleScore;
/// use solverforge_core::domain::PlanningSolution;
///
/// #[derive(Clone)]
/// struct MySolution;
/// impl PlanningSolution for MySolution {
///     type Score = SimpleScore;
///     fn score(&self) -> Option<Self::Score> { None }
///     fn set_score(&mut self, _: Option<Self::Score>) {}
/// }
///
/// let acceptor = TabuSearchAcceptor::<MySolution>::new(7);
/// ```
pub struct TabuSearchAcceptor<S: PlanningSolution> {
    /// Maximum size of the tabu list.
    tabu_size: usize,
    /// List of tabu (forbidden) scores.
    tabu_list: Vec<S::Score>,
    /// Whether to accept improving moves even if tabu.
    aspiration_enabled: bool,
    /// Best score seen so far (for aspiration criterion).
    best_score: Option<S::Score>,
}

impl<S: PlanningSolution> Debug for TabuSearchAcceptor<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TabuSearchAcceptor")
            .field("tabu_size", &self.tabu_size)
            .field("tabu_list_len", &self.tabu_list.len())
            .field("aspiration_enabled", &self.aspiration_enabled)
            .finish()
    }
}

impl<S: PlanningSolution> Clone for TabuSearchAcceptor<S> {
    fn clone(&self) -> Self {
        Self {
            tabu_size: self.tabu_size,
            tabu_list: self.tabu_list.clone(),
            aspiration_enabled: self.aspiration_enabled,
            best_score: self.best_score.clone(),
        }
    }
}

impl<S: PlanningSolution> TabuSearchAcceptor<S> {
    /// Creates a new tabu search acceptor.
    ///
    /// # Arguments
    /// * `tabu_size` - Maximum number of solutions to remember as tabu
    pub fn new(tabu_size: usize) -> Self {
        Self {
            tabu_size,
            tabu_list: Vec::with_capacity(tabu_size),
            aspiration_enabled: true,
            best_score: None,
        }
    }

    /// Creates a tabu search acceptor without aspiration.
    ///
    /// Without aspiration, tabu moves are never accepted, even if they
    /// would lead to a new best solution.
    pub fn without_aspiration(tabu_size: usize) -> Self {
        Self {
            tabu_size,
            tabu_list: Vec::with_capacity(tabu_size),
            aspiration_enabled: false,
            best_score: None,
        }
    }

    /// Returns true if the given score is in the tabu list.
    fn is_tabu(&self, score: &S::Score) -> bool {
        self.tabu_list.iter().any(|s| s == score)
    }

    /// Adds a score to the tabu list, removing the oldest if at capacity.
    fn add_to_tabu(&mut self, score: S::Score) {
        if self.tabu_list.len() >= self.tabu_size {
            self.tabu_list.remove(0);
        }
        self.tabu_list.push(score);
    }
}

impl<S: PlanningSolution> Default for TabuSearchAcceptor<S> {
    fn default() -> Self {
        Self::new(7) // Default tabu tenure of 7
    }
}

impl<S: PlanningSolution> Acceptor<S> for TabuSearchAcceptor<S> {
    fn is_accepted(&self, last_step_score: &S::Score, move_score: &S::Score) -> bool {
        // Check aspiration criterion: accept if this would be a new best score
        if self.aspiration_enabled {
            if let Some(best) = &self.best_score {
                if move_score > best {
                    return true; // Aspiration: accept new best even if tabu
                }
            }
        }

        // Reject if the move leads to a tabu solution
        if self.is_tabu(move_score) {
            return false;
        }

        // Accept improving moves
        if move_score > last_step_score {
            return true;
        }

        // Accept equal moves (allows exploration on plateaus)
        if move_score >= last_step_score {
            return true;
        }

        // Reject worsening moves that aren't tabu-breaking
        false
    }

    fn phase_started(&mut self, initial_score: &S::Score) {
        self.tabu_list.clear();
        self.best_score = Some(initial_score.clone());
    }

    fn phase_ended(&mut self) {
        self.tabu_list.clear();
    }

    fn step_ended(&mut self, step_score: &S::Score) {
        // Add the step score to the tabu list
        self.add_to_tabu(step_score.clone());

        // Update best score
        if let Some(best) = &self.best_score {
            if step_score > best {
                self.best_score = Some(step_score.clone());
            }
        } else {
            self.best_score = Some(step_score.clone());
        }
    }
}
