use solverforge_core::score::Score;

use super::collection_extract::CollectionExtract;
use super::filter::{AndUniFilter, FnUniFilter, UniFilter};
use super::UniConstraintStream;

/// An entity that knows whether its planning variable is unassigned, which
/// enables [`UniConstraintStream::unassigned`] on streams over it.
///
/// `#[planning_entity]` implements it for an entity with exactly one
/// `Option<_>` planning variable: the entity is unassigned while that field is
/// `None`. Implement it by hand for an entity whose "unassigned" state is
/// something else, such as several optional variables or a sentinel value.
///
/// # Examples
///
/// ```
/// use solverforge_core::score::SoftScore;
/// use solverforge_scoring::api::constraint_set::IncrementalConstraint;
/// use solverforge_scoring::stream::{
///     source, ChangeSource, ConstraintFactory, UnassignedEntity,
/// };
///
/// /// A shift whose `employee` is `usize::MAX` until it is assigned.
/// #[derive(Clone, Debug)]
/// struct Shift {
///     employee: usize,
/// }
///
/// #[derive(Clone)]
/// struct Roster {
///     shifts: Vec<Shift>,
/// }
///
/// impl UnassignedEntity<Roster> for Shift {
///     fn is_unassigned(_roster: &Roster, shift: &Self) -> bool {
///         shift.employee == usize::MAX
///     }
/// }
///
/// fn shifts(roster: &Roster) -> &[Shift] {
///     roster.shifts.as_slice()
/// }
///
/// let constraint = ConstraintFactory::<Roster, SoftScore>::new()
///     .for_each(source(shifts as fn(&Roster) -> &[Shift], ChangeSource::Descriptor(0)))
///     .unassigned()
///     .penalize(SoftScore::of(1))
///     .named("unassigned shift");
///
/// let roster = Roster {
///     shifts: vec![Shift { employee: 0 }, Shift { employee: usize::MAX }],
/// };
/// assert_eq!(constraint.evaluate(&roster), SoftScore::of(-1));
/// ```
pub trait UnassignedEntity<S>: Clone + Send + Sync + 'static {
    /// Whether `entity` is unassigned in `solution`.
    fn is_unassigned(solution: &S, entity: &Self) -> bool;
}

impl<S, A, E, F, Sc> UniConstraintStream<S, A, E, F, Sc>
where
    S: Send + Sync + 'static,
    A: UnassignedEntity<S>,
    E: CollectionExtract<S, Item = A>,
    F: UniFilter<S, A>,
    Sc: Score + 'static,
{
    /// Keeps only entities whose planning variable is unassigned, as reported by
    /// their [`UnassignedEntity`] implementation.
    pub fn unassigned(
        self,
    ) -> UniConstraintStream<S, A, E, AndUniFilter<F, FnUniFilter<fn(&S, &A) -> bool>>, Sc> {
        let (extractor, filter) = self.into_parts();
        UniConstraintStream::from_parts(
            extractor,
            AndUniFilter::new(
                filter,
                FnUniFilter::new(A::is_unassigned as fn(&S, &A) -> bool),
            ),
        )
    }
}
