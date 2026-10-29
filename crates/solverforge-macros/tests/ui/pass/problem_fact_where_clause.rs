use solverforge::problem_fact;

#[problem_fact]
pub struct Fact<T>
where
    T: Clone + std::fmt::Debug + PartialEq + Eq + Send + Sync + 'static,
{
    #[planning_id]
    pub id: usize,
    pub value: T,
}

#[problem_fact]
pub struct First<I>
where
    I: Iterator + Clone + std::fmt::Debug + PartialEq + Eq + Send + Sync + 'static,
    I::Item: Clone + std::fmt::Debug + PartialEq + Eq + Send + Sync + 'static,
{
    #[planning_id]
    pub id: usize,
    pub first: Option<I::Item>,
    pub rest: I,
}

fn main() {
    let fact = Fact { id: 0, value: 7_u32 };
    assert_eq!(fact.clone(), fact);
    let first = First {
        id: 1,
        first: Some(1_u8),
        rest: 2_u8..4,
    };
    let _ = format!("{first:?}");
}
