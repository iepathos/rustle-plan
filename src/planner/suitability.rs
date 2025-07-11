use crate::planner::error::PlanError;
use crate::types::*;
use std::collections::HashMap;

pub struct BinarySuitabilityAnalyzer;

impl BinarySuitabilityAnalyzer {
    pub fn new() -> Self {
        Self
    }

    pub fn analyze(&self, tasks: &[TaskPlan]) -> Result<BinarySuitabilityAnalysis, PlanError> {
        let mut suitable_groups = Vec::new();
        let mut unsuitable_tasks = Vec::new();
        let mut reasons = HashMap::new();

        let mut remaining_tasks: Vec<&TaskPlan> = tasks.iter().collect();

        while !remaining_tasks.is_empty() {
            let seed_task = remaining_tasks.remove(0);

            if !self.is_task_binary_suitable(seed_task) {
                unsuitable_tasks.push(seed_task.task_id.clone());
                reasons.insert(
                    seed_task.task_id.clone(),
                    self.get_unsuitability_reason(seed_task),
                );
                continue;
            }

            let mut group = TaskGroup {
                id: format!("group_{}", suitable_groups.len()),
                tasks: vec![seed_task.clone()],
                hosts: seed_task.hosts.clone(),
                modules: vec![seed_task.module.clone()],
                network_operations: self.count_network_operations(seed_task),
            };

            // Find compatible tasks for this group
            remaining_tasks.retain(|&task| {
                if self.is_task_binary_suitable(task) && self.can_group_tasks(seed_task, task) {
                    group.tasks.push(task.clone());
                    group.modules.push(task.module.clone());
                    group.network_operations += self.count_network_operations(task);
                    false // Remove from remaining
                } else {
                    true // Keep in remaining
                }
            });

            if group.tasks.len() >= 2 {
                suitable_groups.push(group);
            } else {
                // Single task group - check if it's still worth binary deployment
                if group.network_operations >= 3 {
                    suitable_groups.push(group);
                } else {
                    unsuitable_tasks.push(seed_task.task_id.clone());
                    reasons.insert(
                        seed_task.task_id.clone(),
                        "Insufficient network operations to justify binary deployment".to_string(),
                    );
                }
            }
        }

        Ok(BinarySuitabilityAnalysis {
            suitable_groups,
            unsuitable_tasks,
            reasons,
        })
    }

    fn is_task_binary_suitable(&self, task: &TaskPlan) -> bool {
        // Check module compatibility
        let compatible_modules = [
            "file", "copy", "template", "shell", "command", "package", "service", "user", "group",
            "cron",
        ];

        if !compatible_modules.contains(&task.module.as_str()) {
            return false;
        }

        // Check risk level
        if task.risk_level == RiskLevel::Critical {
            return false;
        }

        // Check for interactive requirements
        let interactive_modules = ["pause", "prompt", "vars_prompt"];
        if interactive_modules.contains(&task.module.as_str()) {
            return false;
        }

        // Check for specific argument patterns that make binary deployment unsuitable
        if self.has_unsuitable_arguments(task) {
            return false;
        }

        true
    }

    fn has_unsuitable_arguments(&self, task: &TaskPlan) -> bool {
        // Check for arguments that require runtime host-specific resolution
        if task.args.contains_key("delegate_to") {
            return true;
        }

        if task.args.contains_key("local_action") {
            return true;
        }

        // Check for complex conditionals that are hard to evaluate in binary
        if task.conditions.iter().any(|cond| matches!(cond, ExecutionCondition::When { expression } if expression.contains("hostvars"))) {
            return true;
        }

        false
    }

    fn can_group_tasks(&self, task1: &TaskPlan, task2: &TaskPlan) -> bool {
        // Tasks must target same or overlapping hosts
        if !self.has_host_overlap(&task1.hosts, &task2.hosts) {
            return false;
        }

        // Tasks must not have conflicting resource requirements
        if self.has_resource_conflict(task1, task2) {
            return false;
        }

        // Tasks should be part of the same logical operation
        if self.are_logically_related(task1, task2) {
            return true;
        }

        // At minimum, tasks should not interfere with each other
        !self.tasks_interfere(task1, task2)
    }

    fn has_host_overlap(&self, hosts1: &[String], hosts2: &[String]) -> bool {
        hosts1.iter().any(|h| hosts2.contains(h))
    }

    fn has_resource_conflict(&self, task1: &TaskPlan, task2: &TaskPlan) -> bool {
        // Check for file path conflicts
        if let (Some(path1), Some(path2)) = (self.get_file_path(task1), self.get_file_path(task2)) {
            if path1 == path2 {
                return true;
            }
        }

        // Check for service conflicts
        if task1.module == "service" && task2.module == "service" {
            if let (Some(service1), Some(service2)) = (
                task1.args.get("name").and_then(|v| v.as_str()),
                task2.args.get("name").and_then(|v| v.as_str()),
            ) {
                if service1 == service2 {
                    return true;
                }
            }
        }

        false
    }

    fn are_logically_related(&self, task1: &TaskPlan, task2: &TaskPlan) -> bool {
        // Check if tasks share tags
        if !task1.tags.is_empty()
            && !task2.tags.is_empty()
            && task1.tags.iter().any(|tag| task2.tags.contains(tag))
        {
            return true;
        }

        // Check if tasks are part of the same workflow
        if task1.module == "copy" && task2.module == "service" {
            // Common pattern: copy config file then restart service
            return true;
        }

        if task1.module == "package" && task2.module == "service" {
            // Common pattern: install package then start service
            return true;
        }

        false
    }

    fn tasks_interfere(&self, task1: &TaskPlan, task2: &TaskPlan) -> bool {
        // Check for execution order dependencies that would prevent grouping
        if task1.dependencies.contains(&task2.task_id)
            || task2.dependencies.contains(&task1.task_id)
        {
            return true;
        }

        // Check for notification chains
        if task1
            .notify
            .iter()
            .any(|handler| task2.name.contains(handler))
        {
            return true;
        }

        false
    }

    fn get_file_path(&self, task: &TaskPlan) -> Option<String> {
        task.args
            .get("dest")
            .or_else(|| task.args.get("path"))
            .or_else(|| task.args.get("src"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    fn get_unsuitability_reason(&self, task: &TaskPlan) -> String {
        if !self.is_task_binary_suitable(task) {
            return format!(
                "Module '{}' is not compatible with binary deployment",
                task.module
            );
        }

        if task.risk_level == RiskLevel::Critical {
            return "Task has critical risk level".to_string();
        }

        if self.has_unsuitable_arguments(task) {
            return "Task has arguments that require runtime resolution".to_string();
        }

        "Unknown unsuitability reason".to_string()
    }

    fn count_network_operations(&self, task: &TaskPlan) -> u32 {
        match task.module.as_str() {
            "copy" | "template" | "fetch" => 2, // Upload + command
            "package" | "service" => 1,         // Command only
            "shell" | "command" => 1,           // Command only
            _ => 1,
        }
    }
}

impl Default for BinarySuitabilityAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
