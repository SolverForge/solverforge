// Variable type definitions

// The type of a planning variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VariableType {
    // A genuine planning variable that the solver optimizes.
    Genuine,
    // A list variable containing multiple values.
    List,
    // A shadow variable computed from other variables.
    Shadow(ShadowVariableKind),
}

// The kind of shadow variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShadowVariableKind {
    // Custom shadow variable with user-defined listener.
    Custom,
    // Inverse of another variable (bidirectional relationship).
    InverseRelation,
    // Index within a list variable.
    Index,
    // Next element in a list variable.
    NextElement,
    // Previous element in a list variable.
    PreviousElement,
    // Cascading update from other shadow variables.
    Cascading,
    // Piggyback on another shadow variable's listener.
    Piggyback,
}

// The type of value range for a planning variable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValueRangeType {
    // A collection of discrete values.
    Collection,
    // A countable range (e.g., integers from 1 to 100).
    CountableRange {
        // Inclusive start of the range.
        from: i64,
        // Exclusive end of the range.
        to: i64,
    },
    // An entity-dependent value range.
    EntityDependent,
}

impl VariableType {
    /// Returns true if this is a genuine (non-shadow) variable.
    ///
    /// Genuine variables include scalar and list variables.
    pub fn is_genuine(&self) -> bool {
        matches!(self, VariableType::Genuine | VariableType::List)
    }

    pub fn is_shadow(&self) -> bool {
        matches!(self, VariableType::Shadow(_))
    }

    pub fn is_list(&self) -> bool {
        matches!(self, VariableType::List)
    }

    pub fn is_basic(&self) -> bool {
        matches!(self, VariableType::Genuine)
    }
}

impl ShadowVariableKind {
    pub fn requires_listener(&self) -> bool {
        matches!(
            self,
            ShadowVariableKind::Custom | ShadowVariableKind::Cascading
        )
    }

    pub fn is_automatic(&self) -> bool {
        matches!(
            self,
            ShadowVariableKind::InverseRelation
                | ShadowVariableKind::Index
                | ShadowVariableKind::NextElement
                | ShadowVariableKind::PreviousElement
        )
    }

    /// Returns true if this shadow variable piggybacks on another
    /// shadow variable's listener rather than having its own.
    pub fn is_piggyback(&self) -> bool {
        matches!(self, ShadowVariableKind::Piggyback)
    }
}
