use crate::planner::error::PlanError;
use crate::types::*;

pub struct ConditionEvaluator;

impl ConditionEvaluator {
    pub fn new() -> Self {
        Self
    }

    pub fn should_execute_task(
        &self,
        task: &TaskPlan,
        context: &ExecutionContext,
    ) -> Result<bool, PlanError> {
        for condition in &task.conditions {
            if !self.evaluate_condition(condition, context)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn evaluate_condition(
        &self,
        condition: &ExecutionCondition,
        context: &ExecutionContext,
    ) -> Result<bool, PlanError> {
        match condition {
            ExecutionCondition::When { expression } => {
                // Simplified expression evaluation
                // In a real implementation, this would use a proper expression parser
                Ok(!expression.is_empty())
            }
            ExecutionCondition::Tag { tags } => {
                Ok(tags.iter().any(|tag| context.active_tags.contains(tag)))
            }
            ExecutionCondition::Host { pattern } => Ok(context.current_host.contains(pattern)),
            ExecutionCondition::SkipTag { tags } => {
                Ok(!tags.iter().any(|tag| context.active_tags.contains(tag)))
            }
            ExecutionCondition::CheckMode { enabled } => Ok(*enabled == context.check_mode),
        }
    }
}

pub struct ExecutionContext {
    pub current_host: String,
    pub active_tags: Vec<String>,
    pub check_mode: bool,
    pub variables: std::collections::HashMap<String, serde_json::Value>,
}

impl Default for ConditionEvaluator {
    fn default() -> Self {
        Self::new()
    }
}
