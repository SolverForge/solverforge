/* `ConstraintSet` for the empty tuple and for tuples of nested sets.

Tuple children are scored left to right; their weight slots are laid out in
the same order, so each weighted call hands every child the next
`constraint_count()` multipliers.
*/

use solverforge_core::score::Score;
use solverforge_core::ConstraintRef;

use super::super::analysis::ConstraintAnalysis;
use super::incremental::{ConstraintMetadata, ConstraintResult, ConstraintSet};
use super::weighting::take_slots;

// Implement `ConstraintSet` for an empty tuple (no constraints).
impl<S: Send + Sync, Sc: Score> ConstraintSet<S, Sc> for () {
    #[inline]
    fn evaluate_all(&self, _solution: &S) -> Sc {
        Sc::zero()
    }

    #[inline]
    fn constraint_count(&self) -> usize {
        0
    }

    #[inline]
    fn constraint_metadata_entries(&self) -> Vec<ConstraintMetadata<'_>> {
        Vec::new()
    }

    #[inline]
    fn evaluate_each<'a>(&'a self, _solution: &S) -> Vec<ConstraintResult<'a, Sc>> {
        Vec::new()
    }

    #[inline]
    fn evaluate_detailed<'a>(&'a self, _solution: &S) -> Vec<ConstraintAnalysis<'a, Sc>> {
        Vec::new()
    }

    #[inline]
    fn initialize_all(&mut self, _solution: &S) -> Sc {
        Sc::zero()
    }

    #[inline]
    fn on_insert_all(
        &mut self,
        _solution: &S,
        _entity_index: usize,
        _descriptor_index: usize,
    ) -> Sc {
        Sc::zero()
    }

    #[inline]
    fn on_retract_all(
        &mut self,
        _solution: &S,
        _entity_index: usize,
        _descriptor_index: usize,
    ) -> Sc {
        Sc::zero()
    }

    #[inline]
    fn reset_all(&mut self) {}

    fn constraint_weight_refs<'a>(&'a self, _refs: &mut Vec<&'a ConstraintRef>) {}

    #[inline]
    fn evaluate_all_weighted(&self, _solution: &S, _multipliers: &[u32]) -> Sc {
        Sc::zero()
    }

    fn evaluate_each_weighted<'a>(
        &'a self,
        _solution: &S,
        _multipliers: &[u32],
    ) -> Vec<ConstraintResult<'a, Sc>> {
        Vec::new()
    }

    fn evaluate_detailed_weighted<'a>(
        &'a self,
        _solution: &S,
        _multipliers: &[u32],
    ) -> Vec<ConstraintAnalysis<'a, Sc>> {
        Vec::new()
    }

    #[inline]
    fn initialize_all_weighted(&mut self, _solution: &S, _multipliers: &[u32]) -> Sc {
        Sc::zero()
    }

    #[inline]
    fn on_insert_all_weighted(
        &mut self,
        _solution: &S,
        _entity_index: usize,
        _descriptor_index: usize,
        _multipliers: &[u32],
    ) -> Sc {
        Sc::zero()
    }

    #[inline]
    fn on_retract_all_weighted(
        &mut self,
        _solution: &S,
        _entity_index: usize,
        _descriptor_index: usize,
        _multipliers: &[u32],
    ) -> Sc {
        Sc::zero()
    }
}

// Macro to implement `ConstraintSet` for tuples of various sizes.
macro_rules! impl_constraint_set_for_tuple {
    ($($idx:tt: $T:ident),+) => {
        impl<S, Sc, $($T),+> ConstraintSet<S, Sc> for ($($T,)+)
        where
            S: Send + Sync,
            Sc: Score,
            $($T: ConstraintSet<S, Sc>,)+
        {
            #[inline]
            fn evaluate_all(&self, solution: &S) -> Sc {
                let mut total = Sc::zero();
                $(total = total + self.$idx.evaluate_all(solution);)+
                total
            }

            #[inline]
            fn constraint_count(&self) -> usize {
                let mut count = 0;
                $(count += self.$idx.constraint_count();)+
                count
            }

            fn constraint_metadata_entries(&self) -> Vec<ConstraintMetadata<'_>> {
                let mut metadata = Vec::new();
                $(
                    metadata.extend(self.$idx.constraint_metadata_entries());
                )+
                metadata
            }

            fn evaluate_each<'a>(&'a self, solution: &S) -> Vec<ConstraintResult<'a, Sc>> {
                let mut results = Vec::with_capacity(self.constraint_count());
                $(results.extend(self.$idx.evaluate_each(solution));)+
                results
            }

            fn evaluate_detailed<'a>(&'a self, solution: &S) -> Vec<ConstraintAnalysis<'a, Sc>> {
                let mut analyses = Vec::with_capacity(self.constraint_count());
                $(analyses.extend(self.$idx.evaluate_detailed(solution));)+
                analyses
            }

            #[inline]
            fn initialize_all(&mut self, solution: &S) -> Sc {
                let mut total = Sc::zero();
                $(total = total + self.$idx.initialize_all(solution);)+
                total
            }

            #[inline]
            fn on_insert_all(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc {
                let mut total = Sc::zero();
                $(total = total + self.$idx.on_insert_all(solution, entity_index, descriptor_index);)+
                total
            }

            #[inline]
            fn on_retract_all(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc {
                let mut total = Sc::zero();
                $(total = total + self.$idx.on_retract_all(solution, entity_index, descriptor_index);)+
                total
            }

            #[inline]
            fn reset_all(&mut self) {
                $(self.$idx.reset_all();)+
            }

            fn constraint_weight_refs<'a>(&'a self, refs: &mut Vec<&'a ConstraintRef>) {
                $(self.$idx.constraint_weight_refs(refs);)+
            }

            #[inline]
            fn evaluate_all_weighted(&self, solution: &S, multipliers: &[u32]) -> Sc {
                let mut slots = multipliers;
                let mut total = Sc::zero();
                $(
                    let child = take_slots(&mut slots, self.$idx.constraint_count());
                    total = total + self.$idx.evaluate_all_weighted(solution, child);
                )+
                total
            }

            fn evaluate_each_weighted<'a>(
                &'a self,
                solution: &S,
                multipliers: &[u32],
            ) -> Vec<ConstraintResult<'a, Sc>> {
                let mut slots = multipliers;
                let mut results = Vec::with_capacity(self.constraint_count());
                $(
                    let child = take_slots(&mut slots, self.$idx.constraint_count());
                    results.extend(self.$idx.evaluate_each_weighted(solution, child));
                )+
                results
            }

            fn evaluate_detailed_weighted<'a>(
                &'a self,
                solution: &S,
                multipliers: &[u32],
            ) -> Vec<ConstraintAnalysis<'a, Sc>> {
                let mut slots = multipliers;
                let mut analyses = Vec::with_capacity(self.constraint_count());
                $(
                    let child = take_slots(&mut slots, self.$idx.constraint_count());
                    analyses.extend(self.$idx.evaluate_detailed_weighted(solution, child));
                )+
                analyses
            }

            #[inline]
            fn initialize_all_weighted(&mut self, solution: &S, multipliers: &[u32]) -> Sc {
                let mut slots = multipliers;
                let mut total = Sc::zero();
                $(
                    let child = take_slots(&mut slots, self.$idx.constraint_count());
                    total = total + self.$idx.initialize_all_weighted(solution, child);
                )+
                total
            }

            #[inline]
            fn on_insert_all_weighted(
                &mut self,
                solution: &S,
                entity_index: usize,
                descriptor_index: usize,
                multipliers: &[u32],
            ) -> Sc {
                let mut slots = multipliers;
                let mut total = Sc::zero();
                $(
                    let child = take_slots(&mut slots, self.$idx.constraint_count());
                    total = total
                        + self.$idx.on_insert_all_weighted(solution, entity_index, descriptor_index, child);
                )+
                total
            }

            #[inline]
            fn on_retract_all_weighted(
                &mut self,
                solution: &S,
                entity_index: usize,
                descriptor_index: usize,
                multipliers: &[u32],
            ) -> Sc {
                let mut slots = multipliers;
                let mut total = Sc::zero();
                $(
                    let child = take_slots(&mut slots, self.$idx.constraint_count());
                    total = total
                        + self.$idx.on_retract_all_weighted(solution, entity_index, descriptor_index, child);
                )+
                total
            }
        }
    };
}

// Implement for tuples of size 1 through 32.
impl_constraint_set_for_tuple!(0: C0);
impl_constraint_set_for_tuple!(0: C0, 1: C1);
impl_constraint_set_for_tuple!(0: C0, 1: C1, 2: C2);
impl_constraint_set_for_tuple!(0: C0, 1: C1, 2: C2, 3: C3);
impl_constraint_set_for_tuple!(0: C0, 1: C1, 2: C2, 3: C3, 4: C4);
impl_constraint_set_for_tuple!(0: C0, 1: C1, 2: C2, 3: C3, 4: C4, 5: C5);
impl_constraint_set_for_tuple!(0: C0, 1: C1, 2: C2, 3: C3, 4: C4, 5: C5, 6: C6);
impl_constraint_set_for_tuple!(0: C0, 1: C1, 2: C2, 3: C3, 4: C4, 5: C5, 6: C6, 7: C7);
impl_constraint_set_for_tuple!(0: C0, 1: C1, 2: C2, 3: C3, 4: C4, 5: C5, 6: C6, 7: C7, 8: C8);
impl_constraint_set_for_tuple!(0: C0, 1: C1, 2: C2, 3: C3, 4: C4, 5: C5, 6: C6, 7: C7, 8: C8, 9: C9);
impl_constraint_set_for_tuple!(0: C0, 1: C1, 2: C2, 3: C3, 4: C4, 5: C5, 6: C6, 7: C7, 8: C8, 9: C9, 10: C10);
impl_constraint_set_for_tuple!(0: C0, 1: C1, 2: C2, 3: C3, 4: C4, 5: C5, 6: C6, 7: C7, 8: C8, 9: C9, 10: C10, 11: C11);
impl_constraint_set_for_tuple!(0: C0, 1: C1, 2: C2, 3: C3, 4: C4, 5: C5, 6: C6, 7: C7, 8: C8, 9: C9, 10: C10, 11: C11, 12: C12);
impl_constraint_set_for_tuple!(0: C0, 1: C1, 2: C2, 3: C3, 4: C4, 5: C5, 6: C6, 7: C7, 8: C8, 9: C9, 10: C10, 11: C11, 12: C12, 13: C13);
impl_constraint_set_for_tuple!(0: C0, 1: C1, 2: C2, 3: C3, 4: C4, 5: C5, 6: C6, 7: C7, 8: C8, 9: C9, 10: C10, 11: C11, 12: C12, 13: C13, 14: C14);
impl_constraint_set_for_tuple!(0: C0, 1: C1, 2: C2, 3: C3, 4: C4, 5: C5, 6: C6, 7: C7, 8: C8, 9: C9, 10: C10, 11: C11, 12: C12, 13: C13, 14: C14, 15: C15);
impl_constraint_set_for_tuple!(0: C0, 1: C1, 2: C2, 3: C3, 4: C4, 5: C5, 6: C6, 7: C7, 8: C8, 9: C9, 10: C10, 11: C11, 12: C12, 13: C13, 14: C14, 15: C15, 16: C16);
impl_constraint_set_for_tuple!(0: C0, 1: C1, 2: C2, 3: C3, 4: C4, 5: C5, 6: C6, 7: C7, 8: C8, 9: C9, 10: C10, 11: C11, 12: C12, 13: C13, 14: C14, 15: C15, 16: C16, 17: C17);
impl_constraint_set_for_tuple!(0: C0, 1: C1, 2: C2, 3: C3, 4: C4, 5: C5, 6: C6, 7: C7, 8: C8, 9: C9, 10: C10, 11: C11, 12: C12, 13: C13, 14: C14, 15: C15, 16: C16, 17: C17, 18: C18);
impl_constraint_set_for_tuple!(0: C0, 1: C1, 2: C2, 3: C3, 4: C4, 5: C5, 6: C6, 7: C7, 8: C8, 9: C9, 10: C10, 11: C11, 12: C12, 13: C13, 14: C14, 15: C15, 16: C16, 17: C17, 18: C18, 19: C19);
impl_constraint_set_for_tuple!(0: C0, 1: C1, 2: C2, 3: C3, 4: C4, 5: C5, 6: C6, 7: C7, 8: C8, 9: C9, 10: C10, 11: C11, 12: C12, 13: C13, 14: C14, 15: C15, 16: C16, 17: C17, 18: C18, 19: C19, 20: C20);
impl_constraint_set_for_tuple!(0: C0, 1: C1, 2: C2, 3: C3, 4: C4, 5: C5, 6: C6, 7: C7, 8: C8, 9: C9, 10: C10, 11: C11, 12: C12, 13: C13, 14: C14, 15: C15, 16: C16, 17: C17, 18: C18, 19: C19, 20: C20, 21: C21);
impl_constraint_set_for_tuple!(0: C0, 1: C1, 2: C2, 3: C3, 4: C4, 5: C5, 6: C6, 7: C7, 8: C8, 9: C9, 10: C10, 11: C11, 12: C12, 13: C13, 14: C14, 15: C15, 16: C16, 17: C17, 18: C18, 19: C19, 20: C20, 21: C21, 22: C22);
impl_constraint_set_for_tuple!(0: C0, 1: C1, 2: C2, 3: C3, 4: C4, 5: C5, 6: C6, 7: C7, 8: C8, 9: C9, 10: C10, 11: C11, 12: C12, 13: C13, 14: C14, 15: C15, 16: C16, 17: C17, 18: C18, 19: C19, 20: C20, 21: C21, 22: C22, 23: C23);
impl_constraint_set_for_tuple!(0: C0, 1: C1, 2: C2, 3: C3, 4: C4, 5: C5, 6: C6, 7: C7, 8: C8, 9: C9, 10: C10, 11: C11, 12: C12, 13: C13, 14: C14, 15: C15, 16: C16, 17: C17, 18: C18, 19: C19, 20: C20, 21: C21, 22: C22, 23: C23, 24: C24);
impl_constraint_set_for_tuple!(0: C0, 1: C1, 2: C2, 3: C3, 4: C4, 5: C5, 6: C6, 7: C7, 8: C8, 9: C9, 10: C10, 11: C11, 12: C12, 13: C13, 14: C14, 15: C15, 16: C16, 17: C17, 18: C18, 19: C19, 20: C20, 21: C21, 22: C22, 23: C23, 24: C24, 25: C25);
impl_constraint_set_for_tuple!(0: C0, 1: C1, 2: C2, 3: C3, 4: C4, 5: C5, 6: C6, 7: C7, 8: C8, 9: C9, 10: C10, 11: C11, 12: C12, 13: C13, 14: C14, 15: C15, 16: C16, 17: C17, 18: C18, 19: C19, 20: C20, 21: C21, 22: C22, 23: C23, 24: C24, 25: C25, 26: C26);
impl_constraint_set_for_tuple!(0: C0, 1: C1, 2: C2, 3: C3, 4: C4, 5: C5, 6: C6, 7: C7, 8: C8, 9: C9, 10: C10, 11: C11, 12: C12, 13: C13, 14: C14, 15: C15, 16: C16, 17: C17, 18: C18, 19: C19, 20: C20, 21: C21, 22: C22, 23: C23, 24: C24, 25: C25, 26: C26, 27: C27);
impl_constraint_set_for_tuple!(0: C0, 1: C1, 2: C2, 3: C3, 4: C4, 5: C5, 6: C6, 7: C7, 8: C8, 9: C9, 10: C10, 11: C11, 12: C12, 13: C13, 14: C14, 15: C15, 16: C16, 17: C17, 18: C18, 19: C19, 20: C20, 21: C21, 22: C22, 23: C23, 24: C24, 25: C25, 26: C26, 27: C27, 28: C28);
impl_constraint_set_for_tuple!(0: C0, 1: C1, 2: C2, 3: C3, 4: C4, 5: C5, 6: C6, 7: C7, 8: C8, 9: C9, 10: C10, 11: C11, 12: C12, 13: C13, 14: C14, 15: C15, 16: C16, 17: C17, 18: C18, 19: C19, 20: C20, 21: C21, 22: C22, 23: C23, 24: C24, 25: C25, 26: C26, 27: C27, 28: C28, 29: C29);
impl_constraint_set_for_tuple!(0: C0, 1: C1, 2: C2, 3: C3, 4: C4, 5: C5, 6: C6, 7: C7, 8: C8, 9: C9, 10: C10, 11: C11, 12: C12, 13: C13, 14: C14, 15: C15, 16: C16, 17: C17, 18: C18, 19: C19, 20: C20, 21: C21, 22: C22, 23: C23, 24: C24, 25: C25, 26: C26, 27: C27, 28: C28, 29: C29, 30: C30);
impl_constraint_set_for_tuple!(0: C0, 1: C1, 2: C2, 3: C3, 4: C4, 5: C5, 6: C6, 7: C7, 8: C8, 9: C9, 10: C10, 11: C11, 12: C12, 13: C13, 14: C14, 15: C15, 16: C16, 17: C17, 18: C18, 19: C19, 20: C20, 21: C21, 22: C22, 23: C23, 24: C24, 25: C25, 26: C26, 27: C27, 28: C28, 29: C29, 30: C30, 31: C31);
