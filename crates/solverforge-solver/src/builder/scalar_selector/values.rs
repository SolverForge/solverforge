
pub enum ScalarValueSelector<S> {
    Empty,
    CountableRange {
        from: usize,
        to: usize,
    },
    SolutionCount {
        count_fn: fn(&S, usize) -> usize,
        provider_index: usize,
    },
    EntitySlice {
        values_for_entity: for<'a> fn(&'a S, usize, usize) -> &'a [usize],
        variable_index: usize,
    },
}

pub struct ScalarCandidateSelector<S> {
    ctx: ScalarVariableContext<S>,
    value_candidate_limit: Option<usize>,
}

impl<S> ScalarCandidateSelector<S> {
    pub fn new(ctx: ScalarVariableContext<S>, value_candidate_limit: Option<usize>) -> Self {
        Self {
            ctx,
            value_candidate_limit,
        }
    }
}

impl<S> Debug for ScalarCandidateSelector<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ScalarCandidateSelector")
            .field("ctx", &self.ctx)
            .field("value_candidate_limit", &self.value_candidate_limit)
            .finish()
    }
}

impl<S> ValueSelector<S, usize> for ScalarCandidateSelector<S>
where
    S: PlanningSolution,
{
    fn iter<'a, D: Director<S>>(
        &'a self,
        score_director: &D,
        _descriptor_index: usize,
        entity_index: usize,
    ) -> impl Iterator<Item = usize> + 'a {
        self.ctx
            .candidate_values_for_entity(
                score_director.working_solution(),
                entity_index,
                self.value_candidate_limit,
            )
            .into_iter()
    }

    fn size<D: Director<S>>(
        &self,
        score_director: &D,
        _descriptor_index: usize,
        entity_index: usize,
    ) -> usize {
        self.ctx
            .candidate_values_for_entity(
                score_director.working_solution(),
                entity_index,
                self.value_candidate_limit,
            )
            .len()
    }
}

impl<S> ScalarValueSelector<S> {
    fn from_context(ctx: ScalarVariableContext<S>) -> Self {
        match ctx.value_source {
            ValueSource::Empty => Self::Empty,
            ValueSource::CountableRange { from, to } => Self::CountableRange { from, to },
            ValueSource::SolutionCount {
                count_fn,
                provider_index,
            } => Self::SolutionCount {
                count_fn,
                provider_index,
            },
            ValueSource::EntitySlice { values_for_entity } => Self::EntitySlice {
                values_for_entity,
                variable_index: ctx.variable_index,
            },
        }
    }
}

fn scalar_recreate_candidate_source<S>(
    ctx: ScalarVariableContext<S>,
    value_candidate_limit: Option<usize>,
) -> ScalarRecreateValueSource<S> {
    if let Some(candidate_values) = ctx.candidate_values {
        return ScalarRecreateValueSource::CandidateSlice {
            candidate_values,
            variable_index: ctx.variable_index,
            value_candidate_limit,
        };
    }
    match ctx.value_source {
        ValueSource::Empty => ScalarRecreateValueSource::Empty,
        ValueSource::CountableRange { from, to } => {
            let bounded_to = value_candidate_limit
                .map(|limit| from.saturating_add(limit).min(to))
                .unwrap_or(to);
            ScalarRecreateValueSource::CountableRange {
                from,
                to: bounded_to,
            }
        }
        ValueSource::SolutionCount {
            count_fn,
            provider_index,
        } => ScalarRecreateValueSource::SolutionCount {
            count_fn,
            provider_index,
            value_candidate_limit,
        },
        ValueSource::EntitySlice { values_for_entity } => ScalarRecreateValueSource::EntitySlice {
            values_for_entity,
            variable_index: ctx.variable_index,
            value_candidate_limit,
        },
    }
}

fn scalar_legal_values_for_entity<S, VS, D>(
    value_selector: &VS,
    score_director: &D,
    descriptor_index: usize,
    entity_index: usize,
) -> Vec<usize>
where
    S: PlanningSolution,
    VS: ValueSelector<S, usize>,
    D: Director<S>,
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
            Self::SolutionCount { provider_index, .. } => write!(
                f,
                "ScalarValueSelector::SolutionCount(provider={provider_index})"
            ),
            Self::EntitySlice { .. } => write!(f, "ScalarValueSelector::EntitySlice(..)"),
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
        _descriptor_index: usize,
        entity_index: usize,
    ) -> impl Iterator<Item = usize> + 'a {
        match self {
            Self::Empty => ScalarValueIter::Empty,
            Self::CountableRange { from, to } => ScalarValueIter::CountableRange(*from..*to),
            Self::SolutionCount {
                count_fn,
                provider_index,
            } => ScalarValueIter::SolutionCount(
                0..count_fn(score_director.working_solution(), *provider_index),
            ),
            Self::EntitySlice {
                values_for_entity,
                variable_index,
            } => ScalarValueIter::EntitySlice(
                values_for_entity(
                    score_director.working_solution(),
                    entity_index,
                    *variable_index,
                )
                .to_vec()
                .into_iter(),
            ),
        }
    }

    fn size<D: Director<S>>(
        &self,
        score_director: &D,
        _descriptor_index: usize,
        entity_index: usize,
    ) -> usize {
        match self {
            Self::Empty => 0,
            Self::CountableRange { from, to } => to.saturating_sub(*from),
            Self::SolutionCount {
                count_fn,
                provider_index,
            } => count_fn(score_director.working_solution(), *provider_index),
            Self::EntitySlice {
                values_for_entity,
                variable_index,
            } => values_for_entity(
                score_director.working_solution(),
                entity_index,
                *variable_index,
            )
            .len(),
        }
    }
}

enum ScalarValueIter {
    Empty,
    CountableRange(Range<usize>),
    SolutionCount(Range<usize>),
    EntitySlice(std::vec::IntoIter<usize>),
}

impl Iterator for ScalarValueIter {
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
    ChangeMoveSelector<S, usize, FromSolutionEntitySelector, ScalarCandidateSelector<S>>;
type ScalarSwapSelector<S> = SwapLeafSelector<S>;

fn scalar_value_is_legal(
    legal_values: &[usize],
    allows_unassigned: bool,
    value: Option<usize>,
) -> bool {
    match value {
        None => allows_unassigned,
        Some(value) => legal_values.contains(&value),
    }
}

fn scalar_swap_is_legal<S>(
    ctx: ScalarVariableContext<S>,
    legal_values: &[usize],
    value: Option<usize>,
) -> bool {
    if matches!(ctx.value_source, ValueSource::Empty) {
        return value.is_some();
    }
    scalar_value_is_legal(legal_values, ctx.allows_unassigned, value)
}
