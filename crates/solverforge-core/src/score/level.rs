/// Score level representing different constraint priorities.
///
/// Maps to the semantic meaning of each level index within a [`Score`](super::Score).
/// Used by [`Score::level_label`](super::Score::level_label) to classify what
/// a given level index represents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScoreLevel {
    /// Hard constraints - must be satisfied for feasibility.
    Hard,
    /// Medium constraints - secondary priority.
    Medium,
    /// Soft constraints - optimization objectives.
    Soft,
}
