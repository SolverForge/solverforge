# SolverForge Algorithm Specifications

This document contains exact algorithm specifications, bugs found during audit, and fix implementations.

## Acceptance Formulas

### Hill Climbing
```
accept = moveScore >= lastStepScore
```
Simple greedy acceptance - only accepts improving or equal moves.

### Simulated Annealing
Per score level:
```
if moveScore >= lastStepScore:
    accept = true
else:
    delta = lastStepScore[level] - moveScore[level]  # positive = worse
    accept = random() < exp(-delta / temperature)

    # Time-gradient based cooling
    temperature = startingTemperature * (1 - timeGradient)
```
Where `timeGradient` is the progress ratio from 0.0 (start) to 1.0 (end).

### Late Acceptance
```
history = circular_buffer[lateAcceptanceSize]
historyIndex = stepCount % lateAcceptanceSize

accept = moveScore >= history[historyIndex] OR moveScore > lastStepScore

# Update after acceptance
history[historyIndex] = stepScore
```

### Great Deluge
```
# Water level decays toward best score
waterLevel = initialWaterLevel - (initialWaterLevel - bestScore) * timeGradient

accept = moveScore >= waterLevel OR moveScore > lastStepScore
```

### Tabu Search (Identifier-Based)
```
# Rejection based on identifier, not score
for each entity_index in move.entity_indices():
    if entity_index in tabu_map:
        age = current_step - tabu_map[entity_index]
        if age <= tabuSize:
            # Check aspiration
            if aspirationEnabled AND moveScore > bestScore:
                accept = true
                break
            # Check fading
            elif fadingEnabled AND age > tabuSize:
                fadingAge = age - tabuSize
                acceptChance = (fadingSize - fadingAge) / (fadingSize + 1)
                if random() < acceptChance:
                    accept = true
                    break
            else:
                reject = true

# If not rejected, apply normal acceptance
accept = moveScore >= lastStepScore
```

## Construction Heuristic Logic

### First Fit
Pick the first doable move without evaluation.
```
for move in moves:
    if move.is_doable():
        return move
```
Time: O(1) per placement (amortized)

### Best Fit
Evaluate all moves, pick the one with the best score.
```
best = None
for move in moves:
    if move.is_doable():
        score = evaluate(move)
        if best is None or score > best.score:
            best = (move, score)
return best.move
```
Time: O(n) evaluations per placement

### First Feasible
Pick the first move that produces a feasible (hard >= 0) score.
```
fallback = None
for move in moves:
    if move.is_doable():
        score = evaluate(move)
        if score.is_feasible():
            return move
        if fallback is None or score > fallback.score:
            fallback = (move, score)
return fallback.move
```
Time: O(n) evaluations worst case, often O(1) if feasible found early

### Weakest Fit
Pick the doable move with minimum `Move::strength()`.
```
best = None
for move in moves:
    if move.is_doable():
        strength = move.strength()
        if best is None or strength < best.strength:
            best = (move, strength)
return best.move
```
Time: O(n) strength calculations per placement

### Strongest Fit
Pick the doable move with maximum `Move::strength()`.
```
best = None
for move in moves:
    if move.is_doable():
        strength = move.strength()
        if best is None or strength > best.strength:
            best = (move, strength)
return best.move
```
Time: O(n) strength calculations per placement

### Cheapest Insertion (Early-Pick)
Early-pick when `moveScore >= lastStepScore`, fallback to best.
```
best = None
lastStepScore = initial_score or previous_step_score

for move in moves:
    if move.is_doable():
        score = evaluate(move)

        # Early-pick: accept first non-worsening move
        if score >= lastStepScore:
            return move

        # Track best as fallback
        if best is None or score > best.score:
            best = (move, score)

return best.move
```
Time: O(1) best case (early pick), O(n) worst case (all worse)

### Regret Insertion
Pick move with maximum regret = (second_best_score - best_score).
```
# For each entity, find best and second-best assignments
regrets = []
for entity in unassigned_entities:
    entity_moves = moves_for_entity(entity)
    scores = [(move, evaluate(move)) for move in entity_moves if move.is_doable()]
    scores.sort(by=score, descending=True)

    best_score = scores[0].score
    second_best_score = scores[1].score if len(scores) > 1 else worst_possible
    regret = second_best_score - best_score  # More negative = more regret
    regrets.append((entity, scores[0].move, -regret))  # Negate so max finds most regret

# Pick entity with maximum regret (most to lose if not assigned best)
best_entity = max(regrets, key=lambda x: x.regret)
return best_entity.move
```
Time: O(entities * values) evaluations per step

## O(1) Invariants

### Move Tabu
- `tabu_list` is a `Vec<u64>` with max size `tabu_size`
- O(n) contains check where n = tabu_size (typically 7-20)
- Could optimize to `HashSet` for O(1) lookup

### Entity Tabu
- Same structure as Move Tabu
- O(n) contains check, could use HashSet

### Value Tabu
- Same structure as Move Tabu
- O(n) contains check, could use HashSet

### Identifier-Based Tabu (Proposed)
- `tabu_map: HashMap<usize, usize>` maps entity_idx -> step_added
- O(1) lookup per entity
- `tabu_queue: VecDeque<usize>` for FIFO removal
- O(1) push/pop

### Simulated Annealing
- O(1) temperature update per step
- O(levels) probability calculation per move

### Late Acceptance
- O(1) circular buffer lookup and update
- Fixed memory footprint

## Bugs Found

### Severity 1: Tabu Acceptors Don't Reject Tabu Moves

**Location:**
- `crates/solverforge-solver/src/phase/localsearch/acceptor/move_tabu.rs:101-117`
- `crates/solverforge-solver/src/phase/localsearch/acceptor/entity_tabu.rs:68-76`
- `crates/solverforge-solver/src/phase/localsearch/acceptor/value_tabu.rs:68-76`

**Bug:** The `is_accepted()` method never checks if the move is tabu. It only compares scores:
```rust
// Current BROKEN implementation
fn is_accepted(&self, last_step_score: &S::Score, move_score: &S::Score) -> bool {
    // Aspiration check (MoveTabu only)
    if self.aspiration_enabled {
        if let Some(best) = self.best_score {
            if move_value > best {
                return true;
            }
        }
    }
    // BUG: Never checks is_move_tabu() / is_entity_tabu() / is_value_tabu()
    if move_score > last_step_score {
        return true;
    }
    if move_score >= last_step_score {
        return true;
    }
    false
}
```

**Impact:** Tabu search degenerates to simple hill climbing - no diversification.

**Root Cause:** The `is_accepted()` trait method doesn't receive move information, only scores. The tabu state is tracked but never queried.

### Severity 2: SimulatedAnnealing Uses Wrong Algorithm

**Location:** `crates/solverforge-solver/src/phase/localsearch/acceptor/simulated_annealing.rs:59-68`

**Bug:** Uses threshold comparison instead of Boltzmann probability:
```rust
// Current BROKEN implementation
fn is_accepted(&self, last_step_score: &S::Score, move_score: &S::Score) -> bool {
    if move_score > last_step_score {
        return true;
    }
    if self.current_temperature <= 0.0 {
        return false;
    }
    let acceptance_probability = self.current_temperature.min(1.0);
    acceptance_probability > 0.5  // BUG: Deterministic threshold, not probabilistic!
}
```

**Should be:**
```rust
fn is_accepted(&self, last_step_score: &S::Score, move_score: &S::Score) -> bool {
    if move_score >= last_step_score {
        return true;  // Always accept improving or equal moves
    }
    if self.current_temperature <= 0.0 {
        return false;
    }

    // Boltzmann acceptance probability per score level
    let last_levels = last_step_score.to_level_numbers();
    let move_levels = move_score.to_level_numbers();

    for (last_val, move_val) in last_levels.iter().zip(move_levels.iter()) {
        if move_val < last_val {
            let delta = (last_val - move_val) as f64;
            let p = (-delta / self.current_temperature).exp();
            if rand::random::<f64>() >= p {
                return false;  // Rejected at this level
            }
        }
    }
    true  // Accepted at all levels
}
```

**Additional Bug:** Uses multiplicative decay instead of time-gradient cooling:
```rust
// Current: multiplicative decay
fn step_ended(&mut self, _step_score: &S::Score) {
    self.current_temperature *= self.decay_rate;  // Wrong!
}

// Should be: time-gradient cooling
// temperature = starting_temperature * (1 - time_gradient)
```

**Impact:** Algorithm doesn't properly explore solution space. Worsening moves are either always or never accepted based on a deterministic threshold.

### Severity 2: TabuSearch Uses Score-Based Instead of Identifier-Based

**Location:** `crates/solverforge-solver/src/phase/localsearch/acceptor/tabu_search.rs`

**Bug:** Tabu list stores scores instead of move/entity identifiers:
```rust
// Current BROKEN implementation
tabu_list: Vec<S::Score>,  // Wrong! Should be entity/move identifiers

fn is_tabu(&self, score: &S::Score) -> bool {
    self.tabu_list.iter().any(|s| s == score)  // Compares scores, not identifiers
}
```

**Problem:** Two completely different moves that happen to produce the same score will be considered tabu. Meanwhile, revisiting the same solution with a different score (due to incremental changes) won't be detected.

**Should use:**
```rust
tabu_map: HashMap<usize, usize>,  // entity_idx -> step_added
tabu_queue: VecDeque<usize>,      // FIFO for removal
```

**Missing:** Fading tabu implementation:
```
acceptChance = (fadingSize - fadingAge) / (fadingSize + 1)
```

### Severity 3: Missing CheapestInsertion and RegretInsertion Foragers

**Location:** `crates/solverforge-solver/src/phase/construction/forager_impl.rs:50-52`

**Bug:** CheapestInsertion maps to BestFit:
```rust
ConstructionHeuristicType::CheapestInsertion => {
    ConstructionForagerImpl::BestFit(BestFitForager::new())  // Wrong!
}
```

**CheapestInsertion should:**
1. Early-pick when `moveScore >= lastStepScore` (greedy improvement)
2. Fall back to best if none improve

**RegretInsertion is completely missing** from both forager.rs and forager_impl.rs.

## Fix Code

### Fix 1: Tabu Acceptor Trait Extension

The current `Acceptor` trait needs to pass move information for tabu checking. Options:

**Option A:** Add `is_accepted_with_move` method (breaking change)
```rust
pub trait Acceptor<S: PlanningSolution>: Send + Debug {
    fn is_accepted(&self, last_step_score: &S::Score, move_score: &S::Score) -> bool;

    /// Extended acceptance check with move context for tabu acceptors.
    /// Default delegates to is_accepted().
    fn is_accepted_with_context(
        &self,
        last_step_score: &S::Score,
        move_score: &S::Score,
        entity_indices: &[usize],
        move_hash: u64,
    ) -> bool {
        self.is_accepted(last_step_score, move_score)
    }
}
```

**Option B:** Record move before calling is_accepted (current pattern)
The acceptors already have `record_move()`, `record_entity_move()`, etc. The phase must call these BEFORE `is_accepted()`.

### Fix 2: MoveTabuAcceptor.is_accepted()

```rust
impl<S: PlanningSolution> Acceptor<S> for MoveTabuAcceptor<S> {
    fn is_accepted(&self, last_step_score: &S::Score, move_score: &S::Score) -> bool {
        // Check aspiration: accept new best even if tabu
        if self.aspiration_enabled {
            if let Some(best) = self.best_score {
                let move_value = Self::score_to_i64(move_score);
                if move_value > best {
                    return true;
                }
            }
        }

        // Check if current move is tabu
        if let Some(move_hash) = self.current_step_move {
            if self.is_move_tabu(move_hash) {
                return false;  // Reject tabu move
            }
        }

        // Normal acceptance: accept improving or equal
        move_score >= last_step_score
    }
}
```

### Fix 3: EntityTabuAcceptor.is_accepted()

```rust
impl<S: PlanningSolution> Acceptor<S> for EntityTabuAcceptor<S> {
    fn is_accepted(&self, last_step_score: &S::Score, move_score: &S::Score) -> bool {
        // Check if any entity in current move is tabu
        for entity_id in &self.current_step_entities {
            if self.is_entity_tabu(*entity_id) {
                return false;  // Reject if any entity is tabu
            }
        }

        // Normal acceptance
        move_score >= last_step_score
    }
}
```

### Fix 4: ValueTabuAcceptor.is_accepted()

```rust
impl<S: PlanningSolution> Acceptor<S> for ValueTabuAcceptor<S> {
    fn is_accepted(&self, last_step_score: &S::Score, move_score: &S::Score) -> bool {
        // Check if any value in current move is tabu
        for value_hash in &self.current_step_values {
            if self.is_value_tabu(*value_hash) {
                return false;  // Reject if any value is tabu
            }
        }

        // Normal acceptance
        move_score >= last_step_score
    }
}
```

### Fix 5: SimulatedAnnealingAcceptor

```rust
use rand::Rng;

pub struct SimulatedAnnealingAcceptor<S> {
    starting_temperature: f64,
    current_temperature: f64,
    time_gradient: f64,  // Progress from 0.0 to 1.0
    _phantom: PhantomData<fn() -> S>,
}

impl<S: PlanningSolution> Acceptor<S> for SimulatedAnnealingAcceptor<S> {
    fn is_accepted(&self, last_step_score: &S::Score, move_score: &S::Score) -> bool {
        // Always accept improving or equal moves
        if move_score >= last_step_score {
            return true;
        }

        if self.current_temperature <= 0.0 {
            return false;
        }

        // Boltzmann acceptance for worsening moves
        let last_levels = last_step_score.to_level_numbers();
        let move_levels = move_score.to_level_numbers();

        let mut rng = rand::thread_rng();

        for (last_val, move_val) in last_levels.iter().zip(move_levels.iter()) {
            if move_val < last_val {
                let delta = (*last_val - *move_val) as f64;
                let p = (-delta / self.current_temperature).exp();
                if rng.gen::<f64>() >= p {
                    return false;
                }
            }
        }
        true
    }

    fn step_ended(&mut self, _step_score: &S::Score) {
        // Time-gradient based cooling
        // Note: time_gradient must be updated by the phase
        self.current_temperature = self.starting_temperature * (1.0 - self.time_gradient);
    }
}
```

### Fix 6: TabuSearchAcceptor (Identifier-Based)

```rust
use std::collections::{HashMap, VecDeque};

pub struct TabuSearchAcceptor<S: PlanningSolution> {
    tabu_size: usize,
    fading_size: usize,  // Additional steps for fading
    tabu_map: HashMap<usize, usize>,  // entity_idx -> step_added
    tabu_queue: VecDeque<usize>,      // FIFO for removal
    current_step: usize,
    current_move_entities: Vec<usize>,
    aspiration_enabled: bool,
    best_score: Option<S::Score>,
}

impl<S: PlanningSolution> Acceptor<S> for TabuSearchAcceptor<S> {
    fn is_accepted(&self, last_step_score: &S::Score, move_score: &S::Score) -> bool {
        let mut rng = rand::thread_rng();

        for &entity_idx in &self.current_move_entities {
            if let Some(&step_added) = self.tabu_map.get(&entity_idx) {
                let age = self.current_step - step_added;

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
                    return false;  // Reject tabu move
                }

                // Fading period
                if age <= self.tabu_size + self.fading_size {
                    let fading_age = age - self.tabu_size;
                    let accept_chance = (self.fading_size - fading_age) as f64
                                      / (self.fading_size + 1) as f64;
                    if rng.gen::<f64>() >= accept_chance {
                        return false;  // Probabilistic rejection
                    }
                }
            }
        }

        // Normal acceptance
        move_score >= last_step_score
    }

    fn step_started(&mut self) {
        self.current_move_entities.clear();
    }

    fn step_ended(&mut self, step_score: &S::Score) {
        // Add current move entities to tabu
        for &entity_idx in &self.current_move_entities {
            // Remove from queue if already present
            self.tabu_queue.retain(|&e| e != entity_idx);
            self.tabu_map.remove(&entity_idx);

            // Add to tabu
            self.tabu_map.insert(entity_idx, self.current_step);
            self.tabu_queue.push_back(entity_idx);
        }

        // Remove oldest if over capacity (tabu_size + fading_size)
        let max_size = self.tabu_size + self.fading_size;
        while self.tabu_queue.len() > max_size {
            if let Some(oldest) = self.tabu_queue.pop_front() {
                self.tabu_map.remove(&oldest);
            }
        }

        self.current_step += 1;

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
```

### Fix 7: CheapestInsertionForager

```rust
/// Cheapest Insertion forager - early-pick on non-worsening score.
pub struct CheapestInsertionForager<S, M> {
    last_step_score: Option<S::Score>,
    _phantom: PhantomData<fn() -> (S, M)>,
}

impl<S, M> ConstructionForager<S, M> for CheapestInsertionForager<S, M>
where
    S: PlanningSolution + ShadowVariableSupport,
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
        let last_score = self.last_step_score.unwrap_or_else(|| score_director.calculate_score());
        let mut best_idx: Option<usize> = None;
        let mut best_score: Option<S::Score> = None;

        for (idx, m) in placement.moves.iter().enumerate() {
            if !m.is_doable(score_director) {
                continue;
            }

            score_director.save_score_snapshot();
            m.do_move(score_director);
            let score = score_director.calculate_score();
            score_director.undo_changes();

            // Early-pick: accept first non-worsening move
            if score >= last_score {
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

        best_idx
    }
}
```

### Fix 8: RegretInsertionForager

```rust
/// Regret Insertion forager - picks the move with maximum regret.
///
/// Regret = second_best_score - best_score for each entity.
/// High regret means more to lose if not assigned the best value.
pub struct RegretInsertionForager<S, M> {
    _phantom: PhantomData<fn() -> (S, M)>,
}

impl<S, M> ConstructionForager<S, M> for RegretInsertionForager<S, M>
where
    S: PlanningSolution + ShadowVariableSupport,
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
        // Evaluate all moves and track best and second-best per entity
        let mut entity_scores: std::collections::HashMap<usize, Vec<(usize, S::Score)>> =
            std::collections::HashMap::new();

        for (idx, m) in placement.moves.iter().enumerate() {
            if !m.is_doable(score_director) {
                continue;
            }

            score_director.save_score_snapshot();
            m.do_move(score_director);
            let score = score_director.calculate_score();
            score_director.undo_changes();

            // Group by entity
            let entity_idx = m.entity_indices().first().copied().unwrap_or(0);
            entity_scores.entry(entity_idx).or_default().push((idx, score));
        }

        // Find move with maximum regret
        let mut max_regret_move: Option<usize> = None;
        let mut max_regret_value: Option<i64> = None;

        for (_entity, mut scores) in entity_scores {
            if scores.is_empty() {
                continue;
            }

            // Sort by score descending
            scores.sort_by(|a, b| b.1.cmp(&a.1));

            let best_score = &scores[0].1;
            let best_idx = scores[0].0;

            let regret = if scores.len() > 1 {
                // Regret = second_best - best (more negative = more regret)
                let second_best = &scores[1].1;
                let best_levels = best_score.to_level_numbers();
                let second_levels = second_best.to_level_numbers();
                // Sum of level differences (negative when best > second_best)
                second_levels.iter().zip(best_levels.iter())
                    .map(|(s, b)| s - b)
                    .sum::<i64>()
            } else {
                i64::MIN  // Only one option = infinite regret
            };

            // We want to maximize the magnitude of regret (most negative)
            let is_more_regret = match max_regret_value {
                None => true,
                Some(r) => regret < r,  // More negative = more regret
            };

            if is_more_regret {
                max_regret_move = Some(best_idx);
                max_regret_value = Some(regret);
            }
        }

        max_regret_move
    }
}
```

## Verification Tests

### Test 1: MoveTabuAcceptor Rejection
```rust
#[test]
fn test_move_tabu_rejects_tabu_move() {
    let mut acceptor = MoveTabuAcceptor::<TestSolution>::new(3);
    acceptor.phase_started(&SimpleScore::of(0));

    // Record and end step with move hash 42
    acceptor.record_move(42);
    acceptor.step_ended(&SimpleScore::of(10));

    // New step with same move should be rejected
    acceptor.step_started();
    acceptor.record_move(42);
    assert!(!acceptor.is_accepted(&SimpleScore::of(10), &SimpleScore::of(10)));

    // Different move should be accepted
    acceptor.step_started();
    acceptor.record_move(99);
    assert!(acceptor.is_accepted(&SimpleScore::of(10), &SimpleScore::of(10)));
}
```

### Test 2: Aspiration Override
```rust
#[test]
fn test_move_tabu_aspiration_overrides() {
    let mut acceptor = MoveTabuAcceptor::<TestSolution>::new(3);
    acceptor.phase_started(&SimpleScore::of(0));

    // Record and end step
    acceptor.record_move(42);
    acceptor.step_ended(&SimpleScore::of(10));

    // Tabu move with better-than-best score should be accepted
    acceptor.step_started();
    acceptor.record_move(42);
    assert!(acceptor.is_accepted(&SimpleScore::of(10), &SimpleScore::of(20)));
}
```

### Test 3: SimulatedAnnealing Always Accepts Improving
```rust
#[test]
fn test_sa_always_accepts_improving() {
    let acceptor = SimulatedAnnealingAcceptor::<TestSolution>::new(1.0);

    // Improving move always accepted regardless of temperature
    for _ in 0..100 {
        assert!(acceptor.is_accepted(&SimpleScore::of(0), &SimpleScore::of(10)));
    }
}
```

### Test 4: SimulatedAnnealing Probabilistic Worsening
```rust
#[test]
fn test_sa_probabilistic_worsening() {
    let acceptor = SimulatedAnnealingAcceptor::<TestSolution>::new(100.0);

    // With high temperature, worsening moves should sometimes be accepted
    let mut accepted = 0;
    for _ in 0..1000 {
        if acceptor.is_accepted(&SimpleScore::of(100), &SimpleScore::of(90)) {
            accepted += 1;
        }
    }

    // Should accept roughly exp(-10/100) ≈ 90% of the time
    assert!(accepted > 800 && accepted < 950);
}
```

### Test 5: CheapestInsertion Early-Pick
```rust
#[test]
fn test_cheapest_insertion_early_pick() {
    // Setup with moves that produce scores [5, 15, 10]
    // Last step score = 10
    // Should pick move 1 (score=15) immediately, not evaluate move 2

    let forager = CheapestInsertionForager::with_last_score(SimpleScore::of(10));

    // Move 0: score 5 (worse, skip)
    // Move 1: score 15 (better, early-pick!)
    // Move 2: never evaluated

    let picked = forager.pick_move_index(&placement, &mut score_director);
    assert_eq!(picked, Some(1));
}
```

### Test 6: RegretInsertion Maximizes Regret
```rust
#[test]
fn test_regret_insertion_picks_max_regret() {
    // Entity A: best=100, second=90 -> regret = -10
    // Entity B: best=80, second=20 -> regret = -60 (more regret!)
    // Should pick Entity B's best move

    let forager = RegretInsertionForager::new();
    let picked = forager.pick_move_index(&placement, &mut score_director);

    // Picked move should be Entity B's best assignment
    assert_eq!(picked, Some(entity_b_best_move_idx));
}
```
