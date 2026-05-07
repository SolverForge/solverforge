use solverforge::{SolverEvent, SolverManager, SolverTerminalReason};

mod domain;

use domain::{JobShopPlan, Machine, MachineSequence, Operation};

static MANAGER: SolverManager<JobShopPlan> = SolverManager::new();

fn main() {
    let operation_values: Vec<usize> = (0..6).collect();
    let plan = JobShopPlan {
        machines: (0..2).map(|id| Machine { id }).collect(),
        operations: operation_values
            .iter()
            .copied()
            .map(|id| Operation {
                id,
                job: id / 2,
                step: id % 2,
                machine_idx: None,
            })
            .collect(),
        operation_values,
        machine_sequences: (0..2)
            .map(|id| MachineSequence {
                id,
                operations: Vec::new(),
            })
            .collect(),
        score: None,
    };

    let (job_id, mut events) = MANAGER.solve(plan).expect("solver job should start");

    while let Some(event) = events.blocking_recv() {
        match event {
            SolverEvent::Completed { metadata, solution } => {
                assert_eq!(
                    metadata.terminal_reason,
                    Some(SolverTerminalReason::Completed)
                );
                println!("score: {}", solution.score.expect("completed score"));
                for operation in &solution.operations {
                    println!(
                        "operation {} job {} step {} -> machine {:?}",
                        operation.id, operation.job, operation.step, operation.machine_idx
                    );
                }
                for sequence in solution.machine_sequences {
                    println!("machine {} sequence {:?}", sequence.id, sequence.operations);
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
