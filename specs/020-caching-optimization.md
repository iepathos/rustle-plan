# Spec 020: Local Caching & Incremental Planning

## Feature Summary

The local caching and incremental planning feature adds intelligent caching mechanisms to rustle-plan to dramatically improve planning performance for repeated or similar playbooks. It implements content-aware caching, incremental planning updates, and local optimization to minimize planning latency and resource usage while maintaining Unix tool simplicity.

**Problem it solves**: Currently, every planning operation starts from scratch, even for playbooks that have minimal changes or have been planned recently. This leads to unnecessary computational overhead, especially in CI/CD environments where similar playbooks are planned frequently.

**High-level approach**: Implement local caching that stores planning results, dependency graphs, and binary deployment analyses. Use content hashing and change detection to enable incremental updates and cache invalidation strategies. Keep all caching logic self-contained within rustle-plan.

## Goals & Requirements

### Functional Requirements
- Content-aware plan caching with automatic invalidation
- Incremental planning for minimal playbook changes
- Dependency graph caching and reuse
- Binary deployment analysis caching
- Local disk-based cache persistence
- Cache hit rate optimization
- Intelligent cache eviction policies

### Non-functional Requirements
- **Performance**: 90%+ cache hit rate for typical CI/CD workloads
- **Speed**: <100ms planning time for cache hits
- **Memory**: Configurable cache size limits with LRU eviction
- **Storage**: Efficient serialization for persistent caching
- **Consistency**: Cache invalidation within 1 second of changes

### Success Criteria
- 80%+ reduction in planning time for cached scenarios
- 95%+ cache hit rate in typical development workflows
- Zero cache-related correctness issues
- Support for 10,000+ cached plans with <500MB memory usage
- Transparent operation requiring no user workflow changes
- Self-contained operation with no external dependencies

## API/Interface Design

### Core Caching Interface

```rust
pub trait PlanCache {
    fn get_plan(&self, key: &CacheKey) -> Option<CachedPlan>;
    fn store_plan(&self, key: CacheKey, plan: CachedPlan) -> Result<(), CacheError>;
    fn invalidate(&self, key: &CacheKey) -> Result<(), CacheError>;
    fn invalidate_prefix(&self, prefix: &str) -> Result<(), CacheError>;
    fn get_stats(&self) -> CacheStats;
}

pub struct LocalPlanCache {
    memory_cache: MemoryCache,
    disk_cache: DiskCache,
    config: CacheConfig,
    hasher: ContentHasher,
    invalidation_tracker: InvalidationTracker,
}

impl LocalPlanCache {
    pub fn new(config: CacheConfig) -> Self;
    pub fn get_or_compute<F>(&self, key: CacheKey, compute_fn: F) -> Result<ExecutionPlan, CacheError>
    where
        F: FnOnce() -> Result<ExecutionPlan, PlanError>;
    pub fn get_incremental_plan(&self, key: CacheKey, changes: &PlaybookChanges) -> Option<ExecutionPlan>;
    pub fn analyze_cache_effectiveness(&self) -> CacheAnalysis;
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct CacheKey {
    pub playbook_hash: String,
    pub inventory_hash: String,
    pub options_hash: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedPlan {
    pub plan: ExecutionPlan,
    pub metadata: CacheMetadata,
    pub dependency_graph: SerializedDependencyGraph,
    pub binary_analysis: BinaryAnalysisCache,
    pub computation_time: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheMetadata {
    pub created_at: DateTime<Utc>,
    pub last_accessed: DateTime<Utc>,
    pub access_count: u64,
    pub cache_version: u32,
    pub expiry: Option<DateTime<Utc>>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub hit_rate: f64,
    pub miss_rate: f64,
    pub total_requests: u64,
    pub cache_size: usize,
    pub memory_usage: u64,
    pub average_hit_time: Duration,
    pub average_miss_time: Duration,
}
```

### Incremental Planning

```rust
pub struct IncrementalPlanner {
    base_planner: ExecutionPlanner,
    cache: Arc<LocalPlanCache>,
    change_detector: PlaybookChangeDetector,
}

impl IncrementalPlanner {
    pub fn new(base_planner: ExecutionPlanner, cache: Arc<LocalPlanCache>) -> Self;
    
    pub fn plan_incremental(
        &self,
        playbook: &ParsedPlaybook,
        inventory: &ParsedInventory,
        options: &PlanningOptions,
        previous_version: Option<&str>,
    ) -> Result<IncrementalPlanResult, PlanError>;
    
    pub fn detect_changes(
        &self,
        current: &ParsedPlaybook,
        previous: &ParsedPlaybook,
    ) -> PlaybookChanges;
}

#[derive(Debug, Clone)]
pub struct PlaybookChanges {
    pub added_tasks: Vec<String>,
    pub removed_tasks: Vec<String>,
    pub modified_tasks: Vec<TaskChange>,
    pub dependency_changes: Vec<DependencyChange>,
    pub host_changes: Vec<HostChange>,
    pub variable_changes: Vec<VariableChange>,
    pub impact_score: f64,
}

#[derive(Debug, Clone)]
pub struct IncrementalPlanResult {
    pub plan: ExecutionPlan,
    pub was_cached: bool,
    pub was_incremental: bool,
    pub changes_applied: PlaybookChanges,
    pub computation_time: Duration,
    pub cache_savings: Duration,
}

#[derive(Debug, Clone)]
pub enum TaskChange {
    ArgumentsChanged { task_id: String, old_args: HashMap<String, Value>, new_args: HashMap<String, Value> },
    ModuleChanged { task_id: String, old_module: String, new_module: String },
    ConditionsChanged { task_id: String, old_conditions: Vec<ExecutionCondition>, new_conditions: Vec<ExecutionCondition> },
}
```

### Content Hashing

```rust
pub struct ContentHasher {
    hasher_type: HasherType,
    stable_fields: HashSet<String>,
    ignored_fields: HashSet<String>,
}

impl ContentHasher {
    pub fn new(config: HashingConfig) -> Self;
    pub fn hash_playbook(&self, playbook: &ParsedPlaybook) -> String;
    pub fn hash_inventory(&self, inventory: &ParsedInventory) -> String;
    pub fn hash_options(&self, options: &PlanningOptions) -> String;
    pub fn hash_incremental(&self, base_hash: &str, changes: &PlaybookChanges) -> String;
    pub fn verify_hash(&self, content: &str, expected_hash: &str) -> bool;
}

#[derive(Debug, Clone)]
pub enum HasherType {
    Sha256,
    Blake3,
    Xxh3,
}

pub struct StableHasher {
    inner: Box<dyn Hasher>,
}

impl StableHasher {
    pub fn hash_task_stable(&mut self, task: &ParsedTask);
    pub fn hash_play_stable(&mut self, play: &ParsedPlay);
    pub fn finalize(self) -> String;
}
```


## File and Package Structure

```
src/
├── cache/
│   ├── mod.rs                      # Module exports and main cache interface
│   ├── local.rs                    # LocalPlanCache implementation
│   ├── memory.rs                   # In-memory cache
│   ├── disk.rs                     # Persistent disk cache
│   ├── incremental.rs              # Incremental planning logic
│   ├── hasher.rs                   # Content hashing utilities
│   ├── invalidation.rs             # Cache invalidation strategies
│   ├── analytics.rs                # Cache performance analytics
│   ├── serialization.rs            # Efficient plan serialization
│   └── error.rs                    # Cache error types
└── change_detection/
    ├── mod.rs                      # Change detection exports
    ├── playbook_diff.rs            # Playbook difference algorithms
    ├── impact_analysis.rs          # Change impact scoring
    └── fingerprinting.rs           # Content fingerprinting
```

## Implementation Details

### Phase 1: Basic Caching Infrastructure
1. Implement content hashing for playbooks, inventories, and options
2. Create in-memory cache with LRU eviction
3. Add cache integration points to ExecutionPlanner
4. Implement basic cache key generation and validation

### Phase 2: Persistent Caching
1. Add disk-based L2 cache with efficient serialization
2. Implement cache metadata and expiration handling
3. Add cache statistics and monitoring
4. Create cache configuration and management APIs

### Phase 3: Incremental Planning
1. Implement playbook change detection algorithms
2. Create incremental plan computation logic
3. Add dependency graph caching and reuse
4. Implement selective cache invalidation

### Phase 4: Advanced Local Optimization
1. Implement intelligent cache eviction policies
2. Add cache hit rate optimization algorithms  
3. Create automated cache sizing recommendations
4. Implement cache warming for common scenarios

### Phase 5: Performance Monitoring
1. Add comprehensive cache performance metrics
2. Implement cache effectiveness analysis
3. Create optimization recommendations
4. Add cache health monitoring and diagnostics

### Key Algorithms

**Content-Aware Hashing**:
```rust
impl ContentHasher {
    fn hash_playbook_stable(&self, playbook: &ParsedPlaybook) -> String {
        let mut hasher = self.create_hasher();
        
        // Hash playbook name and metadata
        hasher.update(playbook.name.as_bytes());
        
        // Sort plays by name for stable hashing
        let mut plays = playbook.plays.clone();
        plays.sort_by(|a, b| a.name.cmp(&b.name));
        
        for play in plays {
            self.hash_play_stable(&mut hasher, &play);
        }
        
        // Hash variables in sorted order
        let mut vars: Vec<_> = playbook.vars.iter().collect();
        vars.sort_by_key(|(k, _)| *k);
        for (key, value) in vars {
            hasher.update(key.as_bytes());
            hasher.update(&serde_json::to_vec(value).unwrap());
        }
        
        format!("{:x}", hasher.finalize())
    }
    
    fn hash_play_stable(&self, hasher: &mut dyn Hasher, play: &ParsedPlay) {
        hasher.update(play.name.as_bytes());
        
        // Sort hosts for stability
        let mut hosts = play.hosts.clone();
        hosts.sort();
        for host in hosts {
            hasher.update(host.as_bytes());
        }
        
        // Sort tasks by ID for stability
        let mut tasks = play.tasks.clone();
        tasks.sort_by(|a, b| a.id.cmp(&b.id));
        for task in tasks {
            self.hash_task_stable(hasher, &task);
        }
    }
}
```

**Incremental Planning**:
```rust
impl IncrementalPlanner {
    fn compute_incremental_plan(
        &self,
        changes: &PlaybookChanges,
        base_plan: &ExecutionPlan,
    ) -> Result<ExecutionPlan, PlanError> {
        let mut updated_plan = base_plan.clone();
        
        // Handle added tasks
        for task_id in &changes.added_tasks {
            let new_batches = self.integrate_new_task(task_id, &updated_plan)?;
            self.merge_batches(&mut updated_plan, new_batches);
        }
        
        // Handle removed tasks
        for task_id in &changes.removed_tasks {
            self.remove_task_from_plan(&mut updated_plan, task_id);
        }
        
        // Handle modified tasks
        for task_change in &changes.modified_tasks {
            match task_change {
                TaskChange::ArgumentsChanged { task_id, new_args, .. } => {
                    self.update_task_args(&mut updated_plan, task_id, new_args)?;
                }
                TaskChange::ModuleChanged { task_id, new_module, .. } => {
                    // Module changes require more extensive replanning
                    self.replan_task_module(&mut updated_plan, task_id, new_module)?;
                }
                TaskChange::ConditionsChanged { task_id, new_conditions, .. } => {
                    self.update_task_conditions(&mut updated_plan, task_id, new_conditions);
                }
            }
        }
        
        // Recalculate affected metrics
        self.recalculate_plan_metrics(&mut updated_plan);
        
        Ok(updated_plan)
    }
}
```

**Cache Warming Strategy**:
```rust
impl LocalPlanCache {
    fn warm_cache_from_recent(&self, recent_plans: &[ExecutionPlan]) -> Result<(), CacheError> {
        // Simple cache warming based on recently used plans
        for plan in recent_plans {
            let key = self.generate_cache_key_for_plan(plan)?;
            if !self.contains_key(&key) {
                // Pre-compute variations of successful plans
                let variations = self.generate_plan_variations(plan)?;
                for variation in variations {
                    let variation_key = self.generate_cache_key_for_plan(&variation)?;
                    self.store_plan(variation_key, CachedPlan::from(variation))?;
                }
            }
        }
        Ok(())
    }
    
    fn generate_plan_variations(&self, base_plan: &ExecutionPlan) -> Result<Vec<ExecutionPlan>, CacheError> {
        // Generate simple variations (different host subsets, tag filters, etc.)
        let mut variations = Vec::new();
        
        // Host subset variations
        if base_plan.hosts.len() > 1 {
            for chunk_size in [base_plan.hosts.len() / 2, base_plan.hosts.len() / 4] {
                if chunk_size > 0 {
                    let subset_plan = self.create_host_subset_plan(base_plan, chunk_size)?;
                    variations.push(subset_plan);
                }
            }
        }
        
        Ok(variations)
    }
}
```

## Testing Strategy

### Unit Tests
- **Hash stability**: Ensure identical content produces identical hashes
- **Cache operations**: Store, retrieve, eviction, and invalidation
- **Change detection**: Accurate identification of playbook differences
- **Incremental planning**: Correctness of incremental updates

### Integration Tests
- **End-to-end caching**: Full planning pipeline with caching enabled
- **Cache persistence**: Restart scenarios with disk cache
- **Distributed cache**: Multi-node cache consistency
- **Performance regression**: Ensure cache overhead is minimal

### Performance Tests
- **Cache hit rate**: Measure effectiveness with realistic workloads
- **Memory usage**: Cache size vs. performance trade-offs
- **Serialization performance**: Plan encoding/decoding benchmarks
- **Concurrent access**: Multi-threaded cache performance

### Test Data Structure
```
tests/fixtures/cache/
├── playbooks/
│   ├── base_playbook.json          # Base version for incremental tests
│   ├── modified_playbook.json      # Modified version with tracked changes
│   ├── large_playbook.json         # Performance testing
│   └── variants/                   # Playbook variations for pattern testing
├── cache_scenarios/
│   ├── hit_scenarios.json          # Expected cache hit scenarios
│   ├── miss_scenarios.json         # Expected cache miss scenarios
│   └── invalidation_scenarios.json # Cache invalidation test cases
└── expected/
    ├── hashes.json                 # Expected content hashes
    ├── changes.json                # Expected change detection results
    └── incremental_plans.json      # Expected incremental planning results
```

## Edge Cases & Error Handling

### Cache Consistency
- Concurrent modifications during cache operations
- Cache corruption detection and recovery
- Distributed cache network partitions
- Cache version mismatches across deployments

### Memory Management
- Cache size limits and eviction policies
- Memory pressure handling
- Large plan serialization failures
- Out-of-memory scenarios during cache warming

### Content Hashing
- Hash collisions (extremely rare but must be handled)
- Floating point precision in hash calculations
- Unicode normalization in content hashing
- Clock skew affecting timestamp-based invalidation

## Dependencies

### External Crates
```toml
[dependencies]
# Existing dependencies...
blake3 = "1.4"
xxhash-rust = { version = "0.8", features = ["xxh3"] }
lru = "0.11"
dashmap = "5.5"
bincode = "1.3"
```

### Internal Dependencies
- Core planning types from `crate::types`
- Execution planner from `crate::planner`
- Telemetry integration for cache metrics
- Configuration system for cache settings

## Configuration

### Cache Configuration
```toml
[cache]
enabled = true
strategy = "intelligent"  # or "simple", "distributed"

[cache.memory]
max_size_mb = 256
max_entries = 10000
eviction_policy = "lru"

[cache.disk]
enabled = true
directory = "./cache"
max_size_gb = 5
compression = "zstd"

[cache.incremental]
enabled = true
max_change_impact = 0.3  # Threshold for incremental vs full replanning
change_detection_depth = 3

[cache.warming]
enabled = true
recent_plans_window_hours = 24
max_variations_per_plan = 5
```

### Environment Variables
- `RUSTLE_CACHE_ENABLED`: Enable/disable caching
- `RUSTLE_CACHE_DIR`: Cache directory location
- `RUSTLE_CACHE_SIZE_MB`: Maximum memory cache size

## Documentation

### User Documentation
- Cache configuration and tuning guide
- Performance optimization recommendations
- Distributed cache setup instructions
- Troubleshooting cache-related issues

### Developer Documentation
- Cache architecture and design decisions
- Adding new cache backends
- Custom hashing strategies
- Cache analytics and monitoring

### Performance Guide
- Cache sizing recommendations
- Hit rate optimization strategies
- Incremental planning best practices
- Distributed cache deployment patterns

## Example Usage

### Basic Cache Configuration
```rust
use rustle_plan::cache::{LocalPlanCache, CacheConfig};

let config = CacheConfig {
    memory_cache_size: 256 * 1024 * 1024, // 256MB
    disk_cache_enabled: true,
    disk_cache_dir: PathBuf::from("./cache"),
    incremental_enabled: true,
    ..Default::default()
};

let cache = LocalPlanCache::new(config);
let planner = ExecutionPlanner::new().with_cache(cache);
```

### Incremental Planning
```rust
// Plan with caching and incremental updates
let result = planner.plan_incremental(
    &current_playbook,
    &inventory,
    &options,
    Some(&previous_version),
)?;

if result.was_cached {
    println!("Used cached plan (saved {}ms)", result.cache_savings.as_millis());
} else if result.was_incremental {
    println!("Used incremental planning for {} changes", 
             result.changes_applied.added_tasks.len() + 
             result.changes_applied.modified_tasks.len());
}
```

### Cache Analytics
```rust
let stats = cache.get_stats();
println!("Cache hit rate: {:.1}%", stats.hit_rate * 100.0);
println!("Memory usage: {}MB", stats.memory_usage / 1024 / 1024);

let analysis = cache.analyze_cache_effectiveness();
for recommendation in analysis.recommendations {
    println!("Recommendation: {}", recommendation.description);
}
```