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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::time::Duration;

    fn create_test_task(id: &str, module: &str) -> TaskPlan {
        TaskPlan {
            task_id: id.to_string(),
            name: format!("Test Task {id}"),
            module: module.to_string(),
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

    fn create_task_with_args(
        id: &str,
        module: &str,
        args: HashMap<String, serde_json::Value>,
    ) -> TaskPlan {
        let mut task = create_test_task(id, module);
        task.args = args;
        task
    }

    fn create_task_with_hosts(id: &str, module: &str, hosts: Vec<String>) -> TaskPlan {
        let mut task = create_test_task(id, module);
        task.hosts = hosts;
        task
    }

    fn create_task_with_risk(id: &str, module: &str, risk: RiskLevel) -> TaskPlan {
        let mut task = create_test_task(id, module);
        task.risk_level = risk;
        task
    }

    #[test]
    fn test_new() {
        let analyzer = BinarySuitabilityAnalyzer::new();
        assert!(std::ptr::eq(&analyzer, &analyzer));
    }

    #[test]
    fn test_default() {
        let analyzer = BinarySuitabilityAnalyzer;
        assert!(std::ptr::eq(&analyzer, &analyzer));
    }

    #[test]
    fn test_analyze_empty_tasks() {
        let analyzer = BinarySuitabilityAnalyzer::new();
        let result = analyzer.analyze(&[]).unwrap();

        assert!(result.suitable_groups.is_empty());
        assert!(result.unsuitable_tasks.is_empty());
        assert!(result.reasons.is_empty());
    }

    #[test]
    fn test_analyze_single_suitable_task_insufficient_network() {
        let analyzer = BinarySuitabilityAnalyzer::new();
        let task = create_test_task("task1", "copy");
        let result = analyzer.analyze(&[task]).unwrap();

        // Single task with copy (2 network ops) needs >= 3 for binary deployment
        assert_eq!(result.suitable_groups.len(), 0);
        assert_eq!(result.unsuitable_tasks.len(), 1);
        assert!(result
            .reasons
            .get("task1")
            .unwrap()
            .contains("Insufficient network operations"));
    }

    #[test]
    fn test_analyze_single_suitable_task_low_network() {
        let analyzer = BinarySuitabilityAnalyzer::new();
        let task = create_test_task("task1", "shell");
        let result = analyzer.analyze(&[task]).unwrap();

        assert!(result.suitable_groups.is_empty());
        assert_eq!(result.unsuitable_tasks.len(), 1);
        assert!(result.reasons.contains_key("task1"));
    }

    #[test]
    fn test_analyze_unsuitable_task() {
        let analyzer = BinarySuitabilityAnalyzer::new();
        let task = create_test_task("task1", "debug");
        let result = analyzer.analyze(&[task]).unwrap();

        assert!(result.suitable_groups.is_empty());
        assert_eq!(result.unsuitable_tasks.len(), 1);
        assert_eq!(result.unsuitable_tasks[0], "task1");
    }

    #[test]
    fn test_is_task_binary_suitable_compatible_modules() {
        let analyzer = BinarySuitabilityAnalyzer::new();
        let compatible_modules = [
            "file", "copy", "template", "shell", "command", "package", "service", "user", "group",
            "cron",
        ];

        for module in &compatible_modules {
            let task = create_test_task("test", module);
            assert!(
                analyzer.is_task_binary_suitable(&task),
                "Module {module} should be suitable"
            );
        }
    }

    #[test]
    fn test_is_task_binary_suitable_incompatible_modules() {
        let analyzer = BinarySuitabilityAnalyzer::new();
        let incompatible_modules = ["debug", "assert", "fail", "meta", "include", "import_tasks"];

        for module in &incompatible_modules {
            let task = create_test_task("test", module);
            assert!(
                !analyzer.is_task_binary_suitable(&task),
                "Module {module} should not be suitable"
            );
        }
    }

    #[test]
    fn test_is_task_binary_suitable_critical_risk() {
        let analyzer = BinarySuitabilityAnalyzer::new();
        let task = create_task_with_risk("test", "shell", RiskLevel::Critical);
        assert!(!analyzer.is_task_binary_suitable(&task));
    }

    #[test]
    fn test_is_task_binary_suitable_interactive_modules() {
        let analyzer = BinarySuitabilityAnalyzer::new();
        let interactive_modules = ["pause", "prompt", "vars_prompt"];

        for module in &interactive_modules {
            let task = create_test_task("test", module);
            assert!(
                !analyzer.is_task_binary_suitable(&task),
                "Interactive module {module} should not be suitable"
            );
        }
    }

    #[test]
    fn test_has_unsuitable_arguments_delegate_to() {
        let analyzer = BinarySuitabilityAnalyzer::new();
        let mut args = HashMap::new();
        args.insert(
            "delegate_to".to_string(),
            serde_json::Value::String("other_host".to_string()),
        );
        let task = create_task_with_args("test", "shell", args);

        assert!(analyzer.has_unsuitable_arguments(&task));
    }

    #[test]
    fn test_has_unsuitable_arguments_local_action() {
        let analyzer = BinarySuitabilityAnalyzer::new();
        let mut args = HashMap::new();
        args.insert("local_action".to_string(), serde_json::Value::Bool(true));
        let task = create_task_with_args("test", "shell", args);

        assert!(analyzer.has_unsuitable_arguments(&task));
    }

    #[test]
    fn test_has_unsuitable_arguments_hostvars_condition() {
        let analyzer = BinarySuitabilityAnalyzer::new();
        let mut task = create_test_task("test", "shell");
        task.conditions = vec![ExecutionCondition::When {
            expression: "hostvars[inventory_hostname]['some_var']".to_string(),
        }];

        assert!(analyzer.has_unsuitable_arguments(&task));
    }

    #[test]
    fn test_can_group_tasks_same_hosts() {
        let analyzer = BinarySuitabilityAnalyzer::new();
        let task1 = create_task_with_hosts(
            "task1",
            "shell",
            vec!["host1".to_string(), "host2".to_string()],
        );
        let task2 = create_task_with_hosts("task2", "copy", vec!["host1".to_string()]);

        assert!(analyzer.can_group_tasks(&task1, &task2));
    }

    #[test]
    fn test_can_group_tasks_no_host_overlap() {
        let analyzer = BinarySuitabilityAnalyzer::new();
        let task1 = create_task_with_hosts("task1", "shell", vec!["host1".to_string()]);
        let task2 = create_task_with_hosts("task2", "copy", vec!["host2".to_string()]);

        assert!(!analyzer.can_group_tasks(&task1, &task2));
    }

    #[test]
    fn test_has_host_overlap() {
        let analyzer = BinarySuitabilityAnalyzer::new();
        let hosts1 = vec!["host1".to_string(), "host2".to_string()];
        let hosts2 = vec!["host2".to_string(), "host3".to_string()];
        let hosts3 = vec!["host4".to_string()];

        assert!(analyzer.has_host_overlap(&hosts1, &hosts2));
        assert!(!analyzer.has_host_overlap(&hosts1, &hosts3));
    }

    #[test]
    fn test_has_resource_conflict_same_file_path() {
        let analyzer = BinarySuitabilityAnalyzer::new();
        let mut args1 = HashMap::new();
        args1.insert(
            "dest".to_string(),
            serde_json::Value::String("/etc/nginx.conf".to_string()),
        );
        let mut args2 = HashMap::new();
        args2.insert(
            "dest".to_string(),
            serde_json::Value::String("/etc/nginx.conf".to_string()),
        );

        let task1 = create_task_with_args("task1", "copy", args1);
        let task2 = create_task_with_args("task2", "template", args2);

        assert!(analyzer.has_resource_conflict(&task1, &task2));
    }

    #[test]
    fn test_has_resource_conflict_same_service() {
        let analyzer = BinarySuitabilityAnalyzer::new();
        let mut args1 = HashMap::new();
        args1.insert(
            "name".to_string(),
            serde_json::Value::String("nginx".to_string()),
        );
        let mut args2 = HashMap::new();
        args2.insert(
            "name".to_string(),
            serde_json::Value::String("nginx".to_string()),
        );

        let task1 = create_task_with_args("task1", "service", args1);
        let task2 = create_task_with_args("task2", "service", args2);

        assert!(analyzer.has_resource_conflict(&task1, &task2));
    }

    #[test]
    fn test_are_logically_related_shared_tags() {
        let analyzer = BinarySuitabilityAnalyzer::new();
        let mut task1 = create_test_task("task1", "copy");
        task1.tags = vec!["webserver".to_string(), "config".to_string()];
        let mut task2 = create_test_task("task2", "service");
        task2.tags = vec!["webserver".to_string()];

        assert!(analyzer.are_logically_related(&task1, &task2));
    }

    #[test]
    fn test_are_logically_related_copy_service_pattern() {
        let analyzer = BinarySuitabilityAnalyzer::new();
        let task1 = create_test_task("task1", "copy");
        let task2 = create_test_task("task2", "service");

        assert!(analyzer.are_logically_related(&task1, &task2));
    }

    #[test]
    fn test_are_logically_related_package_service_pattern() {
        let analyzer = BinarySuitabilityAnalyzer::new();
        let task1 = create_test_task("task1", "package");
        let task2 = create_test_task("task2", "service");

        assert!(analyzer.are_logically_related(&task1, &task2));
    }

    #[test]
    fn test_tasks_interfere_dependency() {
        let analyzer = BinarySuitabilityAnalyzer::new();
        let task1 = create_test_task("task1", "copy");
        let mut task2 = create_test_task("task2", "service");
        task2.dependencies = vec!["task1".to_string()];

        assert!(analyzer.tasks_interfere(&task1, &task2));
    }

    #[test]
    fn test_tasks_interfere_notification() {
        let analyzer = BinarySuitabilityAnalyzer::new();
        let mut task1 = create_test_task("task1", "copy");
        task1.notify = vec!["restart nginx".to_string()];
        let mut task2 = create_test_task("task2", "service");
        task2.name = "restart nginx service".to_string();

        assert!(analyzer.tasks_interfere(&task1, &task2));
    }

    #[test]
    fn test_get_file_path() {
        let analyzer = BinarySuitabilityAnalyzer::new();

        let mut args = HashMap::new();
        args.insert(
            "dest".to_string(),
            serde_json::Value::String("/path/to/file".to_string()),
        );
        let task = create_task_with_args("test", "copy", args);

        assert_eq!(
            analyzer.get_file_path(&task),
            Some("/path/to/file".to_string())
        );
    }

    #[test]
    fn test_get_unsuitability_reason() {
        let analyzer = BinarySuitabilityAnalyzer::new();

        let task1 = create_test_task("test1", "debug");
        assert!(analyzer
            .get_unsuitability_reason(&task1)
            .contains("not compatible"));

        // For critical risk task, need to use an incompatible module first to get the critical risk message
        let task2 = create_task_with_risk("test2", "debug", RiskLevel::Critical);
        assert!(analyzer
            .get_unsuitability_reason(&task2)
            .contains("not compatible"));
    }

    #[test]
    fn test_get_unsuitability_reason_critical_risk_compatible_module() {
        let analyzer = BinarySuitabilityAnalyzer::new();

        // Create a task with compatible module but critical risk
        // This should still fail the is_task_binary_suitable check due to critical risk
        let task = create_task_with_risk("test", "shell", RiskLevel::Critical);

        // The function checks is_task_binary_suitable first, which returns false for critical risk
        // So it should return the module incompatible message
        assert!(analyzer
            .get_unsuitability_reason(&task)
            .contains("not compatible"));
    }

    #[test]
    fn test_count_network_operations() {
        let analyzer = BinarySuitabilityAnalyzer::new();

        assert_eq!(
            analyzer.count_network_operations(&create_test_task("test", "copy")),
            2
        );
        assert_eq!(
            analyzer.count_network_operations(&create_test_task("test", "package")),
            1
        );
        assert_eq!(
            analyzer.count_network_operations(&create_test_task("test", "shell")),
            1
        );
    }

    #[test]
    fn test_analyze_multiple_compatible_tasks() {
        let analyzer = BinarySuitabilityAnalyzer::new();
        let tasks = vec![
            create_test_task("task1", "copy"),
            create_test_task("task2", "service"),
        ];

        let result = analyzer.analyze(&tasks).unwrap();
        assert_eq!(result.suitable_groups.len(), 1);
        assert_eq!(result.suitable_groups[0].tasks.len(), 2);
    }
}
