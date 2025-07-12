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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_test_context() -> ExecutionContext {
        ExecutionContext {
            current_host: "web-server-01".to_string(),
            active_tags: vec!["production".to_string(), "web".to_string()],
            check_mode: false,
            variables: HashMap::new(),
        }
    }

    fn create_test_task() -> TaskPlan {
        TaskPlan {
            task_id: "test-task".to_string(),
            name: "Test Task".to_string(),
            module: "shell".to_string(),
            args: HashMap::new(),
            hosts: vec!["web-server-01".to_string()],
            dependencies: vec![],
            conditions: vec![],
            tags: vec![],
            notify: vec![],
            execution_order: 1,
            can_run_parallel: true,
            estimated_duration: None,
            risk_level: RiskLevel::Low,
        }
    }

    #[test]
    fn test_new() {
        let evaluator = ConditionEvaluator::new();
        assert!(std::ptr::eq(&evaluator, &evaluator));
    }

    #[test]
    fn test_default() {
        let evaluator = ConditionEvaluator::default();
        assert!(std::ptr::eq(&evaluator, &evaluator));
    }

    #[test]
    fn test_should_execute_task_no_conditions() {
        let evaluator = ConditionEvaluator::new();
        let task = create_test_task();
        let context = create_test_context();

        let result = evaluator.should_execute_task(&task, &context).unwrap();
        assert!(result);
    }

    #[test]
    fn test_should_execute_task_with_passing_conditions() {
        let evaluator = ConditionEvaluator::new();
        let mut task = create_test_task();
        task.conditions = vec![
            ExecutionCondition::When {
                expression: "not_empty".to_string(),
            },
            ExecutionCondition::Tag {
                tags: vec!["production".to_string()],
            },
        ];
        let context = create_test_context();

        let result = evaluator.should_execute_task(&task, &context).unwrap();
        assert!(result);
    }

    #[test]
    fn test_should_execute_task_with_failing_condition() {
        let evaluator = ConditionEvaluator::new();
        let mut task = create_test_task();
        task.conditions = vec![
            ExecutionCondition::When {
                expression: "not_empty".to_string(),
            },
            ExecutionCondition::Tag {
                tags: vec!["staging".to_string()],
            },
        ];
        let context = create_test_context();

        let result = evaluator.should_execute_task(&task, &context).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_evaluate_when_condition_non_empty() {
        let evaluator = ConditionEvaluator::new();
        let condition = ExecutionCondition::When {
            expression: "some_variable".to_string(),
        };
        let context = create_test_context();

        let result = evaluator.evaluate_condition(&condition, &context).unwrap();
        assert!(result);
    }

    #[test]
    fn test_evaluate_when_condition_empty() {
        let evaluator = ConditionEvaluator::new();
        let condition = ExecutionCondition::When {
            expression: "".to_string(),
        };
        let context = create_test_context();

        let result = evaluator.evaluate_condition(&condition, &context).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_evaluate_tag_condition_matching() {
        let evaluator = ConditionEvaluator::new();
        let condition = ExecutionCondition::Tag {
            tags: vec!["production".to_string(), "staging".to_string()],
        };
        let context = create_test_context();

        let result = evaluator.evaluate_condition(&condition, &context).unwrap();
        assert!(result);
    }

    #[test]
    fn test_evaluate_tag_condition_not_matching() {
        let evaluator = ConditionEvaluator::new();
        let condition = ExecutionCondition::Tag {
            tags: vec!["staging".to_string(), "development".to_string()],
        };
        let context = create_test_context();

        let result = evaluator.evaluate_condition(&condition, &context).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_evaluate_host_condition_matching() {
        let evaluator = ConditionEvaluator::new();
        let condition = ExecutionCondition::Host {
            pattern: "web-server".to_string(),
        };
        let context = create_test_context();

        let result = evaluator.evaluate_condition(&condition, &context).unwrap();
        assert!(result);
    }

    #[test]
    fn test_evaluate_host_condition_not_matching() {
        let evaluator = ConditionEvaluator::new();
        let condition = ExecutionCondition::Host {
            pattern: "database".to_string(),
        };
        let context = create_test_context();

        let result = evaluator.evaluate_condition(&condition, &context).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_evaluate_skip_tag_condition_should_skip() {
        let evaluator = ConditionEvaluator::new();
        let condition = ExecutionCondition::SkipTag {
            tags: vec!["production".to_string()],
        };
        let context = create_test_context();

        let result = evaluator.evaluate_condition(&condition, &context).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_evaluate_skip_tag_condition_should_not_skip() {
        let evaluator = ConditionEvaluator::new();
        let condition = ExecutionCondition::SkipTag {
            tags: vec!["staging".to_string()],
        };
        let context = create_test_context();

        let result = evaluator.evaluate_condition(&condition, &context).unwrap();
        assert!(result);
    }

    #[test]
    fn test_evaluate_check_mode_condition_enabled() {
        let evaluator = ConditionEvaluator::new();
        let condition = ExecutionCondition::CheckMode { enabled: false };
        let context = create_test_context();

        let result = evaluator.evaluate_condition(&condition, &context).unwrap();
        assert!(result);
    }

    #[test]
    fn test_evaluate_check_mode_condition_disabled() {
        let evaluator = ConditionEvaluator::new();
        let condition = ExecutionCondition::CheckMode { enabled: true };
        let context = create_test_context();

        let result = evaluator.evaluate_condition(&condition, &context).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_evaluate_check_mode_condition_matching() {
        let evaluator = ConditionEvaluator::new();
        let condition = ExecutionCondition::CheckMode { enabled: false };
        let mut context = create_test_context();
        context.check_mode = false;

        let result = evaluator.evaluate_condition(&condition, &context).unwrap();
        assert!(result);
    }

    #[test]
    fn test_execution_context_creation() {
        let context = ExecutionContext {
            current_host: "test-host".to_string(),
            active_tags: vec!["tag1".to_string(), "tag2".to_string()],
            check_mode: true,
            variables: {
                let mut vars = HashMap::new();
                vars.insert(
                    "key".to_string(),
                    serde_json::Value::String("value".to_string()),
                );
                vars
            },
        };

        assert_eq!(context.current_host, "test-host");
        assert_eq!(context.active_tags.len(), 2);
        assert!(context.check_mode);
        assert_eq!(context.variables.len(), 1);
    }
}
