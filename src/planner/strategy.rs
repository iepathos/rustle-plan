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
