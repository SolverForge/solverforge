pub(crate) mod conflict_repair;
pub(crate) mod scalar;

pub(crate) use conflict_repair::ConflictRepair;
pub(crate) use scalar::{ScalarAssignmentDeclaration, ScalarGroupKind};
pub(crate) use scalar::{
    ScalarAssignmentRule, ScalarCandidate, ScalarCandidateProvider, ScalarEdit, ScalarGroup,
    ScalarGroupLimits,
};
