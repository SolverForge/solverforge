use super::*;

/// Public dynamic scalar variable slot.
pub struct DynamicScalarVariableSlot<S> {
    pub entity: EntityClassId,
    pub variable: VariableId,
    pub entity_type_name: &'static str,
    pub variable_name: &'static str,
    pub allows_unassigned: bool,
    pub(super) descriptor_index: Option<usize>,
    pub(super) descriptor_variable_index: Option<usize>,
    pub(super) access: Arc<dyn DynamicScalarAccess<S>>,
}

impl<S> Clone for DynamicScalarVariableSlot<S> {
    fn clone(&self) -> Self {
        Self {
            entity: self.entity,
            variable: self.variable,
            entity_type_name: self.entity_type_name,
            variable_name: self.variable_name,
            allows_unassigned: self.allows_unassigned,
            descriptor_index: self.descriptor_index,
            descriptor_variable_index: self.descriptor_variable_index,
            access: Arc::clone(&self.access),
        }
    }
}

impl<S> fmt::Debug for DynamicScalarVariableSlot<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DynamicScalarVariableSlot")
            .field("entity", &self.entity)
            .field("variable", &self.variable)
            .field("entity_type_name", &self.entity_type_name)
            .field("variable_name", &self.variable_name)
            .field("allows_unassigned", &self.allows_unassigned)
            .field("descriptor_index", &self.descriptor_index)
            .field("descriptor_variable_index", &self.descriptor_variable_index)
            .finish()
    }
}

impl<S> PartialEq for DynamicScalarVariableSlot<S> {
    fn eq(&self, other: &Self) -> bool {
        self.entity == other.entity
            && self.variable == other.variable
            && self.entity_type_name == other.entity_type_name
            && self.variable_name == other.variable_name
            && self.allows_unassigned == other.allows_unassigned
            && self.descriptor_index == other.descriptor_index
            && self.descriptor_variable_index == other.descriptor_variable_index
    }
}

impl<S> Eq for DynamicScalarVariableSlot<S> {}

impl<S> DynamicScalarVariableSlot<S> {
    pub fn with_access(
        entity: EntityClassId,
        variable: VariableId,
        entity_type_name: &'static str,
        variable_name: &'static str,
        allows_unassigned: bool,
        access: Arc<dyn DynamicScalarAccess<S>>,
    ) -> Self {
        Self {
            entity,
            variable,
            entity_type_name,
            variable_name,
            allows_unassigned,
            descriptor_index: None,
            descriptor_variable_index: None,
            access,
        }
    }

    pub fn resolve_descriptor_index(
        &mut self,
        descriptor: &SolutionDescriptor,
    ) -> Result<(), String> {
        let indexes = resolve_dynamic_descriptor_indexes(
            descriptor,
            self.entity,
            self.variable,
            self.entity_type_name,
            self.variable_name,
            DynamicVariableKind::Scalar,
        )?;
        if let Some(existing) = self.descriptor_index {
            if existing != indexes.descriptor_index {
                return Err(format!(
                    "dynamic scalar variable {}.{} was pre-bound to descriptor index {existing}, but logical entity ID {} resolves to descriptor index {}",
                    self.entity_type_name,
                    self.variable_name,
                    self.entity.0,
                    indexes.descriptor_index
                ));
            }
        }
        self.descriptor_index = Some(indexes.descriptor_index);
        self.descriptor_variable_index = Some(indexes.variable_index);
        Ok(())
    }

    pub fn resolved_against(mut self, descriptor: &SolutionDescriptor) -> Result<Self, String> {
        self.resolve_descriptor_index(descriptor)?;
        Ok(self)
    }

    pub fn is_descriptor_resolved(&self) -> bool {
        self.descriptor_index.is_some() && self.descriptor_variable_index.is_some()
    }

    pub fn descriptor_index(&self) -> usize {
        self.descriptor_index.unwrap_or_else(|| {
            panic!(
                "dynamic scalar variable {}.{} has not been resolved against a SolutionDescriptor",
                self.entity_type_name, self.variable_name
            )
        })
    }

    pub fn descriptor_variable_index(&self) -> usize {
        self.descriptor_variable_index.unwrap_or_else(|| {
            panic!(
                "dynamic scalar variable {}.{} has not been resolved against a SolutionDescriptor",
                self.entity_type_name, self.variable_name
            )
        })
    }

    pub fn matches_target(&self, entity_class: Option<&str>, variable_name: Option<&str>) -> bool {
        entity_class.is_none_or(|entity| entity == self.entity_type_name)
            && variable_name.is_none_or(|variable| variable == self.variable_name)
    }

    pub fn entity_count(&self, solution: &S) -> usize {
        self.access.entity_count(solution)
    }

    pub fn current_value(&self, solution: &S, row: usize) -> Option<usize> {
        self.access.get(solution, row)
    }

    pub fn set_value(&self, solution: &mut S, row: usize, value: Option<usize>) {
        self.access.set(solution, row, value);
    }

    pub fn candidate_values<'a>(&self, solution: &'a S, row: usize) -> &'a [usize] {
        self.access.candidate_values(solution, row)
    }

    pub fn has_nearby_value_candidates(&self) -> bool {
        self.access.has_nearby_value_candidates()
    }

    pub fn visit_nearby_value_candidates(
        &self,
        solution: &S,
        row: usize,
        limit: usize,
        visit: &mut dyn FnMut(usize),
    ) -> bool {
        self.access
            .visit_nearby_value_candidates(solution, row, limit, visit)
    }

    pub fn nearby_value_distance(&self, solution: &S, row: usize, candidate: usize) -> Option<f64> {
        self.access.nearby_value_distance(solution, row, candidate)
    }

    pub fn has_nearby_entity_candidates(&self) -> bool {
        self.access.has_nearby_entity_candidates()
    }

    pub fn visit_nearby_entity_candidates(
        &self,
        solution: &S,
        left_row: usize,
        limit: usize,
        visit: &mut dyn FnMut(usize),
    ) -> bool {
        self.access
            .visit_nearby_entity_candidates(solution, left_row, limit, visit)
    }

    pub fn nearby_entity_distance(
        &self,
        solution: &S,
        left_row: usize,
        right_row: usize,
    ) -> Option<f64> {
        self.access
            .nearby_entity_distance(solution, left_row, right_row)
    }

    pub fn value_is_legal(&self, solution: &S, row: usize, value: Option<usize>) -> bool {
        let Some(value) = value else {
            return self.allows_unassigned;
        };
        self.access.value_is_legal(solution, row, value)
    }
}

impl<S> DynamicScalarVariableSlot<S>
where
    S: DynamicModelBackend,
{
    pub fn new(
        entity: EntityClassId,
        variable: VariableId,
        entity_type_name: &'static str,
        variable_name: &'static str,
        allows_unassigned: bool,
    ) -> Self {
        Self::with_access(
            entity,
            variable,
            entity_type_name,
            variable_name,
            allows_unassigned,
            Arc::new(BackendScalarAccess { entity, variable }),
        )
    }
}
