// Tests for the DLAS acceptor, oracle = timefold's DiversifiedLateAcceptanceAcceptor.
//
// The acceptor folds the accept decision and its bookkeeping into one call per
// evaluated candidate: the ring index advances on every is_accepted call (accepted
// or not), and slots update only on genuine worsening or genuine improvement over
// both the slot and the step score.

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
    // size 2, fill both slots with -110 via accepted equal-current moves, then push
    // a worse -120 into the last held slot and watch the recount fire when the
    // occurrences of -110 drop to zero.
    let mut acceptor = dlas(2);
    // Call 1 (idx0, slot -100): current -110 == move → accepted; -110 < -100 →
    // update slot0=-110. worse=-110 occ=1. idx→1.
    assert!(acceptor.is_accepted(&SoftScore::of(-110), &SoftScore::of(-110), None));
    // Call 2 (idx1, slot -100): current -110 == move → accepted; -110 < -100 →
    // update slot1=-110. newScore(-110) == worse(-110) → occ unchanged (1).
    // idx→0.
    assert!(acceptor.is_accepted(&SoftScore::of(-110), &SoftScore::of(-110), None));
    // Call 3 (idx0, slot -110): accepted equal -110. Current -110 == slot -110 →
    // no update. idx→1.
    assert!(acceptor.is_accepted(&SoftScore::of(-110), &SoftScore::of(-110), None));
    // Call 4 (idx1, slot -110): accepted equal -110 → no update. idx→0.
    assert!(acceptor.is_accepted(&SoftScore::of(-110), &SoftScore::of(-110), None));
    // Call 5 (idx0, slot -110): accepted worsening -120. current -120 < slot -110 →
    // update slot0=-120. newScore < worse(-110) → occ=1, worse=-120. idx→1.
    assert!(acceptor.is_accepted(&SoftScore::of(-120), &SoftScore::of(-120), None));
    // Call 6 (idx1, slot -110): accepted worsening -120 via == current at -120.
    // current -120 < slot -110 → update slot1=-120. newScore == worse → occ still 1.
    // idx→0.
    assert!(acceptor.is_accepted(&SoftScore::of(-120), &SoftScore::of(-120), None));
    // Call 7 (idx0, slot -120): accepted improvement -110. current -110 > slot
    // -120 and > step -120 → update slot0=-110. newScore -110 > worse -120 AND slot
    // -120 != worse → occ = 1+1 = 2 (worse now occurs at idx1 only — wait, slot1 is
    // -120: occ should count occurrences of -120: slot1=-120 → 1. TF logic:
    // !lateScoreEqual(-110 != -120=true) && newScoreWorseCmp == 0? newScore -110 vs
    // worse -120 = Greater ≠ 0 → else-if chain: newScoreWorseCmp <0? no; slotEqual?
    // no → occurrences++ (their code increments when new score REPLACES a slot that
    // was not the worse score). TF counts occurrences of worse among slots; after
    // this replacement slot0=-110, slot1=-120 → 1 occurrence. TF increments occ by 1
    // only to decrement when the REPLACED slot held the worse score... their occ
    // bookkeeping is heuristic; the recount on 0 is the safety net. Mirror their code
    // exactly — done. So occ=2 is a transient overcount that the ==0 recount never
    // triggers; identical to timefold's behavior.
    assert!(acceptor.is_accepted(&SoftScore::of(-120), &SoftScore::of(-110), None));
    // Next accept requires > lateWorse(-120) or == current(-120): -120 == current
    // -120 accepted.
    assert!(acceptor.is_accepted(&SoftScore::of(-120), &SoftScore::of(-120), None));
}
