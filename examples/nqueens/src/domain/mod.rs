solverforge::planning_model! {
    root = "examples/nqueens/src/domain";

    mod board;
    mod queen;
    mod row;

    pub use board::Board;
    pub use queen::Queen;
    pub use row::Row;
}
