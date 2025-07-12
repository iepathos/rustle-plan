# Spec 010: Rustle Plan Tool

## Feature Summary

The `rustle-plan` tool is a specialized execution planner that takes parsed playbooks and generates optimized execution plans with binary deployment strategies. It analyzes task dependencies, determines parallelization opportunities, plans binary compilation requirements, and produces detailed execution graphs that optimize for minimal network overhead through target binary deployment.

**Problem it solves**: Separates execution planning from parsing and execution, enabling better optimization, binary deployment planning, and execution strategy analysis. Determines optimal binary compilation strategies to minimize network round-trips and maximize execution performance.

**High-level approach**: Create a standalone binary that reads parsed playbook data, analyzes dependencies and constraints, determines binary deployment requirements, and outputs detailed execution plans that specify both traditional SSH execution and binary deployment strategies for maximum performance.

## Goals & Requirements

### Functional Requirements
- Generate optimized execution plans from parsed playbooks
- Analyze task dependencies and execution order
- Determine parallelization opportunities within plays and across hosts
- Plan binary compilation and deployment strategies for performance optimization
- Analyze tasks for binary deployment suitability vs SSH execution
- Handle conditional execution (when clauses, tags, limits)
- Support different execution strategies (linear, rolling, batch, binary-hybrid)
- Generate execution graphs and dependency visualizations
- Provide execution time estimates including binary compilation overhead
- Support dry-run and check mode planning
- Determine optimal binary grouping and module bundling strategies

### Non-functional Requirements
- **Performance**: Plan 1000+ task playbooks in &lt;1 second
- **Memory**: Keep memory usage &lt;50MB for typical playbooks
- **Scalability**: Handle plans for 1000+ hosts efficiently
- **Accuracy**: 99%+ accuracy in dependency detection
- **Optimization**: Identify 80%+ of possible parallelization opportunities

### Success Criteria
- Generated plans execute correctly when consumed by rustle-exec and rustle-deploy
- Performance benchmarks show 10x+ improvement over Ansible planning
- Binary deployment strategies reduce network overhead by 80%+ for complex playbooks
- Parallelization strategies reduce execution time by 40%+ on multi-host deployments
- Binary vs SSH execution decisions are optimal for performance in 95%+ of cases
- Dry-run mode provides accurate execution predictions including compilation time

## API/Interface Design

### Command Line Interface
```bash
rustle-plan [OPTIONS] [PARSED_PLAYBOOK]

OPTIONS:
    -i, --inventory &lt;FILE&gt;         Parsed inventory file
    -l, --limit &lt;PATTERN&gt;          Limit execution to specific hosts
    -t, --tags &lt;TAGS&gt;              Only run tasks with these tags
    --skip-tags &lt;TAGS&gt;             Skip tasks with these tags
    -s, --strategy &lt;STRATEGY&gt;      Execution strategy [default: binary-hybrid]
    --serial &lt;NUM&gt;                 Number of hosts to run at once
    --forks &lt;NUM&gt;                  Maximum parallel processes [default: 50]
    -c, --check                    Check mode (don't make changes)
    --diff                         Show file differences
    --binary-threshold &lt;NUM&gt;       Minimum tasks to justify binary compilation [default: 5]
    --force-binary                 Force binary deployment for all suitable tasks
    --force-ssh                    Force SSH execution (disable binary deployment)
    --list-tasks                   List all planned tasks
    --list-hosts                   List all target hosts
    --list-binaries                List planned binary deployments
    --visualize                    Generate execution graph visualization
    -o, --output &lt;FORMAT&gt;          Output format: json, binary, dot [default: json]
    --optimize                     Enable execution optimizations
    --estimate-time                Include execution time estimates
    --dry-run                      Plan but don't output execution plan
    -v, --verbose                  Enable verbose output

ARGS:
    &lt;PARSED_PLAYBOOK&gt;  Path to parsed playbook file (or stdin if -)
```

### Core Data Structures

```rust
// Main execution plan output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPlan {
    pub metadata: PlanMetadata,
    pub plays: Vec&lt;PlayPlan&gt;,
    pub binary_deployments: Vec&lt;BinaryDeployment&gt;,
    pub total_tasks: usize,
    pub estimated_duration: Option&lt;Duration&gt;,
    pub estimated_compilation_time: Option&lt;Duration&gt;,
    pub parallelism_score: f32,
    pub network_efficiency_score: f32,
    pub hosts: Vec&lt;String&gt;,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanMetadata {
    pub created_at: DateTime&lt;Utc&gt;,
    pub rustle_plan_version: String,
    pub playbook_hash: String,
    pub inventory_hash: String,
    pub planning_options: PlanningOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayPlan {
    pub play_id: String,
    pub name: String,
    pub strategy: ExecutionStrategy,
    pub serial: Option&lt;u32&gt;,
    pub hosts: Vec&lt;String&gt;,
    pub batches: Vec&lt;ExecutionBatch&gt;,
    pub handlers: Vec&lt;HandlerPlan&gt;,
    pub estimated_duration: Option&lt;Duration&gt;,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionBatch {
    pub batch_id: String,
    pub hosts: Vec&lt;String&gt;,
    pub tasks: Vec&lt;TaskPlan&gt;,
    pub parallel_groups: Vec&lt;ParallelGroup&gt;,
    pub dependencies: Vec&lt;String&gt;,
    pub estimated_duration: Option&lt;Duration&gt;,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskPlan {
    pub task_id: String,
    pub name: String,
    pub module: String,
    pub args: HashMap&lt;String, Value&gt;,
    pub hosts: Vec&lt;String&gt;,
    pub dependencies: Vec&lt;String&gt;,
    pub conditions: Vec&lt;ExecutionCondition&gt;,
    pub tags: Vec&lt;String&gt;,
    pub notify: Vec&lt;String&gt;,
    pub execution_order: u32,
    pub can_run_parallel: bool,
    pub estimated_duration: Option&lt;Duration&gt;,
    pub risk_level: RiskLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelGroup {
    pub group_id: String,
    pub tasks: Vec&lt;String&gt;,
    pub max_parallelism: u32,
    pub shared_resources: Vec&lt;String&gt;,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionStrategy {
    Linear,
    Rolling { batch_size: u32 },
    Free,
    HostPinned,
    BinaryHybrid, // Mix of binary deployment and SSH execution
    BinaryOnly,   // Force binary deployment where possible
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionCondition {
    When { expression: String },
    Tag { tags: Vec&lt;String&gt; },
    Host { pattern: String },
    SkipTag { tags: Vec&lt;String&gt; },
    CheckMode { enabled: bool },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,    // Read-only operations
    Medium, // File modifications
    High,   // Service restarts, system changes
    Critical, // Destructive operations
}

// Binary deployment planning structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryDeployment {
    pub deployment_id: String,
    pub target_hosts: Vec&lt;String&gt;,
    pub binary_name: String,
    pub tasks: Vec&lt;String&gt;,
    pub modules: Vec&lt;String&gt;,
    pub embedded_data: BinaryEmbeddedData,
    pub execution_mode: BinaryExecutionMode,
    pub estimated_size: u64,
    pub compilation_requirements: CompilationRequirements,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryEmbeddedData {
    pub execution_plan: String, // Subset of execution plan for this binary
    pub static_files: Vec&lt;EmbeddedFile&gt;,
    pub variables: HashMap&lt;String, Value&gt;,
    pub facts_required: Vec&lt;String&gt;,
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
    Standalone,     // Binary runs independently
    Controller,     // Binary reports back to controller
    Hybrid,         // Binary handles some tasks, SSH for others
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilationRequirements {
    pub target_arch: String,
    pub target_os: String,
    pub rust_version: String,
    pub cross_compilation: bool,
    pub static_linking: bool,
}
```

### Planner API

```rust
pub struct ExecutionPlanner {
    strategy: ExecutionStrategy,
    forks: u32,
    optimize: bool,
    check_mode: bool,
    task_estimator: TaskEstimator,
    binary_planner: BinaryDeploymentPlanner,
    binary_threshold: u32,
}

impl ExecutionPlanner {
    pub fn new() -&gt; Self;
    pub fn with_strategy(mut self, strategy: ExecutionStrategy) -&gt; Self;
    pub fn with_forks(mut self, forks: u32) -&gt; Self;
    pub fn with_optimization(mut self, enabled: bool) -&gt; Self;
    pub fn with_check_mode(mut self, enabled: bool) -&gt; Self;
    pub fn with_binary_threshold(mut self, threshold: u32) -&gt; Self;
    
    pub fn plan_execution(
        &amp;self, 
        playbook: &amp;ParsedPlaybook, 
        inventory: &amp;ParsedInventory,
        options: &amp;PlanningOptions,
    ) -&gt; Result&lt;ExecutionPlan, PlanError&gt;;
    
    pub fn plan_binary_deployments(
        &amp;self,
        tasks: &amp;[TaskPlan],
        hosts: &amp;[String],
    ) -&gt; Result&lt;Vec&lt;BinaryDeployment&gt;, PlanError&gt;;
    
    pub fn analyze_dependencies(
        &amp;self, 
        tasks: &amp;[ParsedTask]
    ) -&gt; Result&lt;DependencyGraph, PlanError&gt;;
    
    pub fn optimize_execution_order(
        &amp;self, 
        tasks: &amp;[TaskPlan]
    ) -&gt; Result&lt;Vec&lt;TaskPlan&gt;, PlanError&gt;;
    
    pub fn estimate_duration(
        &amp;self, 
        plan: &amp;ExecutionPlan
    ) -&gt; Result&lt;Duration, PlanError&gt;;
    
    pub fn estimate_compilation_time(
        &amp;self,
        deployments: &amp;[BinaryDeployment]
    ) -&gt; Result&lt;Duration, PlanError&gt;;
    
    pub fn validate_plan(
        &amp;self, 
        plan: &amp;ExecutionPlan
    ) -&gt; Result&lt;ValidationReport, PlanError&gt;;
    
    pub fn analyze_binary_suitability(
        &amp;self,
        tasks: &amp;[TaskPlan]
    ) -&gt; Result&lt;BinarySuitabilityAnalysis, PlanError&gt;;
}

pub struct BinaryDeploymentPlanner {
    compilation_cache: CompilationCache,
    target_profiles: HashMap&lt;String, TargetProfile&gt;,
}

impl BinaryDeploymentPlanner {
    pub fn analyze_task_groups(
        &amp;self,
        tasks: &amp;[TaskPlan]
    ) -&gt; Result&lt;Vec&lt;TaskGroup&gt;, PlanError&gt;;
    
    pub fn should_use_binary(
        &amp;self,
        task_group: &amp;TaskGroup,
        threshold: u32
    ) -&gt; BinaryDeploymentDecision;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanningOptions {
    pub limit: Option&lt;String&gt;,
    pub tags: Vec&lt;String&gt;,
    pub skip_tags: Vec&lt;String&gt;,
    pub check_mode: bool,
    pub diff_mode: bool,
    pub forks: u32,
    pub serial: Option&lt;u32&gt;,
    pub strategy: ExecutionStrategy,
    pub binary_threshold: u32,
    pub force_binary: bool,
    pub force_ssh: bool,
}
```

### Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum PlanError {
    #[error("Circular dependency detected in tasks: {cycle}")]
    CircularDependency { cycle: String },
    
    #[error("Invalid host pattern '{pattern}': {reason}")]
    InvalidHostPattern { pattern: String, reason: String },
    
    #[error("Unknown task '{task_id}' referenced in dependency")]
    UnknownTaskDependency { task_id: String },
    
    #[error("Conflicting execution strategies: {conflict}")]
    StrategyConflict { conflict: String },
    
    #[error("Resource contention detected: {resources}")]
    ResourceContention { resources: Vec&lt;String&gt; },
    
    #[error("Planning timeout exceeded: {timeout_secs}s")]
    PlanningTimeout { timeout_secs: u64 },
    
    #[error("Invalid tag expression: {expression}")]
    InvalidTagExpression { expression: String },
    
    #[error("Insufficient resources for parallelism: required {required}, available {available}")]
    InsufficientResources { required: u32, available: u32 },
    
    #[error("Binary compilation not supported for target: {target}")]
    UnsupportedTarget { target: String },
    
    #[error("Task group too small for binary deployment: {task_count} &lt; {threshold}")]
    BinaryThresholdNotMet { task_count: u32, threshold: u32 },
    
    #[error("Module '{module}' not compatible with binary deployment")]
    IncompatibleModule { module: String },
    
    #[error("Cross-compilation failed for target {target}: {reason}")]
    CrossCompilationFailed { target: String, reason: String },
}
```

## File and Package Structure

```
src/bin/rustle-plan.rs          # Main binary entry point
src/planner/
├── mod.rs                      # Module exports
├── execution_plan.rs           # Core planning logic
├── binary_deployment.rs        # Binary deployment planning
├── dependency.rs               # Dependency analysis
├── optimization.rs             # Execution optimization
├── strategy.rs                 # Execution strategies
├── condition.rs                # Conditional execution
├── estimation.rs               # Time estimation
├── validation.rs               # Plan validation
├── graph.rs                    # Dependency graphs
├── suitability.rs              # Binary suitability analysis
└── error.rs                    # Error types

src/types/
├── plan.rs                     # Plan data structures
└── strategy.rs                 # Strategy definitions

tests/planner/
├── execution_plan_tests.rs
├── dependency_tests.rs
├── optimization_tests.rs
├── strategy_tests.rs
└── integration_tests.rs
```

## Implementation Details

### Phase 1: Basic Planning
1. Implement core execution plan data structures
2. Create basic dependency analysis from parsed tasks
3. Add support for linear execution strategy
4. Implement host filtering and task selection

### Phase 2: Dependency Analysis
1. Build comprehensive dependency graph from tasks
2. Implement topological sorting for execution order
3. Add circular dependency detection
4. Handle handler notification dependencies

### Phase 3: Parallelization Optimization
1. Analyze tasks for parallel execution opportunities
2. Implement resource contention detection
3. Generate parallel execution groups
4. Optimize batch sizing for different strategies

### Phase 4: Binary Deployment Planning
1. Implement binary suitability analysis
2. Add binary deployment planning logic
3. Create compilation requirement analysis
4. Implement binary grouping optimization

### Phase 5: Advanced Features
1. Add execution time estimation including compilation time
2. Implement rolling update strategies with binary deployment
3. Add plan validation and verification
4. Create visualization output formats including binary deployment flows

### Key Algorithms

**Dependency Analysis**:
```rust
fn analyze_task_dependencies(tasks: &amp;[ParsedTask]) -&gt; Result&lt;DependencyGraph, PlanError&gt; {
    let mut graph = DiGraph::new();
    let mut task_map = HashMap::new();
    
    // Add all tasks as nodes
    for task in tasks {
        let node = graph.add_node(task.id.clone());
        task_map.insert(task.id.clone(), (node, task));
    }
    
    // Add explicit dependencies
    for (node, task) in task_map.values() {
        for dep_id in &amp;task.dependencies {
            if let Some((dep_node, _)) = task_map.get(dep_id) {
                graph.add_edge(*dep_node, *node, DependencyType::Explicit);
            }
        }
        
        // Add implicit dependencies (file operations, service management)
        for other_task in tasks {
            if let Some(dependency_type) = detect_implicit_dependency(task, other_task) {
                if let Some((other_node, _)) = task_map.get(&amp;other_task.id) {
                    graph.add_edge(*other_node, *node, dependency_type);
                }
            }
        }
    }
    
    Ok(DependencyGraph::new(graph))
}

fn detect_implicit_dependency(task1: &amp;ParsedTask, task2: &amp;ParsedTask) -&gt; Option&lt;DependencyType&gt; {
    // File-based dependencies
    if let (Some(dest1), Some(dest2)) = (
        task1.args.get("dest").and_then(|v| v.as_str()),
        task2.args.get("src").and_then(|v| v.as_str())
    ) {
        if dest1 == dest2 {
            return Some(DependencyType::FileOutput);
        }
    }
    
    // Service dependencies
    if task1.module == "service" &amp;&amp; task2.module == "package" {
        if let (Some(service), Some(package)) = (
            task1.args.get("name").and_then(|v| v.as_str()),
            task2.args.get("name").and_then(|v| v.as_str())
        ) {
            if service == package {
                return Some(DependencyType::ServicePackage);
            }
        }
    }
    
    None
}
```

**Parallelization Analysis**:
```rust
fn find_parallel_groups(
    tasks: &amp;[TaskPlan], 
    dependency_graph: &amp;DependencyGraph
) -&gt; Vec&lt;ParallelGroup&gt; {
    let mut groups = Vec::new();
    let mut visited = HashSet::new();
    
    for task in tasks {
        if visited.contains(&amp;task.task_id) {
            continue;
        }
        
        let mut group_tasks = vec![task.task_id.clone()];
        visited.insert(task.task_id.clone());
        
        // Find tasks that can run in parallel with this one
        for other_task in tasks {
            if visited.contains(&amp;other_task.task_id) {
                continue;
            }
            
            if can_run_parallel(task, other_task, dependency_graph) {
                group_tasks.push(other_task.task_id.clone());
                visited.insert(other_task.task_id.clone());
            }
        }
        
        if group_tasks.len() &gt; 1 {
            groups.push(ParallelGroup {
                group_id: format!("group_{}", groups.len()),
                tasks: group_tasks,
                max_parallelism: calculate_max_parallelism(&amp;group_tasks),
                shared_resources: find_shared_resources(&amp;group_tasks),
            });
        }
    }
    
    groups
}

fn can_run_parallel(
    task1: &amp;TaskPlan, 
    task2: &amp;TaskPlan, 
    graph: &amp;DependencyGraph
) -&gt; bool {
    // Check for direct dependencies
    if graph.has_path(&amp;task1.task_id, &amp;task2.task_id) ||
       graph.has_path(&amp;task2.task_id, &amp;task1.task_id) {
        return false;
    }
    
    // Check for resource conflicts
    if has_resource_conflict(task1, task2) {
        return false;
    }
    
    // Check module-specific constraints
    if requires_exclusive_access(&amp;task1.module) || requires_exclusive_access(&amp;task2.module) {
        return false;
    }
    
    true
}
```

**Binary Deployment Analysis**:
```rust
fn analyze_binary_deployment_opportunities(
    tasks: &amp;[TaskPlan],
    hosts: &amp;[String],
    threshold: u32
) -&gt; Result&lt;Vec&lt;BinaryDeployment&gt;, PlanError&gt; {
    let mut deployments = Vec::new();
    
    // Group tasks by compatibility and locality
    let task_groups = group_tasks_for_binary_deployment(tasks)?;
    
    for group in task_groups {
        if group.tasks.len() &gt;= threshold as usize {
            let deployment = plan_binary_deployment(&amp;group, hosts)?;
            deployments.push(deployment);
        }
    }
    
    // Optimize deployment grouping
    optimize_binary_deployments(&amp;mut deployments)?;
    
    Ok(deployments)
}

fn group_tasks_for_binary_deployment(
    tasks: &amp;[TaskPlan]
) -&gt; Result&lt;Vec&lt;TaskGroup&gt;, PlanError&gt; {
    let mut groups = Vec::new();
    let mut ungrouped_tasks: Vec&lt;_&gt; = tasks.iter().collect();
    
    while !ungrouped_tasks.is_empty() {
        let seed_task = ungrouped_tasks.remove(0);
        let mut group = TaskGroup {
            id: format!("group_{}", groups.len()),
            tasks: vec![seed_task.clone()],
            hosts: seed_task.hosts.clone(),
            modules: vec![seed_task.module.clone()],
            network_operations: count_network_operations(seed_task),
        };
        
        // Find compatible tasks
        ungrouped_tasks.retain(|&amp;task| {
            if is_binary_compatible(seed_task, task) &amp;&amp; 
               has_host_overlap(&amp;group.hosts, &amp;task.hosts) {
                group.tasks.push(task.clone());
                group.modules.push(task.module.clone());
                group.network_operations += count_network_operations(task);
                false // Remove from ungrouped
            } else {
                true // Keep in ungrouped
            }
        });
        
        if group.network_operations &gt; 3 { // Threshold for binary deployment benefit
            groups.push(group);
        }
    }
    
    Ok(groups)
}

fn is_binary_compatible(task1: &amp;TaskPlan, task2: &amp;TaskPlan) -&gt; bool {
    // Tasks are binary compatible if they:
    // 1. Use modules that can be statically linked
    // 2. Don't require interactive input
    // 3. Don't have conflicting resource requirements
    
    let compatible_modules = ["file", "copy", "template", "shell", "package", "service"];
    let interactive_modules = ["pause", "prompt"];
    
    compatible_modules.contains(&amp;task1.module.as_str()) &amp;&amp;
    compatible_modules.contains(&amp;task2.module.as_str()) &amp;&amp;
    !interactive_modules.contains(&amp;task1.module.as_str()) &amp;&amp;
    !interactive_modules.contains(&amp;task2.module.as_str())
}

fn count_network_operations(task: &amp;TaskPlan) -&gt; u32 {
    match task.module.as_str() {
        "copy" | "template" | "fetch" =&gt; 2, // Upload + command
        "package" | "service" =&gt; 1,         // Command only
        "shell" | "command" =&gt; 1,           // Command only
        _ =&gt; 1,
    }
}

fn plan_binary_deployment(
    group: &amp;TaskGroup,
    hosts: &amp;[String]
) -&gt; Result&lt;BinaryDeployment, PlanError&gt; {
    let deployment_hosts: Vec&lt;String&gt; = hosts.iter()
        .filter(|host| group.hosts.contains(host))
        .cloned()
        .collect();
    
    Ok(BinaryDeployment {
        deployment_id: group.id.clone(),
        target_hosts: deployment_hosts,
        binary_name: format!("rustle-runner-{}", group.id),
        tasks: group.tasks.iter().map(|t| t.task_id.clone()).collect(),
        modules: group.modules.clone(),
        embedded_data: BinaryEmbeddedData {
            execution_plan: serialize_group_plan(group)?,
            static_files: extract_static_files(&amp;group.tasks)?,
            variables: extract_variables(&amp;group.tasks)?,
            facts_required: extract_fact_dependencies(&amp;group.tasks)?,
        },
        execution_mode: BinaryExecutionMode::Controller,
        estimated_size: estimate_binary_size(group)?,
        compilation_requirements: CompilationRequirements {
            target_arch: "x86_64".to_string(), // TODO: detect from inventory
            target_os: "linux".to_string(),
            rust_version: "1.70.0".to_string(),
            cross_compilation: false,
            static_linking: true,
        },
    })
}
```

## Testing Strategy

### Unit Tests
- **Dependency analysis**: Test graph construction with various task types
- **Parallelization**: Test parallel group detection and optimization
- **Strategy handling**: Test different execution strategies
- **Condition evaluation**: Test tag and host filtering logic

### Integration Tests
- **End-to-end planning**: Test complete planning workflows
- **Large playbooks**: Test performance with complex playbooks
- **Error scenarios**: Test error handling with invalid inputs
- **Optimization verification**: Test execution time improvements

### Test Data Structure
```
tests/fixtures/
├── playbooks/
│   ├── simple_plan.json        # Basic parsed playbook
│   ├── complex_plan.json       # Multi-play with dependencies
│   ├── parallel_tasks.json     # Tasks suitable for parallelization
│   └── rolling_update.json     # Rolling deployment scenario
├── inventories/
│   ├── small_inventory.json    # 5 hosts
│   ├── large_inventory.json    # 100+ hosts
│   └── groups_inventory.json   # Complex group structure
└── expected_plans/
    ├── simple_linear.json      # Expected linear execution plan
    ├── optimized_parallel.json # Expected optimized plan
    └── rolling_batches.json    # Expected rolling update plan
```

### Performance Benchmarks
- Planning time vs. playbook size
- Memory usage with large inventories
- Parallelization effectiveness measurements
- Comparison with Ansible planning times

## Edge Cases &amp; Error Handling

### Dependency Analysis
- Circular dependencies between tasks
- Missing task references in dependencies
- Complex conditional dependencies
- Cross-play dependencies

### Resource Management
- Memory limits with large dependency graphs
- Timeout handling for complex planning
- Resource contention detection
- Invalid parallelism constraints

### Execution Strategies
- Conflicting strategy requirements
- Serial constraints with parallel tasks
- Host availability during planning
- Failed host handling in rolling updates

## Dependencies

### External Crates
```toml
[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
clap = { version = "4", features = ["derive"] }
petgraph = "0.6"
anyhow = "1"
thiserror = "1"
tracing = "0.1"
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1", features = ["v4"] }
regex = "1"
tokio = { version = "1", features = ["time"] }
```

### Internal Dependencies
- `rustle::types` - Core type definitions
- `rustle::error` - Error handling
- `rustle-parse` output types
- Shared planning algorithms

## Configuration

### Environment Variables
- `RUSTLE_DEFAULT_FORKS`: Default parallelism level
- `RUSTLE_PLANNING_TIMEOUT`: Maximum planning time
- `RUSTLE_OPTIMIZATION_LEVEL`: Optimization aggressiveness (0-3)
- `RUSTLE_STRATEGY`: Default execution strategy

### Configuration File Support
```toml
[planner]
default_strategy = "linear"
default_forks = 50
enable_optimization = true
planning_timeout_secs = 30
max_parallelism = 100

[estimation]
enable_time_estimation = true
default_task_duration_secs = 5
module_duration_overrides = { "package" = 30, "service" = 10 }

[optimization]
enable_parallel_groups = true
resource_contention_detection = true
batch_size_optimization = true
```

## Documentation

### CLI Help Text
```
rustle-plan - Generate optimized execution plans from parsed playbooks

USAGE:
    rustle-plan [OPTIONS] [PARSED_PLAYBOOK]

ARGS:
    &lt;PARSED_PLAYBOOK&gt;    Path to parsed playbook file (or stdin if -)

OPTIONS:
    -i, --inventory &lt;FILE&gt;         Parsed inventory file
    -l, --limit &lt;PATTERN&gt;          Limit execution to specific hosts
    -t, --tags &lt;TAGS&gt;              Only run tasks with these tags
        --skip-tags &lt;TAGS&gt;         Skip tasks with these tags
    -s, --strategy &lt;STRATEGY&gt;      Execution strategy [default: linear] [possible values: linear, rolling, free]
        --serial &lt;NUM&gt;             Number of hosts to run at once
        --forks &lt;NUM&gt;              Maximum parallel processes [default: 50]
    -c, --check                    Check mode (don't make changes)
        --diff                     Show file differences
        --list-tasks               List all planned tasks
        --list-hosts               List all target hosts
        --visualize                Generate execution graph visualization
    -o, --output &lt;FORMAT&gt;          Output format [default: json] [possible values: json, binary, dot]
        --optimize                 Enable execution optimizations
        --estimate-time            Include execution time estimates
        --dry-run                  Plan but don't output execution plan
    -v, --verbose                  Enable verbose output
    -h, --help                     Print help information
    -V, --version                  Print version information

EXAMPLES:
    rustle-plan parsed_playbook.json                           # Generate basic execution plan
    rustle-plan -i inventory.json --optimize playbook.json     # Optimized plan with inventory
    rustle-plan --strategy rolling --serial 5 playbook.json    # Rolling update strategy
    rustle-plan --list-tasks --tags deploy playbook.json       # List deployment tasks only
    rustle-plan --visualize -o dot playbook.json &gt; graph.dot   # Generate dependency graph
```

### API Documentation
Comprehensive rustdoc documentation including:
- Planning algorithm explanations
- Performance characteristics
- Strategy selection guidelines
- Optimization techniques

### Integration Examples
```bash
# Basic planning pipeline with binary deployment
rustle-parse playbook.yml | rustle-plan --strategy binary-hybrid | rustle-deploy | rustle-exec

# Optimized rolling deployment with binary optimization
rustle-parse -i inventory.ini deploy.yml | \
  rustle-plan --strategy rolling --serial 5 --optimize --binary-threshold 3 | \
  rustle-deploy | \
  rustle-exec

# Force binary deployment for maximum performance
rustle-parse playbook.yml | \
  rustle-plan --force-binary --optimize | \
  rustle-deploy | \
  rustle-exec

# Dry-run with compilation time estimation
rustle-parse playbook.yml | \
  rustle-plan --check --estimate-time | \
  jq '.estimated_duration, .estimated_compilation_time'

# Analyze binary deployment opportunities
rustle-parse complex.yml | \
  rustle-plan --list-binaries --optimize | \
  jq '.binary_deployments[]'

# Traditional SSH-only execution (disable binary deployment)
rustle-parse playbook.yml | \
  rustle-plan --force-ssh | \
  rustle-exec

# Parallel task analysis with binary grouping
rustle-parse complex.yml | \
  rustle-plan --optimize --list-tasks | \
  jq '.plays[].batches[].parallel_groups, .binary_deployments'
```