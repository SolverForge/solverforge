//! Entity placer for construction heuristics.

use solverforge_scoring::ScoreDirector;
use solverforge_solver::heuristic::selector::EntityReference;
use solverforge_solver::phase::construction::{EntityPlacer, Placement};

use super::{get_range_values, DynamicChangeMove};
use crate::solution::DynamicSolution;

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
