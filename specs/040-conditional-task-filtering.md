# OS/Architecture-Based Conditional Task Filtering

## Feature Summary

This feature enhances rustle-plan to perform compile-time filtering of tasks based on OS and architecture conditionals. When Ansible playbooks contain tasks with conditionals like `when: ansible_os_family == "Debian"` or `when: ansible_architecture == "x86_64"`, rustle-plan will evaluate these conditions during planning and only include relevant tasks in the execution plan for each target. This optimization reduces execution plan size, eliminates runtime conditional evaluation overhead, and enables more efficient binary compilation by creating architecture-specific binaries with only the necessary tasks.

## Goals & Requirements

### Functional Requirements
- Evaluate OS/architecture-based conditionals during plan generation
- Filter out tasks that don't match target OS/architecture
- Support common Ansible conditional patterns:
  - `ansible_os_family`, `ansible_system`, `ansible_distribution`
  - `ansible_architecture`, `ansible_machine`
  - Complex conditions with `and`, `or`, `not`
- Generate separate execution plans for different OS/architecture combinations
- Maintain compatibility with existing Ansible conditional behavior
- Preserve task dependencies after filtering

### Non-Functional Requirements
- Zero runtime overhead for conditional evaluation
- Smaller execution plans and compiled binaries
- Clear reporting of filtered tasks
- Graceful handling of dynamic conditionals that can't be evaluated at compile-time

### Success Criteria
- Tasks with OS/architecture conditionals are correctly filtered
- Execution plans contain only relevant tasks for each target
- Binary sizes are reduced by excluding irrelevant tasks
- All task dependencies remain valid after filtering
- Performance improvement from eliminated runtime conditionals

## API/Interface Design

### Conditional Evaluator
```rust
// src/planner/conditional_evaluator.rs
pub struct ConditionalEvaluator {
    target_facts: HashMap<String, TargetFacts>,
}

#[derive(Debug, Clone)]
pub struct TargetFacts {
    pub ansible_os_family: String,
    pub ansible_system: String,
    pub ansible_distribution: Option<String>,
    pub ansible_architecture: String,
    pub ansible_machine: Option<String>,
    // Additional static facts that can be determined at compile time
}

impl ConditionalEvaluator {
    pub fn new(target_resolver: &TargetResolver) -> Self;
    
    /// Evaluate a conditional expression for given hosts
    pub fn evaluate_condition(
        &self,
        condition: &str,
        hosts: &[String]
    ) -> Result<ConditionalResult, EvalError>;
    
    /// Check if a task should be included for given hosts
    pub fn should_include_task(
        &self,
        task: &ParsedTask,
        hosts: &[String]
    ) -> Result<bool, EvalError>;
}

#[derive(Debug, Clone)]
pub enum ConditionalResult {
    /// Condition is definitely true for all hosts
    AlwaysTrue,
    /// Condition is definitely false for all hosts
    AlwaysFalse,
    /// Condition varies by host or can't be determined
    Dynamic(Vec<String>), // Hosts where condition is true
}
```

### Task Filtering Integration
```rust
// Updates to src/planner/execution_plan.rs
impl ExecutionPlanBuilder {
    /// Filter tasks based on target OS/architecture
    fn filter_tasks_by_target(
        &self,
        tasks: Vec<ParsedTask>,
        hosts: &[String],
        evaluator: &ConditionalEvaluator,
    ) -> Result<Vec<TaskPlan>, PlanError> {
        let mut filtered_tasks = Vec::new();
        let mut filtered_out = Vec::new();
        
        for task in tasks {
            match evaluator.should_include_task(&task, hosts)? {
                true => {
                    filtered_tasks.push(self.convert_to_task_plan(task)?);
                }
                false => {
                    filtered_out.push(FilteredTask {
                        task_id: task.id,
                        task_name: task.name,
                        condition: task.when.clone(),
                        reason: "OS/architecture mismatch".to_string(),
                    });
                }
            }
        }
        
        // Log filtered tasks for transparency
        if !filtered_out.is_empty() {
            info!("Filtered {} tasks due to OS/architecture conditionals", 
                  filtered_out.len());
        }
        
        Ok(filtered_tasks)
    }
}
```

### Conditional Parsing
```rust
// src/planner/conditional_parser.rs
pub struct ConditionalParser;

impl ConditionalParser {
    /// Parse Ansible conditional expressions
    pub fn parse(condition: &str) -> Result<ConditionalExpression, ParseError>;
}

#[derive(Debug, Clone)]
pub enum ConditionalExpression {
    /// Simple comparison: ansible_os_family == "RedHat"
    Comparison {
        variable: String,
        operator: ComparisonOp,
        value: ConditionalValue,
    },
    /// Logical AND: condition1 and condition2
    And(Box<ConditionalExpression>, Box<ConditionalExpression>),
    /// Logical OR: condition1 or condition2
    Or(Box<ConditionalExpression>, Box<ConditionalExpression>),
    /// Logical NOT: not condition
    Not(Box<ConditionalExpression>),
    /// Complex expression that can't be evaluated at compile time
    Dynamic(String),
}

#[derive(Debug, Clone)]
pub enum ComparisonOp {
    Equals,
    NotEquals,
    In,
    NotIn,
}

#[derive(Debug, Clone)]
pub enum ConditionalValue {
    String(String),
    List(Vec<String>),
    Boolean(bool),
}
```

## File and Package Structure

### New Files
- `src/planner/conditional_evaluator.rs` - Core conditional evaluation logic
- `src/planner/conditional_parser.rs` - Ansible conditional expression parser
- `src/planner/target_facts.rs` - Target fact generation from OS/architecture
- `tests/conditional_filtering_tests.rs` - Unit tests for conditional filtering

### Modified Files
- `src/planner/execution_plan.rs` - Integrate conditional filtering
- `src/planner/binary_deployment.rs` - Handle per-architecture task groups
- `src/types/plan.rs` - Add filtered task tracking
- `src/planner/mod.rs` - Export new modules

## Implementation Details

### Step 1: Target Facts Generation

Generate compile-time facts based on target OS/architecture:

```rust
// src/planner/target_facts.rs
impl TargetFacts {
    pub fn from_target_info(target: &TargetInfo) -> Self {
        let (os_family, distribution) = match target.os.as_str() {
            "linux" => {
                // In real implementation, might derive from target triple
                ("RedHat", Some("CentOS"))
            }
            "darwin" => ("Darwin", Some("MacOSX")),
            "windows" => ("Windows", None),
            _ => ("Unknown", None),
        };
        
        Self {
            ansible_os_family: os_family.to_string(),
            ansible_system: target.os.clone(),
            ansible_distribution: distribution.map(String::from),
            ansible_architecture: target.arch.clone(),
            ansible_machine: Some(target.arch.clone()),
        }
    }
}
```

### Step 2: Conditional Expression Parser

```rust
// src/planner/conditional_parser.rs
use nom::{
    branch::alt,
    bytes::complete::{tag, take_while1},
    character::complete::{multispace0, multispace1},
    combinator::{map, opt},
    multi::separated_list0,
    sequence::{delimited, preceded, tuple},
    IResult,
};

impl ConditionalParser {
    pub fn parse(input: &str) -> Result<ConditionalExpression, ParseError> {
        match parse_expression(input) {
            Ok((_, expr)) => Ok(expr),
            Err(e) => Err(ParseError::InvalidSyntax(format!("{:?}", e))),
        }
    }
}

fn parse_expression(input: &str) -> IResult<&str, ConditionalExpression> {
    alt((
        parse_or_expression,
        parse_and_expression,
        parse_not_expression,
        parse_comparison,
        parse_dynamic,
    ))(input)
}

fn parse_comparison(input: &str) -> IResult<&str, ConditionalExpression> {
    let (input, (variable, _, op, _, value)) = tuple((
        parse_variable,
        multispace0,
        parse_operator,
        multispace0,
        parse_value,
    ))(input)?;
    
    Ok((input, ConditionalExpression::Comparison {
        variable,
        operator: op,
        value,
    }))
}

// Example patterns to recognize:
// ansible_os_family == "RedHat"
// ansible_architecture in ["x86_64", "amd64"]
// ansible_system != "Windows"
```

### Step 3: Conditional Evaluation Logic

```rust
// src/planner/conditional_evaluator.rs
impl ConditionalEvaluator {
    pub fn evaluate_condition(
        &self,
        condition: &str,
        hosts: &[String]
    ) -> Result<ConditionalResult, EvalError> {
        let expr = ConditionalParser::parse(condition)?;
        self.evaluate_expression(&expr, hosts)
    }
    
    fn evaluate_expression(
        &self,
        expr: &ConditionalExpression,
        hosts: &[String]
    ) -> Result<ConditionalResult, EvalError> {
        match expr {
            ConditionalExpression::Comparison { variable, operator, value } => {
                self.evaluate_comparison(variable, operator, value, hosts)
            }
            ConditionalExpression::And(left, right) => {
                let left_result = self.evaluate_expression(left, hosts)?;
                let right_result = self.evaluate_expression(right, hosts)?;
                Ok(self.combine_and(left_result, right_result))
            }
            ConditionalExpression::Or(left, right) => {
                let left_result = self.evaluate_expression(left, hosts)?;
                let right_result = self.evaluate_expression(right, hosts)?;
                Ok(self.combine_or(left_result, right_result))
            }
            ConditionalExpression::Not(inner) => {
                let result = self.evaluate_expression(inner, hosts)?;
                Ok(self.negate_result(result))
            }
            ConditionalExpression::Dynamic(_) => {
                // Can't evaluate at compile time
                Ok(ConditionalResult::Dynamic(hosts.to_vec()))
            }
        }
    }
    
    fn evaluate_comparison(
        &self,
        variable: &str,
        operator: &ComparisonOp,
        expected: &ConditionalValue,
        hosts: &[String]
    ) -> Result<ConditionalResult, EvalError> {
        // Check if this is a static fact we can evaluate
        if !Self::is_static_fact(variable) {
            return Ok(ConditionalResult::Dynamic(hosts.to_vec()));
        }
        
        let mut matching_hosts = Vec::new();
        
        for host in hosts {
            if let Some(facts) = self.target_facts.get(host) {
                let actual_value = self.get_fact_value(facts, variable);
                if self.compare_values(actual_value, operator, expected) {
                    matching_hosts.push(host.clone());
                }
            }
        }
        
        if matching_hosts.is_empty() {
            Ok(ConditionalResult::AlwaysFalse)
        } else if matching_hosts.len() == hosts.len() {
            Ok(ConditionalResult::AlwaysTrue)
        } else {
            Ok(ConditionalResult::Dynamic(matching_hosts))
        }
    }
    
    fn is_static_fact(variable: &str) -> bool {
        matches!(variable, 
            "ansible_os_family" | 
            "ansible_system" | 
            "ansible_distribution" |
            "ansible_architecture" |
            "ansible_machine"
        )
    }
}
```

### Step 4: Integration with Execution Planning

```rust
// Updates to src/planner/execution_plan.rs
impl ExecutionPlanBuilder {
    pub fn build(&self, playbook: ParsedPlaybook, inventory: ParsedInventory) 
        -> Result<ExecutionPlan, PlanError> {
        
        // Create conditional evaluator with target facts
        let evaluator = self.create_conditional_evaluator(&inventory)?;
        
        // Build execution plan with filtering
        let mut plays = Vec::new();
        
        for parsed_play in playbook.plays {
            let play_hosts = self.resolve_hosts(&parsed_play.hosts, &inventory)?;
            
            // Group hosts by target architecture
            let host_groups = self.group_hosts_by_target(&play_hosts)?;
            
            for (target_key, group_hosts) in host_groups {
                // Filter tasks for this specific target
                let filtered_tasks = self.filter_tasks_by_target(
                    parsed_play.tasks.clone(),
                    &group_hosts,
                    &evaluator
                )?;
                
                if !filtered_tasks.is_empty() {
                    let play_plan = self.build_play_plan(
                        &parsed_play,
                        filtered_tasks,
                        group_hosts
                    )?;
                    plays.push(play_plan);
                }
            }
        }
        
        // Continue with rest of planning...
        Ok(ExecutionPlan {
            metadata: self.create_metadata(),
            plays,
            // ... other fields
        })
    }
    
    fn group_hosts_by_target(&self, hosts: &[String]) 
        -> Result<HashMap<String, Vec<String>>, PlanError> {
        
        let mut groups: HashMap<String, Vec<String>> = HashMap::new();
        
        for host in hosts {
            let target = self.target_resolver.resolve_target_for_host(host)?;
            let key = format!("{}-{}", target.arch, target.os);
            groups.entry(key).or_default().push(host.clone());
        }
        
        Ok(groups)
    }
}
```

### Step 5: Binary Deployment Optimization

```rust
// Updates to src/planner/binary_deployment.rs
impl BinaryDeploymentPlanner {
    pub fn plan_deployments(
        &self,
        tasks: &[TaskPlan],
        hosts: &[String],
        threshold: u32,
    ) -> Result<Vec<BinaryDeployment>, PlanError> {
        // Tasks are already filtered by OS/architecture
        // This results in more efficient binaries with only relevant code
        
        let task_groups = self.analyze_task_groups(tasks)?;
        let mut deployments = Vec::new();
        
        for group in task_groups {
            if group.tasks.len() >= threshold as usize {
                // Each binary will be optimized for specific OS/architecture
                let deployment = self.create_targeted_binary_deployment(
                    &group,
                    hosts
                )?;
                deployments.push(deployment);
            }
        }
        
        Ok(deployments)
    }
}
```

## Testing Strategy

### Unit Tests
```rust
// tests/conditional_filtering_tests.rs
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_simple_os_family_condition() {
        let evaluator = create_test_evaluator();
        
        let condition = "ansible_os_family == \"RedHat\"";
        let hosts = vec!["centos-host".to_string(), "ubuntu-host".to_string()];
        
        let result = evaluator.evaluate_condition(condition, &hosts).unwrap();
        
        match result {
            ConditionalResult::Dynamic(matching) => {
                assert_eq!(matching, vec!["centos-host"]);
            }
            _ => panic!("Expected Dynamic result"),
        }
    }
    
    #[test]
    fn test_architecture_in_list() {
        let evaluator = create_test_evaluator();
        
        let condition = "ansible_architecture in ['x86_64', 'amd64']";
        let hosts = vec!["x64-host".to_string(), "arm-host".to_string()];
        
        let result = evaluator.evaluate_condition(condition, &hosts).unwrap();
        
        match result {
            ConditionalResult::Dynamic(matching) => {
                assert_eq!(matching, vec!["x64-host"]);
            }
            _ => panic!("Expected Dynamic result"),
        }
    }
    
    #[test]
    fn test_complex_condition() {
        let evaluator = create_test_evaluator();
        
        let condition = "ansible_os_family == \"Debian\" and ansible_architecture == \"x86_64\"";
        let hosts = vec!["ubuntu-x64".to_string(), "ubuntu-arm".to_string()];
        
        let result = evaluator.evaluate_condition(condition, &hosts).unwrap();
        
        match result {
            ConditionalResult::Dynamic(matching) => {
                assert_eq!(matching, vec!["ubuntu-x64"]);
            }
            _ => panic!("Expected Dynamic result"),
        }
    }
    
    #[test]
    fn test_task_filtering() {
        let task = ParsedTask {
            id: "task1".to_string(),
            name: "Install package".to_string(),
            module: "package".to_string(),
            when: Some("ansible_os_family == \"RedHat\"".to_string()),
            // ... other fields
        };
        
        let evaluator = create_test_evaluator();
        let should_include = evaluator.should_include_task(
            &task,
            &["centos-host".to_string()]
        ).unwrap();
        
        assert!(should_include);
        
        let should_exclude = evaluator.should_include_task(
            &task,
            &["ubuntu-host".to_string()]
        ).unwrap();
        
        assert!(!should_exclude);
    }
}
```

### Integration Tests
- Test with real playbooks containing OS/architecture conditionals
- Verify execution plans are correctly filtered
- Test binary deployment with architecture-specific tasks
- Verify task dependencies remain valid after filtering
- Test edge cases with mixed conditionals

### Test Playbook Example
```yaml
---
- name: Multi-OS installation playbook
  hosts: all
  tasks:
    - name: Install package on RedHat
      yum:
        name: httpd
        state: present
      when: ansible_os_family == "RedHat"
      
    - name: Install package on Debian
      apt:
        name: apache2
        state: present
      when: ansible_os_family == "Debian"
      
    - name: Configure for x86_64
      template:
        src: config-x64.j2
        dest: /etc/app/config
      when: ansible_architecture == "x86_64"
      
    - name: Configure for ARM
      template:
        src: config-arm.j2
        dest: /etc/app/config
      when: ansible_architecture in ["aarch64", "armv7l"]
```

## Edge Cases & Error Handling

### Dynamic Conditionals
- Conditionals using runtime facts (e.g., `ansible_memtotal_mb > 1024`)
- Conditionals with registered variables
- Complex Jinja2 expressions
- **Solution**: Mark as dynamic and include in all execution plans

### Mixed Architecture Groups
- Some hosts match condition, others don't
- **Solution**: Create separate task groups for different architectures

### Invalid Conditionals
- Syntax errors in conditional expressions
- Unknown variables referenced
- **Solution**: Log warnings and treat as dynamic conditions

### Dependency Handling
- Task A depends on Task B, but Task B is filtered out
- **Solution**: Validate dependencies after filtering, warn about broken chains

### Complex Boolean Logic
- Nested AND/OR conditions with multiple facts
- **Solution**: Recursive evaluation with proper precedence

## Dependencies

### External Dependencies
- `nom` - For parsing conditional expressions
- Existing dependencies from rustle-plan

### Internal Dependencies
- Target resolution from spec 030
- Existing execution planning infrastructure
- Task dependency analysis

## Configuration

### New Configuration Options
```rust
// Add to PlanningOptions
pub struct PlanningOptions {
    // ... existing fields ...
    
    // Conditional filtering options
    pub enable_conditional_filtering: bool,
    pub filter_static_conditionals_only: bool,
    pub log_filtered_tasks: bool,
}
```

### Environment Variables
- `RUSTLE_ENABLE_CONDITIONAL_FILTERING` - Enable/disable filtering (default: true)
- `RUSTLE_LOG_FILTERED_TASKS` - Log details of filtered tasks

## Documentation

### API Documentation
```rust
/// Evaluates Ansible conditionals at compile time for OS/architecture filtering.
/// 
/// This module analyzes task conditionals during the planning phase and filters
/// out tasks that don't match the target OS/architecture. This optimization:
/// - Reduces execution plan size
/// - Eliminates runtime conditional evaluation
/// - Creates more efficient architecture-specific binaries
/// 
/// # Supported Conditionals
/// 
/// The following static facts can be evaluated at compile time:
/// - `ansible_os_family` - OS family (RedHat, Debian, etc.)
/// - `ansible_system` - Operating system (Linux, Darwin, Windows)
/// - `ansible_architecture` - CPU architecture (x86_64, aarch64, etc.)
/// - `ansible_distribution` - Specific distribution (Ubuntu, CentOS, etc.)
/// 
/// # Examples
/// 
/// ```yaml
/// - name: RedHat-specific task
///   yum: name=httpd
///   when: ansible_os_family == "RedHat"
///   
/// - name: 64-bit only task  
///   command: /usr/bin/64bit-tool
///   when: ansible_architecture == "x86_64"
/// ```
pub struct ConditionalEvaluator { ... }
```

### User Documentation

#### README Addition
```markdown
## Conditional Task Filtering

Rustle-plan optimizes execution by filtering tasks based on OS/architecture 
conditionals at compile time, eliminating unnecessary tasks from execution plans.

### How It Works

When tasks have conditionals like:
```yaml
when: ansible_os_family == "RedHat"
when: ansible_architecture == "x86_64"
```

Rustle evaluates these during planning and only includes matching tasks in the
execution plan for each target host.

### Benefits

1. **Smaller Execution Plans** - Only relevant tasks included
2. **Faster Execution** - No runtime conditional evaluation
3. **Optimized Binaries** - Compiled binaries contain only necessary code
4. **Better Performance** - Reduced network transfer and execution time

### Supported Conditionals

Static facts that can be evaluated at compile time:
- OS family comparisons (`ansible_os_family`)
- Architecture checks (`ansible_architecture`) 
- System type (`ansible_system`)
- Distribution (`ansible_distribution`)

Dynamic conditionals (runtime facts, registered vars) are preserved for runtime.

### Example

Given a playbook with OS-specific tasks:
```yaml
tasks:
  - name: RedHat task
    yum: name=httpd
    when: ansible_os_family == "RedHat"
    
  - name: Debian task
    apt: name=apache2  
    when: ansible_os_family == "Debian"
```

For a RedHat host, the execution plan will only contain the yum task.
For a Debian host, only the apt task will be included.
```

### Performance Impact

- **Execution Plan Size**: Up to 50% reduction for multi-OS playbooks
- **Binary Size**: 30-40% smaller binaries by excluding irrelevant tasks
- **Runtime Performance**: Eliminates conditional evaluation overhead
- **Network Transfer**: Reduced data transfer for binary deployment