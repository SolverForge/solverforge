use super::*;

/// A resolved solver phase plan used as candidate-trace provenance.
///
/// `opaque` is explicit rather than silently falling back to a type name.
/// Consumers must reject an incomplete plan when proving work equivalence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateTracePhasePlan {
    pub kind: String,
    /// Structured, sorted effective settings for this resolved node.  Keys
    /// are unique, which makes the canonical framing unambiguous and prevents
    /// a display-only label from hiding material selector/acceptor/forager or
    /// termination differences.
    pub attributes: Vec<CandidateTracePhaseAttribute>,
    pub opaque: bool,
    pub children: Vec<Self>,
}

/// One canonical resolved-plan attribute.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CandidateTracePhaseAttribute {
    pub key: String,
    pub value: String,
}

impl CandidateTracePhasePlan {
    pub fn known<K, V, I>(kind: impl Into<String>, attributes: I, children: Vec<Self>) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        let mut attributes = attributes
            .into_iter()
            .map(|(key, value)| CandidateTracePhaseAttribute {
                key: key.into(),
                value: value.into(),
            })
            .collect::<Vec<_>>();
        attributes.sort();
        assert!(
            attributes.windows(2).all(|pair| pair[0].key != pair[1].key),
            "candidate trace phase-plan attributes must have unique keys"
        );
        Self {
            kind: kind.into(),
            attributes,
            opaque: false,
            children,
        }
    }

    pub fn opaque(kind: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            attributes: Vec::new(),
            opaque: true,
            children: Vec::new(),
        }
    }

    pub fn is_complete(&self) -> bool {
        !self.opaque && self.children.iter().all(Self::is_complete)
    }

    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        self.append_canonical_bytes(&mut out);
        out
    }

    pub(super) fn append_canonical_bytes(&self, out: &mut Vec<u8>) {
        out.push(0x50);
        append_string(out, &self.kind);
        append_bool(out, self.opaque);
        append_len(out, self.attributes.len());
        for attribute in &self.attributes {
            append_string(out, &attribute.key);
            append_string(out, &attribute.value);
        }
        append_len(out, self.children.len());
        for child in &self.children {
            child.append_canonical_bytes(out);
        }
    }
}

/// The execution policy the solver actually installed for this run.
///
/// Configuration input is useful provenance, but it is not itself the
/// execution policy: the configured entrypoint can inject a fallback time
/// limit, discard an invalid score target, or compose a criterion with a
/// time guard.  This value records that resolved result explicitly.  A
/// generic [`crate::solver::Solver`] whose termination is supplied as an
/// arbitrary Rust value is intentionally opaque unless its caller supplies a
/// canonical policy through the internal configured-run seam.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateTraceExecutionPolicy {
    pub kind: String,
    /// Sorted, unique material policy attributes.  These identify the
    /// installed termination composition rather than a display-only summary.
    pub attributes: Vec<CandidateTracePhaseAttribute>,
    pub opaque: bool,
}

impl CandidateTraceExecutionPolicy {
    pub fn known<K, V, I>(kind: impl Into<String>, attributes: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        let mut attributes = attributes
            .into_iter()
            .map(|(key, value)| CandidateTracePhaseAttribute {
                key: key.into(),
                value: value.into(),
            })
            .collect::<Vec<_>>();
        attributes.sort();
        assert!(
            attributes.windows(2).all(|pair| pair[0].key != pair[1].key),
            "candidate trace execution-policy attributes must have unique keys"
        );
        Self {
            kind: kind.into(),
            attributes,
            opaque: false,
        }
    }

    pub fn opaque(kind: impl Into<String>) -> Self {
        Self::opaque_with_attributes(kind, std::iter::empty::<(String, String)>())
    }

    /// Records observable policy fields while explicitly declining to claim
    /// complete policy provenance.  This is used by the generic `Solver`
    /// entrypoint: `with_time_limit` is observable and must be retained, but
    /// the caller's arbitrary Rust termination remains uninspectable.
    pub fn opaque_with_attributes<K, V, I>(kind: impl Into<String>, attributes: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        let mut attributes = attributes
            .into_iter()
            .map(|(key, value)| CandidateTracePhaseAttribute {
                key: key.into(),
                value: value.into(),
            })
            .collect::<Vec<_>>();
        attributes.sort();
        assert!(
            attributes.windows(2).all(|pair| pair[0].key != pair[1].key),
            "candidate trace execution-policy attributes must have unique keys"
        );
        Self {
            kind: kind.into(),
            attributes,
            opaque: true,
        }
    }

    pub const fn is_complete(&self) -> bool {
        !self.opaque
    }

    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.push(0x58);
        append_string(&mut out, &self.kind);
        append_bool(&mut out, self.opaque);
        append_len(&mut out, self.attributes.len());
        for attribute in &self.attributes {
            append_string(&mut out, &attribute.key);
            append_string(&mut out, &attribute.value);
        }
        out
    }
}
