//! Work budgets that switch owner-restricted regret insertion to its cheaper
//! fallbacks.

pub(super) const OWNER_RESTRICTED_REGRET_TRIAL_BUDGET: usize = 16_384;
pub(super) const OWNER_RESTRICTED_BEST_INSERTION_WORK_BUDGET: usize = 8_000_000;
