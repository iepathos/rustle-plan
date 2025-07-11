# Spec 030: Intelligent Caching & Optimization

## Feature Summary

The intelligent caching and optimization feature adds sophisticated caching mechanisms to rustle-plan to dramatically improve planning performance for repeated or similar playbooks. It implements content-aware caching, incremental planning updates, and predictive pre-computation to minimize planning latency and resource usage.

**Problem it solves**: Currently, every planning operation starts from scratch, even for playbooks that have minimal changes or have been planned recently. This leads to unnecessary computational overhead, especially in CI/CD environments where similar playbooks are planned frequently.

**High-level approach**: Implement a multi-layered caching system that caches planning results, dependency graphs, binary deployment analyses, and intermediate computations. Use content hashing and change detection to enable incremental updates and cache invalidation strategies.

## Goals & Requirements

### Functional Requirements
- Content-aware plan caching with automatic invalidation
- Incremental planning for minimal playbook changes
- Dependency graph caching and reuse
- Binary deployment analysis caching
- Predictive pre-computation of likely planning scenarios
- Cache warming strategies for common playbook patterns
- Distributed cache support for team environments
- Cache analytics and hit rate optimization

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

## API/Interface Design

### Core Caching Interface

```rust
pub trait PlanCache {
    fn get_plan(&self, key: &CacheKey) -> Option<CachedPlan>;
    fn store_plan(&self, key: CacheKey, plan: CachedPlan) -> Result<(), CacheError>;
    fn invalidate(&self, key: &CacheKey) -> Result<(), CacheError>;
    fn invalidate_prefix(&self, prefix: &str) -> Result<(), CacheError>;
    fn get_stats(&self) -> CacheStats;
    fn warm_cache(&self, scenarios: &[WarmupScenario]) -> Result<(), CacheError>;
}

pub struct IntelligentPlanCache {
    l1_cache: Arc<MemoryCache>,
    l2_cache: Arc<DiskCache>,
    distributed_cache: Option<Arc<dyn DistributedCache>>,
    config: CacheConfig,
    hasher: ContentHasher,
    invalidation_tracker: InvalidationTracker,
}

impl IntelligentPlanCache {
    pub fn new(config: CacheConfig) -> Self;
    pub fn with_distributed_backend(mut self, backend: Arc<dyn DistributedCache>) -> Self;
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
    cache: Arc<IntelligentPlanCache>,
    change_detector: PlaybookChangeDetector,
}

impl IncrementalPlanner {
    pub fn new(base_planner: ExecutionPlanner, cache: Arc<IntelligentPlanCache>) -> Self;
    
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

### Distributed Cache Support

```rust
pub trait DistributedCache: Send + Sync {
    fn get(&self, key: &str) -> Result<Option<Vec<u8>>, DistributedCacheError>;
    fn set(&self, key: &str, value: Vec<u8>, ttl: Option<Duration>) -> Result<(), DistributedCacheError>;
    fn delete(&self, key: &str) -> Result<(), DistributedCacheError>;
    fn delete_prefix(&self, prefix: &str) -> Result<u64, DistributedCacheError>;
    fn ping(&self) -> Result<(), DistributedCacheError>;
}

pub struct RedisCache {
    client: redis::Client,
    connection_pool: redis::aio::ConnectionManager,
    prefix: String,
}

pub struct EtcdCache {
    client: etcd_client::EtcdClient,
    prefix: String,
}

pub struct S3Cache {
    client: aws_sdk_s3::Client,
    bucket: String,
    prefix: String,
}

impl RedisCache {
    pub fn new(connection_string: &str, prefix: String) -> Result<Self, DistributedCacheError>;
    pub async fn connect(&mut self) -> Result<(), DistributedCacheError>;
}
```

### Predictive Pre-computation

```rust
pub struct PredictiveCache {
    cache: Arc<IntelligentPlanCache>,
    predictor: PlanningPredictor,
    precompute_scheduler: TaskScheduler,
}

impl PredictiveCache {
    pub fn new(cache: Arc<IntelligentPlanCache>, config: PredictiveConfig) -> Self;
    pub fn analyze_patterns(&self, historical_data: &[PlanningRequest]) -> Vec<PredictionPattern>;
    pub fn schedule_precomputation(&self, patterns: &[PredictionPattern]) -> Result<(), PredictiveError>;
    pub fn precompute_likely_scenarios(&self) -> Result<PrecomputeResult, PredictiveError>;
}

#[derive(Debug, Clone)]
pub struct PredictionPattern {
    pub pattern_type: PatternType,
    pub frequency: f64,
    pub variation_points: Vec<VariationPoint>,
    pub confidence: f64,
}

#[derive(Debug, Clone)]
pub enum PatternType {
    RecurringPlaybook { base_hash: String },
    ParametricVariation { template_hash: String },
    BranchingWorkflow { decision_points: Vec<String> },
    ScalingPattern { base_size: usize, scaling_factor: f64 },
}

#[derive(Debug, Clone)]
pub struct VariationPoint {
    pub field_path: String,
    pub variation_type: VariationType,
    pub common_values: Vec<String>,
}
```

## File and Package Structure

```
src/
├── cache/
│   ├── mod.rs                      # Module exports and main cache interface
│   ├── intelligent.rs              # IntelligentPlanCache implementation
│   ├── memory.rs                   # In-memory L1 cache
│   ├── disk.rs                     # Persistent L2 cache
│   ├── distributed/
│   │   ├── mod.rs                  # Distributed cache trait
│   │   ├── redis.rs                # Redis cache implementation
│   │   ├── etcd.rs                 # etcd cache implementation
│   │   └── s3.rs                   # S3 cache implementation
│   ├── incremental.rs              # Incremental planning logic
│   ├── hasher.rs                   # Content hashing utilities
│   ├── invalidation.rs             # Cache invalidation strategies
│   ├── analytics.rs                # Cache performance analytics
│   ├── serialization.rs            # Efficient plan serialization
│   └── error.rs                    # Cache error types
├── prediction/
│   ├── mod.rs                      # Predictive caching exports
│   ├── predictor.rs                # Pattern recognition and prediction
│   ├── scheduler.rs                # Precomputation scheduling
│   ├── patterns.rs                 # Pattern analysis algorithms
│   └── precompute.rs               # Background precomputation
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

### Phase 4: Distributed Caching
1. Implement Redis distributed cache backend
2. Add cache synchronization and consistency mechanisms
3. Create team-wide cache sharing capabilities
4. Implement cache warming and population strategies

### Phase 5: Predictive Optimization
1. Add pattern recognition for common planning scenarios
2. Implement background precomputation scheduling
3. Create predictive cache warming based on usage patterns
4. Add machine learning-based optimization suggestions

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
impl PredictiveCache {
    async fn warm_cache_background(&self) {
        let patterns = self.analyze_recent_patterns().await;
        
        for pattern in patterns {
            if pattern.confidence > 0.8 {
                let scenarios = self.generate_scenarios_from_pattern(&pattern);
                
                for scenario in scenarios {
                    if !self.cache.contains_key(&scenario.cache_key) {
                        tokio::spawn({
                            let cache = self.cache.clone();
                            let scenario = scenario.clone();
                            async move {
                                if let Ok(plan) = scenario.compute_plan().await {
                                    cache.store_plan(scenario.cache_key, plan).await;
                                }
                            }
                        });
                    }
                }
            }
        }
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
redis = { version = "0.23", features = ["aio", "tokio-comp"] }
etcd-client = "0.12"
aws-sdk-s3 = "0.29"
blake3 = "1.4"
xxhash-rust = { version = "0.8", features = ["xxh3"] }
lru = "0.11"
dashmap = "5.5"
bincode = "1.3"
tokio = { version = "1", features = ["rt-multi-thread", "time"] }
moka = { version = "0.11", features = ["future"] }
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

[cache.distributed]
backend = "redis"  # or "etcd", "s3"
connection_string = "redis://localhost:6379"
key_prefix = "rustle-plan:"
ttl_hours = 24

[cache.incremental]
enabled = true
max_change_impact = 0.3  # Threshold for incremental vs full replanning
change_detection_depth = 3

[cache.predictive]
enabled = true
pattern_analysis_window_days = 30
precompute_confidence_threshold = 0.8
max_background_tasks = 4
```

### Environment Variables
- `RUSTLE_CACHE_ENABLED`: Enable/disable caching
- `RUSTLE_CACHE_DIR`: Cache directory location
- `RUSTLE_REDIS_URL`: Redis connection for distributed caching
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
use rustle_plan::cache::{IntelligentPlanCache, CacheConfig};

let config = CacheConfig {
    memory_cache_size: 256 * 1024 * 1024, // 256MB
    disk_cache_enabled: true,
    disk_cache_dir: PathBuf::from("./cache"),
    incremental_enabled: true,
    ..Default::default()
};

let cache = IntelligentPlanCache::new(config);
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