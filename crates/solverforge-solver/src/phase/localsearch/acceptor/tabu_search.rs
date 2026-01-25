//! Tabu search acceptor.

use std::collections::{HashMap, VecDeque};
use std::fmt::Debug;

use rand::Rng;
use solverforge_core::domain::PlanningSolution;

use super::Acceptor;

/// Tabu search acceptor - maintains a tabu list of recently modified entities.
///
/// Uses identifier-based tabu tracking: entities that were recently modified
/// are forbidden (tabu) for a configurable number of steps. This prevents
/// cycling and helps escape local optima.
///
/// # Features
/// - **Aspiration criterion**: Accepts tabu moves if they produce a new best score
/// - **Fading tabu**: After the hard tabu period, probabilistic acceptance gradually increases
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
    /// Hard tabu tenure: number of steps an entity stays fully tabu.
    tabu_size: usize,
    /// Fading tenure: additional steps with probabilistic acceptance.
    fading_size: usize,
    /// Maps entity_idx -> step when it was added to tabu.
    tabu_map: HashMap<usize, usize>,
    /// Queue for FIFO eviction (stores entity indices).
    tabu_queue: VecDeque<usize>,
    /// Current step number.
    current_step: usize,
    /// Entity indices in the current move (set before is_accepted).
    current_move_entities: Vec<usize>,
    /// Whether aspiration criterion is enabled.
    aspiration_enabled: bool,
    /// Best score seen so far (for aspiration).
    best_score: Option<S::Score>,
}

impl<S: PlanningSolution> Debug for TabuSearchAcceptor<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TabuSearchAcceptor")
            .field("tabu_size", &self.tabu_size)
            .field("fading_size", &self.fading_size)
            .field("tabu_map_len", &self.tabu_map.len())
            .field("current_step", &self.current_step)
            .field("aspiration_enabled", &self.aspiration_enabled)
            .finish()
    }
}

impl<S: PlanningSolution> Clone for TabuSearchAcceptor<S> {
    fn clone(&self) -> Self {
        Self {
            tabu_size: self.tabu_size,
            fading_size: self.fading_size,
            tabu_map: self.tabu_map.clone(),
            tabu_queue: self.tabu_queue.clone(),
            current_step: self.current_step,
            current_move_entities: self.current_move_entities.clone(),
            aspiration_enabled: self.aspiration_enabled,
            best_score: self.best_score,
        }
    }
}

impl<S: PlanningSolution> TabuSearchAcceptor<S> {
    /// Creates a new tabu search acceptor with aspiration enabled.
    ///
    /// # Arguments
    /// * `tabu_size` - Number of steps entities stay fully tabu
    pub fn new(tabu_size: usize) -> Self {
        Self {
            tabu_size,
            fading_size: tabu_size, // Default fading same as tabu
            tabu_map: HashMap::new(),
            tabu_queue: VecDeque::new(),
            current_step: 0,
            current_move_entities: Vec::new(),
            aspiration_enabled: true,
            best_score: None,
        }
    }

    /// Creates a new tabu search acceptor with configurable fading.
    ///
    /// # Arguments
    /// * `tabu_size` - Number of steps entities stay fully tabu
    /// * `fading_size` - Additional steps with probabilistic acceptance
    pub fn with_fading(tabu_size: usize, fading_size: usize) -> Self {
        Self {
            tabu_size,
            fading_size,
            tabu_map: HashMap::new(),
            tabu_queue: VecDeque::new(),
            current_step: 0,
            current_move_entities: Vec::new(),
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
            fading_size: tabu_size,
            tabu_map: HashMap::new(),
            tabu_queue: VecDeque::new(),
            current_step: 0,
            current_move_entities: Vec::new(),
            aspiration_enabled: false,
            best_score: None,
        }
    }

    /// Records entity indices that will be modified by the current move.
    ///
    /// Must be called before `is_accepted()` for the acceptor to check tabu status.
    pub fn record_move_entities(&mut self, entity_indices: &[usize]) {
        self.current_move_entities.clear();
        self.current_move_entities.extend_from_slice(entity_indices);
    }

    /// Returns true if the entity is in the hard tabu period.
    pub fn is_entity_tabu(&self, entity_idx: usize) -> bool {
        if let Some(&step_added) = self.tabu_map.get(&entity_idx) {
            let age = self.current_step.saturating_sub(step_added);
            age <= self.tabu_size
        } else {
            false
        }
    }

    /// Returns true if the entity is in the fading tabu period.
    pub fn is_entity_fading(&self, entity_idx: usize) -> bool {
        if let Some(&step_added) = self.tabu_map.get(&entity_idx) {
            let age = self.current_step.saturating_sub(step_added);
            age > self.tabu_size && age <= self.tabu_size + self.fading_size
        } else {
            false
        }
    }
}

impl<S: PlanningSolution> Default for TabuSearchAcceptor<S> {
    fn default() -> Self {
        Self::new(7) // Default tabu tenure of 7
    }
}

impl<S: PlanningSolution> Acceptor<S> for TabuSearchAcceptor<S> {
    fn is_accepted(&self, last_step_score: &S::Score, move_score: &S::Score) -> bool {
        let mut rng = rand::rng();

        // Check each entity in the current move
        for &entity_idx in &self.current_move_entities {
            if let Some(&step_added) = self.tabu_map.get(&entity_idx) {
                let age = self.current_step.saturating_sub(step_added);

                // Hard tabu period
                if age <= self.tabu_size {
                    // Aspiration: accept if new best
                    if self.aspiration_enabled {
                        if let Some(ref best) = self.best_score {
                            if move_score > best {
                                return true;
                            }
                        }
                    }
                    return false; // Reject tabu move
                }

                // Fading period: probabilistic acceptance
                if age <= self.tabu_size + self.fading_size {
                    let fading_age = age - self.tabu_size;
                    // Accept chance decreases as fading_age increases
                    // Formula: (fading_size - fading_age) / (fading_size + 1)
                    let accept_chance =
                        (self.fading_size - fading_age) as f64 / (self.fading_size + 1) as f64;
                    if rng.random::<f64>() >= accept_chance {
                        return false; // Probabilistic rejection
                    }
                }
            }
        }

        // Not tabu, apply normal acceptance
        move_score >= last_step_score
    }

    fn phase_started(&mut self, initial_score: &S::Score) {
        self.tabu_map.clear();
        self.tabu_queue.clear();
        self.current_step = 0;
        self.current_move_entities.clear();
        self.best_score = Some(*initial_score);
    }

    fn phase_ended(&mut self) {
        self.tabu_map.clear();
        self.tabu_queue.clear();
    }

    fn step_started(&mut self) {
        self.current_move_entities.clear();
    }

    fn step_ended(&mut self, step_score: &S::Score) {
        // Add current move entities to tabu
        for &entity_idx in &self.current_move_entities {
            // Remove from queue if already present (update position)
            self.tabu_queue.retain(|&e| e != entity_idx);
            self.tabu_map.remove(&entity_idx);

            // Add to tabu at current step
            self.tabu_map.insert(entity_idx, self.current_step);
            self.tabu_queue.push_back(entity_idx);
        }

        // Remove oldest entries if over capacity (tabu_size + fading_size)
        let max_size = self.tabu_size + self.fading_size;
        while self.tabu_queue.len() > max_size {
            if let Some(oldest) = self.tabu_queue.pop_front() {
                self.tabu_map.remove(&oldest);
            }
        }

        self.current_step += 1;
        self.current_move_entities.clear();

        // Update best score
        if let Some(ref best) = self.best_score {
            if step_score > best {
                self.best_score = Some(*step_score);
            }
        } else {
            self.best_score = Some(*step_score);
        }
    }
}
