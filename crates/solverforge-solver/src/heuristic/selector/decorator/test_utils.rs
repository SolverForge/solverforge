//! Shared test infrastructure for decorator tests.

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::SimpleScore;
use solverforge_scoring::ScoreDirector;

#[derive(Clone, Debug)]
pub struct Task {
    pub priority: Option<i32>,
}

#[derive(Clone, Debug)]
pub struct TaskSolution {
    pub tasks: Vec<Task>,
    pub score: Option<SimpleScore>,
}

impl PlanningSolution for TaskSolution {
    type Score = SimpleScore;
    fn score(&self) -> Option<Self::Score> {
        self.score
    }
    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

pub fn get_tasks(s: &TaskSolution) -> &Vec<Task> {
    &s.tasks
}

pub fn get_tasks_mut(s: &mut TaskSolution) -> &mut Vec<Task> {
    &mut s.tasks
}

pub fn get_priority(s: &TaskSolution, i: usize) -> Option<i32> {
    s.tasks.get(i).and_then(|t| t.priority)
}

pub fn set_priority(s: &mut TaskSolution, i: usize, v: Option<i32>) {
    if let Some(t) = s.tasks.get_mut(i) {
        t.priority = v;
    }
}

pub fn create_director(tasks: Vec<Task>) -> ScoreDirector<TaskSolution, ()> {
    let solution = TaskSolution { tasks, score: None };
    ScoreDirector::new(solution, ())
}
