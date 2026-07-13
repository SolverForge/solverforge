//! Named typed/dynamic policies frozen while compiling one runtime list slot.
//!
//! The public typed list API historically permits a few deliberately absent
//! hooks.  Those meanings are represented here as declared policies before a
//! shared kernel runs; the kernel never decides behavior from a nullable
//! source hook. Dynamic bindings stay strict where their schema contract
//! requires a complete bundle.

/// Source of a complete route read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RouteReadPolicy {
    ExplicitStaticProvider,
    DynamicBaseAccess,
    Unavailable,
}

impl RouteReadPolicy {
    pub(crate) const fn is_available(self) -> bool {
        !matches!(self, Self::Unavailable)
    }

    pub(crate) const fn trace_label(self) -> &'static str {
        match self {
            Self::ExplicitStaticProvider | Self::DynamicBaseAccess => "available",
            Self::Unavailable => "unavailable",
        }
    }
}

/// Source of a whole-route replacement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RouteReplacePolicy {
    ExplicitStaticProvider,
    DynamicBaseAccess,
    Unavailable,
}

impl RouteReplacePolicy {
    pub(crate) const fn is_available(self) -> bool {
        !matches!(self, Self::Unavailable)
    }

    pub(crate) const fn trace_label(self) -> &'static str {
        match self {
            Self::ExplicitStaticProvider | Self::DynamicBaseAccess => "available",
            Self::Unavailable => "unavailable",
        }
    }
}

/// Source of route feasibility for K-opt and route neighborhoods.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RouteFeasibilityPolicy {
    ExplicitStaticProvider,
    /// Typed public default: a typed K-opt route with no feasibility hook is
    /// declared feasible. The binding supplies a non-null callable once.
    DeclaredAlwaysFeasible,
    ExplicitDynamicProvider,
    Unavailable,
}

impl RouteFeasibilityPolicy {
    pub(crate) const fn is_available(self) -> bool {
        !matches!(self, Self::Unavailable)
    }

    pub(crate) const fn is_dynamic_explicit(self) -> bool {
        matches!(self, Self::ExplicitDynamicProvider)
    }

    pub(crate) const fn trace_label(self) -> &'static str {
        match self {
            Self::ExplicitStaticProvider | Self::ExplicitDynamicProvider => "explicit",
            Self::DeclaredAlwaysFeasible => "declared_always_feasible",
            Self::Unavailable => "unavailable",
        }
    }
}

/// Source of the metric-class value consumed by Clarke-Wright.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SavingsMetricClassPolicy {
    ExplicitStaticProvider,
    /// Typed public default: the owner/entity index is the metric class.
    /// This exactly matches `ListClarkeWrightPhase`'s historic
    /// `unique_metric_class` policy.
    DeclaredEntityIdentity,
    ExplicitDynamicProvider,
    Unavailable,
}

impl SavingsMetricClassPolicy {
    pub(crate) const fn is_available(self) -> bool {
        !matches!(self, Self::Unavailable)
    }

    pub(crate) const fn is_dynamic_explicit(self) -> bool {
        matches!(self, Self::ExplicitDynamicProvider)
    }

    pub(crate) const fn trace_label(self) -> &'static str {
        match self {
            Self::ExplicitStaticProvider | Self::ExplicitDynamicProvider => "explicit",
            Self::DeclaredEntityIdentity => "declared_entity_identity",
            Self::Unavailable => "unavailable",
        }
    }
}

/// Source of an optional per-element owner restriction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OwnershipPolicy {
    ExplicitStaticProvider,
    /// No owner hook means every entity remains eligible, exactly as the
    /// typed and dynamic public list contracts specify.
    DeclaredUnrestricted,
    ExplicitDynamicProvider,
}

impl OwnershipPolicy {
    pub(crate) const fn is_explicit(self) -> bool {
        matches!(
            self,
            Self::ExplicitStaticProvider | Self::ExplicitDynamicProvider
        )
    }

    pub(crate) const fn trace_label(self) -> &'static str {
        match self {
            Self::ExplicitStaticProvider | Self::ExplicitDynamicProvider => "explicit",
            Self::DeclaredUnrestricted => "declared_unrestricted",
        }
    }
}

/// Source of construction ordering for list elements.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ConstructionOrderPolicy {
    ExplicitStaticProvider,
    /// No order hook preserves the declaration/source element sequence.
    DeclaredNaturalElementOrder,
    ExplicitDynamicProvider,
}

impl ConstructionOrderPolicy {
    pub(crate) const fn is_explicit(self) -> bool {
        matches!(
            self,
            Self::ExplicitStaticProvider | Self::ExplicitDynamicProvider
        )
    }

    pub(crate) const fn trace_label(self) -> &'static str {
        match self {
            Self::ExplicitStaticProvider | Self::ExplicitDynamicProvider => "explicit",
            Self::DeclaredNaturalElementOrder => "declared_natural_element_order",
        }
    }
}

/// Frozen precedence shape for a list slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PrecedencePolicy {
    Absent,
    /// List ruin consumes successors without a duration source.
    SuccessorsOnly,
    /// Duration and successors are both available for scored construction and
    /// the precedence selector.
    Explicit,
}

impl PrecedencePolicy {
    pub(crate) const fn has_successors(self) -> bool {
        matches!(self, Self::SuccessorsOnly | Self::Explicit)
    }

    pub(crate) const fn is_explicit(self) -> bool {
        matches!(self, Self::Explicit)
    }

    pub(crate) const fn trace_label(self) -> &'static str {
        match self {
            Self::Absent => "absent",
            Self::SuccessorsOnly => "successors_only",
            Self::Explicit => "explicit",
        }
    }
}
