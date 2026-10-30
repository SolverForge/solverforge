//! Read-only queries over a runtime model's variables and scalar groups.

use solverforge_core::domain::{DynamicListVariableSlot, DynamicScalarVariableSlot};

use super::{ListVariableSlot, RuntimeModel, ScalarGroupBinding, ScalarVariableSlot, VariableSlot};

impl<S, V, DM, IDM> RuntimeModel<S, V, DM, IDM> {
    /// Whether the model declares no variables.
    pub fn is_empty(&self) -> bool {
        self.variables.is_empty()
    }

    /// Whether any typed or dynamic list variable is declared.
    pub fn has_list_variables(&self) -> bool {
        self.variables.iter().any(|variable| {
            matches!(
                variable,
                VariableSlot::List(_) | VariableSlot::DynamicList(_)
            )
        })
    }

    /// Whether any typed or dynamic scalar variable is declared.
    pub fn has_scalar_variables(&self) -> bool {
        self.variables.iter().any(|variable| {
            matches!(
                variable,
                VariableSlot::Scalar(_) | VariableSlot::DynamicScalar(_)
            )
        })
    }

    /// Whether any dynamic (host-declared) variable is declared.
    pub fn has_dynamic_variables(&self) -> bool {
        self.variables.iter().any(|variable| {
            matches!(
                variable,
                VariableSlot::DynamicScalar(_) | VariableSlot::DynamicList(_)
            )
        })
    }

    /// Whether any dynamic list variable is declared.
    pub fn has_dynamic_list_variables(&self) -> bool {
        self.variables
            .iter()
            .any(|variable| matches!(variable, VariableSlot::DynamicList(_)))
    }

    /// Whether the model has scalar variables and no list variables.
    pub fn is_scalar_only(&self) -> bool {
        self.has_scalar_variables() && !self.has_list_variables()
    }

    /// Whether any scalar variable supplies nearby value candidates.
    pub fn has_nearby_scalar_change_variables(&self) -> bool {
        self.scalar_variables()
            .any(ScalarVariableSlot::supports_nearby_change)
            || self
                .dynamic_scalar_variables()
                .any(DynamicScalarVariableSlot::has_nearby_value_candidates)
    }

    /// Whether any scalar variable supplies nearby entity candidates.
    pub fn has_nearby_scalar_swap_variables(&self) -> bool {
        self.scalar_variables()
            .any(ScalarVariableSlot::supports_nearby_swap)
            || self
                .dynamic_scalar_variables()
                .any(DynamicScalarVariableSlot::has_nearby_entity_candidates)
    }

    /// Assignment-backed scalar groups with their index among all groups.
    pub fn assignment_scalar_groups(
        &self,
    ) -> impl Iterator<Item = (usize, &ScalarGroupBinding<S>)> {
        self.scalar_groups
            .iter()
            .enumerate()
            .filter(|(_, group)| group.is_assignment())
    }

    /// Whether an assignment group owns the typed scalar `variable`.
    pub fn assignment_group_covers_scalar_variable(
        &self,
        variable: &ScalarVariableSlot<S>,
    ) -> bool {
        self.assignment_scalar_groups().any(|(_, group)| {
            group.members.iter().any(|member| {
                member.descriptor_index == variable.descriptor_index
                    && member.variable_index == variable.variable_index
            })
        })
    }

    /// Whether an assignment group owns the dynamic scalar `variable`.
    pub fn assignment_group_covers_dynamic_scalar_variable(
        &self,
        variable: &DynamicScalarVariableSlot<S>,
    ) -> bool {
        self.assignment_scalar_groups().any(|(_, group)| {
            group.members.iter().any(|member| {
                member.dynamic_identity().is_some_and(|(entity, slot)| {
                    entity == variable.entity && slot == variable.variable
                })
            })
        })
    }

    /// Whether any scalar group is declared.
    pub fn has_scalar_groups(&self) -> bool {
        !self.scalar_groups.is_empty()
    }

    /// Whether any assignment-backed scalar group is declared.
    pub fn has_assignment_scalar_groups(&self) -> bool {
        self.assignment_scalar_groups().next().is_some()
    }

    /// Candidate-backed scalar groups with their index among all groups.
    pub fn candidate_scalar_groups(&self) -> impl Iterator<Item = (usize, &ScalarGroupBinding<S>)> {
        self.scalar_groups
            .iter()
            .enumerate()
            .filter(|(_, group)| group.is_candidate_group())
    }

    /// Whether any candidate-backed scalar group is declared.
    pub fn has_candidate_scalar_groups(&self) -> bool {
        self.candidate_scalar_groups().next().is_some()
    }

    /// Whether any typed list variable supports ruin moves.
    pub fn has_list_ruin_variables(&self) -> bool {
        self.list_variables().any(ListVariableSlot::supports_ruin)
    }

    /// Typed list variables with both precedence hooks.
    pub fn list_precedence_variables(
        &self,
    ) -> impl Iterator<Item = &ListVariableSlot<S, V, DM, IDM>> {
        self.list_variables()
            .filter(|variable| variable.supports_precedence_moves())
    }

    /// Whether any typed list variable has precedence hooks.
    pub fn has_list_precedence_variables(&self) -> bool {
        self.list_precedence_variables().next().is_some()
    }

    /// Whether any typed list variable has the k-opt route hooks.
    pub fn has_k_opt_variables(&self) -> bool {
        self.list_variables().any(ListVariableSlot::supports_k_opt)
    }

    /// Whether any conflict repair is declared.
    pub fn has_conflict_repairs(&self) -> bool {
        !self.conflict_repairs.is_empty()
    }

    /// Typed scalar variables, in declaration order.
    pub fn scalar_variables(&self) -> impl Iterator<Item = &ScalarVariableSlot<S>> {
        self.variables.iter().filter_map(|variable| match variable {
            VariableSlot::Scalar(ctx) => Some(ctx),
            VariableSlot::List(_)
            | VariableSlot::DynamicScalar(_)
            | VariableSlot::DynamicList(_) => None,
        })
    }

    /// Dynamic scalar variables, in declaration order.
    pub fn dynamic_scalar_variables(&self) -> impl Iterator<Item = &DynamicScalarVariableSlot<S>> {
        self.variables.iter().filter_map(|variable| match variable {
            VariableSlot::DynamicScalar(ctx) => Some(ctx),
            VariableSlot::Scalar(_) | VariableSlot::List(_) | VariableSlot::DynamicList(_) => None,
        })
    }

    /// The one typed scalar variable matching `entity_class` and
    /// `variable_name` (`None` matches any).
    ///
    /// # Errors
    ///
    /// Returns a message when no variable or more than one variable matches.
    pub fn scalar_variable_target(
        &self,
        entity_class: Option<&str>,
        variable_name: Option<&str>,
    ) -> Result<ScalarVariableSlot<S>, String> {
        let mut matches = self
            .scalar_variables()
            .filter(|slot| slot.matches_target(entity_class, variable_name));
        let Some(first) = matches.next().copied() else {
            return Err(match (entity_class, variable_name) {
                (Some(entity), Some(variable)) => {
                    format!("no scalar variable `{entity}.{variable}` exists in the runtime model")
                }
                (Some(entity), None) => {
                    format!("no scalar variable for entity `{entity}` exists in the runtime model")
                }
                (None, Some(variable)) => {
                    format!("no scalar variable named `{variable}` exists in the runtime model")
                }
                (None, None) => {
                    "exhaustive search requires exactly one scalar variable or an explicit target"
                        .to_string()
                }
            });
        };
        if matches.next().is_some() {
            return Err(
                "exhaustive search target is ambiguous; specify entity_class and variable_name"
                    .to_string(),
            );
        }
        Ok(first)
    }

    /// Product of the per-entity candidate counts of `slot` (each capped at
    /// `value_candidate_limit`): the size of the full assignment space. `Some(0)`
    /// when an entity has no candidate; `None` when the product overflows.
    pub fn finite_scalar_candidate_space_estimate(
        &self,
        solution: &S,
        slot: ScalarVariableSlot<S>,
        value_candidate_limit: Option<usize>,
    ) -> Option<usize> {
        let entity_count = (slot.entity_count)(solution);
        let mut total: usize = 1;
        for entity_index in 0..entity_count {
            let candidate_count = slot
                .candidate_values_for_entity(solution, entity_index, value_candidate_limit)
                .len();
            if candidate_count == 0 {
                return Some(0);
            }
            total = total.checked_mul(candidate_count)?;
        }
        Some(total)
    }

    /// Typed list variables, in declaration order.
    pub fn list_variables(&self) -> impl Iterator<Item = &ListVariableSlot<S, V, DM, IDM>> {
        self.variables.iter().filter_map(|variable| match variable {
            VariableSlot::List(ctx) => Some(ctx),
            VariableSlot::Scalar(_)
            | VariableSlot::DynamicScalar(_)
            | VariableSlot::DynamicList(_) => None,
        })
    }

    /// Dynamic list variables, in declaration order.
    pub fn dynamic_list_variables(&self) -> impl Iterator<Item = &DynamicListVariableSlot<S>> {
        self.variables.iter().filter_map(|variable| match variable {
            VariableSlot::DynamicList(ctx) => Some(ctx),
            VariableSlot::Scalar(_) | VariableSlot::List(_) | VariableSlot::DynamicScalar(_) => {
                None
            }
        })
    }
}
