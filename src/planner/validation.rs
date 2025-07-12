use crate::planner::error::PlanError;
use crate::types::*;

pub struct PlanValidator;

impl PlanValidator {
    pub fn new() -> Self {
        Self
    }

    pub fn validate(&self, plan: &ExecutionPlan) -> Result<ValidationReport, PlanError> {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Validate hosts
        if plan.hosts.is_empty() {
            errors.push("No target hosts specified".to_string());
        }

        // Validate plays
        if plan.plays.is_empty() {
            warnings.push("No plays in execution plan".to_string());
        }

        for play in &plan.plays {
            self.validate_play(play, &mut errors, &mut warnings);
        }

        // Validate binary deployments
        for deployment in &plan.binary_deployments {
            self.validate_binary_deployment(deployment, &mut errors, &mut warnings);
        }

        Ok(ValidationReport {
            is_valid: errors.is_empty(),
            errors,
            warnings,
        })
    }

    fn validate_play(&self, play: &PlayPlan, errors: &mut Vec<String>, warnings: &mut Vec<String>) {
        if play.batches.is_empty() {
            warnings.push(format!("Play '{}' has no execution batches", play.name));
        }

        for batch in &play.batches {
            self.validate_batch(batch, errors, warnings);
        }
    }

    fn validate_batch(
        &self,
        batch: &ExecutionBatch,
        errors: &mut Vec<String>,
        warnings: &mut Vec<String>,
    ) {
        if batch.tasks.is_empty() {
            warnings.push(format!("Batch '{}' has no tasks", batch.batch_id));
        }

        if batch.hosts.is_empty() {
            errors.push(format!("Batch '{}' has no target hosts", batch.batch_id));
        }

        // Validate task dependencies
        for task in &batch.tasks {
            for dep in &task.dependencies {
                if !batch.tasks.iter().any(|t| t.task_id == *dep) {
                    warnings.push(format!(
                        "Task '{}' depends on '{}' which is not in the same batch",
                        task.task_id, dep
                    ));
                }
            }
        }
    }

    fn validate_binary_deployment(
        &self,
        deployment: &BinaryDeployment,
        errors: &mut Vec<String>,
        warnings: &mut Vec<String>,
    ) {
        if deployment.target_hosts.is_empty() {
            errors.push(format!(
                "Binary deployment '{}' has no target hosts",
                deployment.deployment_id
            ));
        }

        if deployment.tasks.is_empty() {
            warnings.push(format!(
                "Binary deployment '{}' has no tasks",
                deployment.deployment_id
            ));
        }

        if deployment.estimated_size == 0 {
            warnings.push(format!(
                "Binary deployment '{}' has zero estimated size",
                deployment.deployment_id
            ));
        }

        // Validate compilation requirements
        let req = &deployment.compilation_requirements;
        if req.rust_version.is_empty() {
            warnings.push(format!(
                "Binary deployment '{}' has no Rust version specified",
                deployment.deployment_id
            ));
        }
    }
}

impl Default for PlanValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::collections::HashMap;
    use std::time::Duration;

    fn create_test_plan() -> ExecutionPlan {
        ExecutionPlan {
            metadata: PlanMetadata {
                created_at: Utc::now(),
                rustle_version: "1.0.0".to_string(),
                playbook_hash: "abc123".to_string(),
                inventory_hash: "def456".to_string(),
                planning_options: PlanningOptions {
                    limit: None,
                    tags: vec![],
                    skip_tags: vec![],
                    check_mode: false,
                    diff_mode: false,
                    forks: 5,
                    serial: None,
                    strategy: ExecutionStrategy::Linear,
                    binary_threshold: 10,
                    force_binary: false,
                    force_ssh: false,
                },
            },
            plays: vec![],
            binary_deployments: vec![],
            total_tasks: 0,
            estimated_duration: Some(Duration::from_secs(60)),
            estimated_compilation_time: None,
            parallelism_score: 0.8,
            network_efficiency_score: 0.9,
            hosts: vec!["host1".to_string(), "host2".to_string()],
        }
    }

    fn create_test_play() -> PlayPlan {
        PlayPlan {
            play_id: "play-1".to_string(),
            name: "Test Play".to_string(),
            strategy: ExecutionStrategy::Linear,
            serial: None,
            hosts: vec!["host1".to_string()],
            batches: vec![],
            handlers: vec![],
            estimated_duration: Some(Duration::from_secs(30)),
        }
    }

    fn create_test_batch() -> ExecutionBatch {
        ExecutionBatch {
            batch_id: "batch-1".to_string(),
            hosts: vec!["host1".to_string()],
            tasks: vec![],
            parallel_groups: vec![],
            dependencies: vec![],
            estimated_duration: Some(Duration::from_secs(10)),
        }
    }

    fn create_test_task() -> TaskPlan {
        TaskPlan {
            task_id: "task-1".to_string(),
            name: "Test Task".to_string(),
            module: "shell".to_string(),
            args: HashMap::new(),
            hosts: vec!["host1".to_string()],
            dependencies: vec![],
            conditions: vec![],
            tags: vec![],
            notify: vec![],
            execution_order: 1,
            can_run_parallel: true,
            estimated_duration: Some(Duration::from_secs(5)),
            risk_level: RiskLevel::Low,
        }
    }

    fn create_test_binary_deployment() -> BinaryDeployment {
        BinaryDeployment {
            deployment_id: "deploy-1".to_string(),
            target_hosts: vec!["host1".to_string()],
            binary_name: "test-binary".to_string(),
            tasks: vec!["task-1".to_string()],
            modules: vec!["shell".to_string()],
            embedded_data: BinaryEmbeddedData {
                execution_plan: "{}".to_string(),
                static_files: vec![],
                variables: HashMap::new(),
                facts_required: vec![],
            },
            execution_mode: BinaryExecutionMode::Standalone,
            estimated_size: 1024,
            compilation_requirements: CompilationRequirements {
                target_arch: "x86_64".to_string(),
                target_os: "linux".to_string(),
                rust_version: "1.70.0".to_string(),
                cross_compilation: false,
                static_linking: true,
            },
        }
    }

    #[test]
    fn test_new() {
        let validator = PlanValidator::new();
        assert!(std::ptr::eq(&validator, &validator));
    }

    #[test]
    fn test_default() {
        let validator = PlanValidator;
        assert!(std::ptr::eq(&validator, &validator));
    }

    #[test]
    fn test_validate_valid_plan() {
        let validator = PlanValidator::new();
        let plan = create_test_plan();

        let result = validator.validate(&plan).unwrap();
        assert!(result.is_valid);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_validate_plan_no_hosts() {
        let validator = PlanValidator::new();
        let mut plan = create_test_plan();
        plan.hosts.clear();

        let result = validator.validate(&plan).unwrap();
        assert!(!result.is_valid);
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0], "No target hosts specified");
    }

    #[test]
    fn test_validate_plan_no_plays() {
        let validator = PlanValidator::new();
        let plan = create_test_plan();

        let result = validator.validate(&plan).unwrap();
        assert!(result.is_valid);
        assert_eq!(result.warnings.len(), 1);
        assert_eq!(result.warnings[0], "No plays in execution plan");
    }

    #[test]
    fn test_validate_play_no_batches() {
        let validator = PlanValidator::new();
        let mut plan = create_test_plan();
        plan.plays.push(create_test_play());

        let result = validator.validate(&plan).unwrap();
        assert!(result.is_valid);
        assert!(result
            .warnings
            .iter()
            .any(|w| w.contains("has no execution batches")));
    }

    #[test]
    fn test_validate_batch_no_tasks() {
        let validator = PlanValidator::new();
        let mut plan = create_test_plan();
        let mut play = create_test_play();
        play.batches.push(create_test_batch());
        plan.plays.push(play);

        let result = validator.validate(&plan).unwrap();
        assert!(result.is_valid);
        assert!(result.warnings.iter().any(|w| w.contains("has no tasks")));
    }

    #[test]
    fn test_validate_batch_no_hosts() {
        let validator = PlanValidator::new();
        let mut plan = create_test_plan();
        let mut play = create_test_play();
        let mut batch = create_test_batch();
        batch.hosts.clear();
        play.batches.push(batch);
        plan.plays.push(play);

        let result = validator.validate(&plan).unwrap();
        assert!(!result.is_valid);
        assert!(result
            .errors
            .iter()
            .any(|e| e.contains("has no target hosts")));
    }

    #[test]
    fn test_validate_task_dependency_in_same_batch() {
        let validator = PlanValidator::new();
        let mut plan = create_test_plan();
        let mut play = create_test_play();
        let mut batch = create_test_batch();

        let mut task1 = create_test_task();
        task1.task_id = "task-1".to_string();
        let mut task2 = create_test_task();
        task2.task_id = "task-2".to_string();
        task2.dependencies = vec!["task-1".to_string()];

        batch.tasks = vec![task1, task2];
        play.batches.push(batch);
        plan.plays.push(play);

        let result = validator.validate(&plan).unwrap();
        assert!(result.is_valid);
        assert!(
            result.warnings.is_empty()
                || !result
                    .warnings
                    .iter()
                    .any(|w| w.contains("not in the same batch"))
        );
    }

    #[test]
    fn test_validate_task_dependency_not_in_batch() {
        let validator = PlanValidator::new();
        let mut plan = create_test_plan();
        let mut play = create_test_play();
        let mut batch = create_test_batch();

        let mut task = create_test_task();
        task.dependencies = vec!["missing-task".to_string()];
        batch.tasks = vec![task];
        play.batches.push(batch);
        plan.plays.push(play);

        let result = validator.validate(&plan).unwrap();
        assert!(result.is_valid);
        assert!(result
            .warnings
            .iter()
            .any(|w| w.contains("not in the same batch")));
    }

    #[test]
    fn test_validate_binary_deployment_valid() {
        let validator = PlanValidator::new();
        let mut plan = create_test_plan();
        plan.binary_deployments
            .push(create_test_binary_deployment());

        let result = validator.validate(&plan).unwrap();
        assert!(result.is_valid);
    }

    #[test]
    fn test_validate_binary_deployment_no_hosts() {
        let validator = PlanValidator::new();
        let mut plan = create_test_plan();
        let mut deployment = create_test_binary_deployment();
        deployment.target_hosts.clear();
        plan.binary_deployments.push(deployment);

        let result = validator.validate(&plan).unwrap();
        assert!(!result.is_valid);
        assert!(result
            .errors
            .iter()
            .any(|e| e.contains("has no target hosts")));
    }

    #[test]
    fn test_validate_binary_deployment_no_tasks() {
        let validator = PlanValidator::new();
        let mut plan = create_test_plan();
        let mut deployment = create_test_binary_deployment();
        deployment.tasks.clear();
        plan.binary_deployments.push(deployment);

        let result = validator.validate(&plan).unwrap();
        assert!(result.is_valid);
        assert!(result.warnings.iter().any(|w| w.contains("has no tasks")));
    }

    #[test]
    fn test_validate_binary_deployment_zero_size() {
        let validator = PlanValidator::new();
        let mut plan = create_test_plan();
        let mut deployment = create_test_binary_deployment();
        deployment.estimated_size = 0;
        plan.binary_deployments.push(deployment);

        let result = validator.validate(&plan).unwrap();
        assert!(result.is_valid);
        assert!(result
            .warnings
            .iter()
            .any(|w| w.contains("zero estimated size")));
    }

    #[test]
    fn test_validate_binary_deployment_no_rust_version() {
        let validator = PlanValidator::new();
        let mut plan = create_test_plan();
        let mut deployment = create_test_binary_deployment();
        deployment.compilation_requirements.rust_version = "".to_string();
        plan.binary_deployments.push(deployment);

        let result = validator.validate(&plan).unwrap();
        assert!(result.is_valid);
        assert!(result
            .warnings
            .iter()
            .any(|w| w.contains("no Rust version specified")));
    }

    #[test]
    fn test_validation_report_structure() {
        let report = ValidationReport {
            is_valid: false,
            errors: vec!["Error 1".to_string(), "Error 2".to_string()],
            warnings: vec!["Warning 1".to_string()],
        };

        assert!(!report.is_valid);
        assert_eq!(report.errors.len(), 2);
        assert_eq!(report.warnings.len(), 1);
    }
}
