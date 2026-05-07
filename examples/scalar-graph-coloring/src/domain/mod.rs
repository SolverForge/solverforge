solverforge::planning_model! {
    root = "examples/scalar-graph-coloring/src/domain";

    mod color;
    mod graph_coloring;
    mod node;

    pub use color::Color;
    pub use graph_coloring::GraphColoring;
    pub use node::Node;
}
