pub type ScalarAssignmentRule<S> = fn(&S, usize, usize, usize, usize) -> bool;

pub(crate) struct ScalarAssignmentDeclaration<S> {
    pub(crate) required_entity: Option<fn(&S, usize) -> bool>,
    pub(crate) capacity_key: Option<fn(&S, usize, usize) -> Option<usize>>,
    pub(crate) position_key: Option<fn(&S, usize) -> i64>,
    pub(crate) sequence_key: Option<fn(&S, usize, usize) -> Option<usize>>,
    pub(crate) entity_order: Option<fn(&S, usize) -> i64>,
    pub(crate) value_order: Option<fn(&S, usize, usize) -> i64>,
    pub(crate) assignment_rule: Option<ScalarAssignmentRule<S>>,
}

impl<S> Clone for ScalarAssignmentDeclaration<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S> Copy for ScalarAssignmentDeclaration<S> {}

impl<S> Default for ScalarAssignmentDeclaration<S> {
    fn default() -> Self {
        Self {
            required_entity: None,
            capacity_key: None,
            position_key: None,
            sequence_key: None,
            entity_order: None,
            value_order: None,
            assignment_rule: None,
        }
    }
}
