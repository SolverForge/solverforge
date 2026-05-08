use crate::stream::CollectionExtract;

pub use solverforge_solver::{
    ConflictRepair, RepairCandidate, RepairLimits, RepairProvider, ScalarCandidate,
    ScalarCandidateProvider, ScalarEdit, ScalarGroup, ScalarGroupLimits, ScalarTarget,
};

pub trait EntitySourceTargetExt<S>: CollectionExtract<S> {
    fn scalar(&self, variable_name: &'static str) -> ScalarTarget<S> {
        match self.change_source() {
            crate::__internal::ChangeSource::Descriptor(descriptor_index) => {
                ScalarTarget::from_descriptor_index(descriptor_index, variable_name)
            }
            crate::__internal::ChangeSource::Static => {
                panic!("scalar target `{variable_name}` was requested from a static source")
            }
            crate::__internal::ChangeSource::Unknown => {
                panic!(
                    "scalar target `{variable_name}` requires a model-owned planning entity source"
                )
            }
        }
    }
}

impl<S, E> EntitySourceTargetExt<S> for E where E: CollectionExtract<S> + ?Sized {}
