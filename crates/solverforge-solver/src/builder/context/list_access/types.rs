use std::fmt;

/// A required primitive/capability missing from one list slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ListAccessCapability {
    Set,
    Replace,
    Reverse,
    Sublist,
    ElementOwner,
    ConstructionOrderKey,
    Precedence,
    CrossPositionDistance,
    IntraPositionDistance,
    Route,
    Savings,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ListAccessError {
    pub capability: ListAccessCapability,
    pub entity_type_name: &'static str,
    pub variable_name: &'static str,
}

impl fmt::Display for ListAccessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "list variable {}.{} does not provide required {:?} capability",
            self.entity_type_name, self.variable_name, self.capability
        )
    }
}

/// One immutable list access carrier used by canonical construction and
/// neighborhood kernels.
///
/// `Element` deliberately remains an associated type: native typed models
/// keep their real list value type, while dynamic models use `usize`. The
/// protocol exposes all logical mutation operations as one unit so a dynamic
/// implementation cannot silently emulate a range operation through a
/// different callback/notification path.
pub(crate) trait ListAccess<S>: Clone + Send + Sync + fmt::Debug {
    type Element: Clone + PartialEq + Send + Sync + fmt::Debug + 'static;

    fn entity_type_name(&self) -> &'static str;
    fn variable_name(&self) -> &'static str;
    fn descriptor_index(&self) -> usize;

    fn entity_count(&self, solution: &S) -> usize;
    fn element_count(&self, solution: &S) -> usize;
    fn index_to_element(&self, solution: &S, element_index: usize) -> Option<Self::Element>;
    /// Stable declared-stream identity key. A bound runtime resolves it once
    /// to a source index and uses that index for ordering/assignment tracking.
    fn element_source_key(&self, solution: &S, element: &Self::Element) -> usize;
    fn assigned_elements(&self, solution: &S) -> Vec<Self::Element>;

    fn list_len(&self, solution: &S, entity: usize) -> usize;
    fn list_get(&self, solution: &S, entity: usize, pos: usize) -> Option<Self::Element>;
    fn list_insert(&self, solution: &mut S, entity: usize, pos: usize, value: Self::Element);
    fn list_remove(&self, solution: &mut S, entity: usize, pos: usize) -> Option<Self::Element>;
    fn list_set(
        &self,
        solution: &mut S,
        entity: usize,
        pos: usize,
        value: Self::Element,
    ) -> Result<(), ListAccessError>;
    fn list_reverse(
        &self,
        solution: &mut S,
        entity: usize,
        start: usize,
        end: usize,
    ) -> Result<(), ListAccessError>;
    fn sublist_remove(
        &self,
        solution: &mut S,
        entity: usize,
        start: usize,
        end: usize,
    ) -> Result<Vec<Self::Element>, ListAccessError>;
    fn sublist_insert(
        &self,
        solution: &mut S,
        entity: usize,
        pos: usize,
        values: Vec<Self::Element>,
    ) -> Result<(), ListAccessError>;

    fn element_owner(
        &self,
        solution: &S,
        element: &Self::Element,
    ) -> Result<Option<usize>, ListAccessError>;
    fn construction_order_key(
        &self,
        solution: &S,
        element: Self::Element,
    ) -> Result<i64, ListAccessError>;
    fn extend_precedence_successors(
        &self,
        solution: &S,
        element: Self::Element,
        successors: &mut Vec<Self::Element>,
    ) -> Result<(), ListAccessError>;
    fn precedence_duration(
        &self,
        solution: &S,
        element: Self::Element,
    ) -> Result<usize, ListAccessError>;

    fn cross_position_distance(
        &self,
        solution: &S,
        from_entity: usize,
        from_position: usize,
        to_entity: usize,
        to_position: usize,
    ) -> Result<f64, ListAccessError>;
    fn intra_position_distance(
        &self,
        solution: &S,
        entity: usize,
        from_position: usize,
        to_position: usize,
    ) -> Result<f64, ListAccessError>;
}
