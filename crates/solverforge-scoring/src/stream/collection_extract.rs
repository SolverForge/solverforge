/* `CollectionExtract`: the source of a stream's rows.

A stream source is a planning-model collection: the accessor that
`#[planning_solution]` generates for each entity, fact and list-element
collection (`Plan::shifts()`). Each accessor reports which descriptor owns its
rows, so incremental scoring can map an entity notification to the rows it
changes. A raw closure or `fn` item carries no such identity and is not a
source.
*/

/* Extracts a slice of entities from the solution.

The associated type `Item` names the entity type, allowing callers to
write `E: CollectionExtract<S, Item = A>` when `A` must be inferred from `E`
rather than stated as a separate generic parameter.
*/
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a stream source",
    label = "stream sources must come from the planning model",
    note = "pass the generated accessor, e.g. `Plan::shifts()`"
)]
pub trait CollectionExtract<S>: Send + Sync {
    // The entity type yielded by this extractor.
    type Item;

    // Extracts the entity slice from the solution.
    fn extract<'s>(&self, s: &'s S) -> &'s [Self::Item];

    // Reports whether this extractor's own source-level predicate accepts an item.
    fn contains(&self, _s: &S, _item: &Self::Item) -> bool {
        true
    }

    // The descriptor that owns this source's rows, or `Static` when no
    // planning-variable change can alter them. Macro-generated solution source
    // methods attach it through hidden internal support.
    fn change_source(&self) -> ChangeSource;
}

/* Preserves a unary stream's membership at the source boundary, before
indexed joins build keys or materialize pairs. The original slice and index
mapping remain unchanged so descriptor-local mutations keep their identity. */
#[doc(hidden)]
pub struct FilteredExtract<E, F> {
    extractor: E,
    filter: F,
}

impl<E, F> FilteredExtract<E, F> {
    pub fn new(extractor: E, filter: F) -> Self {
        Self { extractor, filter }
    }
}

impl<S, E, F> CollectionExtract<S> for FilteredExtract<E, F>
where
    E: CollectionExtract<S>,
    F: super::filter::UniFilter<S, E::Item>,
{
    type Item = E::Item;

    fn extract<'s>(&self, s: &'s S) -> &'s [Self::Item] {
        self.extractor.extract(s)
    }

    fn contains(&self, s: &S, item: &Self::Item) -> bool {
        self.extractor.contains(s, item) && self.filter.test(s, item)
    }

    fn change_source(&self) -> ChangeSource {
        self.extractor.change_source()
    }
}

#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeSource {
    Static,
    Descriptor(usize),
}

impl ChangeSource {
    #[inline]
    pub fn owns_descriptor(self, descriptor_index: usize) -> bool {
        matches!(self, Self::Descriptor(index) if index == descriptor_index)
    }

    #[inline]
    pub fn same_index_domain(self, other: Self) -> bool {
        matches!((self, other), (Self::Descriptor(left), Self::Descriptor(right)) if left == right)
    }
}

pub trait FlattenExtract<P>: Send + Sync {
    type Item;

    fn extract<'s>(&self, parent: &'s P) -> &'s [Self::Item];
}

impl<P, B, F> FlattenExtract<P> for F
where
    F: for<'a> Fn(&'a P) -> &'a [B] + Send + Sync,
{
    type Item = B;

    #[inline]
    fn extract<'s>(&self, parent: &'s P) -> &'s [B] {
        self(parent)
    }
}

#[derive(Clone, Copy)]
pub struct FlattenVecExtract<F>(pub F);

impl<P, B, F> FlattenExtract<P> for FlattenVecExtract<F>
where
    F: for<'a> Fn(&'a P) -> &'a Vec<B> + Send + Sync,
{
    type Item = B;

    #[inline]
    fn extract<'s>(&self, parent: &'s P) -> &'s [B] {
        (self.0)(parent).as_slice()
    }
}

#[doc(hidden)]
#[derive(Clone, Copy)]
pub struct SourceExtract<E> {
    extractor: E,
    change_source: ChangeSource,
}

impl<E> SourceExtract<E> {
    pub fn new(extractor: E, change_source: ChangeSource) -> Self {
        Self {
            extractor,
            change_source,
        }
    }

    pub fn extractor(&self) -> &E {
        &self.extractor
    }
}

impl<S, A, E> CollectionExtract<S> for SourceExtract<E>
where
    E: for<'a> Fn(&'a S) -> &'a [A] + Send + Sync,
{
    type Item = A;

    #[inline]
    fn extract<'s>(&self, s: &'s S) -> &'s [A] {
        (self.extractor)(s)
    }

    fn change_source(&self) -> ChangeSource {
        self.change_source
    }
}

#[doc(hidden)]
pub fn source<E>(extractor: E, change_source: ChangeSource) -> SourceExtract<E> {
    SourceExtract::new(extractor, change_source)
}
