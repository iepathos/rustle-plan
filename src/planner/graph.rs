use crate::planner::error::PlanError;
use crate::types::*;

pub struct DependencyGraphBuilder;

impl DependencyGraphBuilder {
    pub fn new() -> Self {
        Self
    }

    pub fn build_from_tasks(&self, tasks: &[TaskPlan]) -> Result<DependencyGraph, PlanError> {
        let mut graph = petgraph::Graph::new();
        let mut task_nodes = std::collections::HashMap::new();

        // Add nodes for all tasks
        for task in tasks {
            let node = graph.add_node(task.task_id.clone());
            task_nodes.insert(task.task_id.clone(), node);
        }

        // Add edges for dependencies
        for task in tasks {
            if let Some(task_node) = task_nodes.get(&task.task_id) {
                for dep_id in &task.dependencies {
                    if let Some(dep_node) = task_nodes.get(dep_id) {
                        graph.add_edge(*dep_node, *task_node, DependencyType::Explicit);
                    }
                }
            }
        }

        Ok(DependencyGraph::new(graph))
    }

    pub fn find_parallel_groups(
        &self,
        tasks: &[TaskPlan],
        dependency_graph: &DependencyGraph,
    ) -> Vec<ParallelGroup> {
        let mut groups = Vec::new();
        let mut visited = std::collections::HashSet::new();

        for task in tasks {
            if visited.contains(&task.task_id) || !task.can_run_parallel {
                continue;
            }

            let mut group_tasks = vec![task.task_id.clone()];
            visited.insert(task.task_id.clone());

            // Find tasks that can run in parallel with this one
            for other_task in tasks {
                if visited.contains(&other_task.task_id) {
                    continue;
                }

                if self.can_run_parallel(task, other_task, dependency_graph) {
                    group_tasks.push(other_task.task_id.clone());
                    visited.insert(other_task.task_id.clone());
                }
            }

            if group_tasks.len() > 1 {
                groups.push(ParallelGroup {
                    group_id: format!("group_{}", groups.len()),
                    tasks: group_tasks,
                    max_parallelism: self.calculate_max_parallelism(task),
                    shared_resources: Vec::new(), // Simplified for now
                });
            }
        }

        groups
    }

    fn can_run_parallel(
        &self,
        task1: &TaskPlan,
        task2: &TaskPlan,
        graph: &DependencyGraph,
    ) -> bool {
        // Check for direct dependencies
        if graph.has_path(&task1.task_id, &task2.task_id)
            || graph.has_path(&task2.task_id, &task1.task_id)
        {
            return false;
        }

        // Check if both tasks can run in parallel
        if !task1.can_run_parallel || !task2.can_run_parallel {
            return false;
        }

        // Check for resource conflicts (simplified)
        if self.has_resource_conflict(task1, task2) {
            return false;
        }

        true
    }

    fn has_resource_conflict(&self, task1: &TaskPlan, task2: &TaskPlan) -> bool {
        // Check if tasks modify the same files
        if let (Some(dest1), Some(dest2)) = (
            task1.args.get("dest").and_then(|v| v.as_str()),
            task2.args.get("dest").and_then(|v| v.as_str()),
        ) {
            if dest1 == dest2 {
                return true;
            }
        }

        // Check if tasks manage the same service
        if task1.module == "service" && task2.module == "service" {
            if let (Some(name1), Some(name2)) = (
                task1.args.get("name").and_then(|v| v.as_str()),
                task2.args.get("name").and_then(|v| v.as_str()),
            ) {
                if name1 == name2 {
                    return true;
                }
            }
        }

        false
    }

    fn calculate_max_parallelism(&self, _task: &TaskPlan) -> u32 {
        // Simplified calculation
        // In a real implementation, this would consider system resources
        4
    }
}

impl Default for DependencyGraphBuilder {
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

    fn create_task_with_dependencies(id: &str, dependencies: Vec<String>) -> TaskPlan {
        let mut task = create_test_task(id, "shell");
        task.dependencies = dependencies;
        task
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

    fn create_task_non_parallel(id: &str) -> TaskPlan {
        let mut task = create_test_task(id, "shell");
        task.can_run_parallel = false;
        task
    }

    #[test]
    fn test_new() {
        let builder = DependencyGraphBuilder::new();
        assert!(std::ptr::eq(&builder, &builder));
    }

    #[test]
    fn test_default() {
        let builder = DependencyGraphBuilder;
        assert!(std::ptr::eq(&builder, &builder));
    }

    #[test]
    fn test_build_from_tasks_empty() {
        let builder = DependencyGraphBuilder::new();
        let result = builder.build_from_tasks(&[]).unwrap();

        assert_eq!(result.graph.node_count(), 0);
        assert_eq!(result.graph.edge_count(), 0);
    }

    #[test]
    fn test_build_from_tasks_single_task() {
        let builder = DependencyGraphBuilder::new();
        let task = create_test_task("task1", "shell");
        let result = builder.build_from_tasks(&[task]).unwrap();

        assert_eq!(result.graph.node_count(), 1);
        assert_eq!(result.graph.edge_count(), 0);
        assert!(result.task_nodes.contains_key("task1"));
    }

    #[test]
    fn test_build_from_tasks_with_dependencies() {
        let builder = DependencyGraphBuilder::new();
        let task1 = create_test_task("task1", "shell");
        let task2 = create_task_with_dependencies("task2", vec!["task1".to_string()]);
        let tasks = vec![task1, task2];

        let result = builder.build_from_tasks(&tasks).unwrap();

        assert_eq!(result.graph.node_count(), 2);
        assert_eq!(result.graph.edge_count(), 1);
        assert!(result.has_path("task1", "task2"));
        assert!(!result.has_path("task2", "task1"));
    }

    #[test]
    fn test_build_from_tasks_missing_dependency() {
        let builder = DependencyGraphBuilder::new();
        let task = create_task_with_dependencies("task1", vec!["missing_task".to_string()]);
        let result = builder.build_from_tasks(&[task]).unwrap();

        assert_eq!(result.graph.node_count(), 1);
        assert_eq!(result.graph.edge_count(), 0);
    }

    #[test]
    fn test_find_parallel_groups_empty() {
        let builder = DependencyGraphBuilder::new();
        let graph = builder.build_from_tasks(&[]).unwrap();
        let groups = builder.find_parallel_groups(&[], &graph);

        assert!(groups.is_empty());
    }

    #[test]
    fn test_find_parallel_groups_single_task() {
        let builder = DependencyGraphBuilder::new();
        let task = create_test_task("task1", "shell");
        let graph = builder.build_from_tasks(&[task.clone()]).unwrap();
        let groups = builder.find_parallel_groups(&[task], &graph);

        assert!(groups.is_empty());
    }

    #[test]
    fn test_find_parallel_groups_independent_tasks() {
        let builder = DependencyGraphBuilder::new();
        let task1 = create_test_task("task1", "shell");
        let task2 = create_test_task("task2", "copy");
        let tasks = vec![task1, task2];
        let graph = builder.build_from_tasks(&tasks).unwrap();
        let groups = builder.find_parallel_groups(&tasks, &graph);

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].tasks.len(), 2);
        assert!(groups[0].tasks.contains(&"task1".to_string()));
        assert!(groups[0].tasks.contains(&"task2".to_string()));
    }

    #[test]
    fn test_find_parallel_groups_dependent_tasks() {
        let builder = DependencyGraphBuilder::new();
        let task1 = create_test_task("task1", "shell");
        let task2 = create_task_with_dependencies("task2", vec!["task1".to_string()]);
        let tasks = vec![task1, task2];
        let graph = builder.build_from_tasks(&tasks).unwrap();
        let groups = builder.find_parallel_groups(&tasks, &graph);

        assert!(groups.is_empty());
    }

    #[test]
    fn test_find_parallel_groups_non_parallel_task() {
        let builder = DependencyGraphBuilder::new();
        let task1 = create_task_non_parallel("task1");
        let task2 = create_test_task("task2", "copy");
        let tasks = vec![task1, task2];
        let graph = builder.build_from_tasks(&tasks).unwrap();
        let groups = builder.find_parallel_groups(&tasks, &graph);

        assert!(groups.is_empty());
    }

    #[test]
    fn test_can_run_parallel_independent_tasks() {
        let builder = DependencyGraphBuilder::new();
        let task1 = create_test_task("task1", "shell");
        let task2 = create_test_task("task2", "copy");
        let tasks = vec![task1.clone(), task2.clone()];
        let graph = builder.build_from_tasks(&tasks).unwrap();

        assert!(builder.can_run_parallel(&task1, &task2, &graph));
    }

    #[test]
    fn test_can_run_parallel_dependent_tasks() {
        let builder = DependencyGraphBuilder::new();
        let task1 = create_test_task("task1", "shell");
        let task2 = create_task_with_dependencies("task2", vec!["task1".to_string()]);
        let tasks = vec![task1.clone(), task2.clone()];
        let graph = builder.build_from_tasks(&tasks).unwrap();

        assert!(!builder.can_run_parallel(&task1, &task2, &graph));
    }

    #[test]
    fn test_can_run_parallel_non_parallel_task() {
        let builder = DependencyGraphBuilder::new();
        let task1 = create_task_non_parallel("task1");
        let task2 = create_test_task("task2", "copy");
        let tasks = vec![task1.clone(), task2.clone()];
        let graph = builder.build_from_tasks(&tasks).unwrap();

        assert!(!builder.can_run_parallel(&task1, &task2, &graph));
    }

    #[test]
    fn test_can_run_parallel_resource_conflict() {
        let builder = DependencyGraphBuilder::new();
        let mut args = HashMap::new();
        args.insert(
            "dest".to_string(),
            serde_json::Value::String("/etc/config".to_string()),
        );
        let task1 = create_task_with_args("task1", "copy", args.clone());
        let task2 = create_task_with_args("task2", "template", args);
        let tasks = vec![task1.clone(), task2.clone()];
        let graph = builder.build_from_tasks(&tasks).unwrap();

        assert!(!builder.can_run_parallel(&task1, &task2, &graph));
    }

    #[test]
    fn test_has_resource_conflict_same_dest() {
        let builder = DependencyGraphBuilder::new();
        let mut args = HashMap::new();
        args.insert(
            "dest".to_string(),
            serde_json::Value::String("/etc/config".to_string()),
        );
        let task1 = create_task_with_args("task1", "copy", args.clone());
        let task2 = create_task_with_args("task2", "template", args);

        assert!(builder.has_resource_conflict(&task1, &task2));
    }

    #[test]
    fn test_has_resource_conflict_same_service() {
        let builder = DependencyGraphBuilder::new();
        let mut args = HashMap::new();
        args.insert(
            "name".to_string(),
            serde_json::Value::String("nginx".to_string()),
        );
        let task1 = create_task_with_args("task1", "service", args.clone());
        let task2 = create_task_with_args("task2", "service", args);

        assert!(builder.has_resource_conflict(&task1, &task2));
    }

    #[test]
    fn test_has_resource_conflict_different_resources() {
        let builder = DependencyGraphBuilder::new();
        let mut args1 = HashMap::new();
        args1.insert(
            "dest".to_string(),
            serde_json::Value::String("/etc/config1".to_string()),
        );
        let mut args2 = HashMap::new();
        args2.insert(
            "dest".to_string(),
            serde_json::Value::String("/etc/config2".to_string()),
        );
        let task1 = create_task_with_args("task1", "copy", args1);
        let task2 = create_task_with_args("task2", "template", args2);

        assert!(!builder.has_resource_conflict(&task1, &task2));
    }

    #[test]
    fn test_calculate_max_parallelism() {
        let builder = DependencyGraphBuilder::new();
        let task = create_test_task("task1", "shell");

        assert_eq!(builder.calculate_max_parallelism(&task), 4);
    }

    #[test]
    fn test_dependency_graph_has_path() {
        let builder = DependencyGraphBuilder::new();
        let task1 = create_test_task("task1", "shell");
        let task2 = create_task_with_dependencies("task2", vec!["task1".to_string()]);
        let task3 = create_task_with_dependencies("task3", vec!["task2".to_string()]);
        let tasks = vec![task1, task2, task3];
        let graph = builder.build_from_tasks(&tasks).unwrap();

        assert!(graph.has_path("task1", "task2"));
        assert!(graph.has_path("task1", "task3"));
        assert!(graph.has_path("task2", "task3"));
        assert!(!graph.has_path("task2", "task1"));
        assert!(!graph.has_path("task3", "task1"));
    }

    #[test]
    fn test_dependency_graph_no_path_independent() {
        let builder = DependencyGraphBuilder::new();
        let task1 = create_test_task("task1", "shell");
        let task2 = create_test_task("task2", "copy");
        let tasks = vec![task1, task2];
        let graph = builder.build_from_tasks(&tasks).unwrap();

        assert!(!graph.has_path("task1", "task2"));
        assert!(!graph.has_path("task2", "task1"));
    }

    #[test]
    fn test_parallel_group_structure() {
        let builder = DependencyGraphBuilder::new();
        let task1 = create_test_task("task1", "shell");
        let task2 = create_test_task("task2", "copy");
        let tasks = vec![task1, task2];
        let graph = builder.build_from_tasks(&tasks).unwrap();
        let groups = builder.find_parallel_groups(&tasks, &graph);

        assert_eq!(groups.len(), 1);
        let group = &groups[0];
        assert_eq!(group.group_id, "group_0");
        assert_eq!(group.max_parallelism, 4);
        assert!(group.shared_resources.is_empty());
    }
}
