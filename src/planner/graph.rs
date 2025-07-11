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
