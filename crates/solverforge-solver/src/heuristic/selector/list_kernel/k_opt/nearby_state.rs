//! Lazy distance-pruned cut generation for nearby K-opt.

use crate::heuristic::r#move::CutPoint;
use crate::heuristic::selector::k_opt::ListPositionDistanceMeter;
use crate::heuristic::selector::move_selector::MoveStreamContext;

pub(crate) trait KOptDistanceProbe<S> {
    fn distance(&self, solution: &S, entity: usize, from: usize, to: usize) -> f64;
}

impl<S, D> KOptDistanceProbe<S> for &D
where
    D: ListPositionDistanceMeter<S>,
{
    fn distance(&self, solution: &S, entity: usize, from: usize, to: usize) -> f64 {
        (*self).distance(solution, entity, from, to)
    }
}

fn nearby_positions<S, P>(
    solution: &S,
    distance: &P,
    max_nearby: usize,
    entity: usize,
    origin: usize,
    len: usize,
) -> Vec<usize>
where
    P: KOptDistanceProbe<S>,
{
    let mut positions = (0..len)
        .filter(|&position| position != origin)
        .map(|position| {
            (
                position,
                distance.distance(solution, entity, origin, position),
            )
        })
        .collect::<Vec<_>>();
    positions.sort_by(|left, right| {
        left.1
            .partial_cmp(&right.1)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    positions.truncate(max_nearby);
    positions
        .into_iter()
        .map(|(position, _)| position)
        .collect()
}

pub(crate) struct NearbyCutState {
    entity: usize,
    k: usize,
    len: usize,
    max_nearby: usize,
    min_segment_len: usize,
    stack: Vec<(usize, usize)>,
    nearby_cache: Vec<Vec<usize>>,
    first_positions: Vec<usize>,
    first_offset: usize,
    context: MoveStreamContext,
    salt: u64,
    done: bool,
}

impl NearbyCutState {
    pub(crate) fn new(
        entity: usize,
        k: usize,
        len: usize,
        min_segment_len: usize,
        max_nearby: usize,
        context: MoveStreamContext,
        salt: u64,
    ) -> Self {
        let min_len = (k + 1) * min_segment_len;
        if len < min_len {
            return Self {
                entity,
                k,
                len,
                max_nearby,
                min_segment_len,
                stack: Vec::new(),
                nearby_cache: Vec::new(),
                first_positions: Vec::new(),
                first_offset: 0,
                context,
                salt,
                done: true,
            };
        }
        let maximum_first = len - min_segment_len * k;
        let mut first_positions = (min_segment_len..=maximum_first).collect::<Vec<_>>();
        context.apply_selection_order_without_replacement(
            &mut first_positions,
            salt ^ 0x4B0F_7E11_72EA_0001,
        );
        let first = first_positions[0];
        Self {
            entity,
            k,
            len,
            max_nearby,
            min_segment_len,
            stack: vec![(first, 0)],
            nearby_cache: vec![Vec::new()],
            first_positions,
            first_offset: 0,
            context,
            salt,
            done: false,
        }
    }

    pub(crate) fn is_done(&self) -> bool {
        self.done
    }

    fn extend_stack<S, P>(&mut self, solution: &S, distance: &P)
    where
        P: KOptDistanceProbe<S>,
    {
        while self.stack.len() < self.k && !self.done {
            let (last_position, _) = *self.stack.last().expect("nearby K-opt stack is nonempty");
            let nearby = nearby_positions(
                solution,
                distance,
                self.max_nearby,
                self.entity,
                last_position,
                self.len,
            );
            let remaining_cuts = self.k - self.stack.len();
            let minimum_position = last_position + self.min_segment_len;
            let maximum_position = self.len - self.min_segment_len * remaining_cuts;
            let mut valid = nearby
                .into_iter()
                .filter(|&position| position >= minimum_position && position <= maximum_position)
                .collect::<Vec<_>>();
            self.context.apply_selection_order(
                &mut valid,
                self.salt
                    ^ 0x4B0F_7E11_72EA_0002
                    ^ (last_position as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
                    ^ self.stack.len() as u64,
            );
            if valid.is_empty() {
                if !self.backtrack() {
                    self.done = true;
                    return;
                }
            } else {
                self.nearby_cache.push(valid);
                let next_position = self
                    .nearby_cache
                    .last()
                    .and_then(|positions| positions.first())
                    .copied()
                    .expect("nonempty nearby cache");
                self.stack.push((next_position, 0));
            }
        }
    }

    fn backtrack(&mut self) -> bool {
        while self.stack.pop().is_some() {
            self.nearby_cache.pop();
            if let Some((_, last_index)) = self.stack.last_mut() {
                let cache_index = self.nearby_cache.len();
                if cache_index > 0 {
                    let cache = &self.nearby_cache[cache_index - 1];
                    let next_index = *last_index + 1;
                    if next_index < cache.len() {
                        *last_index = next_index;
                        let (position, _) = self.stack.last().expect("nearby stack is nonempty");
                        let next_position = cache[next_index];
                        if next_position > *position {
                            self.stack.pop();
                            self.stack.push((next_position, next_index));
                            return true;
                        }
                    }
                }
            } else {
                self.first_offset += 1;
                if let Some(&next_first) = self.first_positions.get(self.first_offset) {
                    self.stack.push((next_first, 0));
                    self.nearby_cache.push(Vec::new());
                    return true;
                }
            }
        }
        false
    }

    fn advance<S, P>(&mut self, solution: &S, distance: &P)
    where
        P: KOptDistanceProbe<S>,
    {
        if self.done || self.stack.is_empty() {
            self.done = true;
            return;
        }
        if let Some((_, index)) = self.stack.last_mut() {
            let cache_index = self.nearby_cache.len() - 1;
            let cache = &self.nearby_cache[cache_index];
            let next_index = *index + 1;
            if next_index < cache.len() {
                *index = next_index;
                let next_position = cache[next_index];
                self.stack.pop();
                self.stack.push((next_position, next_index));
                return;
            }
        }
        if self.backtrack() {
            self.extend_stack(solution, distance);
        } else {
            self.done = true;
        }
    }

    pub(crate) fn next_cuts<S, P>(&mut self, solution: &S, distance: &P) -> Option<Vec<CutPoint>>
    where
        P: KOptDistanceProbe<S>,
    {
        self.extend_stack(solution, distance);
        if self.done || self.stack.len() != self.k {
            return None;
        }
        let cuts = self
            .stack
            .iter()
            .map(|(position, _)| CutPoint::new(self.entity, *position))
            .collect::<Vec<_>>();
        self.advance(solution, distance);
        Some(cuts)
    }
}
