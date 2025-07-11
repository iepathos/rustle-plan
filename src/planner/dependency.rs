use crate::planner::error::PlanError;
use crate::types::*;
use petgraph::Graph;
use std::collections::HashMap;

pub struct DependencyAnalyzer;

impl DependencyAnalyzer {
    pub fn new() -> Self {
        Self
    }

    pub fn analyze(&self, tasks: &[ParsedTask]) -> Result<DependencyGraph, PlanError> {
        let mut graph = Graph::new();
        let mut task_map = HashMap::new();

        // Add all tasks as nodes
        for task in tasks {
            let node = graph.add_node(task.id.clone());
            task_map.insert(task.id.clone(), (node, task));
        }

        // Add explicit dependencies
        for (task_id, (node, task)) in &task_map {
            for dep_id in &task.dependencies {
                if let Some((dep_node, _)) = task_map.get(dep_id) {
                    graph.add_edge(*dep_node, *node, DependencyType::Explicit);
                } else {
                    return Err(PlanError::UnknownTaskDependency {
                        task_id: dep_id.clone(),
                    });
                }
            }

            // Add implicit dependencies
            for (other_task_id, (other_node, other_task)) in &task_map {
                if task_id != other_task_id {
                    // Check if there's already an explicit dependency
                    let has_explicit_dep = task.dependencies.contains(other_task_id)
                        || other_task.dependencies.contains(task_id);

                    if !has_explicit_dep {
                        if let Some(dependency_type) =
                            self.detect_implicit_dependency(other_task, task)
                        {
                            graph.add_edge(*other_node, *node, dependency_type);
                        }
                    }
                }
            }
        }

        // Check for circular dependencies
        if let Err(cycle) = petgraph::algo::toposort(&graph, None) {
            let cycle_description = format!(
                "Cycle detected involving task at node {:?}",
                cycle.node_id()
            );
            return Err(PlanError::CircularDependency {
                cycle: cycle_description,
            });
        }

        Ok(DependencyGraph::new(graph))
    }

    fn detect_implicit_dependency(
        &self,
        task1: &ParsedTask,
        task2: &ParsedTask,
    ) -> Option<DependencyType> {
        // File-based dependencies
        if let (Some(dest1), Some(src2)) = (
            task1.args.get("dest").and_then(|v| v.as_str()),
            task2.args.get("src").and_then(|v| v.as_str()),
        ) {
            if dest1 == src2 {
                return Some(DependencyType::FileOutput);
            }
        }

        // Service and package dependencies
        // Package installation should happen before service management
        if task1.module == "package" && task2.module == "service" {
            if let (Some(package), Some(service)) = (
                task1.args.get("name").and_then(|v| v.as_str()),
                task2.args.get("name").and_then(|v| v.as_str()),
            ) {
                if package == service {
                    return Some(DependencyType::ServicePackage);
                }
            }
        }

        // File creation before modification
        if task1.module == "file" && task2.module == "lineinfile" {
            if let (Some(path1), Some(path2)) = (
                task1.args.get("path").and_then(|v| v.as_str()),
                task2.args.get("path").and_then(|v| v.as_str()),
            ) {
                if path1 == path2 {
                    return Some(DependencyType::FileOutput);
                }
            }
        }

        None
    }
}

impl Default for DependencyAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
