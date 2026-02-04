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

    pub fn generate_moves(&self, solution: &DynamicSolution) -> Vec<DynamicChangeMove> {
        use rand::seq::SliceRandom;

        let mut moves = Vec::new();

        for (class_idx, class_def) in solution.descriptor.entity_classes.iter().enumerate() {
            for entity_idx in 0..solution
                .entities
                .get(class_idx)
                .map(|e| e.len())
                .unwrap_or(0)
            {
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
            }
        }

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
    fn iter_moves<'a, D: ScoreDirector<DynamicSolution>>(
        &'a self,
        score_director: &'a D,
    ) -> Box<dyn Iterator<Item = DynamicChangeMove> + 'a> {
        let solution = score_director.working_solution();
        let moves = self.generate_moves(solution);
        Box::new(moves.into_iter())
    }

    fn size<D: ScoreDirector<DynamicSolution>>(&self, score_director: &D) -> usize {
        self.generate_moves(score_director.working_solution()).len()
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
        let moves = selector.generate_moves(&solution);

        // 2 entities * 4 possible row values = 8 moves
        assert_eq!(moves.len(), 8);
    }
}
