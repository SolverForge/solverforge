mod assignment;
mod candidate;
mod group;
mod target;

pub(crate) use assignment::ScalarAssignmentDeclaration;
pub use assignment::ScalarAssignmentRule;
pub use candidate::{ScalarCandidate, ScalarCandidateProvider, ScalarEdit};
pub(crate) use group::ScalarGroupKind;
pub use group::{ScalarGroup, ScalarGroupLimits};
pub use target::ScalarTarget;
