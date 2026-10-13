// Acceptance, slot replacement, and exact ring minimum/count invariants.

use super::*;
use solverforge_core::score::SoftScore;

#[derive(Clone)]
struct TestSolution;

impl PlanningSolution for TestSolution {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        None
    }

    fn set_score(&mut self, _: Option<Self::Score>) {}
}

fn dlas(size: usize) -> DiversifiedLateAcceptanceAcceptor<TestSolution> {
    let mut acceptor = DiversifiedLateAcceptanceAcceptor::new(size);
    acceptor.phase_started(&SoftScore::of(-100));
    acceptor
}

#[test]
fn repeated_minimum_scores_count_every_occurrence() {
    let mut acceptor = dlas(2);
    for _ in 0..2 {
        assert!(acceptor.is_accepted(&SoftScore::of(-110), &SoftScore::of(-110), None));
    }
    assert_eq!(acceptor.late_worse_occurrences, 2);
    assert_eq!(acceptor.late_worse_score, SoftScore::of(-110));
}

#[test]
fn replacing_non_minimum_with_non_minimum_keeps_occurrence_count() {
    let mut acceptor = dlas(3);
    acceptor.previous_scores = vec![
        SoftScore::of(-120),
        SoftScore::of(-100),
        SoftScore::of(-100),
    ];
    acceptor.late_worse_score = SoftScore::of(-120);
    acceptor.late_worse_occurrences = 1;
    acceptor.late_score_index = 1;
    assert!(acceptor.is_accepted(&SoftScore::of(-100), &SoftScore::of(-90), None));
    assert_eq!(acceptor.late_worse_occurrences, 1);
}

#[test]
fn cached_minimum_and_count_match_ring_after_each_candidate() {
    for size in 1..=8 {
        let mut acceptor = dlas(size);
        let mut draw = 0x5a17_u64;
        for _ in 0..2_000 {
            draw = draw.wrapping_mul(6364136223846793005).wrapping_add(1);
            let current = SoftScore::of(-100 - ((draw >> 32) % 80) as i64);
            draw = draw.wrapping_mul(6364136223846793005).wrapping_add(1);
            let candidate = SoftScore::of(-100 - ((draw >> 32) % 80) as i64);
            let minimum = *acceptor.previous_scores.iter().min().unwrap();
            let expected_acceptance = candidate == current || candidate > minimum;
            assert_eq!(
                acceptor.is_accepted(&current, &candidate, None),
                expected_acceptance
            );
            let minimum = *acceptor.previous_scores.iter().min().unwrap();
            let count = acceptor
                .previous_scores
                .iter()
                .filter(|score| **score == minimum)
                .count();
            assert_eq!(acceptor.late_worse_score, minimum);
            assert_eq!(acceptor.late_worse_occurrences, count);
        }
    }
}

#[test]
fn accepts_improvement_over_current() {
    let mut acceptor = dlas(5);
    assert!(acceptor.is_accepted(&SoftScore::of(-100), &SoftScore::of(-90), None));
}

#[test]
fn accepts_equality_with_current() {
    // Paper arm one: accept == current. -100 == -100.
    let mut acceptor = dlas(3);
    assert!(acceptor.is_accepted(&SoftScore::of(-100), &SoftScore::of(-100), None));
}

#[test]
fn rejects_worse_than_late_worse() {
    // size 5, all slots = -100 → lateWorse = -100. Move -110 is worse than the
    // current step AND not > lateWorse: reject.
    let mut acceptor = dlas(5);
    assert!(!acceptor.is_accepted(&SoftScore::of(-90), &SoftScore::of(-110), None));
}

#[test]
fn accepts_better_than_late_worse_after_worsening() {
    // Drive the ring worse: accepted equal-to-current moves at -100 for size 3
    // slots each store -100 (current = -100, slot = -100: current == late, so
    // neither worsening (-100 < -100 is false) nor better arms fire — slots stay
    // -100). Then a worsening accepted move -110 at the front slot: accept arm
    // fails (move == -110 > lateWorse -100? no; move == current -110? yes →
    // accepted), current = -110 < slot -100 → update to -110 at index 0.
    // lateWorse = -110, occurrences = 1.
    let mut acceptor = dlas(3);
    assert!(acceptor.is_accepted(&SoftScore::of(-110), &SoftScore::of(-110), None));
    // Slot 0 now -110, ring index 1. lateWorse = -110.
    // A move -105 vs current -110: improves current → accept via > lateWorse.
    assert!(acceptor.is_accepted(&SoftScore::of(-110), &SoftScore::of(-105), None));
    // Slot 1 was -100, current(-105) > late(-100)? No. current > step(-110)?
    // Yes but both arms required for better; worsening? -105 < -100? No →
    // no slot update. lateWorse stays -110. Index → 2.
}

#[test]
fn diversification_slot_update_on_worsening() {
    // size 1: every accepted worsening move replaces the single slot.
    let mut acceptor = dlas(1);
    assert!(acceptor.is_accepted(&SoftScore::of(-95), &SoftScore::of(-95), None));
    // current -95 == slot -100? -95 > -100 → better arm needs > step too, step -95:
    // current(-95) > late(-100) && > previous(-95)=false → no update.
    // lateWorse stays -100 (slot untouched).
    let mut acceptor = dlas(1);
    assert!(acceptor.is_accepted(&SoftScore::of(-120), &SoftScore::of(-120), None));
    // Accepted (== current). current -120 < slot -100 → update → slot/lateWorse -120.
    // Next: move -115 (> lateWorse -120) accepted.
    assert!(acceptor.is_accepted(&SoftScore::of(-120), &SoftScore::of(-115), None));
}

#[test]
fn ring_advances_per_candidate_even_on_reject() {
    // size 2. Call with a rejected candidate: index still advances.
    let mut acceptor = dlas(2);
    // Call 1: reject (move -110, current -90, lateWorse -100). Index 0 → 1.
    assert!(!acceptor.is_accepted(&SoftScore::of(-90), &SoftScore::of(-110), None));
    // Call 2: slot at index 1 = -100. Make index-1 slot -110 via accepted equal:
    // move == current == -110 → accepted; current -110 < slot -100 → update slot 1
    // to -110. Index 1 → 0.
    assert!(acceptor.is_accepted(&SoftScore::of(-110), &SoftScore::of(-110), None));
    // Call 3 (index 0, slot -100): accepted worsening -120: current -120 < slot
    // -100 → update slot 0 → -120. lateWorse now min(-100?,-120...) = -120 with
    // occurrences 1. Index 0 → 1.
    assert!(acceptor.is_accepted(&SoftScore::of(-120), &SoftScore::of(-120), None));
    // Call 4 (index 1, slot -110): accepted equal -110: current -110 == slot -110 →
    // neither arm → no update.
    assert!(acceptor.is_accepted(&SoftScore::of(-110), &SoftScore::of(-110), None));
    // Call 5 (index 0, slot -120): move -130 rejected (== current? no, current
    // -110; > lateWorse -120? no).
    assert!(!acceptor.is_accepted(&SoftScore::of(-110), &SoftScore::of(-130), None));
}

#[test]
fn late_worse_recount_when_occurrences_exhaust() {
    let mut acceptor = dlas(2);
    for _ in 0..2 {
        assert!(acceptor.is_accepted(&SoftScore::of(-110), &SoftScore::of(-110), None));
    }
    assert!(acceptor.is_accepted(&SoftScore::of(-110), &SoftScore::of(-100), None));
    assert_eq!(acceptor.late_worse_score, SoftScore::of(-110));
    assert_eq!(acceptor.late_worse_occurrences, 1);
    assert!(acceptor.is_accepted(&SoftScore::of(-110), &SoftScore::of(-100), None));
    assert_eq!(acceptor.late_worse_score, SoftScore::of(-100));
    assert_eq!(acceptor.late_worse_occurrences, 2);
    assert!(!acceptor.is_accepted(&SoftScore::of(-100), &SoftScore::of(-105), None));
}
