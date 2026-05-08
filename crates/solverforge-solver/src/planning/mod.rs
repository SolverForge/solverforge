mod conflict_repair;
mod scalar;

pub use conflict_repair::{ConflictRepair, RepairCandidate, RepairLimits, RepairProvider};
pub(crate) use scalar::{ScalarAssignmentDeclaration, ScalarGroupKind};
pub use scalar::{
    ScalarCandidate, ScalarCandidateProvider, ScalarEdit, ScalarGroup, ScalarGroupLimits,
    ScalarTarget,
};
