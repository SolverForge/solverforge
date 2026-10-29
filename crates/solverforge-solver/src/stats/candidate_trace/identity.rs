use super::*;

/// Opaque handle for one retained pull. It exists only after the recorder
/// accepted capacity, so trace-off and overflow paths cannot allocate or
/// update per-candidate diagnostic state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct CandidateTracePullToken {
    pub(super) ordinal: u64,
}

/// A descriptor/entity location that a construction candidate was generated
/// for.  It is absent for local-search candidates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CandidateTraceConstructionTarget {
    pub descriptor_index: usize,
    pub entity_index: usize,
}

/// Canonical, owned identity for one traced candidate.
///
/// This is deliberately structured rather than debug-formatted or pointer
/// based. The grammar supports a leaf operation with its own logical scope
/// and an ordered composite of independently scoped child identities. That
/// means grouped and cartesian moves do not have to flatten cross-descriptor
/// edits into a misleading parent scope. The framing preserves every field
/// and every list boundary, so a trace consumer can compare exact candidate
/// ordering without relying on a process-local hash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CandidateTraceIdentity {
    /// One logical operation scoped to a descriptor and, when applicable, a
    /// planning variable.
    Operation(CandidateTraceOperationIdentity),
    /// An ordered composition of logical child operations. The composite has
    /// no implicit descriptor/variable scope; every child carries its own.
    Composite(CandidateTraceCompositeIdentity),
}

/// Canonical identity for an explicit non-cursor operation performed by a
/// specialized list construction/search engine.
///
/// The operation name is framed verbatim (never hashed), and components are
/// ordered raw coordinates chosen by that engine's actual trial loop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateTraceOperationIdentity {
    pub descriptor_index: usize,
    /// Present for a move-family identity and absent for a phase-local
    /// specialized operation that has no variable scope.
    pub variable_name: Option<String>,
    /// Explicit family/operation grammar token, never a pointer, debug value,
    /// or tabu-derived hash.
    pub operation: String,
    pub components: Vec<CandidateTraceCoordinate>,
}

/// Canonical identity for a compound/cartesian move.
///
/// `operation` describes composition semantics (for example
/// `compound_scalar` or `cartesian_product`) while `children` preserves the
/// exact generation/execution order. Children may themselves be composites.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateTraceCompositeIdentity {
    pub operation: String,
    pub children: Vec<CandidateTraceIdentity>,
}

/// One explicitly framed logical-coordinate component.
///
/// `Absent` is distinct from every integer value, so an unassigned scalar
/// target never needs a magic sentinel. `Bytes` is reserved for a declared
/// pure typed-value codec; it is never populated from `Debug` output.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CandidateTraceCoordinate {
    Unsigned(u64),
    Absent,
    /// A declared logical token such as a variable name or operation-local
    /// label. It is framed as text, never debug-rendered or hashed.
    Text(String),
    /// A declared pure typed-value codec output. This is intentionally
    /// distinct from [`Self::Text`] and must never be populated from debug or
    /// tabu metadata.
    Bytes(Vec<u8>),
}

impl From<u64> for CandidateTraceCoordinate {
    fn from(value: u64) -> Self {
        Self::Unsigned(value)
    }
}

impl From<usize> for CandidateTraceCoordinate {
    fn from(value: usize) -> Self {
        Self::Unsigned(value as u64)
    }
}

impl From<Option<usize>> for CandidateTraceCoordinate {
    fn from(value: Option<usize>) -> Self {
        value.map_or(Self::Absent, Self::from)
    }
}

impl From<String> for CandidateTraceCoordinate {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}

impl From<&str> for CandidateTraceCoordinate {
    fn from(value: &str) -> Self {
        Self::Text(value.to_owned())
    }
}

impl CandidateTraceIdentity {
    pub fn operation<I, T>(
        descriptor_index: usize,
        operation: impl Into<String>,
        components: I,
    ) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<CandidateTraceCoordinate>,
    {
        Self::Operation(CandidateTraceOperationIdentity {
            descriptor_index,
            variable_name: None,
            operation: operation.into(),
            components: components.into_iter().map(Into::into).collect(),
        })
    }

    pub fn logical_move<I, T>(
        descriptor_index: usize,
        variable_name: impl Into<String>,
        family: impl Into<String>,
        coordinates: I,
    ) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<CandidateTraceCoordinate>,
    {
        Self::Operation(CandidateTraceOperationIdentity {
            descriptor_index,
            variable_name: Some(variable_name.into()),
            operation: family.into(),
            components: coordinates.into_iter().map(Into::into).collect(),
        })
    }

    /// Builds an ordered composite identity without inventing a shared scope
    /// for child operations that may target different descriptors/variables.
    pub fn composite(
        operation: impl Into<String>,
        children: impl IntoIterator<Item = CandidateTraceIdentity>,
    ) -> Self {
        Self::Composite(CandidateTraceCompositeIdentity {
            operation: operation.into(),
            children: children.into_iter().collect(),
        })
    }

    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        self.append_canonical_bytes(&mut out);
        out
    }

    pub(super) fn append_canonical_bytes(&self, out: &mut Vec<u8>) {
        match self {
            Self::Operation(identity) => {
                out.push(0x4f);
                append_usize(out, identity.descriptor_index);
                match &identity.variable_name {
                    Some(variable_name) => {
                        append_bool(out, true);
                        append_string(out, variable_name);
                    }
                    None => append_bool(out, false),
                }
                append_string(out, &identity.operation);
                append_coordinate_list(out, &identity.components);
            }
            Self::Composite(identity) => {
                out.push(0x43);
                append_string(out, &identity.operation);
                append_len(out, identity.children.len());
                for child in &identity.children {
                    child.append_canonical_bytes(out);
                }
            }
        }
    }
}
