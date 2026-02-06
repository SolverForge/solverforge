//! Variable type definitions

use std::any::TypeId;

/// The type of a planning variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VariableType {
    /// A genuine planning variable that the solver optimizes.
    Genuine,
    /// A chained planning variable where entities form chains rooted at anchors.
    ///
    /// Chained variables are used for problems like vehicle routing where:
    /// - Each entity points to either an anchor (problem fact) or another entity
    /// - Entities form chains: Anchor ← Entity1 ← Entity2 ← Entity3
    /// - No cycles or branching allowed
    Chained,
    /// A list variable containing multiple values.
    List,
    /// A shadow variable computed from other variables.
    Shadow(ShadowVariableKind),
}

/// The kind of shadow variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShadowVariableKind {
    /// Custom shadow variable with user-defined listener.
    Custom,
    /// Inverse of another variable (bidirectional relationship).
    InverseRelation,
    /// Index within a list variable.
    Index,
    /// Next element in a list variable.
    NextElement,
    /// Previous element in a list variable.
    PreviousElement,
    /// Anchor in a chained variable.
    Anchor,
    /// Cascading update from other shadow variables.
    Cascading,
    /// Piggyback on another shadow variable's listener.
    Piggyback,
}

/// The type of value range for a planning variable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValueRangeType {
    /// A collection of discrete values.
    Collection,
    /// A countable range (e.g., integers from 1 to 100).
    CountableRange {
        /// Inclusive start of the range.
        from: i64,
        /// Exclusive end of the range.
        to: i64,
    },
    /// An entity-dependent value range.
    EntityDependent,
}

impl VariableType {
    /// Returns true if this is a genuine (non-shadow) variable.
    ///
    /// Genuine variables include basic, chained, and list variables.
    pub fn is_genuine(&self) -> bool {
        matches!(
            self,
            VariableType::Genuine | VariableType::Chained | VariableType::List
        )
    }

    /// Returns true if this is a shadow variable.
    pub fn is_shadow(&self) -> bool {
        matches!(self, VariableType::Shadow(_))
    }

    /// Returns true if this is a list variable.
    pub fn is_list(&self) -> bool {
        matches!(self, VariableType::List)
    }

    /// Returns true if this is a chained variable.
    ///
    /// Chained variables form chains rooted at anchor problem facts.
    pub fn is_chained(&self) -> bool {
        matches!(self, VariableType::Chained)
    }

    /// Returns true if this is a basic genuine variable (not chained or list).
    pub fn is_basic(&self) -> bool {
        matches!(self, VariableType::Genuine)
    }
}

impl ShadowVariableKind {
    /// Returns true if this shadow variable requires a custom listener.
    pub fn requires_listener(&self) -> bool {
        matches!(
            self,
            ShadowVariableKind::Custom | ShadowVariableKind::Cascading
        )
    }

    /// Returns true if this shadow variable is automatically maintained.
    pub fn is_automatic(&self) -> bool {
        matches!(
            self,
            ShadowVariableKind::InverseRelation
                | ShadowVariableKind::Index
                | ShadowVariableKind::NextElement
                | ShadowVariableKind::PreviousElement
                | ShadowVariableKind::Anchor
        )
    }

    /// Returns true if this shadow variable piggybacks on another
    /// shadow variable's listener rather than having its own.
    pub fn is_piggyback(&self) -> bool {
        matches!(self, ShadowVariableKind::Piggyback)
    }
}

/// Information about a chained variable's configuration.
///
/// Chained variables require knowledge of the anchor type to distinguish
/// between anchor values (chain roots) and entity values (chain members).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainedVariableInfo {
    /// The TypeId of the anchor type (problem fact at chain root).
    pub anchor_type_id: TypeId,
    /// The TypeId of the entity type (chain members).
    pub entity_type_id: TypeId,
    /// Whether this variable has an associated anchor shadow variable.
    pub has_anchor_shadow: bool,
}

impl ChainedVariableInfo {
    /// Creates new chained variable info.
    pub fn new<Anchor: 'static, Entity: 'static>() -> Self {
        Self {
            anchor_type_id: TypeId::of::<Anchor>(),
            entity_type_id: TypeId::of::<Entity>(),
            has_anchor_shadow: false,
        }
    }

    /// Creates new chained variable info with anchor shadow variable.
    pub fn with_anchor_shadow<Anchor: 'static, Entity: 'static>() -> Self {
        Self {
            anchor_type_id: TypeId::of::<Anchor>(),
            entity_type_id: TypeId::of::<Entity>(),
            has_anchor_shadow: true,
        }
    }

    /// Returns true if the given TypeId is the anchor type.
    pub fn is_anchor_type(&self, type_id: TypeId) -> bool {
        self.anchor_type_id == type_id
    }

    /// Returns true if the given TypeId is the entity type.
    pub fn is_entity_type(&self, type_id: TypeId) -> bool {
        self.entity_type_id == type_id
    }
}
