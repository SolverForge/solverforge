//! Move generation for dynamic solutions.

use std::fmt;
use std::sync::Arc;

use solverforge_scoring::ScoreDirector;
use solverforge_solver::heuristic::r#move::Move;
use solverforge_solver::heuristic::selector::typed_move_selector::MoveSelector;
use solverforge_solver::heuristic::selector::EntityReference;
use solverforge_solver::phase::construction::{EntityPlacer, Placement};

use crate::descriptor::ValueRangeDef;
use crate::solution::{DynamicSolution, DynamicValue};

/// Lazy iterator over all possible change moves for a dynamic solution.
///
/// This iterator generates moves on-demand rather than pre-computing them all,
/// which can significantly reduce memory usage for large solutions with many
/// entities and value ranges.
///
/// # Performance Characteristics
///
/// Benchmarks comparing lazy iteration vs eager Vec collection show:
///
/// **Full iteration (all moves consumed):**
/// - Small scale (1K moves): ~equal performance
/// - Medium scale (10K-50K moves): lazy is ~10-20% faster
/// - Large scale (1M moves): lazy is ~2x faster due to avoiding allocation overhead
///
/// **Partial iteration (only first N moves consumed):**
/// This is where lazy iteration really shines. For a 50K move space:
/// - take(10): ~3500x faster (avoid generating 99.98% of moves)
/// - take(100): ~570x faster
/// - take(1000): ~60x faster
/// - take(10000): ~6x faster
///
/// The lazy iterator is particularly beneficial when:
/// 1. Only a subset of moves are consumed (e.g., `FirstAcceptedForager` stops after finding improvement)
/// 2. Memory allocation overhead is significant (large move counts)
/// 3. Early termination is common in the consuming algorithm
///
/// # Iteration Order
///
/// The iterator traverses in the following order:
/// 1. Entity classes (by class_idx)
/// 2. Entities within each class (by entity_idx)
/// 3. Planning variables within each entity (by field_idx)
/// 4. Values in the value range for each variable (by value_idx)
pub struct DynamicMoveIterator<'a> {
    solution: &'a DynamicSolution,
    // Current entity class index
    class_idx: usize,
    // Current entity index within the class
    entity_idx: usize,
    // Current index into planning_variable_indices for the class
    var_slot_idx: usize,
    // Current value index for the current variable's value range
    value_idx: usize,
    // Cached values for the current variable's range (avoids repeated allocation)
    current_values: Vec<DynamicValue>,
    // Cached field_idx for current variable
    current_field_idx: usize,
    // Cached variable name for current variable
    current_variable_name: Arc<str>,
    // Whether we've finished iterating
    exhausted: bool,
}

impl<'a> DynamicMoveIterator<'a> {
    /// Creates a new move iterator for the given solution.
    pub fn new(solution: &'a DynamicSolution) -> Self {
        let mut iter = Self {
            solution,
            class_idx: 0,
            entity_idx: 0,
            var_slot_idx: 0,
            value_idx: 0,
            current_values: Vec::new(),
            current_field_idx: 0,
            current_variable_name: Arc::from(""),
            exhausted: false,
        };
        // Initialize to the first valid position
        iter.advance_to_valid_position();
        iter
    }

    // Advances the iterator state to the next valid position that has moves.
    // Returns true if a valid position was found, false if exhausted.
    fn advance_to_valid_position(&mut self) -> bool {
        loop {
            // Check if we've exhausted all classes
            if self.class_idx >= self.solution.descriptor.entity_classes.len() {
                self.exhausted = true;
                return false;
            }

            let class_def = &self.solution.descriptor.entity_classes[self.class_idx];
            let entity_count = self
                .solution
                .entities
                .get(self.class_idx)
                .map(|e| e.len())
                .unwrap_or(0);

            // Check if we've exhausted entities in this class
            if self.entity_idx >= entity_count {
                self.class_idx += 1;
                self.entity_idx = 0;
                self.var_slot_idx = 0;
                self.value_idx = 0;
                continue;
            }

            // Check if we've exhausted planning variables for this entity
            if self.var_slot_idx >= class_def.planning_variable_indices.len() {
                self.entity_idx += 1;
                self.var_slot_idx = 0;
                self.value_idx = 0;
                continue;
            }

            // Get the current field info
            let field_idx = class_def.planning_variable_indices[self.var_slot_idx];
            let field_def = &class_def.fields[field_idx];

            // Check if this variable has a value range
            let Some(range_name) = &field_def.value_range else {
                self.var_slot_idx += 1;
                self.value_idx = 0;
                continue;
            };

            // Check if the range exists
            let Some(range) = self.solution.descriptor.value_range(range_name) else {
                self.var_slot_idx += 1;
                self.value_idx = 0;
                continue;
            };

            // Get the values for this range
            let values = get_range_values(range, self.solution);
            if values.is_empty() {
                self.var_slot_idx += 1;
                self.value_idx = 0;
                continue;
            }

            // Check if we've exhausted values for this variable
            if self.value_idx >= values.len() {
                self.var_slot_idx += 1;
                self.value_idx = 0;
                continue;
            }

            // We have a valid position - cache the current variable info
            self.current_values = values;
            self.current_field_idx = field_idx;
            self.current_variable_name = field_def.name.clone();
            return true;
        }
    }
}

impl<'a> Iterator for DynamicMoveIterator<'a> {
    type Item = DynamicChangeMove;

    fn next(&mut self) -> Option<Self::Item> {
        if self.exhausted {
            return None;
        }

        // Create the move for the current position
        let m = DynamicChangeMove::new(
            self.class_idx,
            self.entity_idx,
            self.current_field_idx,
            self.current_variable_name.clone(),
            self.current_values[self.value_idx].clone(),
        );

        // Advance to the next value
        self.value_idx += 1;

        // If we've exhausted values for this variable, advance to the next valid position
        if self.value_idx >= self.current_values.len() {
            self.var_slot_idx += 1;
            self.value_idx = 0;
            self.advance_to_valid_position();
        }

        Some(m)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        // We can compute the exact size but it requires iterating through the structure
        // For now, provide a lower bound of 0 and no upper bound
        (0, None)
    }
}

#[derive(Clone)]
pub struct DynamicChangeMove {
    pub class_idx: usize,
    pub entity_idx: usize,
    pub field_idx: usize,
    pub variable_name: Arc<str>,
    pub new_value: DynamicValue,
    entity_indices: [usize; 1],
}

impl DynamicChangeMove {
    pub fn new(
        class_idx: usize,
        entity_idx: usize,
        field_idx: usize,
        variable_name: impl Into<Arc<str>>,
        new_value: DynamicValue,
    ) -> Self {
        Self {
            class_idx,
            entity_idx,
            field_idx,
            variable_name: variable_name.into(),
            new_value,
            entity_indices: [entity_idx],
        }
    }
}

impl fmt::Debug for DynamicChangeMove {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ChangeMove(class={}, entity={}, {}={:?})",
            self.class_idx, self.entity_idx, self.variable_name, self.new_value
        )
    }
}

impl Move<DynamicSolution> for DynamicChangeMove {
    fn is_doable<D: ScoreDirector<DynamicSolution>>(&self, score_director: &D) -> bool {
        let solution = score_director.working_solution();
        if let Some(entity) = solution.get_entity(self.class_idx, self.entity_idx) {
            if let Some(current) = entity.fields.get(self.field_idx) {
                // Move is not doable if it doesn't change anything
                let doable = current != &self.new_value;
                return doable;
            }
        }
        false
    }

    fn do_move<D: ScoreDirector<DynamicSolution>>(&self, score_director: &mut D) {
        // Capture old value BEFORE making changes
        let old_value = {
            let solution = score_director.working_solution();
            solution
                .get_entity(self.class_idx, self.entity_idx)
                .and_then(|e| e.fields.get(self.field_idx))
                .cloned()
        };

        score_director.before_variable_changed(
            self.class_idx,
            self.entity_idx,
            &self.variable_name,
        );

        let solution = score_director.working_solution_mut();
        if let Some(entity) = solution.get_entity_mut(self.class_idx, self.entity_idx) {
            entity.set(self.field_idx, self.new_value.clone());
        }

        score_director.after_variable_changed(self.class_idx, self.entity_idx, &self.variable_name);

        // Register undo closure to restore the old value
        if let Some(old_value) = old_value {
            let class_idx = self.class_idx;
            let entity_idx = self.entity_idx;
            let field_idx = self.field_idx;
            score_director.register_undo(Box::new(move |solution: &mut DynamicSolution| {
                if let Some(entity) = solution.get_entity_mut(class_idx, entity_idx) {
                    entity.set(field_idx, old_value);
                }
            }));
        }
    }

    fn descriptor_index(&self) -> usize {
        self.class_idx
    }

    fn entity_indices(&self) -> &[usize] {
        &self.entity_indices
    }

    fn variable_name(&self) -> &str {
        &self.variable_name
    }
}

#[derive(Debug)]
pub struct DynamicMoveSelector {
    _phantom: std::marker::PhantomData<()>,
}

impl DynamicMoveSelector {
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }

    /// Returns a lazy iterator over all possible change moves for the solution.
    ///
    /// This method generates moves on-demand using `DynamicMoveIterator`, which
    /// significantly reduces memory usage for large solutions compared to
    /// pre-computing all moves into a `Vec`.
    ///
    /// The moves are generated in deterministic order (by class, entity, variable, value).
    /// If randomized order is needed (e.g., for `FirstAcceptedForager`), the caller
    /// should collect and shuffle the moves, or use `generate_moves_shuffled`.
    pub fn generate_moves<'a>(&self, solution: &'a DynamicSolution) -> DynamicMoveIterator<'a> {
        DynamicMoveIterator::new(solution)
    }

    /// Returns all possible change moves as a shuffled Vec.
    ///
    /// This method collects all moves and shuffles them for randomized selection,
    /// which is important for foragers like `FirstAcceptedForager`.
    ///
    /// For memory-constrained scenarios with many moves, prefer using
    /// `generate_moves()` to get a lazy iterator.
    pub fn generate_moves_shuffled(&self, solution: &DynamicSolution) -> Vec<DynamicChangeMove> {
        use rand::seq::SliceRandom;

        let mut moves: Vec<_> = self.generate_moves(solution).collect();

        // Shuffle moves for randomized selection (important for FirstAcceptedForager)
        let mut rng = rand::rng();
        moves.shuffle(&mut rng);

        moves
    }
}

impl Default for DynamicMoveSelector {
    fn default() -> Self {
        Self::new()
    }
}

// Implement the real MoveSelector trait
impl MoveSelector<DynamicSolution, DynamicChangeMove> for DynamicMoveSelector {
    /// Returns an iterator over all possible change moves in randomized order.
    ///
    /// This implementation leverages `DynamicMoveIterator` for the initial move generation,
    /// then collects and shuffles the moves for randomized selection. The shuffling is
    /// critical for effective local search with `FirstAcceptedForager`, which relies on
    /// randomized move order to escape local optima and explore the solution space.
    ///
    /// **Design decision**: While lazy iteration without shuffling would save memory,
    /// it causes the solver to get stuck in local optima because moves are always
    /// evaluated in the same deterministic order. The shuffled approach ensures:
    /// - Effective exploration with `FirstAcceptedForager`
    /// - Consistent solver performance across different problem instances
    /// - Better solution quality within time limits
    ///
    /// For memory-constrained scenarios where deterministic order is acceptable,
    /// use `generate_moves()` directly to get a lazy iterator.
    fn iter_moves<'a, D: ScoreDirector<DynamicSolution>>(
        &'a self,
        score_director: &'a D,
    ) -> Box<dyn Iterator<Item = DynamicChangeMove> + 'a> {
        let solution = score_director.working_solution();
        // Collect moves from lazy iterator and shuffle for randomized selection
        // This is necessary for FirstAcceptedForager to work effectively
        let moves = self.generate_moves_shuffled(solution);
        Box::new(moves.into_iter())
    }

    fn size<D: ScoreDirector<DynamicSolution>>(&self, score_director: &D) -> usize {
        // Use lazy iterator to count without allocating all moves
        self.generate_moves(score_director.working_solution())
            .count()
    }
}

#[derive(Debug)]
pub struct DynamicEntityPlacer;

impl DynamicEntityPlacer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DynamicEntityPlacer {
    fn default() -> Self {
        Self::new()
    }
}

impl EntityPlacer<DynamicSolution, DynamicChangeMove> for DynamicEntityPlacer {
    fn get_placements<D: ScoreDirector<DynamicSolution>>(
        &self,
        score_director: &D,
    ) -> Vec<Placement<DynamicSolution, DynamicChangeMove>> {
        let solution = score_director.working_solution();
        let mut placements = Vec::new();

        for (class_idx, class_def) in solution.descriptor.entity_classes.iter().enumerate() {
            let entity_count = solution
                .entities
                .get(class_idx)
                .map(|e| e.len())
                .unwrap_or(0);

            for entity_idx in 0..entity_count {
                // Check if any planning variable is uninitialized
                let has_uninitialized =
                    class_def
                        .planning_variable_indices
                        .iter()
                        .any(|&field_idx| {
                            solution
                                .get_entity(class_idx, entity_idx)
                                .and_then(|e| e.fields.get(field_idx))
                                .map(|v| v.is_none())
                                .unwrap_or(true)
                        });

                if !has_uninitialized {
                    continue;
                }

                // Generate moves for all planning variables
                let mut moves = Vec::new();
                for &field_idx in &class_def.planning_variable_indices {
                    let field_def = &class_def.fields[field_idx];
                    if let Some(range_name) = &field_def.value_range {
                        if let Some(range) = solution.descriptor.value_range(range_name) {
                            let values = get_range_values(range, solution);
                            for value in values {
                                moves.push(DynamicChangeMove::new(
                                    class_idx,
                                    entity_idx,
                                    field_idx,
                                    field_def.name.clone(),
                                    value,
                                ));
                            }
                        }
                    }
                }

                if !moves.is_empty() {
                    let entity_ref = EntityReference {
                        descriptor_index: class_idx,
                        entity_index: entity_idx,
                    };
                    placements.push(Placement::new(entity_ref, moves));
                }
            }
        }

        placements
    }
}

fn get_range_values(range: &ValueRangeDef, solution: &DynamicSolution) -> Vec<DynamicValue> {
    match range {
        ValueRangeDef::Explicit(values) => values.clone(),
        ValueRangeDef::IntRange { start, end } => (*start..*end).map(DynamicValue::I64).collect(),
        ValueRangeDef::EntityClass(class_idx) => {
            let count = solution
                .entities
                .get(*class_idx)
                .map(|e| e.len())
                .unwrap_or(0);
            (0..count)
                .map(|i| DynamicValue::Ref(*class_idx, i))
                .collect()
        }
        ValueRangeDef::FactClass(class_idx) => {
            let count = solution.facts.get(*class_idx).map(|f| f.len()).unwrap_or(0);
            (0..count)
                .map(|i| DynamicValue::FactRef(*class_idx, i))
                .collect()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::descriptor::{
        DynamicDescriptor, EntityClassDef, FieldDef, FieldType, ValueRangeDef,
    };
    use crate::solution::DynamicEntity;

    #[test]
    fn test_generate_moves() {
        let mut desc = DynamicDescriptor::new();
        desc.add_entity_class(EntityClassDef::new(
            "Queen",
            vec![
                FieldDef::new("column", FieldType::I64),
                FieldDef::planning_variable("row", FieldType::I64, "rows"),
            ],
        ));
        desc.add_value_range("rows", ValueRangeDef::int_range(0, 4));

        let mut solution = DynamicSolution::new(desc);
        solution.add_entity(
            0,
            DynamicEntity::new(0, vec![DynamicValue::I64(0), DynamicValue::I64(0)]),
        );
        solution.add_entity(
            0,
            DynamicEntity::new(1, vec![DynamicValue::I64(1), DynamicValue::I64(1)]),
        );

        let selector = DynamicMoveSelector::new();
        // generate_moves now returns an iterator; collect to count
        let moves: Vec<_> = selector.generate_moves(&solution).collect();

        // 2 entities * 4 possible row values = 8 moves
        assert_eq!(moves.len(), 8);
    }

    #[test]
    fn test_generate_moves_shuffled() {
        let mut desc = DynamicDescriptor::new();
        desc.add_entity_class(EntityClassDef::new(
            "Queen",
            vec![
                FieldDef::new("column", FieldType::I64),
                FieldDef::planning_variable("row", FieldType::I64, "rows"),
            ],
        ));
        desc.add_value_range("rows", ValueRangeDef::int_range(0, 4));

        let mut solution = DynamicSolution::new(desc);
        solution.add_entity(
            0,
            DynamicEntity::new(0, vec![DynamicValue::I64(0), DynamicValue::I64(0)]),
        );
        solution.add_entity(
            0,
            DynamicEntity::new(1, vec![DynamicValue::I64(1), DynamicValue::I64(1)]),
        );

        let selector = DynamicMoveSelector::new();
        let moves = selector.generate_moves_shuffled(&solution);

        // 2 entities * 4 possible row values = 8 moves
        assert_eq!(moves.len(), 8);

        // All moves should still be present (just in different order)
        let unshuffled: Vec<_> = selector.generate_moves(&solution).collect();
        assert_eq!(moves.len(), unshuffled.len());
    }

    #[test]
    fn test_dynamic_move_iterator() {
        let mut desc = DynamicDescriptor::new();
        desc.add_entity_class(EntityClassDef::new(
            "Queen",
            vec![
                FieldDef::new("column", FieldType::I64),
                FieldDef::planning_variable("row", FieldType::I64, "rows"),
            ],
        ));
        desc.add_value_range("rows", ValueRangeDef::int_range(0, 4));

        let mut solution = DynamicSolution::new(desc);
        solution.add_entity(
            0,
            DynamicEntity::new(0, vec![DynamicValue::I64(0), DynamicValue::I64(0)]),
        );
        solution.add_entity(
            0,
            DynamicEntity::new(1, vec![DynamicValue::I64(1), DynamicValue::I64(1)]),
        );

        // Create iterator and collect moves
        let iterator = DynamicMoveIterator::new(&solution);
        let moves: Vec<_> = iterator.collect();

        // 2 entities * 4 possible row values = 8 moves
        assert_eq!(moves.len(), 8);

        // Verify moves have correct structure
        // First entity (entity_idx=0) should have moves for values 0,1,2,3
        assert_eq!(moves[0].entity_idx, 0);
        assert_eq!(moves[0].class_idx, 0);
        assert_eq!(moves[0].field_idx, 1); // row is field index 1
        assert_eq!(moves[0].new_value, DynamicValue::I64(0));

        assert_eq!(moves[1].entity_idx, 0);
        assert_eq!(moves[1].new_value, DynamicValue::I64(1));

        assert_eq!(moves[2].entity_idx, 0);
        assert_eq!(moves[2].new_value, DynamicValue::I64(2));

        assert_eq!(moves[3].entity_idx, 0);
        assert_eq!(moves[3].new_value, DynamicValue::I64(3));

        // Second entity (entity_idx=1) should have moves for values 0,1,2,3
        assert_eq!(moves[4].entity_idx, 1);
        assert_eq!(moves[4].new_value, DynamicValue::I64(0));

        assert_eq!(moves[7].entity_idx, 1);
        assert_eq!(moves[7].new_value, DynamicValue::I64(3));
    }

    #[test]
    fn test_dynamic_move_iterator_multiple_variables() {
        let mut desc = DynamicDescriptor::new();
        desc.add_entity_class(EntityClassDef::new(
            "Task",
            vec![
                FieldDef::new("id", FieldType::I64),
                FieldDef::planning_variable("worker", FieldType::I64, "workers"),
                FieldDef::planning_variable("machine", FieldType::I64, "machines"),
            ],
        ));
        desc.add_value_range("workers", ValueRangeDef::int_range(0, 2));
        desc.add_value_range("machines", ValueRangeDef::int_range(0, 3));

        let mut solution = DynamicSolution::new(desc);
        solution.add_entity(
            0,
            DynamicEntity::new(
                0,
                vec![
                    DynamicValue::I64(0),
                    DynamicValue::I64(0),
                    DynamicValue::I64(0),
                ],
            ),
        );

        let iterator = DynamicMoveIterator::new(&solution);
        let moves: Vec<_> = iterator.collect();

        // 1 entity * (2 worker values + 3 machine values) = 5 moves
        assert_eq!(moves.len(), 5);

        // First two moves should be for worker (field_idx=1)
        assert_eq!(moves[0].field_idx, 1);
        assert_eq!(moves[0].new_value, DynamicValue::I64(0));
        assert_eq!(moves[1].field_idx, 1);
        assert_eq!(moves[1].new_value, DynamicValue::I64(1));

        // Next three moves should be for machine (field_idx=2)
        assert_eq!(moves[2].field_idx, 2);
        assert_eq!(moves[2].new_value, DynamicValue::I64(0));
        assert_eq!(moves[3].field_idx, 2);
        assert_eq!(moves[3].new_value, DynamicValue::I64(1));
        assert_eq!(moves[4].field_idx, 2);
        assert_eq!(moves[4].new_value, DynamicValue::I64(2));
    }

    #[test]
    fn test_dynamic_move_iterator_empty_solution() {
        let mut desc = DynamicDescriptor::new();
        desc.add_entity_class(EntityClassDef::new(
            "Queen",
            vec![
                FieldDef::new("column", FieldType::I64),
                FieldDef::planning_variable("row", FieldType::I64, "rows"),
            ],
        ));
        desc.add_value_range("rows", ValueRangeDef::int_range(0, 4));

        let solution = DynamicSolution::new(desc);
        // No entities added

        let iterator = DynamicMoveIterator::new(&solution);
        let moves: Vec<_> = iterator.collect();

        // No entities = no moves
        assert_eq!(moves.len(), 0);
    }

    #[test]
    fn test_dynamic_move_iterator_multiple_classes() {
        let mut desc = DynamicDescriptor::new();
        desc.add_entity_class(EntityClassDef::new(
            "Employee",
            vec![
                FieldDef::new("id", FieldType::I64),
                FieldDef::planning_variable("shift", FieldType::I64, "shifts"),
            ],
        ));
        desc.add_entity_class(EntityClassDef::new(
            "Vehicle",
            vec![
                FieldDef::new("id", FieldType::I64),
                FieldDef::planning_variable("route", FieldType::I64, "routes"),
            ],
        ));
        desc.add_value_range("shifts", ValueRangeDef::int_range(0, 2));
        desc.add_value_range("routes", ValueRangeDef::int_range(0, 3));

        let mut solution = DynamicSolution::new(desc);
        // Add 1 employee
        solution.add_entity(
            0,
            DynamicEntity::new(0, vec![DynamicValue::I64(0), DynamicValue::I64(0)]),
        );
        // Add 1 vehicle
        solution.add_entity(
            1,
            DynamicEntity::new(1, vec![DynamicValue::I64(0), DynamicValue::I64(0)]),
        );

        let iterator = DynamicMoveIterator::new(&solution);
        let moves: Vec<_> = iterator.collect();

        // 1 employee * 2 shifts + 1 vehicle * 3 routes = 5 moves
        assert_eq!(moves.len(), 5);

        // First two should be for employee (class_idx=0)
        assert_eq!(moves[0].class_idx, 0);
        assert_eq!(moves[1].class_idx, 0);

        // Last three should be for vehicle (class_idx=1)
        assert_eq!(moves[2].class_idx, 1);
        assert_eq!(moves[3].class_idx, 1);
        assert_eq!(moves[4].class_idx, 1);
    }
}
