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
