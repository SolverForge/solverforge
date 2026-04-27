use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;

use super::collection_extract::{ChangeSource, CollectionExtract};
use super::collector::UniCollector;
use super::filter::{AndUniFilter, FnUniFilter, TrueFilter, UniFilter};
use super::uni_stream::UniConstraintStream;

#[doc(hidden)]
pub trait ProjectedSource<S, Out>: Send + Sync {
    fn source_count(&self) -> usize;
    fn change_source(&self, slot: usize) -> ChangeSource;
    fn collect_all<V>(&self, solution: &S, visit: V)
    where
        V: FnMut(usize, usize, Out);
    fn collect_entity<V>(&self, solution: &S, slot: usize, entity_index: usize, visit: V)
    where
        V: FnMut(Out);
}

pub struct SingleProjectedSource<S, A, E, F, P, Out> {
    extractor: E,
    filter: F,
    projection: P,
    _phantom: PhantomData<(fn() -> S, fn() -> A, fn() -> Out)>,
}

impl<S, A, E, F, P, Out> SingleProjectedSource<S, A, E, F, P, Out> {
    pub(crate) fn new(extractor: E, filter: F, projection: P) -> Self {
        Self {
            extractor,
            filter,
            projection,
            _phantom: PhantomData,
        }
    }
}

impl<S, A, E, F, P, Out> ProjectedSource<S, Out> for SingleProjectedSource<S, A, E, F, P, Out>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    E: CollectionExtract<S, Item = A>,
    F: UniFilter<S, A>,
    P: Fn(&A) -> Vec<Out> + Send + Sync,
    Out: Clone + Send + Sync + 'static,
{
    fn source_count(&self) -> usize {
        1
    }

    fn change_source(&self, slot: usize) -> ChangeSource {
        if slot == 0 {
            self.extractor.change_source()
        } else {
            ChangeSource::Static
        }
    }

    fn collect_all<V>(&self, solution: &S, mut visit: V)
    where
        V: FnMut(usize, usize, Out),
    {
        for (idx, entity) in self.extractor.extract(solution).iter().enumerate() {
            if !self.filter.test(solution, entity) {
                continue;
            }
            for output in (self.projection)(entity) {
                visit(0, idx, output);
            }
        }
    }

    fn collect_entity<V>(&self, solution: &S, slot: usize, entity_index: usize, mut visit: V)
    where
        V: FnMut(Out),
    {
        if slot != 0 {
            return;
        }
        let entities = self.extractor.extract(solution);
        let Some(entity) = entities.get(entity_index) else {
            return;
        };
        if !self.filter.test(solution, entity) {
            return;
        }
        for output in (self.projection)(entity) {
            visit(output);
        }
    }
}

pub struct FilteredProjectedSource<S, Out, Src, F> {
    source: Src,
    filter: F,
    _phantom: PhantomData<(fn() -> S, fn() -> Out)>,
}

impl<S, Out, Src, F> FilteredProjectedSource<S, Out, Src, F> {
    fn new(source: Src, filter: F) -> Self {
        Self {
            source,
            filter,
            _phantom: PhantomData,
        }
    }
}

impl<S, Out, Src, F> ProjectedSource<S, Out> for FilteredProjectedSource<S, Out, Src, F>
where
    S: Send + Sync + 'static,
    Out: Clone + Send + Sync + 'static,
    Src: ProjectedSource<S, Out>,
    F: UniFilter<S, Out>,
{
    fn source_count(&self) -> usize {
        self.source.source_count()
    }

    fn change_source(&self, slot: usize) -> ChangeSource {
        self.source.change_source(slot)
    }

    fn collect_all<V>(&self, solution: &S, mut visit: V)
    where
        V: FnMut(usize, usize, Out),
    {
        self.source.collect_all(solution, |slot, idx, output| {
            if self.filter.test(solution, &output) {
                visit(slot, idx, output);
            }
        });
    }

    fn collect_entity<V>(&self, solution: &S, slot: usize, entity_index: usize, mut visit: V)
    where
        V: FnMut(Out),
    {
        self.source
            .collect_entity(solution, slot, entity_index, |output| {
                if self.filter.test(solution, &output) {
                    visit(output);
                }
            });
    }
}

pub struct MergedProjectedSource<Left, Right> {
    left: Left,
    right: Right,
}

impl<Left, Right> MergedProjectedSource<Left, Right> {
    fn new(left: Left, right: Right) -> Self {
        Self { left, right }
    }
}

impl<S, Out, Left, Right> ProjectedSource<S, Out> for MergedProjectedSource<Left, Right>
where
    S: Send + Sync + 'static,
    Out: Clone + Send + Sync + 'static,
    Left: ProjectedSource<S, Out>,
    Right: ProjectedSource<S, Out>,
{
    fn source_count(&self) -> usize {
        self.left.source_count() + self.right.source_count()
    }

    fn change_source(&self, slot: usize) -> ChangeSource {
        let left_count = self.left.source_count();
        if slot < left_count {
            self.left.change_source(slot)
        } else {
            self.right.change_source(slot - left_count)
        }
    }

    fn collect_all<V>(&self, solution: &S, mut visit: V)
    where
        V: FnMut(usize, usize, Out),
    {
        self.left.collect_all(solution, &mut visit);
        let left_count = self.left.source_count();
        self.right.collect_all(solution, |slot, idx, output| {
            visit(left_count + slot, idx, output);
        });
    }

    fn collect_entity<V>(&self, solution: &S, slot: usize, entity_index: usize, visit: V)
    where
        V: FnMut(Out),
    {
        let left_count = self.left.source_count();
        if slot < left_count {
            self.left
                .collect_entity(solution, slot, entity_index, visit);
        } else {
            self.right
                .collect_entity(solution, slot - left_count, entity_index, visit);
        }
    }
}

pub struct ProjectedConstraintStream<S, Out, Src, F, Sc>
where
    Sc: Score,
{
    pub(crate) source: Src,
    pub(crate) filter: F,
    pub(crate) _phantom: PhantomData<(fn() -> S, fn() -> Out, fn() -> Sc)>,
}

impl<S, Out, Src, F, Sc> ProjectedConstraintStream<S, Out, Src, F, Sc>
where
    S: Send + Sync + 'static,
    Out: Clone + Send + Sync + 'static,
    Src: ProjectedSource<S, Out>,
    F: UniFilter<S, Out>,
    Sc: Score + 'static,
{
    pub(crate) fn new(source: Src) -> ProjectedConstraintStream<S, Out, Src, TrueFilter, Sc> {
        ProjectedConstraintStream {
            source,
            filter: TrueFilter,
            _phantom: PhantomData,
        }
    }

    pub fn filter<P>(
        self,
        predicate: P,
    ) -> ProjectedConstraintStream<
        S,
        Out,
        Src,
        AndUniFilter<F, FnUniFilter<impl Fn(&S, &Out) -> bool + Send + Sync>>,
        Sc,
    >
    where
        P: Fn(&Out) -> bool + Send + Sync + 'static,
    {
        ProjectedConstraintStream {
            source: self.source,
            filter: AndUniFilter::new(
                self.filter,
                FnUniFilter::new(move |_s: &S, output: &Out| predicate(output)),
            ),
            _phantom: PhantomData,
        }
    }

    pub fn merge<OtherSrc, OtherF>(
        self,
        other: ProjectedConstraintStream<S, Out, OtherSrc, OtherF, Sc>,
    ) -> ProjectedConstraintStream<
        S,
        Out,
        MergedProjectedSource<
            FilteredProjectedSource<S, Out, Src, F>,
            FilteredProjectedSource<S, Out, OtherSrc, OtherF>,
        >,
        TrueFilter,
        Sc,
    >
    where
        OtherSrc: ProjectedSource<S, Out>,
        OtherF: UniFilter<S, Out>,
    {
        let left = FilteredProjectedSource::new(self.source, self.filter);
        let right = FilteredProjectedSource::new(other.source, other.filter);
        ProjectedConstraintStream {
            source: MergedProjectedSource::new(left, right),
            filter: TrueFilter,
            _phantom: PhantomData,
        }
    }

    pub fn group_by<K, KF, C>(
        self,
        key_fn: KF,
        collector: C,
    ) -> ProjectedGroupedConstraintStream<S, Out, K, Src, F, KF, C, Sc>
    where
        K: Clone + Eq + Hash + Send + Sync + 'static,
        KF: Fn(&Out) -> K + Send + Sync,
        C: UniCollector<Out> + Send + Sync + 'static,
        C::Accumulator: Send + Sync,
        C::Value: Clone + Send + Sync,
        C::Result: Clone + Send + Sync,
    {
        ProjectedGroupedConstraintStream {
            source: self.source,
            filter: self.filter,
            key_fn,
            collector,
            _phantom: PhantomData,
        }
    }

    fn into_weighted_builder<W>(
        self,
        impact_type: solverforge_core::ImpactType,
        weight: W,
        is_hard: bool,
    ) -> ProjectedConstraintBuilder<S, Out, Src, F, W, Sc>
    where
        W: Fn(&Out) -> Sc + Send + Sync,
    {
        ProjectedConstraintBuilder {
            source: self.source,
            filter: self.filter,
            impact_type,
            weight,
            is_hard,
            _phantom: PhantomData,
        }
    }

    pub fn penalize_hard_with<W>(
        self,
        weight: W,
    ) -> ProjectedConstraintBuilder<S, Out, Src, F, W, Sc>
    where
        W: Fn(&Out) -> Sc + Send + Sync,
    {
        self.into_weighted_builder(solverforge_core::ImpactType::Penalty, weight, true)
    }

    pub fn penalize_with<W>(self, weight: W) -> ProjectedConstraintBuilder<S, Out, Src, F, W, Sc>
    where
        W: Fn(&Out) -> Sc + Send + Sync,
    {
        self.into_weighted_builder(solverforge_core::ImpactType::Penalty, weight, false)
    }
}

pub struct ProjectedConstraintBuilder<S, Out, Src, F, W, Sc>
where
    Sc: Score,
{
    source: Src,
    filter: F,
    impact_type: solverforge_core::ImpactType,
    weight: W,
    is_hard: bool,
    _phantom: PhantomData<(fn() -> S, fn() -> Out, fn() -> Sc)>,
}

impl<S, Out, Src, F, W, Sc> ProjectedConstraintBuilder<S, Out, Src, F, W, Sc>
where
    S: Send + Sync + 'static,
    Out: Clone + Send + Sync + 'static,
    Src: ProjectedSource<S, Out>,
    F: UniFilter<S, Out>,
    W: Fn(&Out) -> Sc + Send + Sync,
    Sc: Score + 'static,
{
    pub fn named(
        self,
        name: &str,
    ) -> crate::constraint::projected::ProjectedUniConstraint<S, Out, Src, F, W, Sc> {
        crate::constraint::projected::ProjectedUniConstraint::new(
            solverforge_core::ConstraintRef::new("", name),
            self.impact_type,
            self.source,
            self.filter,
            self.weight,
            self.is_hard,
        )
    }
}

impl<S, A, E, F, Sc> UniConstraintStream<S, A, E, F, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    E: CollectionExtract<S, Item = A>,
    F: UniFilter<S, A>,
    Sc: Score + 'static,
{
    pub fn project<Out, P>(
        self,
        projection: P,
    ) -> ProjectedConstraintStream<S, Out, SingleProjectedSource<S, A, E, F, P, Out>, TrueFilter, Sc>
    where
        Out: Clone + Send + Sync + 'static,
        P: Fn(&A) -> Vec<Out> + Send + Sync + 'static,
    {
        let (extractor, filter) = self.into_parts();
        ProjectedConstraintStream::<
            S,
            Out,
            SingleProjectedSource<S, A, E, F, P, Out>,
            TrueFilter,
            Sc,
        >::new(SingleProjectedSource::new(extractor, filter, projection))
    }
}

pub struct ProjectedGroupedConstraintStream<S, Out, K, Src, F, KF, C, Sc>
where
    Sc: Score,
{
    pub(crate) source: Src,
    pub(crate) filter: F,
    pub(crate) key_fn: KF,
    pub(crate) collector: C,
    pub(crate) _phantom: PhantomData<(fn() -> S, fn() -> Out, fn() -> K, fn() -> Sc)>,
}

impl<S, Out, K, Src, F, KF, C, Sc> ProjectedGroupedConstraintStream<S, Out, K, Src, F, KF, C, Sc>
where
    S: Send + Sync + 'static,
    Out: Clone + Send + Sync + 'static,
    K: Clone + Eq + Hash + Send + Sync + 'static,
    Src: ProjectedSource<S, Out>,
    F: UniFilter<S, Out>,
    KF: Fn(&Out) -> K + Send + Sync,
    C: UniCollector<Out> + Send + Sync + 'static,
    C::Accumulator: Send + Sync,
    C::Value: Clone + Send + Sync,
    C::Result: Clone + Send + Sync,
    Sc: Score + 'static,
{
    fn into_weighted_builder<W>(
        self,
        impact_type: solverforge_core::ImpactType,
        weight_fn: W,
        is_hard: bool,
    ) -> super::projected_stream::ProjectedGroupedConstraintBuilder<S, Out, K, Src, F, KF, C, W, Sc>
    where
        W: Fn(&C::Result) -> Sc + Send + Sync,
    {
        ProjectedGroupedConstraintBuilder {
            source: self.source,
            filter: self.filter,
            key_fn: self.key_fn,
            collector: self.collector,
            impact_type,
            weight_fn,
            is_hard,
            _phantom: PhantomData,
        }
    }

    pub fn penalize_hard_with<W>(
        self,
        weight_fn: W,
    ) -> ProjectedGroupedConstraintBuilder<S, Out, K, Src, F, KF, C, W, Sc>
    where
        W: Fn(&C::Result) -> Sc + Send + Sync,
    {
        self.into_weighted_builder(solverforge_core::ImpactType::Penalty, weight_fn, true)
    }

    pub fn penalize_with<W>(
        self,
        weight_fn: W,
    ) -> ProjectedGroupedConstraintBuilder<S, Out, K, Src, F, KF, C, W, Sc>
    where
        W: Fn(&C::Result) -> Sc + Send + Sync,
    {
        self.into_weighted_builder(solverforge_core::ImpactType::Penalty, weight_fn, false)
    }
}

pub struct ProjectedGroupedConstraintBuilder<S, Out, K, Src, F, KF, C, W, Sc>
where
    Sc: Score,
{
    pub(crate) source: Src,
    pub(crate) filter: F,
    pub(crate) key_fn: KF,
    pub(crate) collector: C,
    pub(crate) impact_type: solverforge_core::ImpactType,
    pub(crate) weight_fn: W,
    pub(crate) is_hard: bool,
    pub(crate) _phantom: PhantomData<(fn() -> S, fn() -> Out, fn() -> K, fn() -> Sc)>,
}

impl<S, Out, K, Src, F, KF, C, W, Sc>
    ProjectedGroupedConstraintBuilder<S, Out, K, Src, F, KF, C, W, Sc>
where
    S: Send + Sync + 'static,
    Out: Clone + Send + Sync + 'static,
    K: Clone + Eq + Hash + Send + Sync + 'static,
    Src: ProjectedSource<S, Out>,
    F: UniFilter<S, Out>,
    KF: Fn(&Out) -> K + Send + Sync,
    C: UniCollector<Out> + Send + Sync + 'static,
    C::Accumulator: Send + Sync,
    C::Value: Clone + Send + Sync,
    C::Result: Clone + Send + Sync,
    W: Fn(&C::Result) -> Sc + Send + Sync,
    Sc: Score + 'static,
{
    pub fn named(
        self,
        name: &str,
    ) -> crate::constraint::projected::ProjectedGroupedConstraint<S, Out, K, Src, F, KF, C, W, Sc>
    {
        crate::constraint::projected::ProjectedGroupedConstraint::new(
            solverforge_core::ConstraintRef::new("", name),
            self.impact_type,
            self.source,
            self.filter,
            self.key_fn,
            self.collector,
            self.weight_fn,
            self.is_hard,
        )
    }
}
