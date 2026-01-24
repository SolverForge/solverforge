//! Selection order configuration for selectors.
//!
//! Defines the order in which elements are selected from a selector.

/// Defines the order in which elements are selected from a selector.
///
/// This enum controls how entities, values, or moves are ordered when
/// iterating through a selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SelectionOrder {
    /// Inherit the selection order from the parent configuration.
    ///
    /// If the parent is cached, defaults to `Original`.
    /// If there is no parent, defaults to `Random`.
    #[default]
    Inherit,

    /// Select elements in their original order.
    ///
    /// Elements are returned in the order they appear in the underlying
    /// collection. This is deterministic and reproducible.
    Original,

    /// Select elements in random order without shuffling.
    ///
    /// Elements are selected randomly from the pool on each call to next().
    /// The same element may be selected multiple times.
    /// This scales well because it does not require caching.
    Random,

    /// Select elements in random order by shuffling.
    ///
    /// Elements are shuffled when a selection iterator is created.
    /// Each element will be selected exactly once (if all elements are consumed).
    /// Requires caching (at least step-level).
    Shuffled,

    /// Select elements in sorted order.
    ///
    /// Elements are sorted according to a sorter before iteration.
    /// Each element will be selected exactly once (if all elements are consumed).
    /// Requires caching (at least step-level).
    Sorted,

    /// Select elements based on probability weights.
    ///
    /// Elements with higher probability have a greater chance of being selected.
    /// The same element may be selected multiple times.
    /// Requires caching (at least step-level).
    Probabilistic,
}

impl SelectionOrder {
    /// Resolves the selection order by inheriting from a parent if necessary.
    ///
    /// # Arguments
    ///
    /// * `inherited` - The selection order to inherit from if this is `Inherit`
    ///
    /// # Returns
    ///
    /// The resolved selection order (never `Inherit`)
    pub fn resolve(self, inherited: SelectionOrder) -> SelectionOrder {
        match self {
            SelectionOrder::Inherit => {
                if inherited == SelectionOrder::Inherit {
                    SelectionOrder::Random
                } else {
                    inherited
                }
            }
            other => other,
        }
    }

    /// Returns `true` if this selection order implies random selection.
    ///
    /// This is used to determine whether a selector should use random iteration
    /// or deterministic iteration.
    pub fn is_random(&self) -> bool {
        matches!(
            self,
            SelectionOrder::Random | SelectionOrder::Shuffled | SelectionOrder::Probabilistic
        )
    }

    /// Returns `true` if this selection order requires caching.
    ///
    /// Some selection orders need to collect all elements before iteration
    /// can begin (e.g., Shuffled, Sorted, Probabilistic).
    pub fn requires_caching(&self) -> bool {
        matches!(
            self,
            SelectionOrder::Shuffled | SelectionOrder::Sorted | SelectionOrder::Probabilistic
        )
    }

    /// Converts from a boolean random selection flag.
    ///
    /// # Arguments
    ///
    /// * `random` - `true` for `Random`, `false` for `Original`
    pub fn from_random_selection(random: bool) -> Self {
        if random {
            SelectionOrder::Random
        } else {
            SelectionOrder::Original
        }
    }

    /// Converts to a boolean random selection flag.
    ///
    /// # Panics
    ///
    /// Panics if this is not `Random` or `Original`.
    pub fn to_random_selection(&self) -> bool {
        match self {
            SelectionOrder::Random => true,
            SelectionOrder::Original => false,
            _ => panic!(
                "Selection order {:?} cannot be converted to a random selection boolean",
                self
            ),
        }
    }
}
