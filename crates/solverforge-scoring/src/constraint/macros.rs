//! Macros for reducing boilerplate in N-ary constraint implementations.
//!
//! The `impl_get_matches_nary!` macro generates `get_matches()` implementations
//! for self-join constraints with identical structure but varying arity.

/// Generates `get_matches()` implementation for N-ary self-join constraints.
///
/// All N-ary constraints share the same pattern:
/// 1. Extract entities and build key index
/// 2. Iterate over N-tuples within each key group
/// 3. Filter and collect DetailedConstraintMatch with EntityRefs
///
/// # Usage
///
/// This macro is used internally in constraint implementations:
///
/// ```text
/// fn get_matches(&self, solution: &S) -> Vec<DetailedConstraintMatch<Sc>> {
///     impl_get_matches_nary!(bi: self, solution)
/// }
/// ```
///
/// Available arities: `bi`, `tri`, `quad`, `penta`
#[macro_export]
macro_rules! impl_get_matches_nary {
    // Bi-constraint: 2 entities
    (bi: $self:expr, $solution:expr) => {{
        use std::collections::HashMap;
        use $crate::api::analysis::{ConstraintJustification, DetailedConstraintMatch, EntityRef};

        let entities = ($self.extractor)($solution);
        let cref = $self.constraint_ref.clone();

        let mut temp_index: HashMap<_, Vec<usize>> = HashMap::new();
        for (i, entity) in entities.iter().enumerate() {
            let key = ($self.key_extractor)(entity);
            temp_index.entry(key).or_default().push(i);
        }

        let mut matches = Vec::new();
        for indices in temp_index.values() {
            for i in 0..indices.len() {
                for j in (i + 1)..indices.len() {
                    let idx_a = indices[i];
                    let idx_b = indices[j];
                    let a = &entities[idx_a];
                    let b = &entities[idx_b];
                    if ($self.filter)($solution, a, b) {
                        let justification = ConstraintJustification::new(vec![
                            EntityRef::new(a),
                            EntityRef::new(b),
                        ]);
                        let score = $self.compute_score(a, b);
                        matches.push(DetailedConstraintMatch::new(
                            cref.clone(),
                            score,
                            justification,
                        ));
                    }
                }
            }
        }
        matches
    }};

    // Tri-constraint: 3 entities
    (tri: $self:expr, $solution:expr) => {{
        use std::collections::HashMap;
        use $crate::api::analysis::{ConstraintJustification, DetailedConstraintMatch, EntityRef};

        let entities = ($self.extractor)($solution);
        let cref = $self.constraint_ref.clone();

        let mut temp_index: HashMap<_, Vec<usize>> = HashMap::new();
        for (i, entity) in entities.iter().enumerate() {
            let key = ($self.key_extractor)(entity);
            temp_index.entry(key).or_default().push(i);
        }

        let mut matches = Vec::new();
        for indices in temp_index.values() {
            for pos_i in 0..indices.len() {
                for pos_j in (pos_i + 1)..indices.len() {
                    for pos_k in (pos_j + 1)..indices.len() {
                        let i = indices[pos_i];
                        let j = indices[pos_j];
                        let k = indices[pos_k];
                        let a = &entities[i];
                        let b = &entities[j];
                        let c = &entities[k];
                        if ($self.filter)($solution, a, b, c) {
                            let justification = ConstraintJustification::new(vec![
                                EntityRef::new(a),
                                EntityRef::new(b),
                                EntityRef::new(c),
                            ]);
                            let score = $self.compute_score(a, b, c);
                            matches.push(DetailedConstraintMatch::new(
                                cref.clone(),
                                score,
                                justification,
                            ));
                        }
                    }
                }
            }
        }
        matches
    }};

    // Quad-constraint: 4 entities
    (quad: $self:expr, $solution:expr) => {{
        use std::collections::HashMap;
        use $crate::api::analysis::{ConstraintJustification, DetailedConstraintMatch, EntityRef};

        let entities = ($self.extractor)($solution);
        let cref = $self.constraint_ref.clone();

        let mut temp_index: HashMap<_, Vec<usize>> = HashMap::new();
        for (i, entity) in entities.iter().enumerate() {
            let key = ($self.key_extractor)(entity);
            temp_index.entry(key).or_default().push(i);
        }

        let mut matches = Vec::new();
        for indices in temp_index.values() {
            for pos_i in 0..indices.len() {
                for pos_j in (pos_i + 1)..indices.len() {
                    for pos_k in (pos_j + 1)..indices.len() {
                        for pos_l in (pos_k + 1)..indices.len() {
                            let i = indices[pos_i];
                            let j = indices[pos_j];
                            let k = indices[pos_k];
                            let l = indices[pos_l];
                            let a = &entities[i];
                            let b = &entities[j];
                            let c = &entities[k];
                            let d = &entities[l];
                            if ($self.filter)($solution, a, b, c, d) {
                                let justification = ConstraintJustification::new(vec![
                                    EntityRef::new(a),
                                    EntityRef::new(b),
                                    EntityRef::new(c),
                                    EntityRef::new(d),
                                ]);
                                let score = $self.compute_score(a, b, c, d);
                                matches.push(DetailedConstraintMatch::new(
                                    cref.clone(),
                                    score,
                                    justification,
                                ));
                            }
                        }
                    }
                }
            }
        }
        matches
    }};

    // Penta-constraint: 5 entities
    (penta: $self:expr, $solution:expr) => {{
        use std::collections::HashMap;
        use $crate::api::analysis::{ConstraintJustification, DetailedConstraintMatch, EntityRef};

        let entities = ($self.extractor)($solution);
        let cref = $self.constraint_ref.clone();

        let mut temp_index: HashMap<_, Vec<usize>> = HashMap::new();
        for (i, entity) in entities.iter().enumerate() {
            let key = ($self.key_extractor)(entity);
            temp_index.entry(key).or_default().push(i);
        }

        let mut matches = Vec::new();
        for indices in temp_index.values() {
            for pos_i in 0..indices.len() {
                for pos_j in (pos_i + 1)..indices.len() {
                    for pos_k in (pos_j + 1)..indices.len() {
                        for pos_l in (pos_k + 1)..indices.len() {
                            for pos_m in (pos_l + 1)..indices.len() {
                                let i = indices[pos_i];
                                let j = indices[pos_j];
                                let k = indices[pos_k];
                                let l = indices[pos_l];
                                let m = indices[pos_m];
                                let a = &entities[i];
                                let b = &entities[j];
                                let c = &entities[k];
                                let d = &entities[l];
                                let e = &entities[m];
                                if ($self.filter)($solution, a, b, c, d, e) {
                                    let justification = ConstraintJustification::new(vec![
                                        EntityRef::new(a),
                                        EntityRef::new(b),
                                        EntityRef::new(c),
                                        EntityRef::new(d),
                                        EntityRef::new(e),
                                    ]);
                                    let score = $self.compute_score(a, b, c, d, e);
                                    matches.push(DetailedConstraintMatch::new(
                                        cref.clone(),
                                        score,
                                        justification,
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
        matches
    }};
}

pub use impl_get_matches_nary;
