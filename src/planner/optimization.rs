use crate::planner::error::PlanError;
use crate::types::*;

pub struct ExecutionOptimizer;

impl ExecutionOptimizer {
    pub fn new() -> Self {
        Self
    }

    pub fn optimize_order(&self, tasks: &[TaskPlan]) -> Result<Vec<TaskPlan>, PlanError> {
        // Simple optimization: move low-risk, fast tasks first
        let mut optimized_tasks = tasks.to_vec();

        optimized_tasks.sort_by(|a, b| {
            // Sort by risk level first (low risk first)
            match a.risk_level.cmp(&b.risk_level) {
                std::cmp::Ordering::Equal => {
                    // Then by estimated duration (shorter first)
                    a.estimated_duration.cmp(&b.estimated_duration)
                }
                other => other,
            }
        });

        Ok(optimized_tasks)
    }
}

impl Default for ExecutionOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialOrd for RiskLevel {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for RiskLevel {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (RiskLevel::Low, RiskLevel::Low) => std::cmp::Ordering::Equal,
            (RiskLevel::Low, _) => std::cmp::Ordering::Less,
            (RiskLevel::Medium, RiskLevel::Low) => std::cmp::Ordering::Greater,
            (RiskLevel::Medium, RiskLevel::Medium) => std::cmp::Ordering::Equal,
            (RiskLevel::Medium, _) => std::cmp::Ordering::Less,
            (RiskLevel::High, RiskLevel::Critical) => std::cmp::Ordering::Less,
            (RiskLevel::High, RiskLevel::High) => std::cmp::Ordering::Equal,
            (RiskLevel::High, _) => std::cmp::Ordering::Greater,
            (RiskLevel::Critical, RiskLevel::Critical) => std::cmp::Ordering::Equal,
            (RiskLevel::Critical, _) => std::cmp::Ordering::Greater,
        }
    }
}

impl PartialEq for RiskLevel {
    fn eq(&self, other: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

impl Eq for RiskLevel {}
