use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum ExecutionStrategy {
    #[default]
    Linear,
    Rolling {
        batch_size: u32,
    },
    Free,
    HostPinned,
    BinaryHybrid, // Mix of binary deployment and SSH execution
    BinaryOnly,   // Force binary deployment where possible
}
