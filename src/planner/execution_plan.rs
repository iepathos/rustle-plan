use crate::planner::*;
use crate::types::*;
use anyhow::Result;
use chrono::Utc;
use std::time::Duration;

pub struct ExecutionPlanner {
    strategy: ExecutionStrategy,
    forks: u32,
    optimize: bool,
    check_mode: bool,
    task_estimator: TaskEstimator,
    binary_planner: BinaryDeploymentPlanner,
    binary_threshold: u32,
}

impl ExecutionPlanner {
    pub fn new() -> Self {
        Self {
            strategy: ExecutionStrategy::default(),
            forks: 50,
            optimize: false,
            check_mode: false,
            task_estimator: TaskEstimator::new(),
            binary_planner: BinaryDeploymentPlanner::new(),
            binary_threshold: 5,
        }
    }

    pub fn with_strategy(mut self, strategy: ExecutionStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    pub fn with_forks(mut self, forks: u32) -> Self {
        self.forks = forks;
        self
    }

    pub fn with_optimization(mut self, enabled: bool) -> Self {
        self.optimize = enabled;
        self
    }

    pub fn with_check_mode(mut self, enabled: bool) -> Self {
        self.check_mode = enabled;
        self
    }

    pub fn with_binary_threshold(mut self, threshold: u32) -> Self {
        self.binary_threshold = threshold;
        self
    }

    pub fn plan_execution(
        &self,
        playbook: &ParsedPlaybook,
        inventory: &ParsedInventory,
        options: &PlanningOptions,
    ) -> Result<ExecutionPlan, PlanError> {
        let start_time = std::time::Instant::now();

        // Apply host filtering
        let filtered_hosts = self.filter_hosts(&inventory.hosts, &options.limit)?;

        // Plan each play
        let mut plays = Vec::new();
        let mut all_binary_deployments = Vec::new();
        let mut total_tasks = 0;

        for (play_index, parsed_play) in playbook.plays.iter().enumerate() {
            let play_hosts = self.resolve_play_hosts(parsed_play, &filtered_hosts, inventory)?;

            // Filter tasks by tags
            let filtered_tasks = self.filter_tasks_by_tags(&parsed_play.tasks, options)?;
            total_tasks += filtered_tasks.len();

            // Analyze dependencies
            let _dependency_graph = self.analyze_dependencies(&filtered_tasks)?;

            // Convert parsed tasks to task plans
            let mut task_plans = self.create_task_plans(&filtered_tasks, &play_hosts)?;

            // Optimize execution order if enabled
            if self.optimize {
                task_plans = self.optimize_execution_order(&task_plans)?;
            }

            // Create execution batches based on strategy
            let batches =
                self.create_execution_batches(&task_plans, &options.strategy, options.serial)?;

            // Plan binary deployments for this play
            let binary_deployments = if !options.force_ssh {
                self.plan_binary_deployments(&task_plans, &play_hosts)?
            } else {
                Vec::new()
            };

            all_binary_deployments.extend(binary_deployments);

            // Create handlers plans
            let handler_plans = self.create_handler_plans(&parsed_play.handlers)?;

            let play_plan = PlayPlan {
                play_id: format!("play-{play_index}"),
                name: parsed_play.name.clone(),
                strategy: options.strategy.clone(),
                serial: options.serial,
                hosts: play_hosts,
                batches,
                handlers: handler_plans,
                estimated_duration: None, // Will be calculated later
            };

            plays.push(play_plan);
        }

        // Estimate durations
        let estimated_duration = if options.strategy != ExecutionStrategy::BinaryOnly {
            Some(self.estimate_duration_for_plays(&plays)?)
        } else {
            None
        };

        let estimated_compilation_time = if !all_binary_deployments.is_empty() {
            Some(self.estimate_compilation_time(&all_binary_deployments)?)
        } else {
            None
        };

        // Calculate scores
        let parallelism_score = self.calculate_parallelism_score(&plays);
        let network_efficiency_score =
            self.calculate_network_efficiency_score(&plays, &all_binary_deployments);

        let execution_plan = ExecutionPlan {
            metadata: PlanMetadata {
                created_at: Utc::now(),
                rustle_version: env!("CARGO_PKG_VERSION").to_string(),
                playbook_hash: self.calculate_playbook_hash(playbook)?,
                inventory_hash: self.calculate_inventory_hash(inventory)?,
                planning_options: options.clone(),
            },
            plays,
            binary_deployments: all_binary_deployments,
            total_tasks,
            estimated_duration,
            estimated_compilation_time,
            parallelism_score,
            network_efficiency_score,
            hosts: filtered_hosts,
        };

        let planning_duration = start_time.elapsed();
        tracing::info!(
            "Execution planning completed in {:?} for {} tasks across {} hosts",
            planning_duration,
            total_tasks,
            execution_plan.hosts.len()
        );

        Ok(execution_plan)
    }

    fn filter_hosts(
        &self,
        hosts: &[String],
        limit: &Option<String>,
    ) -> Result<Vec<String>, PlanError> {
        if let Some(pattern) = limit {
            // Simple pattern matching for now
            // In a real implementation, this would support complex patterns
            if pattern == "all" {
                Ok(hosts.to_vec())
            } else {
                let filtered: Vec<String> = hosts
                    .iter()
                    .filter(|host| host.contains(pattern))
                    .cloned()
                    .collect();

                if filtered.is_empty() {
                    return Err(PlanError::InvalidHostPattern {
                        pattern: pattern.clone(),
                        reason: "No hosts match the pattern".to_string(),
                    });
                }

                Ok(filtered)
            }
        } else {
            Ok(hosts.to_vec())
        }
    }

    fn resolve_play_hosts(
        &self,
        play: &ParsedPlay,
        available_hosts: &[String],
        _inventory: &ParsedInventory,
    ) -> Result<Vec<String>, PlanError> {
        // Simple host resolution - in a real implementation this would handle groups
        if play.hosts.contains(&"all".to_string()) {
            Ok(available_hosts.to_vec())
        } else {
            let resolved: Vec<String> = play
                .hosts
                .iter()
                .filter(|host| available_hosts.contains(host))
                .cloned()
                .collect();
            Ok(resolved)
        }
    }

    fn filter_tasks_by_tags(
        &self,
        tasks: &[ParsedTask],
        options: &PlanningOptions,
    ) -> Result<Vec<ParsedTask>, PlanError> {
        let mut filtered_tasks = Vec::new();

        for task in tasks {
            // Check skip tags first
            if !options.skip_tags.is_empty() {
                let should_skip = task.tags.iter().any(|tag| options.skip_tags.contains(tag));
                if should_skip {
                    continue;
                }
            }

            // Check required tags
            if !options.tags.is_empty() {
                let has_required_tag = task.tags.iter().any(|tag| options.tags.contains(tag));
                if !has_required_tag {
                    continue;
                }
            }

            filtered_tasks.push(task.clone());
        }

        Ok(filtered_tasks)
    }

    fn create_task_plans(
        &self,
        tasks: &[ParsedTask],
        hosts: &[String],
    ) -> Result<Vec<TaskPlan>, PlanError> {
        let mut task_plans = Vec::new();

        for (index, task) in tasks.iter().enumerate() {
            let risk_level = self.assess_task_risk(&task.module);
            let can_run_parallel = self.can_task_run_parallel(task, &risk_level);

            let task_plan = TaskPlan {
                task_id: task.id.clone(),
                name: task.name.clone(),
                module: task.module.clone(),
                args: task.args.clone(),
                hosts: hosts.to_vec(),
                dependencies: task.dependencies.clone(),
                conditions: self.create_execution_conditions(task)?,
                tags: task.tags.clone(),
                notify: task.notify.clone(),
                execution_order: index as u32,
                can_run_parallel,
                estimated_duration: self.task_estimator.estimate_task_duration(task),
                risk_level,
            };

            task_plans.push(task_plan);
        }

        Ok(task_plans)
    }

    fn create_execution_conditions(
        &self,
        task: &ParsedTask,
    ) -> Result<Vec<ExecutionCondition>, PlanError> {
        let mut conditions = Vec::new();

        if let Some(when_expr) = &task.when {
            conditions.push(ExecutionCondition::When {
                expression: when_expr.clone(),
            });
        }

        if !task.tags.is_empty() {
            conditions.push(ExecutionCondition::Tag {
                tags: task.tags.clone(),
            });
        }

        if self.check_mode {
            conditions.push(ExecutionCondition::CheckMode { enabled: true });
        }

        Ok(conditions)
    }

    fn assess_task_risk(&self, module: &str) -> RiskLevel {
        match module {
            "debug" | "assert" | "fail" | "meta" => RiskLevel::Low,
            "copy" | "template" | "file" | "lineinfile" => RiskLevel::Medium,
            "service" | "systemd" | "package" | "yum" | "apt" => RiskLevel::High,
            "shell" | "command" | "raw" => RiskLevel::Critical,
            _ => RiskLevel::Medium,
        }
    }

    fn can_task_run_parallel(&self, task: &ParsedTask, risk_level: &RiskLevel) -> bool {
        // Tasks that modify the same resources or have high risk generally can't run in parallel
        match risk_level {
            RiskLevel::Critical => false,
            RiskLevel::High => {
                // Package installations and service operations should be serialized
                matches!(task.module.as_str(), "debug" | "assert" | "meta")
            }
            _ => true,
        }
    }

    fn create_execution_batches(
        &self,
        tasks: &[TaskPlan],
        strategy: &ExecutionStrategy,
        serial: Option<u32>,
    ) -> Result<Vec<ExecutionBatch>, PlanError> {
        match strategy {
            ExecutionStrategy::Linear => {
                // All tasks in sequence, one batch per task
                let batches: Vec<ExecutionBatch> = tasks
                    .iter()
                    .enumerate()
                    .map(|(index, task)| ExecutionBatch {
                        batch_id: format!("batch-{index}"),
                        hosts: task.hosts.clone(),
                        tasks: vec![task.clone()],
                        parallel_groups: Vec::new(),
                        dependencies: if index > 0 {
                            vec![format!("batch-{}", index - 1)]
                        } else {
                            Vec::new()
                        },
                        estimated_duration: task.estimated_duration,
                    })
                    .collect();
                Ok(batches)
            }
            ExecutionStrategy::Free => {
                // All tasks that can run in parallel
                let parallel_tasks: Vec<TaskPlan> = tasks
                    .iter()
                    .filter(|task| task.can_run_parallel)
                    .cloned()
                    .collect();

                let sequential_tasks: Vec<TaskPlan> = tasks
                    .iter()
                    .filter(|task| !task.can_run_parallel)
                    .cloned()
                    .collect();

                let mut batches = Vec::new();

                // Add parallel batch if any
                if !parallel_tasks.is_empty() {
                    batches.push(ExecutionBatch {
                        batch_id: "parallel-batch".to_string(),
                        hosts: parallel_tasks[0].hosts.clone(),
                        tasks: parallel_tasks,
                        parallel_groups: Vec::new(),
                        dependencies: Vec::new(),
                        estimated_duration: None,
                    });
                }

                // Add sequential batches
                for (index, task) in sequential_tasks.iter().enumerate() {
                    batches.push(ExecutionBatch {
                        batch_id: format!("sequential-batch-{index}"),
                        hosts: task.hosts.clone(),
                        tasks: vec![task.clone()],
                        parallel_groups: Vec::new(),
                        dependencies: if index > 0 {
                            vec![format!("sequential-batch-{}", index - 1)]
                        } else if !batches.is_empty() {
                            vec!["parallel-batch".to_string()]
                        } else {
                            Vec::new()
                        },
                        estimated_duration: task.estimated_duration,
                    });
                }

                Ok(batches)
            }
            ExecutionStrategy::Rolling { batch_size } => {
                // Rolling deployment with specified batch size
                let batch_size = serial.unwrap_or(*batch_size) as usize;
                let host_count = tasks.first().map(|t| t.hosts.len()).unwrap_or(0);

                if host_count == 0 {
                    return Ok(Vec::new());
                }

                let num_batches = host_count.div_ceil(batch_size);
                let mut batches = Vec::new();

                for batch_index in 0..num_batches {
                    let start_host = batch_index * batch_size;
                    let end_host = std::cmp::min(start_host + batch_size, host_count);

                    let batch_hosts: Vec<String> = tasks[0].hosts[start_host..end_host].to_vec();

                    let batch_tasks: Vec<TaskPlan> = tasks
                        .iter()
                        .map(|task| {
                            let mut task_clone = task.clone();
                            task_clone.hosts = batch_hosts.clone();
                            task_clone
                        })
                        .collect();

                    batches.push(ExecutionBatch {
                        batch_id: format!("rolling-batch-{batch_index}"),
                        hosts: batch_hosts,
                        tasks: batch_tasks,
                        parallel_groups: Vec::new(),
                        dependencies: if batch_index > 0 {
                            vec![format!("rolling-batch-{}", batch_index - 1)]
                        } else {
                            Vec::new()
                        },
                        estimated_duration: None,
                    });
                }

                Ok(batches)
            }
            _ => {
                // For binary strategies, create simple batches for now
                Ok(vec![ExecutionBatch {
                    batch_id: "binary-batch".to_string(),
                    hosts: tasks.first().map(|t| t.hosts.clone()).unwrap_or_default(),
                    tasks: tasks.to_vec(),
                    parallel_groups: Vec::new(),
                    dependencies: Vec::new(),
                    estimated_duration: None,
                }])
            }
        }
    }

    fn create_handler_plans(
        &self,
        handlers: &[ParsedHandler],
    ) -> Result<Vec<HandlerPlan>, PlanError> {
        let mut handler_plans = Vec::new();

        for (index, handler) in handlers.iter().enumerate() {
            let conditions = if let Some(when_expr) = &handler.when {
                vec![ExecutionCondition::When {
                    expression: when_expr.clone(),
                }]
            } else {
                Vec::new()
            };

            handler_plans.push(HandlerPlan {
                handler_id: handler.id.clone(),
                name: handler.name.clone(),
                module: handler.module.clone(),
                args: handler.args.clone(),
                conditions,
                execution_order: index as u32,
            });
        }

        Ok(handler_plans)
    }

    fn calculate_playbook_hash(&self, playbook: &ParsedPlaybook) -> Result<String, PlanError> {
        let serialized = serde_json::to_string(playbook)?;
        Ok(format!("{:x}", md5::compute(serialized.as_bytes())))
    }

    fn calculate_inventory_hash(&self, inventory: &ParsedInventory) -> Result<String, PlanError> {
        let serialized = serde_json::to_string(inventory)?;
        Ok(format!("{:x}", md5::compute(serialized.as_bytes())))
    }

    fn calculate_parallelism_score(&self, plays: &[PlayPlan]) -> f32 {
        let total_tasks: usize = plays
            .iter()
            .map(|p| p.batches.iter().map(|b| b.tasks.len()).sum::<usize>())
            .sum();

        if total_tasks == 0 {
            return 0.0;
        }

        let parallel_tasks: usize = plays
            .iter()
            .map(|p| {
                p.batches
                    .iter()
                    .map(|b| b.tasks.iter().filter(|t| t.can_run_parallel).count())
                    .sum::<usize>()
            })
            .sum();

        (parallel_tasks as f32) / (total_tasks as f32)
    }

    fn calculate_network_efficiency_score(
        &self,
        plays: &[PlayPlan],
        binary_deployments: &[BinaryDeployment],
    ) -> f32 {
        let total_tasks: usize = plays
            .iter()
            .map(|p| p.batches.iter().map(|b| b.tasks.len()).sum::<usize>())
            .sum();

        if total_tasks == 0 {
            return 1.0;
        }

        let binary_tasks: usize = binary_deployments.iter().map(|d| d.tasks.len()).sum();

        // Binary deployment reduces network overhead significantly
        (binary_tasks as f32) / (total_tasks as f32) * 0.8 + 0.2
    }

    pub fn plan_binary_deployments(
        &self,
        tasks: &[TaskPlan],
        hosts: &[String],
    ) -> Result<Vec<BinaryDeployment>, PlanError> {
        self.binary_planner
            .plan_deployments(tasks, hosts, self.binary_threshold)
    }

    pub fn analyze_dependencies(&self, tasks: &[ParsedTask]) -> Result<DependencyGraph, PlanError> {
        DependencyAnalyzer::new().analyze(tasks)
    }

    pub fn optimize_execution_order(&self, tasks: &[TaskPlan]) -> Result<Vec<TaskPlan>, PlanError> {
        ExecutionOptimizer::new().optimize_order(tasks)
    }

    pub fn estimate_duration(&self, plan: &ExecutionPlan) -> Result<Duration, PlanError> {
        self.task_estimator.estimate_plan_duration(plan)
    }

    pub fn estimate_duration_for_plays(&self, plays: &[PlayPlan]) -> Result<Duration, PlanError> {
        let total_duration = plays.iter().try_fold(Duration::ZERO, |acc, play| {
            let play_duration = self.task_estimator.estimate_play_duration(play)?;
            Ok::<Duration, PlanError>(acc + play_duration)
        })?;

        Ok(total_duration)
    }

    pub fn estimate_compilation_time(
        &self,
        deployments: &[BinaryDeployment],
    ) -> Result<Duration, PlanError> {
        self.binary_planner.estimate_compilation_time(deployments)
    }

    pub fn validate_plan(&self, plan: &ExecutionPlan) -> Result<ValidationReport, PlanError> {
        PlanValidator::new().validate(plan)
    }

    pub fn analyze_binary_suitability(
        &self,
        tasks: &[TaskPlan],
    ) -> Result<BinarySuitabilityAnalysis, PlanError> {
        BinarySuitabilityAnalyzer::new().analyze(tasks)
    }
}

impl Default for ExecutionPlanner {
    fn default() -> Self {
        Self::new()
    }
}

// Add md5 dependency to Cargo.toml for hash calculation
use md5;
