use super::*;

#[test]
fn cartesian_product_size_does_not_open_child_cursors() {
    let director = create_director(vec![Task {
        x: Some(0),
        y: Some(0),
        shadow_y_target: None,
    }]);
    let left = CountingSelector::new(left_x_to_one_then_two);
    let right = CountingSelector::new(left_x_to_one_then_two);
    let selector = CartesianProductSelector::new(left, right);

    assert_eq!(selector.size(&director), 4);
    assert_eq!(selector.left.open_count.get(), 0);
    assert_eq!(selector.right.open_count.get(), 0);
}

#[test]
fn cartesian_product_opens_right_rows_only_as_iteration_reaches_them() {
    let mut director = create_director(vec![Task {
        x: Some(0),
        y: Some(0),
        shadow_y_target: None,
    }]);
    let selector = CartesianProductSelector::new(
        TestSelector {
            build: left_x_to_one_then_two,
        },
        CountingSelector::new(right_y_to_ten_then_twenty),
    );

    let mut cursor = selector.open_cursor(&director);
    assert_eq!(selector.right.open_count.get(), 0);

    let first = cursor
        .next_candidate_with_director(&mut director)
        .expect("first pair must exist");
    assert_eq!(selector.right.open_count.get(), 1);
    let second = cursor
        .next_candidate_with_director(&mut director)
        .expect("second pair must exist");
    assert_eq!(selector.right.open_count.get(), 1);
    let third = cursor
        .next_candidate_with_director(&mut director)
        .expect("third pair must exist");
    assert_eq!(selector.right.open_count.get(), 2);

    assert!(cursor.candidate(first).is_some());
    assert!(cursor.candidate(second).is_some());
    assert!(cursor.candidate(third).is_some());
}

#[test]
fn cartesian_product_releases_all_losers_before_owned_commit() {
    let mut director = create_director(vec![Task {
        x: Some(0),
        y: Some(0),
        shadow_y_target: None,
    }]);
    let selector = CartesianProductSelector::new(
        TestSelector {
            build: left_x_to_one_then_two,
        },
        TestSelector {
            build: right_y_to_ten_then_twenty,
        },
    );

    let mut cursor = selector.open_cursor(&director);
    let indices = collect_indices(&mut cursor, &mut director);
    assert_eq!(indices.len(), 4);
    for &loser in &indices[1..] {
        assert!(cursor.release_candidate(loser));
        assert!(cursor.candidate(loser).is_none());
    }

    cursor.apply_owned_candidate(indices[0], &mut director);
    assert!(cursor.candidate(indices[0]).is_none());
    assert_eq!(get_x(director.working_solution(), 0, 0), Some(1));
    assert_eq!(get_y(director.working_solution(), 0, 0), Some(10));
}

#[test]
fn cartesian_product_reverse_signature_matches_true_inverse_order() {
    let mut director = create_director(vec![Task {
        x: Some(0),
        y: Some(0),
        shadow_y_target: None,
    }]);
    let forward_selector = CartesianProductSelector::new(
        TestSelector {
            build: |_| vec![x_move(1)],
        },
        TestSelector {
            build: |_| vec![y_move(10)],
        },
    );

    let mut forward_cursor = forward_selector.open_cursor(&director);
    let forward_index = collect_indices(&mut forward_cursor, &mut director)[0];
    let forward_move = forward_cursor
        .candidate(forward_index)
        .expect("forward cartesian candidate must remain valid");
    let forward_signature = forward_move.tabu_signature(&director);
    forward_move.do_move(&mut director);

    let reverse_selector = CartesianProductSelector::new(
        TestSelector {
            build: |_| vec![y_move(0)],
        },
        TestSelector {
            build: |_| vec![x_move(0)],
        },
    );

    let mut reverse_cursor = reverse_selector.open_cursor(&director);
    let reverse_index = collect_indices(&mut reverse_cursor, &mut director)[0];
    let reverse_signature = reverse_cursor
        .candidate(reverse_index)
        .expect("reverse cartesian candidate must remain valid")
        .tabu_signature(&director)
        .into_owned();

    assert_eq!(forward_signature.undo_id(), &reverse_signature.move_id);

    let mut acceptor = TabuSearchAcceptor::<Sol>::new(
        TabuSearchPolicy {
            entity_tabu_size: None,
            value_tabu_size: None,
            move_tabu_size: None,
            undo_move_tabu_size: Some(2),
            aspiration_enabled: false,
        }
        .validated(),
    );
    acceptor.phase_started(&SoftScore::of(0));
    acceptor.step_ended(
        &SoftScore::of(0),
        Some(forward_signature.clone().into_owned()),
    );

    assert!(!acceptor.is_accepted(
        &SoftScore::of(0),
        &SoftScore::of(0),
        Some(&reverse_signature),
    ));
}

/// Director wrapper counting `clone_working_solution` calls. The old cartesian
/// cursor cloned once at open (`SequentialPreviewDirector::from_director`);
/// the live-director cursor must clone zero times and preview left on the
/// caller's director itself.
struct CloneCountingDirector<'a, D: Director<Sol>> {
    inner: &'a mut D,
    clones: &'a std::sync::atomic::AtomicUsize,
}

impl<D: Director<Sol>> Director<Sol> for CloneCountingDirector<'_, D> {
    fn working_solution(&self) -> &Sol {
        self.inner.working_solution()
    }

    fn working_solution_mut(&mut self) -> &mut Sol {
        self.inner.working_solution_mut()
    }

    fn calculate_score(&mut self) -> SoftScore {
        self.inner.calculate_score()
    }

    fn solution_descriptor(&self) -> &SolutionDescriptor {
        self.inner.solution_descriptor()
    }

    fn clone_working_solution(&self) -> Sol {
        self.clones
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self.inner.clone_working_solution()
    }

    fn before_variable_changed(&mut self, descriptor_index: usize, entity_index: usize) {
        self.inner
            .before_variable_changed(descriptor_index, entity_index);
    }

    fn after_variable_changed(&mut self, descriptor_index: usize, entity_index: usize) {
        self.inner
            .after_variable_changed(descriptor_index, entity_index);
    }

    fn entity_count(&self, descriptor_index: usize) -> Option<usize> {
        self.inner.entity_count(descriptor_index)
    }

    fn total_entity_count(&self) -> Option<usize> {
        self.inner.total_entity_count()
    }

    fn constraint_metadata(&self) -> Vec<solverforge_scoring::ConstraintMetadata<'_>> {
        self.inner.constraint_metadata()
    }

    fn snapshot_score_state(&self) -> solverforge_scoring::DirectorScoreState<SoftScore> {
        self.inner.snapshot_score_state()
    }

    fn restore_score_state(&mut self, state: solverforge_scoring::DirectorScoreState<SoftScore>) {
        self.inner.restore_score_state(state);
    }
}

#[test]
fn cartesian_product_cursor_previews_left_on_the_live_director() {
    let clones = std::sync::atomic::AtomicUsize::new(0);
    let mut director = create_director(vec![Task {
        x: Some(0),
        y: Some(0),
        shadow_y_target: None,
    }]);
    let selector = CartesianProductSelector::new(
        TestSelector {
            build: left_x_to_one_then_two,
        },
        TestSelector {
            build: right_shadow_backed_y,
        },
    );

    {
        let mut counter = CloneCountingDirector {
            inner: &mut director,
            clones: &clones,
        };
        let mut cursor = selector.open_cursor(&counter);
        let indices = collect_indices(&mut cursor, &mut counter);
        assert_eq!(
            indices.len(),
            2,
            "both left rows must materialize shadow-backed right pairs"
        );
        assert_eq!(
            clones.load(std::sync::atomic::Ordering::SeqCst),
            0,
            "cartesian opening and pulls must use the live director, not a cloned preview"
        );
    }
    // Exact restoration: the enumeration must leave committed rows untouched.
    assert_eq!(director.working_solution().tasks[0].x, Some(0));
    assert_eq!(director.working_solution().tasks[0].y, Some(0));
}
