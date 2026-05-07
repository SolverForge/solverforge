
fn validate_ruin_recreate_bounds(min_ruin_count: usize, max_ruin_count: usize) {
    assert!(
        min_ruin_count >= 1,
        "descriptor ruin_recreate_move_selector requires min_ruin_count >= 1"
    );
    assert!(
        max_ruin_count >= min_ruin_count,
        "descriptor ruin_recreate_move_selector requires max_ruin_count >= min_ruin_count"
    );
}

enum SwapLegalityDomain {
    Unspecified,
    Empty,
    CountableRange {
        from: i64,
        to: i64,
    },
    SolutionCount {
        count: usize,
    },
    EntityCurrentValueBits {
        current_value_ids: Vec<usize>,
        accepted_value_words: Vec<Vec<usize>>,
    },
}

struct SwapLegalityIndex {
    current_values: Vec<Option<usize>>,
    allows_unassigned: bool,
    domain: SwapLegalityDomain,
}

impl SwapLegalityIndex {
    fn new(
        binding: &VariableBinding,
        descriptor: &SolutionDescriptor,
        solution: &dyn Any,
        count: usize,
        lookup_context: &str,
    ) -> Self {
        let current_values = (0..count)
            .map(|entity_index| {
                let entity = descriptor
                    .get_entity(solution, binding.descriptor_index, entity_index)
                    .expect(lookup_context);
                (binding.getter)(entity)
            })
            .collect::<Vec<_>>();

        let domain = match (&binding.provider, &binding.range_type) {
            (Some(_), _) => Self::build_entity_domain(
                binding,
                descriptor,
                solution,
                lookup_context,
                &current_values,
            ),
            (_, ValueRangeType::CountableRange { from, to }) => {
                SwapLegalityDomain::CountableRange {
                    from: *from,
                    to: *to,
                }
            }
            _ if binding.has_unspecified_value_range() => SwapLegalityDomain::Unspecified,
            _ => binding
                .solution_value_count(descriptor, solution)
                .map(|count| SwapLegalityDomain::SolutionCount { count })
                .unwrap_or(SwapLegalityDomain::Empty),
        };

        Self {
            current_values,
            allows_unassigned: binding.allows_unassigned,
            domain,
        }
    }

    fn build_entity_domain(
        binding: &VariableBinding,
        descriptor: &SolutionDescriptor,
        solution: &dyn Any,
        lookup_context: &str,
        current_values: &[Option<usize>],
    ) -> SwapLegalityDomain {
        let mut assigned_value_ids = HashMap::new();
        let mut unassigned_value_id = None;
        let mut current_value_ids = Vec::with_capacity(current_values.len());
        for current_value in current_values {
            let value_id = match current_value {
                Some(value) => match assigned_value_ids.get(value) {
                    Some(value_id) => *value_id,
                    None => {
                        let value_id =
                            assigned_value_ids.len() + usize::from(unassigned_value_id.is_some());
                        assigned_value_ids.insert(*value, value_id);
                        value_id
                    }
                },
                None => match unassigned_value_id {
                    Some(value_id) => value_id,
                    None => {
                        let value_id = assigned_value_ids.len();
                        unassigned_value_id = Some(value_id);
                        value_id
                    }
                },
            };
            current_value_ids.push(value_id);
        }

        let value_count = assigned_value_ids.len() + usize::from(unassigned_value_id.is_some());
        let word_count =
            value_count.saturating_add(SWAP_LEGALITY_WORD_BITS - 1) / SWAP_LEGALITY_WORD_BITS;
        let mut accepted_value_words = Vec::with_capacity(current_values.len());
        for entity_index in 0..current_values.len() {
            let entity = descriptor
                .get_entity(solution, binding.descriptor_index, entity_index)
                .expect(lookup_context);
            let mut words = vec![0usize; word_count];
            if binding.allows_unassigned {
                if let Some(value_id) = unassigned_value_id {
                    Self::set_bit(&mut words, value_id);
                }
            }
            for allowed_value in binding.values_for_entity(descriptor, solution, entity) {
                if let Some(value_id) = assigned_value_ids.get(&allowed_value) {
                    Self::set_bit(&mut words, *value_id);
                }
            }
            accepted_value_words.push(words);
        }

        SwapLegalityDomain::EntityCurrentValueBits {
            current_value_ids,
            accepted_value_words,
        }
    }

    fn set_bit(words: &mut [usize], value_id: usize) {
        words[value_id / SWAP_LEGALITY_WORD_BITS] |= 1usize << (value_id % SWAP_LEGALITY_WORD_BITS);
    }

    fn has_bit(words: &[usize], value_id: usize) -> bool {
        words[value_id / SWAP_LEGALITY_WORD_BITS] & (1usize << (value_id % SWAP_LEGALITY_WORD_BITS))
            != 0
    }

    fn accepts_value_from_entity(&self, entity_index: usize, value_entity_index: usize) -> bool {
        let candidate = self.current_values[value_entity_index];
        match &self.domain {
            SwapLegalityDomain::Unspecified => candidate.is_some(),
            SwapLegalityDomain::Empty => candidate.is_none() && self.allows_unassigned,
            SwapLegalityDomain::CountableRange { from, to } => candidate
                .map_or(self.allows_unassigned, |value| {
                    VariableBinding::countable_range_contains(*from, *to, value)
                }),
            SwapLegalityDomain::SolutionCount { count } => {
                candidate.map_or(self.allows_unassigned, |value| value < *count)
            }
            SwapLegalityDomain::EntityCurrentValueBits {
                current_value_ids,
                accepted_value_words,
            } => Self::has_bit(
                &accepted_value_words[entity_index],
                current_value_ids[value_entity_index],
            ),
        }
    }

    fn can_swap(&self, left_entity_index: usize, right_entity_index: usize) -> bool {
        if left_entity_index == right_entity_index
            || self.current_values[left_entity_index] == self.current_values[right_entity_index]
        {
            return false;
        }

        self.accepts_value_from_entity(left_entity_index, right_entity_index)
            && self.accepts_value_from_entity(right_entity_index, left_entity_index)
    }

    fn values_for_swap(
        &self,
        left_entity_index: usize,
        right_entity_index: usize,
    ) -> Option<(Option<usize>, Option<usize>)> {
        self.can_swap(left_entity_index, right_entity_index)
            .then(|| {
                (
                    self.current_values[left_entity_index],
                    self.current_values[right_entity_index],
                )
            })
    }

    fn count_legal_pairs(&self) -> usize {
        let mut total = 0;
        for left_entity_index in 0..self.current_values.len() {
            for right_entity_index in (left_entity_index + 1)..self.current_values.len() {
                if self.can_swap(left_entity_index, right_entity_index) {
                    total += 1;
                }
            }
        }
        total
    }
}

