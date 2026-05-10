use std::marker::PhantomData;

use super::ScalarEdit;

#[derive(Debug)]
pub struct ScalarTarget<S> {
    descriptor_index: usize,
    variable_name: &'static str,
    _phantom: PhantomData<fn() -> S>,
}

impl<S> Clone for ScalarTarget<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S> Copy for ScalarTarget<S> {}

impl<S> PartialEq for ScalarTarget<S> {
    fn eq(&self, other: &Self) -> bool {
        self.descriptor_index == other.descriptor_index && self.variable_name == other.variable_name
    }
}

impl<S> Eq for ScalarTarget<S> {}

impl<S> std::hash::Hash for ScalarTarget<S> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.descriptor_index.hash(state);
        self.variable_name.hash(state);
    }
}

impl<S> ScalarTarget<S> {
    #[doc(hidden)]
    pub const fn from_descriptor_index(
        descriptor_index: usize,
        variable_name: &'static str,
    ) -> Self {
        Self {
            descriptor_index,
            variable_name,
            _phantom: PhantomData,
        }
    }

    #[inline]
    pub fn set(self, entity_index: usize, to_value: Option<usize>) -> ScalarEdit<S> {
        ScalarEdit::from_descriptor_index(
            self.descriptor_index,
            entity_index,
            self.variable_name,
            to_value,
        )
    }

    #[doc(hidden)]
    #[inline]
    pub fn descriptor_index(self) -> usize {
        self.descriptor_index
    }

    #[doc(hidden)]
    #[inline]
    pub fn variable_name(self) -> &'static str {
        self.variable_name
    }
}
