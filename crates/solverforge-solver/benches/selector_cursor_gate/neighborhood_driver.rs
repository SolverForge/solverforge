/* Drives one compiled neighborhood the way a local-search step drives it:
open the composition cursor for a step context, pull every candidate with
the live director, inspect it by reference, and release it.

The cross-tree order hash comes from one untimed pass over the same cursor
that folds each candidate's logical-coordinate identity
(`Move::candidate_trace_identity`), so it is independent of tabu encodings
and stays out of the measured allocation counters. The measured loop folds
each candidate's entity indices into the per-tree identity hash. */

const FNV_OFFSET: u64 = 0xCBF2_9CE4_8422_2325;

fn step_context() -> MoveStreamContext {
    MoveStreamContext::new(0, SEED, None)
}

fn mix_text(hash: &mut u64, text: &str) {
    mix(hash, text.len());
    for byte in text.bytes() {
        mix(hash, usize::from(byte));
    }
}

fn mix_trace_identity(hash: &mut u64, identity: &CandidateTraceIdentity) {
    match identity {
        CandidateTraceIdentity::Operation(operation) => {
            mix(hash, 1);
            mix(hash, operation.descriptor_index);
            mix_text(hash, operation.variable_name.as_deref().unwrap_or(""));
            mix_text(hash, &operation.operation);
            mix(hash, operation.components.len());
            for component in &operation.components {
                match component {
                    CandidateTraceCoordinate::Unsigned(value) => mix_u64(hash, *value),
                    CandidateTraceCoordinate::Absent => mix(hash, usize::MAX),
                    CandidateTraceCoordinate::Text(text) => mix_text(hash, text),
                    CandidateTraceCoordinate::Bytes(bytes) => {
                        mix(hash, bytes.len());
                        bytes.iter().for_each(|byte| mix(hash, usize::from(*byte)));
                    }
                }
            }
        }
        CandidateTraceIdentity::Composite(composite) => {
            mix(hash, 2);
            mix_text(hash, &composite.operation);
            mix(hash, composite.children.len());
            for child in &composite.children {
                mix_trace_identity(hash, child);
            }
        }
    }
}

/// Folds a candidate's logical-coordinate identity into the order hash.
struct TraceIdentityFold<'a>(&'a mut u64);

impl CandidateVisitor<Plan> for TraceIdentityFold<'_> {
    type Output = ();

    fn visit<M: Move<Plan>>(self, candidate: MoveCandidateRef<'_, Plan, M>) {
        let identity = candidate
            .candidate_trace_identity()
            .expect("compiled runtime candidates expose a logical identity");
        mix_trace_identity(self.0, &identity);
    }
}

/// Folds a candidate's entity indices into the per-tree identity hash.
struct EntityFold<'a>(&'a mut u64);

impl CandidateVisitor<Plan> for EntityFold<'_> {
    type Output = ();

    fn visit<M: Move<Plan>>(self, candidate: MoveCandidateRef<'_, Plan, M>) {
        for &entity_index in candidate.entity_indices() {
            mix(self.0, entity_index);
        }
        black_box(&candidate);
    }
}

fn neighborhood_order_hash(
    case: &str,
    expected_count: usize,
    director: &mut ScoreDirector<Plan, ()>,
    neighborhood: &mut Neighborhood,
) -> u64 {
    let mut count = 0;
    let mut hash = FNV_OFFSET;
    let mut cursor = neighborhood.open_cursor(director, step_context());
    while let Some(candidate_id) = cursor.next_candidate(director) {
        cursor
            .visit_candidate(candidate_id, TraceIdentityFold(&mut hash))
            .expect("a pulled candidate stays live until released");
        assert!(cursor.release_candidate(candidate_id));
        count += 1;
    }
    assert_eq!(count, expected_count, "{case} candidate count changed");
    hash
}

fn neighborhood_case(
    case: &str,
    expected_count: usize,
    mut director: ScoreDirector<Plan, ()>,
    mut neighborhood: Neighborhood,
) {
    let order_hash =
        neighborhood_order_hash(case, expected_count, &mut director, &mut neighborhood);
    emit(case, expected_count, || {
        let mut count = 0;
        let mut identity = FNV_OFFSET;
        for _ in 0..iterations() {
            let mut cursor = neighborhood.open_cursor(&director, step_context());
            while let Some(candidate_id) = cursor.next_candidate(&mut director) {
                cursor
                    .visit_candidate(candidate_id, EntityFold(&mut identity))
                    .expect("a pulled candidate stays live until released");
                assert!(cursor.release_candidate(candidate_id));
                count += 1;
            }
        }
        (count, order_hash, identity)
    });
}
