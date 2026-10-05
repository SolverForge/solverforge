use super::super::{BindingId, DenseRowStore, Leaf, RowHandle};
use super::Operator;
use crate::stream::collection_extract::CollectionExtract;
use std::marker::PhantomData;

pub struct CollectionNode<S, E> {
    extractor: E,
    binding: BindingId,
    rows: DenseRowStore<usize>,
    handles: Vec<Option<RowHandle>>,
    marker: PhantomData<fn() -> S>,
}

impl<S, E> CollectionNode<S, E> {
    pub fn new(extractor: E, binding: u32) -> Self {
        Self {
            extractor,
            binding: BindingId(binding),
            rows: DenseRowStore::new(),
            handles: Vec::new(),
            marker: PhantomData,
        }
    }
}

impl<S: 'static, E: CollectionExtract<S> + 'static> Operator<S> for CollectionNode<S, E>
where
    E::Item: 'static,
{
    type View<'a> = Leaf<'a, E::Item>;
    type Evaluation = ();
    #[inline]
    fn prepare_evaluation(&self, _: &S) {}
    #[inline]
    fn visit_evaluation<'a>(
        &'a self,
        solution: &'a S,
        _: &'a (),
        visitor: &mut impl FnMut(Self::View<'a>),
    ) {
        for (index, entity) in self.extractor.extract(solution).iter().enumerate() {
            if self.extractor.contains(solution, entity) {
                visitor(Leaf::new(entity, index));
            }
        }
    }
    fn clear(&mut self) {
        self.rows.clear();
        self.handles.clear();
    }
    fn initialize(&mut self, solution: &S) {
        self.rows.clear();
        self.handles.clear();
        for (index, entity) in self.extractor.extract(solution).iter().enumerate() {
            let handle = self
                .extractor
                .contains(solution, entity)
                .then(|| self.rows.insert(index));
            self.handles.push(handle);
        }
    }
    fn handles(&self) -> Vec<RowHandle> {
        self.rows.iter().map(|(h, _)| h).collect()
    }
    fn resolve<'a>(&'a self, solution: &'a S, handle: RowHandle) -> Option<Self::View<'a>> {
        let index = *self.rows.get(handle)?;
        Some(Leaf::new(
            self.extractor.extract(solution).get(index)?,
            index,
        ))
    }
    fn visit_provenance(&self, handle: RowHandle, visitor: &mut impl FnMut(u32, usize, usize)) {
        if let Some(index) = self.rows.get(handle) {
            let descriptor = match self.extractor.change_source() {
                crate::stream::collection_extract::ChangeSource::Descriptor(d) => d,
                _ => usize::MAX,
            };
            visitor(self.binding.0, descriptor, *index);
        }
    }
    fn retract(&mut self, _: &S, descriptor: usize, index: usize) -> Vec<RowHandle> {
        if !self
            .extractor
            .change_source()
            .assert_localizes(descriptor, "relational source")
        {
            return Vec::new();
        }
        if let Some(slot) = self.handles.get_mut(index) {
            if let Some(handle) = slot.take() {
                self.rows.retract(handle);
                return vec![handle];
            }
        }
        Vec::new()
    }
    fn insert(&mut self, solution: &S, descriptor: usize, index: usize) -> Vec<RowHandle> {
        if !self
            .extractor
            .change_source()
            .assert_localizes(descriptor, "relational source")
        {
            return Vec::new();
        }
        if self.handles.get(index).is_some_and(Option::is_some) {
            return Vec::new();
        }
        if let Some(entity) = self.extractor.extract(solution).get(index) {
            if self.extractor.contains(solution, entity) {
                let handle = self.rows.insert(index);
                self.handles.resize(self.handles.len().max(index + 1), None);
                self.handles[index] = Some(handle);
                return vec![handle];
            }
        }
        Vec::new()
    }
}
