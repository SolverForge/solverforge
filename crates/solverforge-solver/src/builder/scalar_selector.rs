use std::fmt::{self, Debug};
use std::ops::Range;

use solverforge_config::{MoveSelectorConfig, RecreateHeuristicType};
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use solverforge_scoring::Director;

use crate::heuristic::r#move::{
    Move, MoveArena, PillarChangeMove, PillarSwapMove, RuinRecreateMove, ScalarMoveUnion,
    ScalarRecreateValueSource, SequentialCompositeMove,
};
use crate::heuristic::selector::decorator::{CartesianProductSelector, VecUnionSelector};
use crate::heuristic::selector::{
    nearby_support::truncate_nearby_candidates,
    pillar_support::{intersect_legal_values_for_pillar, pillars_are_swap_compatible, PillarGroup},
    seed::scoped_seed,
    ChangeMoveSelector, DefaultPillarSelector, FromSolutionEntitySelector, MoveSelector,
    PerEntitySliceValueSelector, PillarSelector, RangeValueSelector, RuinMoveSelector,
    SwapMoveSelector, ValueSelector,
};

use super::context::{ScalarVariableContext, ValueSource};

pub type ScalarFlatSelector<S> =
    VecUnionSelector<S, ScalarMoveUnion<S, usize>, ScalarLeafSelector<S>>;
#[cfg_attr(not(test), allow(dead_code))]
pub type ScalarSelector<S> = VecUnionSelector<S, ScalarMoveUnion<S, usize>, ScalarSelectorNode<S>>;
#[cfg_attr(not(test), allow(dead_code))]
type ScalarCartesianSelector<S> = CartesianProductSelector<
    S,
    ScalarMoveUnion<S, usize>,
    ScalarFlatSelector<S>,
    ScalarFlatSelector<S>,
>;

pub enum ScalarValueSelector<S> {
    Empty,
    CountableRange { from: usize, to: usize },
    SolutionCount(RangeValueSelector<S>),
    EntitySlice(PerEntitySliceValueSelector<S, usize>),
}

impl<S> ScalarValueSelector<S> {
    fn from_source(source: ValueSource<S>) -> Self {
        match source {
            ValueSource::Empty => Self::Empty,
            ValueSource::CountableRange { from, to } => Self::CountableRange { from, to },
            ValueSource::SolutionCount { count_fn } => {
                Self::SolutionCount(RangeValueSelector::new(count_fn))
            }
            ValueSource::EntitySlice { values_for_entity } => {
                Self::EntitySlice(PerEntitySliceValueSelector::new(values_for_entity))
            }
        }
    }
}

fn scalar_recreate_value_source<S>(source: ValueSource<S>) -> ScalarRecreateValueSource<S> {
    match source {
        ValueSource::Empty => ScalarRecreateValueSource::Empty,
        ValueSource::CountableRange { from, to } => {
            ScalarRecreateValueSource::CountableRange { from, to }
        }
        ValueSource::SolutionCount { count_fn } => {
            ScalarRecreateValueSource::SolutionCount { count_fn }
        }
        ValueSource::EntitySlice { values_for_entity } => {
            ScalarRecreateValueSource::EntitySlice { values_for_entity }
        }
    }
}

fn scalar_legal_values_for_entity<S, D: Director<S>>(
    value_selector: &ScalarValueSelector<S>,
    score_director: &D,
    descriptor_index: usize,
    entity_index: usize,
) -> Vec<usize>
where
    S: PlanningSolution,
{
    value_selector
        .iter(score_director, descriptor_index, entity_index)
        .collect()
}

impl<S> Debug for ScalarValueSelector<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "ScalarValueSelector::Empty"),
            Self::CountableRange { from, to } => {
                write!(f, "ScalarValueSelector::CountableRange({from}..{to})")
            }
            Self::SolutionCount(_) => write!(f, "ScalarValueSelector::SolutionCount(..)"),
            Self::EntitySlice(_) => write!(f, "ScalarValueSelector::EntitySlice(..)"),
        }
    }
}

impl<S> ValueSelector<S, usize> for ScalarValueSelector<S>
where
    S: PlanningSolution,
{
    fn iter<'a, D: Director<S>>(
        &'a self,
        score_director: &D,
        descriptor_index: usize,
        entity_index: usize,
    ) -> impl Iterator<Item = usize> + 'a {
        match self {
            Self::Empty => ScalarValueIter::Empty,
            Self::CountableRange { from, to } => ScalarValueIter::CountableRange(*from..*to),
            Self::SolutionCount(selector) => ScalarValueIter::SolutionCount(selector.iter(
                score_director,
                descriptor_index,
                entity_index,
            )),
            Self::EntitySlice(selector) => ScalarValueIter::EntitySlice(selector.iter(
                score_director,
                descriptor_index,
                entity_index,
            )),
        }
    }

    fn size<D: Director<S>>(
        &self,
        score_director: &D,
        descriptor_index: usize,
        entity_index: usize,
    ) -> usize {
        match self {
            Self::Empty => 0,
            Self::CountableRange { from, to } => to.saturating_sub(*from),
            Self::SolutionCount(selector) => {
                selector.size(score_director, descriptor_index, entity_index)
            }
            Self::EntitySlice(selector) => {
                selector.size(score_director, descriptor_index, entity_index)
            }
        }
    }
}

enum ScalarValueIter<A, B> {
    Empty,
    CountableRange(Range<usize>),
    SolutionCount(A),
    EntitySlice(B),
}

impl<A, B> Iterator for ScalarValueIter<A, B>
where
    A: Iterator<Item = usize>,
    B: Iterator<Item = usize>,
{
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Empty => None,
            Self::CountableRange(iter) => iter.next(),
            Self::SolutionCount(iter) => iter.next(),
            Self::EntitySlice(iter) => iter.next(),
        }
    }
}

type ScalarChangeSelector<S> =
    ChangeMoveSelector<S, usize, FromSolutionEntitySelector, ScalarValueSelector<S>>;
type ScalarSwapSelector<S> =
    SwapMoveSelector<S, usize, FromSolutionEntitySelector, FromSolutionEntitySelector>;

#[derive(Clone, Copy)]
pub struct NearbyChangeLeafSelector<S> {
    ctx: ScalarVariableContext<S>,
    max_nearby: usize,
}

impl<S> Debug for NearbyChangeLeafSelector<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NearbyChangeLeafSelector")
            .field("descriptor_index", &self.ctx.descriptor_index)
            .field("variable_name", &self.ctx.variable_name)
            .field("max_nearby", &self.max_nearby)
            .finish()
    }
}

impl<S> MoveSelector<S, ScalarMoveUnion<S, usize>> for NearbyChangeLeafSelector<S>
where
    S: PlanningSolution + 'static,
{
    fn open_cursor<'a, D: Director<S>>(
        &'a self,
        score_director: &D,
    ) -> impl Iterator<Item = ScalarMoveUnion<S, usize>> + 'a {
        let solution = score_director.working_solution();
        let distance_meter = self
            .ctx
            .nearby_value_distance_meter
            .expect("nearby change requires a nearby value distance meter");
        let value_selector = ScalarValueSelector::from_source(self.ctx.value_source);
        let mut moves = Vec::new();

        for entity_index in 0..(self.ctx.entity_count)(solution) {
            let current_value = (self.ctx.getter)(solution, entity_index);
            let current_assigned = current_value.is_some();
            let mut candidates: Vec<(usize, f64, usize)> = value_selector
                .iter(score_director, self.ctx.descriptor_index, entity_index)
                .enumerate()
                .filter_map(|(order, value)| {
                    if current_value == Some(value) {
                        return None;
                    }
                    let distance = distance_meter(solution, entity_index, value);
                    distance.is_finite().then_some((value, distance, order))
                })
                .collect();

            truncate_nearby_candidates(&mut candidates, self.max_nearby);

            moves.extend(candidates.into_iter().map(|(value, _, _)| {
                ScalarMoveUnion::Change(crate::heuristic::r#move::ChangeMove::new(
                    entity_index,
                    Some(value),
                    self.ctx.getter,
                    self.ctx.setter,
                    self.ctx.variable_name,
                    self.ctx.descriptor_index,
                ))
            }));

            if self.ctx.allows_unassigned && current_assigned {
                moves.push(ScalarMoveUnion::Change(
                    crate::heuristic::r#move::ChangeMove::new(
                        entity_index,
                        None,
                        self.ctx.getter,
                        self.ctx.setter,
                        self.ctx.variable_name,
                        self.ctx.descriptor_index,
                    ),
                ));
            }
        }

        moves.into_iter()
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        self.open_cursor(score_director).count()
    }

    fn append_moves<D: Director<S>>(
        &self,
        score_director: &D,
        arena: &mut MoveArena<ScalarMoveUnion<S, usize>>,
    ) {
        arena.extend(self.open_cursor(score_director));
    }
}

#[derive(Clone, Copy)]
pub struct NearbySwapLeafSelector<S> {
    ctx: ScalarVariableContext<S>,
    max_nearby: usize,
}

impl<S> Debug for NearbySwapLeafSelector<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NearbySwapLeafSelector")
            .field("descriptor_index", &self.ctx.descriptor_index)
            .field("variable_name", &self.ctx.variable_name)
            .field("max_nearby", &self.max_nearby)
            .finish()
    }
}

impl<S> MoveSelector<S, ScalarMoveUnion<S, usize>> for NearbySwapLeafSelector<S>
where
    S: PlanningSolution + 'static,
{
    fn open_cursor<'a, D: Director<S>>(
        &'a self,
        score_director: &D,
    ) -> impl Iterator<Item = ScalarMoveUnion<S, usize>> + 'a {
        let solution = score_director.working_solution();
        let distance_meter = self
            .ctx
            .nearby_entity_distance_meter
            .expect("nearby swap requires a nearby entity distance meter");
        let entity_count = (self.ctx.entity_count)(solution);
        let current_values: Vec<_> = (0..entity_count)
            .map(|entity_index| (self.ctx.getter)(solution, entity_index))
            .collect();
        let mut moves = Vec::new();

        for left_entity_index in 0..entity_count {
            let left_value = current_values[left_entity_index];
            let mut candidates: Vec<(usize, f64, usize)> = ((left_entity_index + 1)..entity_count)
                .enumerate()
                .filter_map(|(order, right_entity_index)| {
                    if left_value == current_values[right_entity_index] {
                        return None;
                    }
                    let distance = distance_meter(solution, left_entity_index, right_entity_index);
                    distance
                        .is_finite()
                        .then_some((right_entity_index, distance, order))
                })
                .collect();

            truncate_nearby_candidates(&mut candidates, self.max_nearby);

            moves.extend(candidates.into_iter().map(|(right_entity_index, _, _)| {
                ScalarMoveUnion::Swap(crate::heuristic::r#move::SwapMove::new(
                    left_entity_index,
                    right_entity_index,
                    self.ctx.getter,
                    self.ctx.setter,
                    self.ctx.variable_name,
                    self.ctx.descriptor_index,
                ))
            }));
        }

        moves.into_iter()
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        self.open_cursor(score_director).count()
    }

    fn append_moves<D: Director<S>>(
        &self,
        score_director: &D,
        arena: &mut MoveArena<ScalarMoveUnion<S, usize>>,
    ) {
        arena.extend(self.open_cursor(score_director));
    }
}

#[derive(Clone, Copy)]
pub struct PillarChangeLeafSelector<S> {
    ctx: ScalarVariableContext<S>,
    minimum_sub_pillar_size: usize,
    maximum_sub_pillar_size: usize,
}

impl<S> Debug for PillarChangeLeafSelector<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PillarChangeLeafSelector")
            .field("descriptor_index", &self.ctx.descriptor_index)
            .field("variable_name", &self.ctx.variable_name)
            .field("minimum_sub_pillar_size", &self.minimum_sub_pillar_size)
            .field("maximum_sub_pillar_size", &self.maximum_sub_pillar_size)
            .finish()
    }
}

impl<S> MoveSelector<S, ScalarMoveUnion<S, usize>> for PillarChangeLeafSelector<S>
where
    S: PlanningSolution + 'static,
{
    fn open_cursor<'a, D: Director<S>>(
        &'a self,
        score_director: &D,
    ) -> impl Iterator<Item = ScalarMoveUnion<S, usize>> + 'a {
        let pillar_selector = DefaultPillarSelector::<S, usize, _, _>::new(
            FromSolutionEntitySelector::new(self.ctx.descriptor_index),
            self.ctx.descriptor_index,
            self.ctx.variable_name,
            |sd: &dyn Director<S>, _descriptor_index, entity_index| {
                (self.ctx.getter)(sd.working_solution(), entity_index)
            },
        )
        .with_sub_pillar_config(build_sub_pillar_config(
            self.minimum_sub_pillar_size,
            self.maximum_sub_pillar_size,
        ));

        let value_selector = ScalarValueSelector::from_source(self.ctx.value_source);
        let mut moves = Vec::new();
        for pillar in pillar_selector.iter(score_director) {
            let Some(first) = pillar.first() else {
                continue;
            };
            let Some(current_value) =
                (self.ctx.getter)(score_director.working_solution(), first.entity_index)
            else {
                continue;
            };
            let entity_indices: Vec<usize> =
                pillar.iter().map(|entity| entity.entity_index).collect();
            let legal_values = intersect_legal_values_for_pillar(&pillar, |entity_index| {
                scalar_legal_values_for_entity(
                    &value_selector,
                    score_director,
                    self.ctx.descriptor_index,
                    entity_index,
                )
            });
            moves.extend(
                legal_values
                    .into_iter()
                    .filter(|&value| value != current_value)
                    .map(|value| {
                        ScalarMoveUnion::PillarChange(PillarChangeMove::new(
                            entity_indices.clone(),
                            Some(value),
                            self.ctx.getter,
                            self.ctx.setter,
                            self.ctx.variable_name,
                            self.ctx.descriptor_index,
                        ))
                    }),
            );
        }

        moves.into_iter()
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        self.open_cursor(score_director).count()
    }

    fn append_moves<D: Director<S>>(
        &self,
        score_director: &D,
        arena: &mut MoveArena<ScalarMoveUnion<S, usize>>,
    ) {
        arena.extend(self.open_cursor(score_director));
    }
}

#[derive(Clone, Copy)]
pub struct PillarSwapLeafSelector<S> {
    ctx: ScalarVariableContext<S>,
    minimum_sub_pillar_size: usize,
    maximum_sub_pillar_size: usize,
}

impl<S> Debug for PillarSwapLeafSelector<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PillarSwapLeafSelector")
            .field("descriptor_index", &self.ctx.descriptor_index)
            .field("variable_name", &self.ctx.variable_name)
            .field("minimum_sub_pillar_size", &self.minimum_sub_pillar_size)
            .field("maximum_sub_pillar_size", &self.maximum_sub_pillar_size)
            .finish()
    }
}

impl<S> MoveSelector<S, ScalarMoveUnion<S, usize>> for PillarSwapLeafSelector<S>
where
    S: PlanningSolution + 'static,
{
    fn open_cursor<'a, D: Director<S>>(
        &'a self,
        score_director: &D,
    ) -> impl Iterator<Item = ScalarMoveUnion<S, usize>> + 'a {
        let pillar_selector = DefaultPillarSelector::<S, usize, _, _>::new(
            FromSolutionEntitySelector::new(self.ctx.descriptor_index),
            self.ctx.descriptor_index,
            self.ctx.variable_name,
            |sd: &dyn Director<S>, _descriptor_index, entity_index| {
                (self.ctx.getter)(sd.working_solution(), entity_index)
            },
        )
        .with_sub_pillar_config(build_sub_pillar_config(
            self.minimum_sub_pillar_size,
            self.maximum_sub_pillar_size,
        ));

        let value_selector = ScalarValueSelector::from_source(self.ctx.value_source);
        let pillars: Vec<_> = pillar_selector.iter(score_director).collect();
        let mut moves = Vec::new();
        for left_index in 0..pillars.len() {
            let Some(left_first) = pillars[left_index].first() else {
                continue;
            };
            let Some(left_value) =
                (self.ctx.getter)(score_director.working_solution(), left_first.entity_index)
            else {
                continue;
            };
            let left_entities: Vec<usize> = pillars[left_index]
                .iter()
                .map(|entity| entity.entity_index)
                .collect();
            for right_pillar in pillars.iter().skip(left_index + 1) {
                let Some(right_first) = right_pillar.first() else {
                    continue;
                };
                let Some(right_value) =
                    (self.ctx.getter)(score_director.working_solution(), right_first.entity_index)
                else {
                    continue;
                };
                let left_group = PillarGroup::new(left_value, pillars[left_index].clone());
                let right_group = PillarGroup::new(right_value, right_pillar.clone());
                if !pillars_are_swap_compatible(&left_group, &right_group, |entity_index| {
                    scalar_legal_values_for_entity(
                        &value_selector,
                        score_director,
                        self.ctx.descriptor_index,
                        entity_index,
                    )
                }) {
                    continue;
                }
                let right_entities: Vec<usize> = right_pillar
                    .iter()
                    .map(|entity| entity.entity_index)
                    .collect();
                moves.push(ScalarMoveUnion::PillarSwap(PillarSwapMove::new(
                    left_entities.clone(),
                    right_entities,
                    self.ctx.getter,
                    self.ctx.setter,
                    self.ctx.variable_name,
                    self.ctx.descriptor_index,
                )));
            }
        }

        moves.into_iter()
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        self.open_cursor(score_director).count()
    }

    fn append_moves<D: Director<S>>(
        &self,
        score_director: &D,
        arena: &mut MoveArena<ScalarMoveUnion<S, usize>>,
    ) {
        arena.extend(self.open_cursor(score_director));
    }
}

pub struct RuinRecreateLeafSelector<S> {
    selector: RuinMoveSelector<S, usize>,
    getter: fn(&S, usize) -> Option<usize>,
    setter: fn(&mut S, usize, Option<usize>),
    descriptor_index: usize,
    variable_name: &'static str,
    value_source: ScalarRecreateValueSource<S>,
    recreate_heuristic_type: RecreateHeuristicType,
    allows_unassigned: bool,
}

impl<S> Debug for RuinRecreateLeafSelector<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RuinRecreateLeafSelector")
            .field("selector", &self.selector)
            .field("descriptor_index", &self.descriptor_index)
            .field("variable_name", &self.variable_name)
            .field("recreate_heuristic_type", &self.recreate_heuristic_type)
            .field("allows_unassigned", &self.allows_unassigned)
            .finish()
    }
}

impl<S> MoveSelector<S, ScalarMoveUnion<S, usize>> for RuinRecreateLeafSelector<S>
where
    S: PlanningSolution + 'static,
    S::Score: solverforge_core::score::Score,
{
    fn open_cursor<'a, D: Director<S>>(
        &'a self,
        score_director: &D,
    ) -> impl Iterator<Item = ScalarMoveUnion<S, usize>> + 'a {
        let value_source = self.value_source;
        let moves: Vec<_> = self
            .selector
            .open_cursor(score_director)
            .filter_map(move |ruin| {
                let mov = RuinRecreateMove::new(
                    ruin.entity_indices_slice(),
                    self.getter,
                    self.setter,
                    self.descriptor_index,
                    self.variable_name,
                    value_source,
                    self.recreate_heuristic_type,
                    self.allows_unassigned,
                );
                mov.is_doable(score_director)
                    .then_some(ScalarMoveUnion::RuinRecreate(mov))
            })
            .collect();
        moves.into_iter()
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        self.selector.size(score_director)
    }

    fn append_moves<D: Director<S>>(
        &self,
        score_director: &D,
        arena: &mut MoveArena<ScalarMoveUnion<S, usize>>,
    ) {
        arena.extend(self.open_cursor(score_director));
    }
}

pub enum ScalarLeafSelector<S> {
    Change(ScalarChangeSelector<S>),
    Swap(ScalarSwapSelector<S>),
    NearbyChange(NearbyChangeLeafSelector<S>),
    NearbySwap(NearbySwapLeafSelector<S>),
    PillarChange(PillarChangeLeafSelector<S>),
    PillarSwap(PillarSwapLeafSelector<S>),
    RuinRecreate(RuinRecreateLeafSelector<S>),
}

#[cfg_attr(not(test), allow(dead_code))]
pub enum ScalarSelectorNode<S> {
    Leaf(ScalarLeafSelector<S>),
    Cartesian(ScalarCartesianSelector<S>),
}

impl<S> Debug for ScalarSelectorNode<S>
where
    S: PlanningSolution + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Leaf(selector) => selector.fmt(f),
            Self::Cartesian(selector) => selector.fmt(f),
        }
    }
}

impl<S> MoveSelector<S, ScalarMoveUnion<S, usize>> for ScalarSelectorNode<S>
where
    S: PlanningSolution + 'static,
    S::Score: Score,
{
    fn open_cursor<'a, D: Director<S>>(
        &'a self,
        score_director: &D,
    ) -> impl Iterator<Item = ScalarMoveUnion<S, usize>> + 'a {
        enum ScalarNodeIter<A, B> {
            Leaf(A),
            Cartesian(B),
        }

        impl<T, A, B> Iterator for ScalarNodeIter<A, B>
        where
            A: Iterator<Item = T>,
            B: Iterator<Item = T>,
        {
            type Item = T;

            fn next(&mut self) -> Option<Self::Item> {
                match self {
                    Self::Leaf(iter) => iter.next(),
                    Self::Cartesian(iter) => iter.next(),
                }
            }
        }

        match self {
            Self::Leaf(selector) => ScalarNodeIter::Leaf(selector.open_cursor(score_director)),
            Self::Cartesian(selector) => {
                ScalarNodeIter::Cartesian(selector.open_cursor(score_director))
            }
        }
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        match self {
            Self::Leaf(selector) => selector.size(score_director),
            Self::Cartesian(selector) => selector.size(score_director),
        }
    }

    fn append_moves<D: Director<S>>(
        &self,
        score_director: &D,
        arena: &mut MoveArena<ScalarMoveUnion<S, usize>>,
    ) {
        match self {
            Self::Leaf(selector) => selector.append_moves(score_director, arena),
            Self::Cartesian(selector) => selector.append_moves(score_director, arena),
        }
    }
}

impl<S> Debug for ScalarLeafSelector<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Change(selector) => selector.fmt(f),
            Self::Swap(selector) => selector.fmt(f),
            Self::NearbyChange(selector) => selector.fmt(f),
            Self::NearbySwap(selector) => selector.fmt(f),
            Self::PillarChange(selector) => selector.fmt(f),
            Self::PillarSwap(selector) => selector.fmt(f),
            Self::RuinRecreate(selector) => selector.fmt(f),
        }
    }
}

impl<S> MoveSelector<S, ScalarMoveUnion<S, usize>> for ScalarLeafSelector<S>
where
    S: PlanningSolution + 'static,
    S::Score: Score,
{
    fn open_cursor<'a, D: Director<S>>(
        &'a self,
        score_director: &D,
    ) -> impl Iterator<Item = ScalarMoveUnion<S, usize>> + 'a {
        enum ScalarLeafIter<A, B, C, Dd, E, F, G> {
            Change(A),
            Swap(B),
            NearbyChange(C),
            NearbySwap(Dd),
            PillarChange(E),
            PillarSwap(F),
            RuinRecreate(G),
        }

        impl<T, A, B, C, Dd, E, F, G> Iterator for ScalarLeafIter<A, B, C, Dd, E, F, G>
        where
            A: Iterator<Item = T>,
            B: Iterator<Item = T>,
            C: Iterator<Item = T>,
            Dd: Iterator<Item = T>,
            E: Iterator<Item = T>,
            F: Iterator<Item = T>,
            G: Iterator<Item = T>,
        {
            type Item = T;

            fn next(&mut self) -> Option<Self::Item> {
                match self {
                    Self::Change(iter) => iter.next(),
                    Self::Swap(iter) => iter.next(),
                    Self::NearbyChange(iter) => iter.next(),
                    Self::NearbySwap(iter) => iter.next(),
                    Self::PillarChange(iter) => iter.next(),
                    Self::PillarSwap(iter) => iter.next(),
                    Self::RuinRecreate(iter) => iter.next(),
                }
            }
        }

        match self {
            Self::Change(selector) => ScalarLeafIter::Change(
                selector
                    .open_cursor(score_director)
                    .map(ScalarMoveUnion::Change),
            ),
            Self::Swap(selector) => ScalarLeafIter::Swap(
                selector
                    .open_cursor(score_director)
                    .map(ScalarMoveUnion::Swap),
            ),
            Self::NearbyChange(selector) => {
                ScalarLeafIter::NearbyChange(selector.open_cursor(score_director))
            }
            Self::NearbySwap(selector) => {
                ScalarLeafIter::NearbySwap(selector.open_cursor(score_director))
            }
            Self::PillarChange(selector) => {
                ScalarLeafIter::PillarChange(selector.open_cursor(score_director))
            }
            Self::PillarSwap(selector) => {
                ScalarLeafIter::PillarSwap(selector.open_cursor(score_director))
            }
            Self::RuinRecreate(selector) => {
                ScalarLeafIter::RuinRecreate(selector.open_cursor(score_director))
            }
        }
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        match self {
            Self::Change(selector) => selector.size(score_director),
            Self::Swap(selector) => selector.size(score_director),
            Self::NearbyChange(selector) => selector.size(score_director),
            Self::NearbySwap(selector) => selector.size(score_director),
            Self::PillarChange(selector) => selector.size(score_director),
            Self::PillarSwap(selector) => selector.size(score_director),
            Self::RuinRecreate(selector) => selector.size(score_director),
        }
    }

    fn append_moves<D: Director<S>>(
        &self,
        score_director: &D,
        arena: &mut MoveArena<ScalarMoveUnion<S, usize>>,
    ) {
        match self {
            Self::Change(selector) => arena.extend(
                selector
                    .open_cursor(score_director)
                    .map(ScalarMoveUnion::Change),
            ),
            Self::Swap(selector) => arena.extend(
                selector
                    .open_cursor(score_director)
                    .map(ScalarMoveUnion::Swap),
            ),
            Self::NearbyChange(selector) => selector.append_moves(score_director, arena),
            Self::NearbySwap(selector) => selector.append_moves(score_director, arena),
            Self::PillarChange(selector) => selector.append_moves(score_director, arena),
            Self::PillarSwap(selector) => selector.append_moves(score_director, arena),
            Self::RuinRecreate(selector) => selector.append_moves(score_director, arena),
        }
    }
}

#[cfg_attr(not(test), allow(dead_code))]
fn wrap_scalar_composite<S>(
    mov: SequentialCompositeMove<S, ScalarMoveUnion<S, usize>>,
) -> ScalarMoveUnion<S, usize>
where
    S: PlanningSolution,
{
    ScalarMoveUnion::Composite(mov)
}

pub(super) fn build_scalar_flat_selector<S>(
    config: Option<&MoveSelectorConfig>,
    scalar_variables: &[ScalarVariableContext<S>],
    random_seed: Option<u64>,
) -> ScalarFlatSelector<S>
where
    S: PlanningSolution + 'static,
    S::Score: Score,
{
    let mut leaves = Vec::new();
    collect_scalar_leaf_selectors(config, scalar_variables, random_seed, &mut leaves);
    assert!(
        !leaves.is_empty(),
        "move selector configuration produced no scalar neighborhoods"
    );
    VecUnionSelector::new(leaves)
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn build_scalar_move_selector<S>(
    config: Option<&MoveSelectorConfig>,
    scalar_variables: &[ScalarVariableContext<S>],
    random_seed: Option<u64>,
) -> ScalarSelector<S>
where
    S: PlanningSolution + 'static,
    S::Score: Score,
{
    fn collect_nodes<S: PlanningSolution + 'static>(
        config: Option<&MoveSelectorConfig>,
        scalar_variables: &[ScalarVariableContext<S>],
        random_seed: Option<u64>,
        nodes: &mut Vec<ScalarSelectorNode<S>>,
    ) where
        S::Score: Score,
    {
        match config {
            Some(MoveSelectorConfig::UnionMoveSelector(union)) => {
                for child in &union.selectors {
                    collect_nodes(Some(child), scalar_variables, random_seed, nodes);
                }
            }
            Some(MoveSelectorConfig::CartesianProductMoveSelector(cartesian)) => {
                assert_eq!(
                    cartesian.selectors.len(),
                    2,
                    "cartesian_product move selector requires exactly two child selectors"
                );
                let left = build_scalar_flat_selector(
                    Some(&cartesian.selectors[0]),
                    scalar_variables,
                    random_seed,
                );
                let right = build_scalar_flat_selector(
                    Some(&cartesian.selectors[1]),
                    scalar_variables,
                    random_seed,
                );
                nodes.push(ScalarSelectorNode::Cartesian(
                    CartesianProductSelector::new(left, right, wrap_scalar_composite::<S>),
                ));
            }
            other => {
                let flat = build_scalar_flat_selector(other, scalar_variables, random_seed);
                nodes.extend(
                    flat.into_selectors()
                        .into_iter()
                        .map(ScalarSelectorNode::Leaf),
                );
            }
        }
    }

    let mut nodes = Vec::new();
    collect_nodes(config, scalar_variables, random_seed, &mut nodes);
    assert!(
        !nodes.is_empty(),
        "move selector configuration produced no scalar neighborhoods"
    );
    VecUnionSelector::new(nodes)
}

fn build_sub_pillar_config(
    minimum_size: usize,
    maximum_size: usize,
) -> crate::heuristic::selector::SubPillarConfig {
    if minimum_size == 0 || maximum_size == 0 {
        crate::heuristic::selector::SubPillarConfig::none()
    } else {
        crate::heuristic::selector::SubPillarConfig {
            enabled: true,
            minimum_size: minimum_size.max(2),
            maximum_size: maximum_size.max(minimum_size.max(2)),
        }
    }
}

fn collect_scalar_leaf_selectors<S>(
    config: Option<&MoveSelectorConfig>,
    scalar_variables: &[ScalarVariableContext<S>],
    random_seed: Option<u64>,
    leaves: &mut Vec<ScalarLeafSelector<S>>,
) where
    S: PlanningSolution + 'static,
    S::Score: Score,
{
    fn matching_variables<S: PlanningSolution + 'static>(
        scalar_variables: &[ScalarVariableContext<S>],
        entity_class: Option<&str>,
        variable_name: Option<&str>,
    ) -> Vec<ScalarVariableContext<S>> {
        scalar_variables
            .iter()
            .copied()
            .filter(|ctx| ctx.matches_target(entity_class, variable_name))
            .collect()
    }

    fn push_change<S: PlanningSolution + 'static>(
        ctx: &ScalarVariableContext<S>,
        leaves: &mut Vec<ScalarLeafSelector<S>>,
    ) {
        leaves.push(ScalarLeafSelector::Change(
            ChangeMoveSelector::new(
                FromSolutionEntitySelector::new(ctx.descriptor_index),
                ScalarValueSelector::from_source(ctx.value_source),
                ctx.getter,
                ctx.setter,
                ctx.descriptor_index,
                ctx.variable_name,
            )
            .with_allows_unassigned(ctx.allows_unassigned),
        ));
    }

    fn push_swap<S: PlanningSolution + 'static>(
        ctx: &ScalarVariableContext<S>,
        leaves: &mut Vec<ScalarLeafSelector<S>>,
    ) {
        leaves.push(ScalarLeafSelector::Swap(SwapMoveSelector::new(
            FromSolutionEntitySelector::new(ctx.descriptor_index),
            FromSolutionEntitySelector::new(ctx.descriptor_index),
            ctx.getter,
            ctx.setter,
            ctx.descriptor_index,
            ctx.variable_name,
        )));
    }

    fn push_nearby_change<S: PlanningSolution + 'static>(
        ctx: &ScalarVariableContext<S>,
        max_nearby: usize,
        leaves: &mut Vec<ScalarLeafSelector<S>>,
    ) {
        assert!(
            ctx.nearby_value_distance_meter.is_some(),
            "nearby_change_move selector requires nearby_value_distance_meter for {}::{}",
            ctx.entity_type_name,
            ctx.variable_name,
        );
        leaves.push(ScalarLeafSelector::NearbyChange(NearbyChangeLeafSelector {
            ctx: *ctx,
            max_nearby,
        }));
    }

    fn push_nearby_swap<S: PlanningSolution + 'static>(
        ctx: &ScalarVariableContext<S>,
        max_nearby: usize,
        leaves: &mut Vec<ScalarLeafSelector<S>>,
    ) {
        assert!(
            ctx.nearby_entity_distance_meter.is_some(),
            "nearby_swap_move selector requires nearby_entity_distance_meter for {}::{}",
            ctx.entity_type_name,
            ctx.variable_name,
        );
        leaves.push(ScalarLeafSelector::NearbySwap(NearbySwapLeafSelector {
            ctx: *ctx,
            max_nearby,
        }));
    }

    fn push_pillar_change<S: PlanningSolution + 'static>(
        ctx: &ScalarVariableContext<S>,
        minimum_sub_pillar_size: usize,
        maximum_sub_pillar_size: usize,
        leaves: &mut Vec<ScalarLeafSelector<S>>,
    ) {
        leaves.push(ScalarLeafSelector::PillarChange(PillarChangeLeafSelector {
            ctx: *ctx,
            minimum_sub_pillar_size,
            maximum_sub_pillar_size,
        }));
    }

    fn push_pillar_swap<S: PlanningSolution + 'static>(
        ctx: &ScalarVariableContext<S>,
        minimum_sub_pillar_size: usize,
        maximum_sub_pillar_size: usize,
        leaves: &mut Vec<ScalarLeafSelector<S>>,
    ) {
        leaves.push(ScalarLeafSelector::PillarSwap(PillarSwapLeafSelector {
            ctx: *ctx,
            minimum_sub_pillar_size,
            maximum_sub_pillar_size,
        }));
    }

    fn push_ruin_recreate<S: PlanningSolution + 'static>(
        ctx: &ScalarVariableContext<S>,
        min_ruin_count: usize,
        max_ruin_count: usize,
        moves_per_step: Option<usize>,
        recreate_heuristic_type: RecreateHeuristicType,
        random_seed: Option<u64>,
        leaves: &mut Vec<ScalarLeafSelector<S>>,
    ) {
        let selector = RuinMoveSelector::new(
            min_ruin_count.max(1),
            max_ruin_count.max(1),
            ctx.entity_count,
            ctx.getter,
            ctx.setter,
            ctx.variable_name,
            ctx.descriptor_index,
        )
        .with_moves_per_step(moves_per_step.unwrap_or(10).max(1));
        let selector = match scoped_seed(
            random_seed,
            ctx.descriptor_index,
            ctx.variable_name,
            "scalar_ruin_recreate_move_selector",
        ) {
            Some(seed) => selector.with_seed(seed),
            None => selector,
        };
        leaves.push(ScalarLeafSelector::RuinRecreate(RuinRecreateLeafSelector {
            selector,
            getter: ctx.getter,
            setter: ctx.setter,
            descriptor_index: ctx.descriptor_index,
            variable_name: ctx.variable_name,
            value_source: scalar_recreate_value_source(ctx.value_source),
            recreate_heuristic_type,
            allows_unassigned: ctx.allows_unassigned,
        }));
    }

    fn require_matches<S: PlanningSolution + 'static>(
        label: &str,
        entity_class: Option<&str>,
        variable_name: Option<&str>,
        matched: &[ScalarVariableContext<S>],
    ) {
        assert!(
            !matched.is_empty(),
            "{label} selector matched no scalar planning variables for entity_class={:?} variable_name={:?}",
            entity_class,
            variable_name,
        );
    }

    fn collect<S: PlanningSolution + 'static>(
        cfg: &MoveSelectorConfig,
        scalar_variables: &[ScalarVariableContext<S>],
        random_seed: Option<u64>,
        leaves: &mut Vec<ScalarLeafSelector<S>>,
    ) {
        match cfg {
            MoveSelectorConfig::ChangeMoveSelector(change) => {
                let matched = matching_variables(
                    scalar_variables,
                    change.target.entity_class.as_deref(),
                    change.target.variable_name.as_deref(),
                );
                require_matches(
                    "change_move",
                    change.target.entity_class.as_deref(),
                    change.target.variable_name.as_deref(),
                    &matched,
                );
                for ctx in matched {
                    push_change(&ctx, leaves);
                }
            }
            MoveSelectorConfig::SwapMoveSelector(swap) => {
                let matched = matching_variables(
                    scalar_variables,
                    swap.target.entity_class.as_deref(),
                    swap.target.variable_name.as_deref(),
                );
                require_matches(
                    "swap_move",
                    swap.target.entity_class.as_deref(),
                    swap.target.variable_name.as_deref(),
                    &matched,
                );
                for ctx in matched {
                    push_swap(&ctx, leaves);
                }
            }
            MoveSelectorConfig::NearbyChangeMoveSelector(nearby_change) => {
                let matched = matching_variables(
                    scalar_variables,
                    nearby_change.target.entity_class.as_deref(),
                    nearby_change.target.variable_name.as_deref(),
                );
                require_matches(
                    "nearby_change_move",
                    nearby_change.target.entity_class.as_deref(),
                    nearby_change.target.variable_name.as_deref(),
                    &matched,
                );
                for ctx in matched {
                    push_nearby_change(&ctx, nearby_change.max_nearby, leaves);
                }
            }
            MoveSelectorConfig::NearbySwapMoveSelector(nearby_swap) => {
                let matched = matching_variables(
                    scalar_variables,
                    nearby_swap.target.entity_class.as_deref(),
                    nearby_swap.target.variable_name.as_deref(),
                );
                require_matches(
                    "nearby_swap_move",
                    nearby_swap.target.entity_class.as_deref(),
                    nearby_swap.target.variable_name.as_deref(),
                    &matched,
                );
                for ctx in matched {
                    push_nearby_swap(&ctx, nearby_swap.max_nearby, leaves);
                }
            }
            MoveSelectorConfig::PillarChangeMoveSelector(pillar_change) => {
                let matched = matching_variables(
                    scalar_variables,
                    pillar_change.target.entity_class.as_deref(),
                    pillar_change.target.variable_name.as_deref(),
                );
                require_matches(
                    "pillar_change_move",
                    pillar_change.target.entity_class.as_deref(),
                    pillar_change.target.variable_name.as_deref(),
                    &matched,
                );
                for ctx in matched {
                    push_pillar_change(
                        &ctx,
                        pillar_change.minimum_sub_pillar_size,
                        pillar_change.maximum_sub_pillar_size,
                        leaves,
                    );
                }
            }
            MoveSelectorConfig::PillarSwapMoveSelector(pillar_swap) => {
                let matched = matching_variables(
                    scalar_variables,
                    pillar_swap.target.entity_class.as_deref(),
                    pillar_swap.target.variable_name.as_deref(),
                );
                require_matches(
                    "pillar_swap_move",
                    pillar_swap.target.entity_class.as_deref(),
                    pillar_swap.target.variable_name.as_deref(),
                    &matched,
                );
                for ctx in matched {
                    push_pillar_swap(
                        &ctx,
                        pillar_swap.minimum_sub_pillar_size,
                        pillar_swap.maximum_sub_pillar_size,
                        leaves,
                    );
                }
            }
            MoveSelectorConfig::RuinRecreateMoveSelector(ruin_recreate) => {
                let matched = matching_variables(
                    scalar_variables,
                    ruin_recreate.target.entity_class.as_deref(),
                    ruin_recreate.target.variable_name.as_deref(),
                );
                require_matches(
                    "ruin_recreate_move",
                    ruin_recreate.target.entity_class.as_deref(),
                    ruin_recreate.target.variable_name.as_deref(),
                    &matched,
                );
                for ctx in matched {
                    push_ruin_recreate(
                        &ctx,
                        ruin_recreate.min_ruin_count,
                        ruin_recreate.max_ruin_count,
                        ruin_recreate.moves_per_step,
                        ruin_recreate.recreate_heuristic_type,
                        random_seed,
                        leaves,
                    );
                }
            }
            MoveSelectorConfig::UnionMoveSelector(union) => {
                for child in &union.selectors {
                    collect(child, scalar_variables, random_seed, leaves);
                }
            }
            MoveSelectorConfig::LimitedNeighborhood(_) => {
                panic!("limited_neighborhood must be handled by the canonical runtime");
            }
            MoveSelectorConfig::ListChangeMoveSelector(_)
            | MoveSelectorConfig::NearbyListChangeMoveSelector(_)
            | MoveSelectorConfig::ListSwapMoveSelector(_)
            | MoveSelectorConfig::NearbyListSwapMoveSelector(_)
            | MoveSelectorConfig::SublistChangeMoveSelector(_)
            | MoveSelectorConfig::SublistSwapMoveSelector(_)
            | MoveSelectorConfig::ListReverseMoveSelector(_)
            | MoveSelectorConfig::KOptMoveSelector(_)
            | MoveSelectorConfig::ListRuinMoveSelector(_) => {
                panic!("list move selector configured against a scalar-variable context");
            }
            MoveSelectorConfig::CartesianProductMoveSelector(_) => {
                panic!("nested cartesian_product move selectors are not supported inside scalar cartesian children");
            }
        }
    }

    match config {
        Some(cfg) => collect(cfg, scalar_variables, random_seed, leaves),
        None => {
            for ctx in scalar_variables {
                push_change(ctx, leaves);
                push_swap(ctx, leaves);
            }
        }
    }
}

#[cfg(test)]
#[path = "scalar_selector_tests.rs"]
mod tests;
