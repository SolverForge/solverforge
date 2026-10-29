pub(crate) mod assignment;
pub(crate) mod candidate;
pub(crate) mod group;
pub(crate) mod target;

pub(crate) use assignment::ScalarAssignmentDeclaration;
pub(crate) use assignment::ScalarAssignmentRule;
pub(crate) use candidate::{ScalarCandidate, ScalarCandidateProvider, ScalarEdit};
pub(crate) use group::ScalarGroupKind;
pub(crate) use group::{ScalarGroup, ScalarGroupLimits};
pub(crate) use target::ScalarTarget;
