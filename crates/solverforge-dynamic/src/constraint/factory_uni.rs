//! Factory function for building unary (single entity) constraints.
//!
//! Zero-fallback policy: all closures JIT-compiled via compile_1.
//! Panics on failure. No interpreter fallback.

use std::sync::Arc;

use solverforge_core::score::HardSoftScore;
use solverforge_core::{ConstraintRef, ImpactType};
use solverforge_scoring::api::constraint_set::IncrementalConstraint;
use solverforge_scoring::constraint::incremental::IncrementalUniConstraint;

use super::closures_extract::make_extractor;
use crate::expr::Expr;
use crate::jit;
use crate::solution::{DynamicEntity, DynamicSolution};

/// Builds a unary constraint (single entity class, no joins).
///
/// All closures are JIT-compiled. Zero fallback.
pub fn build_uni_constraint(
    constraint_ref: ConstraintRef,
    impact_type: ImpactType,
    class_idx: usize,
    filter_expr: Expr,
    weight_expr: Expr,
    is_hard: bool,
) -> Box<dyn IncrementalConstraint<DynamicSolution, HardSoftScore> + Send + Sync> {
    let extractor = make_extractor(class_idx);

    // --- Filter: JIT compile_1 ---
    let jit_filter = Arc::new(jit::compile_1(&filter_expr));
    let ci_filter = class_idx;
    let filter = Box::new(
        move |solution: &DynamicSolution, entity: &DynamicEntity| -> bool {
            let loc = solution.get_entity_location(entity.id);
            let Some((_, entity_idx)) = loc else {
                return false;
            };
            let ptr = solution.flat_entity_ptr(ci_filter, entity_idx);
            jit_filter.call_1(ptr) != 0
        },
    ) as Box<dyn Fn(&DynamicSolution, &DynamicEntity) -> bool + Send + Sync>;

    // --- Weight: JIT compile_1 ---
    let jit_weight = Arc::new(jit::compile_1(&weight_expr));
    let hard = is_hard;
    let weight = Box::new(move |entity: &DynamicEntity| -> HardSoftScore {
        // Uni weight cannot access solution (signature is Fn(&A) -> Sc).
        // Use a fixed-size stack buffer — no heap allocation.
        const MAX_FIELDS: usize = 32;
        let fields = entity.fields();
        let n = fields.len().min(MAX_FIELDS);
        let mut buf = [0i64; MAX_FIELDS];
        for i in 0..n {
            buf[i] = fields[i].to_flat_i64();
        }
        let w = jit_weight.call_1(buf.as_ptr());
        if hard {
            HardSoftScore::of_hard(w)
        } else {
            HardSoftScore::of_soft(w)
        }
    }) as Box<dyn Fn(&DynamicEntity) -> HardSoftScore + Send + Sync>;

    Box::new(IncrementalUniConstraint::new(
        constraint_ref,
        impact_type,
        extractor,
        filter,
        weight,
        is_hard,
    ))
}
