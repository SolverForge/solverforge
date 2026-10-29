use super::*;

/// Public dynamic list variable slot.
pub struct DynamicListVariableSlot<S> {
    pub entity: EntityClassId,
    pub variable: VariableId,
    pub entity_type_name: &'static str,
    pub variable_name: &'static str,
    pub(super) descriptor_index: Option<usize>,
    pub(super) descriptor_variable_index: Option<usize>,
    pub(super) access: Arc<dyn DynamicListAccess<S>>,
    pub(super) metadata: Option<Arc<dyn DynamicListMetadata<S>>>,
}

impl<S> Clone for DynamicListVariableSlot<S> {
    fn clone(&self) -> Self {
        Self {
            entity: self.entity,
            variable: self.variable,
            entity_type_name: self.entity_type_name,
            variable_name: self.variable_name,
            descriptor_index: self.descriptor_index,
            descriptor_variable_index: self.descriptor_variable_index,
            access: Arc::clone(&self.access),
            metadata: self.metadata.as_ref().map(Arc::clone),
        }
    }
}

impl<S> fmt::Debug for DynamicListVariableSlot<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DynamicListVariableSlot")
            .field("entity", &self.entity)
            .field("variable", &self.variable)
            .field("entity_type_name", &self.entity_type_name)
            .field("variable_name", &self.variable_name)
            .field("descriptor_index", &self.descriptor_index)
            .field("descriptor_variable_index", &self.descriptor_variable_index)
            .field(
                "metadata_capabilities",
                &self
                    .metadata
                    .as_ref()
                    .map(|metadata| metadata.capabilities()),
            )
            .finish()
    }
}

impl<S> PartialEq for DynamicListVariableSlot<S> {
    fn eq(&self, other: &Self) -> bool {
        self.entity == other.entity
            && self.variable == other.variable
            && self.entity_type_name == other.entity_type_name
            && self.variable_name == other.variable_name
            && self.descriptor_index == other.descriptor_index
            && self.descriptor_variable_index == other.descriptor_variable_index
    }
}

impl<S> Eq for DynamicListVariableSlot<S> {}

impl<S> DynamicListVariableSlot<S> {
    /// Fallible access constructor for integrations that compile a slot from
    /// runtime schema. This verifies that the object supplying mutations is
    /// bound to exactly the same logical entity and variable as the slot.
    ///
    /// Integrations requiring immutable metadata can use
    /// [`Self::with_access_and_metadata`] to validate both bindings together.
    pub fn try_with_access(
        entity: EntityClassId,
        variable: VariableId,
        entity_type_name: &'static str,
        variable_name: &'static str,
        access: Arc<dyn DynamicListAccess<S>>,
    ) -> Result<Self, String> {
        if access.entity_class() != entity || access.variable() != variable {
            return Err(format!(
                "dynamic list access is bound to entity ID {} variable ID {}, but slot {}.{} is entity ID {} variable ID {}",
                access.entity_class().0,
                access.variable().0,
                entity_type_name,
                variable_name,
                entity.0,
                variable.0,
            ));
        }
        Ok(Self {
            entity,
            variable,
            entity_type_name,
            variable_name,
            descriptor_index: None,
            descriptor_variable_index: None,
            access,
            metadata: None,
        })
    }

    /// Constructs a slot with its immutable, structurally bound metadata.
    ///
    /// Metadata identity is checked immediately; a binding implementation
    /// cannot accidentally be reused for a same-named variable in another
    /// entity class. Phase assembly remains responsible for rejecting a
    /// selected family whose specific capability bit is absent.
    pub fn with_access_and_metadata(
        entity: EntityClassId,
        variable: VariableId,
        entity_type_name: &'static str,
        variable_name: &'static str,
        access: Arc<dyn DynamicListAccess<S>>,
        metadata: Arc<dyn DynamicListMetadata<S>>,
    ) -> Result<Self, String> {
        Self::try_with_access(entity, variable, entity_type_name, variable_name, access)?
            .with_metadata(metadata)
    }

    /// Attaches immutable metadata after access construction while preserving
    /// the same structural identity validation as
    /// [`Self::with_access_and_metadata`].
    pub fn with_metadata(
        mut self,
        metadata: Arc<dyn DynamicListMetadata<S>>,
    ) -> Result<Self, String> {
        if metadata.entity_class() != self.entity || metadata.variable() != self.variable {
            return Err(format!(
                "dynamic list metadata is bound to entity ID {} variable ID {}, but slot {}.{} is entity ID {} variable ID {}",
                metadata.entity_class().0,
                metadata.variable().0,
                self.entity_type_name,
                self.variable_name,
                self.entity.0,
                self.variable.0,
            ));
        }
        self.metadata = Some(metadata);
        Ok(self)
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
            DynamicVariableKind::List,
        )?;
        if let Some(existing) = self.descriptor_index {
            if existing != indexes.descriptor_index {
                return Err(format!(
                    "dynamic list variable {}.{} was pre-bound to descriptor index {existing}, but logical entity ID {} resolves to descriptor index {}",
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
                "dynamic list variable {}.{} has not been resolved against a SolutionDescriptor",
                self.entity_type_name, self.variable_name
            )
        })
    }

    pub fn descriptor_variable_index(&self) -> usize {
        self.descriptor_variable_index.unwrap_or_else(|| {
            panic!(
                "dynamic list variable {}.{} has not been resolved against a SolutionDescriptor",
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

    pub fn element_count(&self, solution: &S) -> usize {
        self.access.element_count(solution)
    }

    pub fn element(&self, solution: &S, element_index: usize) -> Option<usize> {
        self.access.element(solution, element_index)
    }

    pub fn assigned_elements(&self, solution: &S) -> Vec<usize> {
        self.access.assigned_elements(solution)
    }

    pub fn list_len(&self, solution: &S, row: usize) -> usize {
        self.access.len(solution, row)
    }

    pub fn list_get(&self, solution: &S, row: usize, pos: usize) -> Option<usize> {
        self.access.get(solution, row, pos)
    }

    pub fn list_insert(&self, solution: &mut S, row: usize, pos: usize, value: usize) {
        self.access.insert(solution, row, pos, value);
    }

    pub fn list_remove(&self, solution: &mut S, row: usize, pos: usize) -> Option<usize> {
        self.access.remove(solution, row, pos)
    }

    pub fn access_capabilities(&self) -> DynamicListAccessCapabilities {
        self.access.capabilities()
    }

    pub fn list_set(&self, solution: &mut S, row: usize, pos: usize, value: usize) -> bool {
        self.access.set(solution, row, pos, value)
    }

    /// Replaces a complete list row as one logical mutation. Dynamic kernels
    /// use this for route replacement rather than composing remove/insert and
    /// accidentally publishing multiple dirty/revision transitions.
    pub fn list_replace(&self, solution: &mut S, row: usize, values: Vec<usize>) -> bool {
        self.access.replace(solution, row, values)
    }

    pub fn list_reverse(&self, solution: &mut S, row: usize, start: usize, end: usize) -> bool {
        self.access.reverse(solution, row, start, end)
    }

    pub fn sublist_remove(
        &self,
        solution: &mut S,
        row: usize,
        start: usize,
        end: usize,
    ) -> Option<Vec<usize>> {
        self.access.sublist_remove(solution, row, start, end)
    }

    pub fn sublist_insert(
        &self,
        solution: &mut S,
        row: usize,
        pos: usize,
        values: Vec<usize>,
    ) -> bool {
        self.access.sublist_insert(solution, row, pos, values)
    }

    /// Returns slot-bound metadata when the integration compiled it.
    ///
    /// Absence is intentional and must cause a selected metadata-dependent
    /// phase to fail validation; it is never a request to synthesize defaults.
    pub fn metadata(&self) -> Option<&dyn DynamicListMetadata<S>> {
        self.metadata.as_deref()
    }

    pub fn metadata_capabilities(&self) -> Option<DynamicListMetadataCapabilities> {
        self.metadata
            .as_ref()
            .map(|metadata| metadata.capabilities())
    }

    pub fn require_metadata(&self) -> Result<&dyn DynamicListMetadata<S>, String> {
        self.metadata().ok_or_else(|| {
            format!(
                "dynamic list variable {}.{} has no immutable list metadata; selected list phases must declare and bind their required capability bundle before solve",
                self.entity_type_name, self.variable_name,
            )
        })
    }
}

impl<S> DynamicListVariableSlot<S>
where
    S: DynamicModelBackend,
{
    pub fn new(
        entity: EntityClassId,
        variable: VariableId,
        entity_type_name: &'static str,
        variable_name: &'static str,
    ) -> Self {
        Self {
            entity,
            variable,
            entity_type_name,
            variable_name,
            descriptor_index: None,
            descriptor_variable_index: None,
            access: Arc::new(BackendListAccess { entity, variable }),
            metadata: None,
        }
    }

    /// Backend-access convenience constructor with immutable metadata.
    pub fn new_with_metadata(
        entity: EntityClassId,
        variable: VariableId,
        entity_type_name: &'static str,
        variable_name: &'static str,
        metadata: Arc<dyn DynamicListMetadata<S>>,
    ) -> Result<Self, String> {
        Self::with_access_and_metadata(
            entity,
            variable,
            entity_type_name,
            variable_name,
            Arc::new(BackendListAccess { entity, variable }),
            metadata,
        )
    }
}
