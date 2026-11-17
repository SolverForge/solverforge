/* Route cases: a routing model whose element rows carry list shadows and a
cascading arrival time, and whose stops come in pick-up and drop-off pairs
(element groups). Unlike the selector cases, these execute moves against
a director with constraints, so they measure what a route model's solve
pays per move:
- `route_rows`: list changes on a route through the score director, each
  rescoring the stop rows whose shadows the change touched;
- `group_ruin`: the compiled grouped ruin-and-recreate neighborhood, every
  candidate done, scored and undone, with joint recreate of each pair.

The cross-tree order hash covers only the change or candidate sequence;
scores feed the per-tree identity hash. */

fn mix_route_score(hash: &mut u64, score: HardSoftScore) {
    mix_u64(hash, score.hard() as u64);
    mix_u64(hash, score.soft() as u64);
}

/* Moves one stop to another position of its route or of the next route,
then back, through the director: two owner changes per direction. */
fn route_rows() {
    let mut director = route_director();
    black_box(director.calculate_score());
    emit("route_rows", ROUTE_CHANGES, || {
        let mut count = 0;
        let mut order = FNV_OFFSET;
        let mut scores = FNV_OFFSET;
        for _ in 0..iterations() {
            for change in 0..ROUTE_CHANGES {
                let from = change % ROUTE_VEHICLES;
                let to = if change % 3 == 0 {
                    (from + 1) % ROUTE_VEHICLES
                } else {
                    from
                };
                let len = director.working_solution().vehicles[from].visits.len();
                let position = change.wrapping_mul(7) % len;
                let target = change.wrapping_mul(13) % len;
                director.before_variable_changed(0, from);
                if to != from {
                    director.before_variable_changed(0, to);
                }
                let stop = director.working_solution_mut().vehicles[from]
                    .visits
                    .remove(position);
                director.working_solution_mut().vehicles[to]
                    .visits
                    .insert(target, stop);
                director.after_variable_changed(0, from);
                if to != from {
                    director.after_variable_changed(0, to);
                }
                mix_route_score(&mut scores, director.get_score());
                director.before_variable_changed(0, from);
                if to != from {
                    director.before_variable_changed(0, to);
                }
                let stop = director.working_solution_mut().vehicles[to]
                    .visits
                    .remove(target);
                director.working_solution_mut().vehicles[from]
                    .visits
                    .insert(position, stop);
                director.after_variable_changed(0, from);
                if to != from {
                    director.after_variable_changed(0, to);
                }
                mix(&mut order, from);
                mix(&mut order, to);
                mix(&mut order, position);
                mix(&mut order, target);
                mix_route_score(&mut scores, director.get_score());
                count += 1;
            }
        }
        (count, order, scores)
    });
}

/// Does, scores and undoes a candidate, folding its score.
struct DoScoreUndo<'a, D>(&'a mut D, &'a mut u64);

impl<D: Director<RoutePlan>> CandidateVisitor<RoutePlan> for DoScoreUndo<'_, D> {
    type Output = ();

    fn visit<M: Move<RoutePlan>>(self, candidate: MoveCandidateRef<'_, RoutePlan, M>) {
        let undo = candidate.do_move(self.0);
        mix_route_score(self.1, self.0.calculate_score());
        candidate.undo_move(self.0, undo);
    }
}

/// Folds a route candidate's logical-coordinate identity into the order hash.
struct RouteTraceFold<'a>(&'a mut u64);

impl CandidateVisitor<RoutePlan> for RouteTraceFold<'_> {
    type Output = ();

    fn visit<M: Move<RoutePlan>>(self, candidate: MoveCandidateRef<'_, RoutePlan, M>) {
        let identity = candidate
            .candidate_trace_identity()
            .expect("compiled runtime candidates expose a logical identity");
        mix_trace_identity(self.0, &identity);
    }
}

fn group_ruin_neighborhood() -> RouteNeighborhood {
    let config = local_search_config(MoveSelectorConfig::ListRuinMoveSelector(
        ListRuinMoveSelectorConfig {
            min_ruin_count: 4,
            max_ruin_count: 8,
            moves_per_step: Some(GROUP_RUIN_MOVES),
            selection_order: CANONICAL,
            max_member_distance: std::num::NonZeroU8::new(7),
            max_group_owners: std::num::NonZeroU8::new(4),
            ..ListRuinMoveSelectorConfig::default()
        },
    ));
    RouteRunner::compile(&config, route_context())
        .expect("the group ruin configuration must compile")
        .into_local_search_neighborhood()
        .expect("the group ruin configuration's first phase is a local search")
}

fn group_ruin() {
    let mut director = route_director();
    black_box(director.calculate_score());
    let mut neighborhood = group_ruin_neighborhood();
    let mut order_hash = FNV_OFFSET;
    {
        let mut count = 0;
        let mut cursor = neighborhood.open_cursor(&director, step_context());
        while let Some(candidate_id) = cursor.next_candidate(&mut director) {
            cursor
                .visit_candidate(candidate_id, RouteTraceFold(&mut order_hash))
                .expect("a pulled candidate stays live until released");
            assert!(cursor.release_candidate(candidate_id));
            count += 1;
        }
        assert_eq!(
            count, GROUP_RUIN_MOVES,
            "group_ruin candidate count changed"
        );
    }
    emit("group_ruin", GROUP_RUIN_MOVES, || {
        let mut count = 0;
        let mut scores = FNV_OFFSET;
        for _ in 0..iterations() {
            let mut cursor = neighborhood.open_cursor(&director, step_context());
            while let Some(candidate_id) = cursor.next_candidate(&mut director) {
                cursor
                    .visit_candidate(candidate_id, DoScoreUndo(&mut director, &mut scores))
                    .expect("a pulled candidate stays live until released");
                assert!(cursor.release_candidate(candidate_id));
                count += 1;
            }
        }
        (count, order_hash, scores)
    });
}
