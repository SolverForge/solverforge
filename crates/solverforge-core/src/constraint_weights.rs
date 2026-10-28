/* Runtime per-constraint weight multipliers.

`ConstraintWeights` lives on a planning solution (through the
`#[constraint_weights]` field attribute) and scales each named constraint's
coded contribution by a non-negative integer multiplier.
*/

use std::collections::{BTreeMap, BTreeSet};

use crate::constraint::ConstraintRef;

/* Largest accepted constraint weight multiplier.

Multipliers scale exact `i64` score levels. Capping them at one million keeps
a coded per-match contribution of up to about 9.2e12 per level (for example
92 million hard units of a `HardSoftDecimalScore`, which stores five decimal
places) representable after scaling, while still letting UIs ship baseline
multipliers such as `{"Travel time": 10}` with generous slider ranges.
Score scaling itself stays strict and panics on any remaining overflow.
*/
pub const MAX_CONSTRAINT_WEIGHT_MULTIPLIER: u32 = 1_000_000;

/* Per-constraint multipliers keyed by `ConstraintRef::full_name()`.

A constraint that is not listed keeps multiplier `1`, its coded behavior.
Multiplier `0` disables the constraint and `n` scales its contribution `n`
times. An override applies to every terminal carrying that constraint name.

# Example

```
use solverforge_core::{ConstraintRef, ConstraintWeights};

let mut weights = ConstraintWeights::new();
weights.set("Travel time", 10).set("Overtime", 0);

assert_eq!(weights.get("Travel time"), Some(10));
assert_eq!(weights.multiplier_for(&ConstraintRef::new("", "Overtime")), 0);
assert_eq!(weights.multiplier_for(&ConstraintRef::new("", "Unlisted")), 1);

let known = [
    ConstraintRef::new("", "Travel time"),
    ConstraintRef::new("", "Overtime"),
];
assert!(weights.validate(&known).is_ok());

weights.set("Travle time", 2);
let error = weights.validate(&known).unwrap_err();
assert!(error.to_string().contains("did you mean \"Travel time\""));
```
*/
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(transparent)
)]
pub struct ConstraintWeights {
    multipliers: BTreeMap<String, u32>,
}

impl ConstraintWeights {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(&mut self, constraint: impl Into<String>, multiplier: u32) -> &mut Self {
        self.multipliers.insert(constraint.into(), multiplier);
        self
    }

    pub fn remove(&mut self, constraint: &str) -> Option<u32> {
        self.multipliers.remove(constraint)
    }

    pub fn get(&self, constraint: &str) -> Option<u32> {
        self.multipliers.get(constraint).copied()
    }

    // Returns the multiplier for `constraint`, `1` when it is not listed.
    pub fn multiplier_for(&self, constraint: &ConstraintRef) -> u32 {
        if self.multipliers.is_empty() {
            return 1;
        }
        self.get(&constraint.full_name()).unwrap_or(1)
    }

    pub fn is_empty(&self) -> bool {
        self.multipliers.is_empty()
    }

    pub fn len(&self) -> usize {
        self.multipliers.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, u32)> {
        self.multipliers
            .iter()
            .map(|(name, multiplier)| (name.as_str(), *multiplier))
    }

    /* Checks every listed name against the known constraints and every
    multiplier against `MAX_CONSTRAINT_WEIGHT_MULTIPLIER`.

    Names are checked in sorted order, so the reported error is deterministic.
    */
    pub fn validate<'a>(
        &self,
        known: impl IntoIterator<Item = &'a ConstraintRef>,
    ) -> Result<(), ConstraintWeightError> {
        let known: BTreeSet<String> = known.into_iter().map(ConstraintRef::full_name).collect();
        for (name, &multiplier) in &self.multipliers {
            if !known.contains(name) {
                return Err(ConstraintWeightError::UnknownConstraint {
                    name: name.clone(),
                    suggestions: close_matches(name, &known),
                    known: known.iter().cloned().collect(),
                });
            }
            if multiplier > MAX_CONSTRAINT_WEIGHT_MULTIPLIER {
                return Err(ConstraintWeightError::MultiplierTooLarge {
                    name: name.clone(),
                    multiplier,
                    max: MAX_CONSTRAINT_WEIGHT_MULTIPLIER,
                });
            }
        }
        Ok(())
    }
}

// Error reported when constraint weights do not fit the model's constraints.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ConstraintWeightError {
    #[error(
        "constraint weight names unknown constraint \"{name}\"{}; known constraints: {}",
        suggestion_text(suggestions),
        known_text(known)
    )]
    UnknownConstraint {
        name: String,
        suggestions: Vec<String>,
        known: Vec<String>,
    },
    #[error("constraint weight multiplier {multiplier} for \"{name}\" exceeds the maximum {max}")]
    MultiplierTooLarge {
        name: String,
        multiplier: u32,
        max: u32,
    },
}

fn suggestion_text(suggestions: &[String]) -> String {
    match suggestions {
        [] => String::new(),
        [only] => format!(" (did you mean \"{only}\"?)"),
        many => format!(
            " (did you mean one of {}?)",
            many.iter()
                .map(|name| format!("\"{name}\""))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

fn known_text(known: &[String]) -> String {
    if known.is_empty() {
        return "none".to_string();
    }
    known
        .iter()
        .map(|name| format!("\"{name}\""))
        .collect::<Vec<_>>()
        .join(", ")
}

fn close_matches(name: &str, known: &BTreeSet<String>) -> Vec<String> {
    let lowered = name.to_lowercase();
    let limit = (name.chars().count() / 3).max(1);
    known
        .iter()
        .filter(|candidate| {
            let candidate = candidate.to_lowercase();
            candidate == lowered || edit_distance(&candidate, &lowered) <= limit
        })
        .cloned()
        .collect()
}

fn edit_distance(left: &str, right: &str) -> usize {
    let right: Vec<char> = right.chars().collect();
    let mut previous: Vec<usize> = (0..=right.len()).collect();
    let mut current = vec![0; right.len() + 1];
    for (i, l) in left.chars().enumerate() {
        current[0] = i + 1;
        for (j, r) in right.iter().enumerate() {
            let substitution = previous[j] + usize::from(l != *r);
            current[j + 1] = substitution.min(previous[j + 1] + 1).min(current[j] + 1);
        }
        std::mem::swap(&mut previous, &mut current);
    }
    previous[right.len()]
}
