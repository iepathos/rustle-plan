use crate::planner::error::PlanError;
use crate::types::*;
use std::collections::HashMap;
use std::time::Duration;

pub struct TaskEstimator {
    module_durations: HashMap<String, Duration>,
}

impl TaskEstimator {
    pub fn new() -> Self {
        let mut module_durations = HashMap::new();

        // Default duration estimates for common modules
        module_durations.insert("debug".to_string(), Duration::from_millis(100));
        module_durations.insert("assert".to_string(), Duration::from_millis(100));
        module_durations.insert("fail".to_string(), Duration::from_millis(100));
        module_durations.insert("meta".to_string(), Duration::from_millis(100));

        module_durations.insert("file".to_string(), Duration::from_secs(1));
        module_durations.insert("copy".to_string(), Duration::from_secs(2));
        module_durations.insert("template".to_string(), Duration::from_secs(2));
        module_durations.insert("lineinfile".to_string(), Duration::from_secs(1));

        module_durations.insert("shell".to_string(), Duration::from_secs(3));
        module_durations.insert("command".to_string(), Duration::from_secs(3));
        module_durations.insert("raw".to_string(), Duration::from_secs(3));

        module_durations.insert("package".to_string(), Duration::from_secs(30));
        module_durations.insert("yum".to_string(), Duration::from_secs(30));
        module_durations.insert("apt".to_string(), Duration::from_secs(30));

        module_durations.insert("service".to_string(), Duration::from_secs(5));
        module_durations.insert("systemd".to_string(), Duration::from_secs(5));

        module_durations.insert("user".to_string(), Duration::from_secs(2));
        module_durations.insert("group".to_string(), Duration::from_secs(2));
        module_durations.insert("cron".to_string(), Duration::from_secs(1));

        Self { module_durations }
    }

    pub fn estimate_task_duration(&self, task: &ParsedTask) -> Option<Duration> {
        let base_duration = self
            .module_durations
            .get(&task.module)
            .copied()
            .unwrap_or(Duration::from_secs(5)); // Default fallback

        // Adjust based on task complexity
        let complexity_multiplier = self.calculate_complexity_multiplier(task);

        Some(Duration::from_nanos(
            (base_duration.as_nanos() as f64 * complexity_multiplier) as u64,
        ))
    }

    pub fn estimate_plan_duration(&self, plan: &ExecutionPlan) -> Result<Duration, PlanError> {
        let mut total_duration = Duration::ZERO;

        for play in &plan.plays {
            let play_duration = self.estimate_play_duration(play)?;
            total_duration += play_duration;
        }

        Ok(total_duration)
    }

    pub fn estimate_play_duration(&self, play: &PlayPlan) -> Result<Duration, PlanError> {
        match &play.strategy {
            ExecutionStrategy::Linear => {
                // Sequential execution - sum all batch durations
                let mut total_duration = Duration::ZERO;
                for batch in &play.batches {
                    let batch_duration = self.estimate_batch_duration(batch)?;
                    total_duration += batch_duration;
                }
                Ok(total_duration)
            }
            ExecutionStrategy::Free => {
                // Parallel execution - take the maximum batch duration
                let max_duration = play
                    .batches
                    .iter()
                    .map(|batch| self.estimate_batch_duration(batch))
                    .try_fold(Duration::ZERO, |acc, duration| {
                        Ok::<Duration, PlanError>(acc.max(duration?))
                    })?;
                Ok(max_duration)
            }
            ExecutionStrategy::Rolling { .. } => {
                // Rolling deployment - sum batch durations but account for overlap
                let mut total_duration = Duration::ZERO;
                for batch in &play.batches {
                    let batch_duration = self.estimate_batch_duration(batch)?;
                    // Rolling updates have some parallelism, so reduce total time
                    total_duration +=
                        Duration::from_nanos((batch_duration.as_nanos() as f64 * 0.8) as u64);
                }
                Ok(total_duration)
            }
            ExecutionStrategy::BinaryHybrid | ExecutionStrategy::BinaryOnly => {
                // Binary deployment reduces execution time significantly
                let traditional_duration = play
                    .batches
                    .iter()
                    .map(|batch| self.estimate_batch_duration(batch))
                    .try_fold(Duration::ZERO, |acc, duration| {
                        Ok::<Duration, PlanError>(acc + duration?)
                    })?;

                // Binary deployment overhead + reduced execution time
                let binary_overhead = Duration::from_secs(10); // Upload and start binary
                let execution_speedup = 0.3; // 70% time reduction

                Ok(binary_overhead
                    + Duration::from_nanos(
                        (traditional_duration.as_nanos() as f64 * execution_speedup) as u64,
                    ))
            }
            _ => {
                // Default to linear estimation
                let mut total_duration = Duration::ZERO;
                for batch in &play.batches {
                    let batch_duration = self.estimate_batch_duration(batch)?;
                    total_duration += batch_duration;
                }
                Ok(total_duration)
            }
        }
    }

    fn estimate_batch_duration(&self, batch: &ExecutionBatch) -> Result<Duration, PlanError> {
        if batch.tasks.is_empty() {
            return Ok(Duration::ZERO);
        }

        // For parallel groups, take the maximum task duration
        // For sequential tasks, sum the durations
        let parallel_task_count = batch
            .parallel_groups
            .iter()
            .map(|group| group.tasks.len())
            .sum::<usize>();

        if parallel_task_count > 0 {
            // Mixed parallel and sequential execution
            let parallel_duration = batch
                .parallel_groups
                .iter()
                .map(|group| {
                    group
                        .tasks
                        .iter()
                        .filter_map(|task_id| {
                            batch
                                .tasks
                                .iter()
                                .find(|t| &t.task_id == task_id)
                                .and_then(|task| task.estimated_duration)
                        })
                        .max()
                        .unwrap_or(Duration::ZERO)
                })
                .max()
                .unwrap_or(Duration::ZERO);

            let sequential_duration: Duration = batch
                .tasks
                .iter()
                .filter(|task| {
                    !batch
                        .parallel_groups
                        .iter()
                        .any(|group| group.tasks.contains(&task.task_id))
                })
                .filter_map(|task| task.estimated_duration)
                .sum();

            Ok(parallel_duration + sequential_duration)
        } else {
            // All tasks are sequential
            let total_duration: Duration = batch
                .tasks
                .iter()
                .filter_map(|task| task.estimated_duration)
                .sum();
            Ok(total_duration)
        }
    }

    fn calculate_complexity_multiplier(&self, task: &ParsedTask) -> f64 {
        let mut multiplier = 1.0;

        // More arguments = more complexity
        let arg_count = task.args.len();
        if arg_count > 5 {
            multiplier *= 1.2;
        } else if arg_count > 10 {
            multiplier *= 1.5;
        }

        // Conditional tasks take longer
        if task.when.is_some() {
            multiplier *= 1.1;
        }

        // Tasks with notifications have overhead
        if !task.notify.is_empty() {
            multiplier *= 1.1;
        }

        // Module-specific adjustments
        match task.module.as_str() {
            "shell" | "command" | "raw" => {
                // Command execution can vary widely
                multiplier *= 1.5;
            }
            "package" => {
                // Package operations can be slow on first install
                if let Some(state) = task.args.get("state").and_then(|v| v.as_str()) {
                    if state == "present" || state == "latest" {
                        multiplier *= 2.0;
                    }
                }
            }
            "copy" | "template" => {
                // File size affects copy time (simplified estimation)
                if task.args.contains_key("backup") {
                    multiplier *= 1.3;
                }
            }
            _ => {}
        }

        multiplier
    }
}

impl Default for TaskEstimator {
    fn default() -> Self {
        Self::new()
    }
}
