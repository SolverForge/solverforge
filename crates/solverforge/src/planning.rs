use crate::stream::CollectionExtract;

pub use solverforge_solver::{
    ConflictRepair, RepairCandidate, RepairLimits, RepairProvider, ScalarAssignmentRule,
    ScalarCandidate, ScalarCandidateProvider, ScalarEdit, ScalarGroup, ScalarGroupLimits,
    ScalarTarget,
};

/// Builds a scalar-variable target from a model-owned entity source.
pub trait EntitySourceTargetExt<S>: CollectionExtract<S> {
    /// The scalar planning variable `variable_name` of this source's entity.
    ///
    /// # Panics
    /// When the source is static or not a model-owned planning entity source.
    fn scalar(&self, variable_name: &'static str) -> ScalarTarget<S> {
        match self.change_source() {
            crate::__internal::ChangeSource::Descriptor(descriptor_index) => {
                ScalarTarget::from_descriptor_index(descriptor_index, variable_name)
            }
            crate::__internal::ChangeSource::Static => {
                panic!("scalar target `{variable_name}` was requested from a static source")
            }
        }
    }
}

impl<S, E> EntitySourceTargetExt<S> for E where E: CollectionExtract<S> + ?Sized {}
