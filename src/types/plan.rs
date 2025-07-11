use chrono::{DateTime, Utc};
use petgraph::graph::NodeIndex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

use super::strategy::ExecutionStrategy;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPlan {
    pub metadata: PlanMetadata,
    pub plays: Vec<PlayPlan>,
    pub binary_deployments: Vec<BinaryDeployment>,
    pub total_tasks: usize,
    pub estimated_duration: Option<Duration>,
    pub estimated_compilation_time: Option<Duration>,
    pub parallelism_score: f32,
    pub network_efficiency_score: f32,
    pub hosts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanMetadata {
    pub created_at: DateTime<Utc>,
    pub rustle_version: String,
    pub playbook_hash: String,
    pub inventory_hash: String,
    pub planning_options: PlanningOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayPlan {
    pub play_id: String,
    pub name: String,
    pub strategy: ExecutionStrategy,
    pub serial: Option<u32>,
    pub hosts: Vec<String>,
    pub batches: Vec<ExecutionBatch>,
    pub handlers: Vec<HandlerPlan>,
    pub estimated_duration: Option<Duration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionBatch {
    pub batch_id: String,
    pub hosts: Vec<String>,
    pub tasks: Vec<TaskPlan>,
    pub parallel_groups: Vec<ParallelGroup>,
    pub dependencies: Vec<String>,
    pub estimated_duration: Option<Duration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskPlan {
    pub task_id: String,
    pub name: String,
    pub module: String,
    pub args: HashMap<String, serde_json::Value>,
    pub hosts: Vec<String>,
    pub dependencies: Vec<String>,
    pub conditions: Vec<ExecutionCondition>,
    pub tags: Vec<String>,
    pub notify: Vec<String>,
    pub execution_order: u32,
    pub can_run_parallel: bool,
    pub estimated_duration: Option<Duration>,
    pub risk_level: RiskLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelGroup {
    pub group_id: String,
    pub tasks: Vec<String>,
    pub max_parallelism: u32,
    pub shared_resources: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandlerPlan {
    pub handler_id: String,
    pub name: String,
    pub module: String,
    pub args: HashMap<String, serde_json::Value>,
    pub conditions: Vec<ExecutionCondition>,
    pub execution_order: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionCondition {
    When { expression: String },
    Tag { tags: Vec<String> },
    Host { pattern: String },
    SkipTag { tags: Vec<String> },
    CheckMode { enabled: bool },
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum RiskLevel {
    #[default]
    Low, // Read-only operations
    Medium,   // File modifications
    High,     // Service restarts, system changes
    Critical, // Destructive operations
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryDeployment {
    pub deployment_id: String,
    pub target_hosts: Vec<String>,
    pub binary_name: String,
    pub tasks: Vec<String>,
    pub modules: Vec<String>,
    pub embedded_data: BinaryEmbeddedData,
    pub execution_mode: BinaryExecutionMode,
    pub estimated_size: u64,
    pub compilation_requirements: CompilationRequirements,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryEmbeddedData {
    pub execution_plan: String, // Subset of execution plan for this binary
    pub static_files: Vec<EmbeddedFile>,
    pub variables: HashMap<String, serde_json::Value>,
    pub facts_required: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddedFile {
    pub src_path: String,
    pub dest_path: String,
    pub checksum: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BinaryExecutionMode {
    Standalone, // Binary runs independently
    Controller, // Binary reports back to controller
    Hybrid,     // Binary handles some tasks, SSH for others
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilationRequirements {
    pub target_arch: String,
    pub target_os: String,
    pub rust_version: String,
    pub cross_compilation: bool,
    pub static_linking: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanningOptions {
    pub limit: Option<String>,
    pub tags: Vec<String>,
    pub skip_tags: Vec<String>,
    pub check_mode: bool,
    pub diff_mode: bool,
    pub forks: u32,
    pub serial: Option<u32>,
    pub strategy: ExecutionStrategy,
    pub binary_threshold: u32,
    pub force_binary: bool,
    pub force_ssh: bool,
}

// Input data structures (from rustle-parse)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedPlaybook {
    pub name: String,
    pub plays: Vec<ParsedPlay>,
    pub vars: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedPlay {
    pub name: String,
    pub hosts: Vec<String>,
    pub tasks: Vec<ParsedTask>,
    pub handlers: Vec<ParsedHandler>,
    pub vars: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedTask {
    pub id: String,
    pub name: String,
    pub module: String,
    pub args: HashMap<String, serde_json::Value>,
    pub dependencies: Vec<String>,
    pub tags: Vec<String>,
    pub when: Option<String>,
    pub notify: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedHandler {
    pub id: String,
    pub name: String,
    pub module: String,
    pub args: HashMap<String, serde_json::Value>,
    pub when: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedInventory {
    pub hosts: Vec<String>,
    pub groups: HashMap<String, Vec<String>>,
    pub vars: HashMap<String, serde_json::Value>,
}

// Analysis structures
#[derive(Debug, Clone)]
pub struct DependencyGraph {
    pub graph: petgraph::Graph<String, DependencyType>,
    pub task_nodes: HashMap<String, NodeIndex>,
}

#[derive(Debug, Clone)]
pub enum DependencyType {
    Explicit,
    FileOutput,
    ServicePackage,
    ImplicitOrder,
}

#[derive(Debug, Clone)]
pub struct TaskGroup {
    pub id: String,
    pub tasks: Vec<TaskPlan>,
    pub hosts: Vec<String>,
    pub modules: Vec<String>,
    pub network_operations: u32,
}

#[derive(Debug, Clone)]
pub struct BinarySuitabilityAnalysis {
    pub suitable_groups: Vec<TaskGroup>,
    pub unsuitable_tasks: Vec<String>,
    pub reasons: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub enum BinaryDeploymentDecision {
    Deploy {
        reason: String,
        estimated_benefit: f32,
    },
    Skip {
        reason: String,
    },
}

#[derive(Debug, Clone)]
pub struct ValidationReport {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl DependencyGraph {
    pub fn new(graph: petgraph::Graph<String, DependencyType>) -> Self {
        let mut task_nodes = HashMap::new();
        for node_index in graph.node_indices() {
            if let Some(task_id) = graph.node_weight(node_index) {
                task_nodes.insert(task_id.clone(), node_index);
            }
        }

        Self { graph, task_nodes }
    }

    pub fn has_path(&self, from: &str, to: &str) -> bool {
        if let (Some(&from_node), Some(&to_node)) =
            (self.task_nodes.get(from), self.task_nodes.get(to))
        {
            petgraph::algo::has_path_connecting(&self.graph, from_node, to_node, None)
        } else {
            false
        }
    }
}
