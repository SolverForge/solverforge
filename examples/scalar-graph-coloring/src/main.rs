use solverforge::{SolverEvent, SolverManager, SolverTerminalReason};

mod domain;

use domain::{Color, GraphColoring, Node};

static MANAGER: SolverManager<GraphColoring> = SolverManager::new();

fn main() {
    let graph = GraphColoring {
        colors: vec![
            Color {
                id: 0,
                name: "red".to_string(),
            },
            Color {
                id: 1,
                name: "blue".to_string(),
            },
            Color {
                id: 2,
                name: "green".to_string(),
            },
        ],
        nodes: vec![
            Node {
                id: 0,
                neighbors: vec![1, 2],
                color_idx: None,
            },
            Node {
                id: 1,
                neighbors: vec![0, 2, 3],
                color_idx: None,
            },
            Node {
                id: 2,
                neighbors: vec![0, 1, 4],
                color_idx: None,
            },
            Node {
                id: 3,
                neighbors: vec![1, 4],
                color_idx: None,
            },
            Node {
                id: 4,
                neighbors: vec![2, 3],
                color_idx: None,
            },
        ],
        score: None,
    };

    let (job_id, mut events) = MANAGER.solve(graph).expect("solver job should start");

    while let Some(event) = events.blocking_recv() {
        match event {
            SolverEvent::Completed { metadata, solution } => {
                assert_eq!(
                    metadata.terminal_reason,
                    Some(SolverTerminalReason::Completed)
                );
                println!("score: {}", solution.score.expect("completed score"));
                for node in solution.nodes {
                    let color = node
                        .color_idx
                        .and_then(|idx| solution.colors.get(idx))
                        .map(|color| color.name.as_str())
                        .unwrap_or("unassigned");
                    println!("node {} -> {color}", node.id);
                }
                MANAGER.delete(job_id).expect("delete completed job");
                break;
            }
            SolverEvent::Failed { error, .. } => panic!("solver failed: {error}"),
            SolverEvent::Cancelled { .. } => panic!("solver was cancelled"),
            _ => {}
        }
    }
}
