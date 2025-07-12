use crate::types::*;

pub struct StrategyPlanner;

impl StrategyPlanner {
    pub fn new() -> Self {
        Self
    }

    pub fn plan_strategy(
        &self,
        strategy: &ExecutionStrategy,
        tasks: &[TaskPlan],
        hosts: &[String],
    ) -> Vec<ExecutionBatch> {
        match strategy {
            ExecutionStrategy::Linear => self.plan_linear(tasks, hosts),
            ExecutionStrategy::Free => self.plan_free(tasks, hosts),
            ExecutionStrategy::Rolling { batch_size } => {
                self.plan_rolling(tasks, hosts, *batch_size)
            }
            ExecutionStrategy::HostPinned => self.plan_host_pinned(tasks, hosts),
            ExecutionStrategy::BinaryHybrid => self.plan_binary_hybrid(tasks, hosts),
            ExecutionStrategy::BinaryOnly => self.plan_binary_only(tasks, hosts),
        }
    }

    fn plan_linear(&self, tasks: &[TaskPlan], hosts: &[String]) -> Vec<ExecutionBatch> {
        tasks
            .iter()
            .enumerate()
            .map(|(index, task)| ExecutionBatch {
                batch_id: format!("linear-batch-{index}"),
                hosts: hosts.to_vec(),
                tasks: vec![(*task).clone()],
                parallel_groups: Vec::new(),
                dependencies: if index > 0 {
                    vec![format!("linear-batch-{}", index - 1)]
                } else {
                    Vec::new()
                },
                estimated_duration: task.estimated_duration,
            })
            .collect()
    }

    fn plan_free(&self, tasks: &[TaskPlan], hosts: &[String]) -> Vec<ExecutionBatch> {
        let (parallel_tasks, sequential_tasks): (Vec<_>, Vec<_>) =
            tasks.iter().partition(|task| task.can_run_parallel);

        let mut batches = Vec::new();

        if !parallel_tasks.is_empty() {
            batches.push(ExecutionBatch {
                batch_id: "free-parallel".to_string(),
                hosts: hosts.to_vec(),
                tasks: parallel_tasks.into_iter().cloned().collect(),
                parallel_groups: Vec::new(),
                dependencies: Vec::new(),
                estimated_duration: None,
            });
        }

        for (index, task) in sequential_tasks.iter().enumerate() {
            batches.push(ExecutionBatch {
                batch_id: format!("free-sequential-{index}"),
                hosts: hosts.to_vec(),
                tasks: vec![(*task).clone()],
                parallel_groups: Vec::new(),
                dependencies: if index > 0 {
                    vec![format!("free-sequential-{}", index - 1)]
                } else if !batches.is_empty() {
                    vec!["free-parallel".to_string()]
                } else {
                    Vec::new()
                },
                estimated_duration: task.estimated_duration,
            });
        }

        batches
    }

    fn plan_rolling(
        &self,
        tasks: &[TaskPlan],
        hosts: &[String],
        batch_size: u32,
    ) -> Vec<ExecutionBatch> {
        let host_chunks: Vec<Vec<String>> = hosts
            .chunks(batch_size as usize)
            .map(|chunk| chunk.to_vec())
            .collect();

        let mut batches = Vec::new();

        for (chunk_index, host_chunk) in host_chunks.iter().enumerate() {
            let batch_tasks: Vec<TaskPlan> = tasks
                .iter()
                .map(|task| {
                    let mut task_clone = task.clone();
                    task_clone.hosts = host_chunk.clone();
                    task_clone
                })
                .collect();

            batches.push(ExecutionBatch {
                batch_id: format!("rolling-{chunk_index}"),
                hosts: host_chunk.clone(),
                tasks: batch_tasks,
                parallel_groups: Vec::new(),
                dependencies: if chunk_index > 0 {
                    vec![format!("rolling-{}", chunk_index - 1)]
                } else {
                    Vec::new()
                },
                estimated_duration: None,
            });
        }

        batches
    }

    fn plan_host_pinned(&self, tasks: &[TaskPlan], hosts: &[String]) -> Vec<ExecutionBatch> {
        hosts
            .iter()
            .enumerate()
            .map(|(index, host)| {
                let host_tasks: Vec<TaskPlan> = tasks
                    .iter()
                    .map(|task| {
                        let mut task_clone = task.clone();
                        task_clone.hosts = vec![host.clone()];
                        task_clone
                    })
                    .collect();

                ExecutionBatch {
                    batch_id: format!("host-{index}"),
                    hosts: vec![host.clone()],
                    tasks: host_tasks,
                    parallel_groups: Vec::new(),
                    dependencies: Vec::new(),
                    estimated_duration: None,
                }
            })
            .collect()
    }

    fn plan_binary_hybrid(&self, tasks: &[TaskPlan], hosts: &[String]) -> Vec<ExecutionBatch> {
        // For now, use linear strategy - binary deployment is handled separately
        self.plan_linear(tasks, hosts)
    }

    fn plan_binary_only(&self, tasks: &[TaskPlan], hosts: &[String]) -> Vec<ExecutionBatch> {
        // For now, use linear strategy - binary deployment is handled separately
        self.plan_linear(tasks, hosts)
    }
}

impl Default for StrategyPlanner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn create_test_task(id: &str, can_parallel: bool) -> TaskPlan {
        TaskPlan {
            task_id: id.to_string(),
            name: format!("Test task {}", id),
            module: "shell".to_string(),
            args: std::collections::HashMap::new(),
            hosts: vec!["host1".to_string()],
            dependencies: Vec::new(),
            conditions: Vec::new(),
            tags: Vec::new(),
            notify: Vec::new(),
            execution_order: 0,
            can_run_parallel: can_parallel,
            estimated_duration: Some(Duration::from_secs(1)),
            risk_level: RiskLevel::Low,
        }
    }

    #[test]
    fn test_new_and_default() {
        let planner1 = StrategyPlanner::new();
        let planner2 = StrategyPlanner::default();

        // Both should create instances successfully
        let _ = (planner1, planner2);
    }

    #[test]
    fn test_plan_linear_strategy() {
        let planner = StrategyPlanner::new();
        let tasks = vec![
            create_test_task("task1", true),
            create_test_task("task2", true),
            create_test_task("task3", false),
        ];
        let hosts = vec!["host1".to_string(), "host2".to_string()];

        let batches = planner.plan_linear(&tasks, &hosts);

        assert_eq!(batches.len(), 3);
        assert_eq!(batches[0].batch_id, "linear-batch-0");
        assert_eq!(batches[1].batch_id, "linear-batch-1");
        assert_eq!(batches[2].batch_id, "linear-batch-2");

        // Each batch should have one task
        for (i, batch) in batches.iter().enumerate() {
            assert_eq!(batch.tasks.len(), 1);
            assert_eq!(batch.tasks[0].task_id, format!("task{}", i + 1));
            assert_eq!(batch.hosts, hosts);
        }

        // Check dependencies
        assert!(batches[0].dependencies.is_empty());
        assert_eq!(batches[1].dependencies, vec!["linear-batch-0"]);
        assert_eq!(batches[2].dependencies, vec!["linear-batch-1"]);
    }

    #[test]
    fn test_plan_free_strategy() {
        let planner = StrategyPlanner::new();
        let tasks = vec![
            create_test_task("task1", true),
            create_test_task("task2", true),
            create_test_task("task3", false),
            create_test_task("task4", false),
        ];
        let hosts = vec!["host1".to_string(), "host2".to_string()];

        let batches = planner.plan_free(&tasks, &hosts);

        // Should have one parallel batch and two sequential batches
        assert_eq!(batches.len(), 3);
        assert_eq!(batches[0].batch_id, "free-parallel");
        assert_eq!(batches[0].tasks.len(), 2); // Two parallel tasks

        assert_eq!(batches[1].batch_id, "free-sequential-0");
        assert_eq!(batches[1].tasks.len(), 1);
        assert_eq!(batches[1].dependencies, vec!["free-parallel"]);

        assert_eq!(batches[2].batch_id, "free-sequential-1");
        assert_eq!(batches[2].tasks.len(), 1);
        assert_eq!(batches[2].dependencies, vec!["free-sequential-0"]);
    }

    #[test]
    fn test_plan_free_strategy_only_sequential() {
        let planner = StrategyPlanner::new();
        let tasks = vec![
            create_test_task("task1", false),
            create_test_task("task2", false),
        ];
        let hosts = vec!["host1".to_string()];

        let batches = planner.plan_free(&tasks, &hosts);

        assert_eq!(batches.len(), 2);
        assert_eq!(batches[0].batch_id, "free-sequential-0");
        assert!(batches[0].dependencies.is_empty());

        assert_eq!(batches[1].batch_id, "free-sequential-1");
        assert_eq!(batches[1].dependencies, vec!["free-sequential-0"]);
    }

    #[test]
    fn test_plan_rolling_strategy() {
        let planner = StrategyPlanner::new();
        let tasks = vec![
            create_test_task("task1", true),
            create_test_task("task2", true),
        ];
        let hosts = vec![
            "host1".to_string(),
            "host2".to_string(),
            "host3".to_string(),
            "host4".to_string(),
            "host5".to_string(),
        ];

        let batches = planner.plan_rolling(&tasks, &hosts, 2);

        // With 5 hosts and batch size 2, should have 3 batches
        assert_eq!(batches.len(), 3);

        // First batch should have 2 hosts
        assert_eq!(batches[0].batch_id, "rolling-0");
        assert_eq!(batches[0].hosts, vec!["host1", "host2"]);
        assert_eq!(batches[0].tasks.len(), 2);
        assert!(batches[0].dependencies.is_empty());

        // Second batch should have 2 hosts
        assert_eq!(batches[1].batch_id, "rolling-1");
        assert_eq!(batches[1].hosts, vec!["host3", "host4"]);
        assert_eq!(batches[1].dependencies, vec!["rolling-0"]);

        // Third batch should have 1 host
        assert_eq!(batches[2].batch_id, "rolling-2");
        assert_eq!(batches[2].hosts, vec!["host5"]);
        assert_eq!(batches[2].dependencies, vec!["rolling-1"]);
    }

    #[test]
    fn test_plan_host_pinned_strategy() {
        let planner = StrategyPlanner::new();
        let tasks = vec![
            create_test_task("task1", true),
            create_test_task("task2", false),
        ];
        let hosts = vec![
            "host1".to_string(),
            "host2".to_string(),
            "host3".to_string(),
        ];

        let batches = planner.plan_host_pinned(&tasks, &hosts);

        // Should have one batch per host
        assert_eq!(batches.len(), 3);

        for (i, batch) in batches.iter().enumerate() {
            assert_eq!(batch.batch_id, format!("host-{}", i));
            assert_eq!(batch.hosts, vec![format!("host{}", i + 1)]);
            assert_eq!(batch.tasks.len(), 2); // Each host gets all tasks
            assert!(batch.dependencies.is_empty()); // Host-pinned runs in parallel

            // Verify tasks are assigned to correct host
            for task in &batch.tasks {
                assert_eq!(task.hosts, vec![format!("host{}", i + 1)]);
            }
        }
    }

    #[test]
    fn test_plan_binary_hybrid_strategy() {
        let planner = StrategyPlanner::new();
        let tasks = vec![create_test_task("task1", true)];
        let hosts = vec!["host1".to_string()];

        // Binary hybrid should currently use linear strategy
        let batches = planner.plan_binary_hybrid(&tasks, &hosts);
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].batch_id, "linear-batch-0");
    }

    #[test]
    fn test_plan_binary_only_strategy() {
        let planner = StrategyPlanner::new();
        let tasks = vec![create_test_task("task1", true)];
        let hosts = vec!["host1".to_string()];

        // Binary only should currently use linear strategy
        let batches = planner.plan_binary_only(&tasks, &hosts);
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].batch_id, "linear-batch-0");
    }

    #[test]
    fn test_plan_strategy_all_variants() {
        let planner = StrategyPlanner::new();
        let tasks = vec![
            create_test_task("task1", true),
            create_test_task("task2", false),
        ];
        let hosts = vec!["host1".to_string(), "host2".to_string()];

        let strategies = vec![
            ExecutionStrategy::Linear,
            ExecutionStrategy::Free,
            ExecutionStrategy::Rolling { batch_size: 1 },
            ExecutionStrategy::HostPinned,
            ExecutionStrategy::BinaryHybrid,
            ExecutionStrategy::BinaryOnly,
        ];

        for strategy in strategies {
            let batches = planner.plan_strategy(&strategy, &tasks, &hosts);
            assert!(
                !batches.is_empty(),
                "Strategy {:?} produced no batches",
                strategy
            );
        }
    }

    #[test]
    fn test_empty_tasks() {
        let planner = StrategyPlanner::new();
        let tasks = vec![];
        let hosts = vec!["host1".to_string()];

        let batches = planner.plan_linear(&tasks, &hosts);
        assert!(batches.is_empty());

        let batches = planner.plan_free(&tasks, &hosts);
        assert!(batches.is_empty());
    }

    #[test]
    fn test_single_host() {
        let planner = StrategyPlanner::new();
        let tasks = vec![create_test_task("task1", true)];
        let hosts = vec!["host1".to_string()];

        let batches = planner.plan_strategy(&ExecutionStrategy::Linear, &tasks, &hosts);
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].hosts, vec!["host1"]);
    }
}
