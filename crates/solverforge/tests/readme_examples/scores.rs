pub fn scores() {
// readme:begin
use solverforge::prelude::*;

// Single-level score
let score = SoftScore::of(-5);

// Two-level score (hard + soft)
let score = HardSoftScore::of(-2, 100);
assert!(!score.is_feasible());  // Hard score < 0

// Three-level score
let score = HardMediumSoftScore::of(0, -50, 200);

// Decimal precision
let score = HardSoftDecimalScore::of_scaled(0, -12_345_000);

// N-level configurable
let score = BendableScore::<2, 2>::of([0, -1], [-50, -100]);
// readme:end
}
