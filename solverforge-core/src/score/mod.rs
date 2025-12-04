mod simple;
mod hard_soft;
mod simple_decimal;
mod hard_soft_decimal;

pub use simple::SimpleScore;
pub use hard_soft::{HardSoftScore, HardMediumSoftScore};
pub use simple_decimal::SimpleDecimalScore;
pub use hard_soft_decimal::{HardSoftDecimalScore, HardMediumSoftDecimalScore};

use serde::{Deserialize, Serialize};
use std::fmt::Display;

pub trait Score: Clone + PartialOrd + Display + Serialize + for<'de> Deserialize<'de> {
    fn is_feasible(&self) -> bool;
    fn is_solution_initialized(&self) -> bool;
    fn zero() -> Self where Self: Sized;
    fn negate(&self) -> Self;
    fn add(&self, other: &Self) -> Self;
    fn subtract(&self, other: &Self) -> Self;
}
