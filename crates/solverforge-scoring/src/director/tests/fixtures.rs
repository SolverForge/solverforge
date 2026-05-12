use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::SoftScore;

#[derive(Clone, Debug, PartialEq)]
pub struct Queen {
    pub id: i64,
    pub column: i64,
    pub row: Option<i64>,
}

impl Queen {
    pub fn assigned(id: i64, column: i64, row: i64) -> Self {
        Self {
            id,
            column,
            row: Some(row),
        }
    }

    pub fn unassigned(id: i64, column: i64) -> Self {
        Self {
            id,
            column,
            row: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct NQueensSolution {
    pub queens: Vec<Queen>,
    pub score: Option<SoftScore>,
}

impl NQueensSolution {
    pub fn new(queens: Vec<Queen>) -> Self {
        Self {
            queens,
            score: None,
        }
    }
}

impl PlanningSolution for NQueensSolution {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

pub fn get_queen_row(s: &NQueensSolution, idx: usize, _variable_index: usize) -> Option<i64> {
    s.queens.get(idx).and_then(|q| q.row)
}

pub fn set_queen_row(s: &mut NQueensSolution, idx: usize, _variable_index: usize, v: Option<i64>) {
    if let Some(queen) = s.queens.get_mut(idx) {
        queen.row = v;
    }
}

#[derive(Clone, Debug)]
pub struct ShadowSolution {
    pub values: Vec<i32>,
    pub cached_sum: i32,
    pub score: Option<SoftScore>,
}

impl ShadowSolution {
    pub fn new(values: Vec<i32>) -> Self {
        Self {
            values,
            cached_sum: 0,
            score: None,
        }
    }

    pub fn with_cached_sum(values: Vec<i32>, cached_sum: i32) -> Self {
        Self {
            values,
            cached_sum,
            score: None,
        }
    }
}

impl Default for ShadowSolution {
    fn default() -> Self {
        Self::new(vec![])
    }
}

impl PlanningSolution for ShadowSolution {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }

    fn update_entity_shadows(&mut self, _descriptor_index: usize, _entity_index: usize) {
        self.cached_sum = self.values.iter().sum();
    }

    fn update_all_shadows(&mut self) {
        self.cached_sum = self.values.iter().sum();
    }
}

#[cfg(test)]
#[path = "fixtures_tests.rs"]
mod tests;
