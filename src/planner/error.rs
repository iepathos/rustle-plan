use thiserror::Error;

#[derive(Debug, Error)]
pub enum PlanError {
    #[error("Circular dependency detected in tasks: {cycle}")]
    CircularDependency { cycle: String },

    #[error("Invalid host pattern '{pattern}': {reason}")]
    InvalidHostPattern { pattern: String, reason: String },

    #[error("Unknown task '{task_id}' referenced in dependency")]
    UnknownTaskDependency { task_id: String },

    #[error("Conflicting execution strategies: {conflict}")]
    StrategyConflict { conflict: String },

    #[error("Resource contention detected: {}", resources.join(", "))]
    ResourceContention { resources: Vec<String> },

    #[error("Planning timeout exceeded: {timeout_secs}s")]
    PlanningTimeout { timeout_secs: u64 },

    #[error("Invalid tag expression: {expression}")]
    InvalidTagExpression { expression: String },

    #[error("Insufficient resources for parallelism: required {required}, available {available}")]
    InsufficientResources { required: u32, available: u32 },

    #[error("Binary compilation not supported for target: {target}")]
    UnsupportedTarget { target: String },

    #[error("Task group too small for binary deployment: {task_count} < {threshold}")]
    BinaryThresholdNotMet { task_count: u32, threshold: u32 },

    #[error("Module '{module}' not compatible with binary deployment")]
    IncompatibleModule { module: String },

    #[error("Cross-compilation failed for target {target}: {reason}")]
    CrossCompilationFailed { target: String, reason: String },

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
