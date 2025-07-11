# Spec 020: Execution Monitoring & Telemetry

## Feature Summary

The execution monitoring and telemetry feature adds real-time monitoring capabilities to rustle-plan, enabling it to track execution plan performance, binary deployment effectiveness, and system resource utilization. This feature provides observability into the planning process itself and generates telemetry data that can be used to improve future planning decisions.

**Problem it solves**: Currently, rustle-plan generates execution plans but provides no visibility into how those plans perform in practice. This lack of feedback makes it difficult to validate planning assumptions, optimize algorithms, or improve binary deployment decisions over time.

**High-level approach**: Implement a telemetry system that collects metrics during plan generation and execution, stores historical data, and provides APIs for monitoring tools. Include performance profiling, binary deployment success tracking, and adaptive learning capabilities.

## Goals & Requirements

### Functional Requirements
- Real-time metrics collection during plan generation
- Historical execution plan performance tracking
- Binary deployment success/failure monitoring
- Resource utilization monitoring (CPU, memory, network)
- Adaptive learning from execution outcomes
- Integration with popular monitoring systems (Prometheus, Grafana)
- Performance regression detection
- Plan optimization recommendations based on historical data

### Non-functional Requirements
- **Performance**: Telemetry overhead <5% of planning time
- **Storage**: Efficient time-series data storage for metrics
- **Reliability**: Telemetry failures must not affect core planning functionality
- **Scalability**: Handle metrics from 1000+ concurrent planning operations
- **Privacy**: Ensure no sensitive data is collected in metrics

### Success Criteria
- 95% of planning operations generate complete telemetry data
- Historical data enables 20%+ improvement in plan optimization
- Binary deployment success rate tracking with 99% accuracy
- Integration with at least 2 major monitoring platforms
- Zero impact on core functionality when telemetry is disabled

## API/Interface Design

### Core Telemetry Interface

```rust
pub trait TelemetryCollector {
    fn record_metric(&self, name: &str, value: f64, tags: Option<&[(&str, &str)]>);
    fn record_histogram(&self, name: &str, value: f64, tags: Option<&[(&str, &str)]>);
    fn record_counter(&self, name: &str, increment: u64, tags: Option<&[(&str, &str)]>);
    fn start_timer(&self, name: &str, tags: Option<&[(&str, &str)]>) -> TimerGuard;
    fn record_event(&self, event: &TelemetryEvent);
}

pub struct TelemetryManager {
    collectors: Vec<Box<dyn TelemetryCollector>>,
    config: TelemetryConfig,
    historical_data: Arc<HistoricalDataStore>,
}

impl TelemetryManager {
    pub fn new(config: TelemetryConfig) -> Self;
    pub fn add_collector(&mut self, collector: Box<dyn TelemetryCollector>);
    pub fn record_planning_metrics(&self, plan: &ExecutionPlan, duration: Duration);
    pub fn record_binary_deployment_outcome(&self, deployment_id: &str, outcome: DeploymentOutcome);
    pub fn get_optimization_recommendations(&self, context: &PlanningContext) -> Vec<OptimizationRecommendation>;
    pub fn export_metrics(&self, format: MetricsFormat) -> Result<String, TelemetryError>;
}

#[derive(Debug, Clone)]
pub struct TelemetryEvent {
    pub timestamp: DateTime<Utc>,
    pub event_type: EventType,
    pub context: HashMap<String, String>,
    pub metrics: HashMap<String, f64>,
}

#[derive(Debug, Clone)]
pub enum EventType {
    PlanningStarted,
    PlanningCompleted,
    BinaryDeploymentPlanned,
    DependencyAnalysisCompleted,
    OptimizationApplied,
    Error { error_type: String },
}

#[derive(Debug, Clone)]
pub struct DeploymentOutcome {
    pub deployment_id: String,
    pub success: bool,
    pub execution_time: Duration,
    pub network_savings: f64,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct OptimizationRecommendation {
    pub recommendation_type: RecommendationType,
    pub description: String,
    pub expected_improvement: f64,
    pub confidence: f64,
}

#[derive(Debug, Clone)]
pub enum RecommendationType {
    AdjustBinaryThreshold,
    ChangeExecutionStrategy,
    OptimizeParallelization,
    ImproveTaskGrouping,
}
```

### Monitoring Integration

```rust
pub struct PrometheusCollector {
    registry: prometheus::Registry,
    metrics: PrometheusMetrics,
}

pub struct PrometheusMetrics {
    pub planning_duration: prometheus::Histogram,
    pub plans_total: prometheus::Counter,
    pub binary_deployments_planned: prometheus::Gauge,
    pub binary_deployment_success_rate: prometheus::Gauge,
    pub network_efficiency_score: prometheus::Gauge,
    pub parallelism_score: prometheus::Gauge,
    pub task_count: prometheus::Histogram,
    pub host_count: prometheus::Histogram,
}

impl PrometheusCollector {
    pub fn new() -> Self;
    pub fn register_metrics(&self) -> Result<(), prometheus::Error>;
    pub fn get_metrics_handler(&self) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone;
}

pub struct GrafanaIntegration {
    client: grafana_client::Client,
    dashboard_config: GrafanaDashboardConfig,
}

impl GrafanaIntegration {
    pub fn create_dashboards(&self) -> Result<(), GrafanaError>;
    pub fn update_dashboard_config(&self, config: GrafanaDashboardConfig) -> Result<(), GrafanaError>;
}
```

### Historical Data Storage

```rust
pub trait HistoricalDataStore {
    fn store_execution_plan(&self, plan: &ExecutionPlan, metrics: &PlanningMetrics) -> Result<(), StorageError>;
    fn get_historical_plans(&self, filters: &PlanFilters) -> Result<Vec<HistoricalPlan>, StorageError>;
    fn get_performance_trends(&self, timeframe: Duration) -> Result<PerformanceTrends, StorageError>;
    fn get_binary_deployment_stats(&self, timeframe: Duration) -> Result<BinaryDeploymentStats, StorageError>;
}

pub struct SqliteHistoricalStore {
    connection: rusqlite::Connection,
    schema_version: u32,
}

pub struct PostgresHistoricalStore {
    pool: sqlx::postgres::PgPool,
}

#[derive(Debug, Clone)]
pub struct PlanningMetrics {
    pub planning_duration: Duration,
    pub memory_usage: u64,
    pub cpu_usage: f64,
    pub dependency_analysis_time: Duration,
    pub optimization_time: Duration,
    pub binary_planning_time: Duration,
}

#[derive(Debug, Clone)]
pub struct PerformanceTrends {
    pub average_planning_time: Duration,
    pub planning_time_trend: TrendDirection,
    pub binary_deployment_adoption: f64,
    pub optimization_effectiveness: f64,
}
```

## File and Package Structure

```
src/
├── telemetry/
│   ├── mod.rs                      # Module exports and public API
│   ├── manager.rs                  # TelemetryManager implementation
│   ├── collectors/
│   │   ├── mod.rs                  # Collector trait and exports
│   │   ├── prometheus.rs           # Prometheus metrics collector
│   │   ├── statsd.rs               # StatsD collector
│   │   ├── opentelemetry.rs        # OpenTelemetry collector
│   │   └── console.rs              # Console/debug collector
│   ├── storage/
│   │   ├── mod.rs                  # Storage trait and exports
│   │   ├── sqlite.rs               # SQLite historical storage
│   │   ├── postgres.rs             # PostgreSQL storage
│   │   └── memory.rs               # In-memory storage for tests
│   ├── events.rs                   # Event types and serialization
│   ├── metrics.rs                  # Metric definitions and helpers
│   ├── optimization.rs             # Optimization recommendations
│   └── error.rs                    # Telemetry error types
├── monitoring/
│   ├── mod.rs                      # Monitoring integrations
│   ├── grafana.rs                  # Grafana dashboard management
│   ├── alerting.rs                 # Alert rule management
│   └── health.rs                   # Health check endpoints
└── config/
    └── telemetry.rs                # Telemetry configuration
```

## Implementation Details

### Phase 1: Core Telemetry Infrastructure
1. Implement the `TelemetryCollector` trait and `TelemetryManager`
2. Create basic metric collection points in the execution planner
3. Add console collector for debugging and development
4. Implement configuration system for telemetry settings

### Phase 2: Metrics Collection
1. Add telemetry collection to key planning operations:
   - Plan generation start/completion
   - Dependency analysis timing
   - Binary deployment decision points
   - Optimization algorithm execution
2. Implement resource monitoring (CPU, memory)
3. Add error tracking and categorization

### Phase 3: Historical Data Storage
1. Implement SQLite storage for development/testing
2. Add PostgreSQL storage for production use
3. Create database schema and migration system
4. Implement data retention policies

### Phase 4: Prometheus Integration
1. Implement Prometheus metrics collector
2. Create comprehensive metric definitions
3. Add HTTP metrics endpoint
4. Create example Grafana dashboards

### Phase 5: Adaptive Learning
1. Implement historical data analysis
2. Create optimization recommendation engine
3. Add performance trend analysis
4. Implement feedback loop for plan optimization

### Key Algorithms

**Metric Aggregation**:
```rust
impl TelemetryManager {
    fn aggregate_metrics(&self, timeframe: Duration) -> AggregatedMetrics {
        let plans = self.historical_data.get_historical_plans(&PlanFilters {
            start_time: Utc::now() - timeframe,
            end_time: Utc::now(),
        })?;
        
        let mut aggregated = AggregatedMetrics::new();
        
        for plan in plans {
            aggregated.total_plans += 1;
            aggregated.total_planning_time += plan.metrics.planning_duration;
            aggregated.binary_deployments += plan.binary_deployments.len();
            
            if plan.binary_deployments_successful > 0 {
                aggregated.successful_binary_deployments += plan.binary_deployments_successful;
            }
        }
        
        aggregated.average_planning_time = aggregated.total_planning_time / aggregated.total_plans as u32;
        aggregated.binary_success_rate = 
            aggregated.successful_binary_deployments as f64 / aggregated.binary_deployments as f64;
        
        aggregated
    }
}
```

**Optimization Recommendations**:
```rust
impl TelemetryManager {
    fn generate_recommendations(&self, context: &PlanningContext) -> Vec<OptimizationRecommendation> {
        let mut recommendations = Vec::new();
        let trends = self.historical_data.get_performance_trends(Duration::days(30))?;
        
        // Analyze binary threshold effectiveness
        if trends.binary_deployment_adoption < 0.3 && trends.average_planning_time > Duration::seconds(5) {
            recommendations.push(OptimizationRecommendation {
                recommendation_type: RecommendationType::AdjustBinaryThreshold,
                description: "Consider lowering binary threshold to increase adoption".to_string(),
                expected_improvement: 0.25,
                confidence: 0.8,
            });
        }
        
        // Analyze parallelization effectiveness
        if context.host_count > 10 && trends.optimization_effectiveness < 0.5 {
            recommendations.push(OptimizationRecommendation {
                recommendation_type: RecommendationType ChangeExecutionStrategy,
                description: "Rolling strategy may be more effective for this host count".to_string(),
                expected_improvement: 0.3,
                confidence: 0.7,
            });
        }
        
        recommendations
    }
}
```

## Testing Strategy

### Unit Tests
- **Collector tests**: Mock metrics collection and verification
- **Storage tests**: Database operations with test fixtures
- **Aggregation tests**: Metric calculation accuracy
- **Recommendation tests**: Algorithm correctness with various scenarios

### Integration Tests
- **End-to-end telemetry**: Full planning pipeline with metrics collection
- **Prometheus integration**: Metrics endpoint functionality
- **Historical data**: Storage and retrieval across different backends
- **Performance regression**: Ensure telemetry overhead stays within limits

### Test Data Structure
```
tests/fixtures/telemetry/
├── metrics/
│   ├── planning_metrics.json       # Sample planning metrics
│   ├── historical_plans.json       # Historical execution plans
│   └── performance_trends.json     # Sample trend data
├── configs/
│   ├── telemetry_config.toml      # Test configurations
│   └── prometheus_config.yml      # Prometheus test config
└── expected/
    ├── recommendations.json        # Expected optimization recommendations
    └── aggregated_metrics.json     # Expected aggregation results
```

### Performance Tests
- Telemetry overhead benchmarking
- High-volume metrics collection stress tests
- Database performance with large datasets
- Memory usage monitoring during extended operations

## Edge Cases & Error Handling

### Telemetry Failures
- Network connectivity issues with remote collectors
- Database unavailability for historical storage
- Disk space exhaustion for local storage
- Metric collection errors during planning

### Data Consistency
- Clock synchronization across distributed systems
- Partial metric collection due to interruptions
- Schema migration failures
- Data corruption recovery

### Performance Degradation
- High metric volume impact on planning performance
- Memory leaks in long-running telemetry processes
- Database connection pool exhaustion
- Metric export timeouts

## Dependencies

### External Crates
```toml
[dependencies]
# Existing dependencies...
prometheus = "0.13"
tokio-metrics = "0.3"
opentelemetry = "0.20"
opentelemetry-prometheus = "0.13"
sqlx = { version = "0.7", features = ["postgres", "sqlite", "chrono", "uuid"] }
rusqlite = { version = "0.29", features = ["bundled"] }
statsd = "0.16"
grafana-client = "0.4"
reqwest = { version = "0.11", features = ["json"] }
```

### Internal Dependencies
- Core planning types from `crate::types`
- Execution planner from `crate::planner`
- Configuration system integration
- Logging integration with `tracing`

## Configuration

### Telemetry Configuration
```toml
[telemetry]
enabled = true
collectors = ["prometheus", "console"]
export_interval_seconds = 30
max_events_buffer = 10000

[telemetry.prometheus]
enabled = true
listen_address = "127.0.0.1:9090"
metrics_path = "/metrics"

[telemetry.historical_storage]
backend = "sqlite"  # or "postgres"
connection_string = "rustle_telemetry.db"
retention_days = 90

[telemetry.grafana]
enabled = false
server_url = "http://localhost:3000"
api_key = "${GRAFANA_API_KEY}"
create_dashboards = true
```

### Environment Variables
- `RUSTLE_TELEMETRY_ENABLED`: Enable/disable telemetry
- `RUSTLE_PROMETHEUS_PORT`: Prometheus metrics port
- `RUSTLE_TELEMETRY_DB_URL`: Database connection string
- `GRAFANA_API_KEY`: Grafana API key for dashboard management

## Documentation

### User Documentation
- Telemetry configuration guide
- Metrics reference documentation
- Grafana dashboard setup instructions
- Troubleshooting common telemetry issues

### Developer Documentation
- Telemetry architecture overview
- Adding new metrics and collectors
- Historical data schema documentation
- Performance optimization guidelines

### API Documentation
- Comprehensive rustdoc for all public interfaces
- Example usage for custom collectors
- Integration patterns for monitoring systems
- Best practices for metric naming and tagging

## Example Usage

### Basic Telemetry Setup
```rust
use rustle_plan::telemetry::{TelemetryManager, TelemetryConfig};

let config = TelemetryConfig::from_file("telemetry.toml")?;
let mut telemetry = TelemetryManager::new(config);

// Add Prometheus collector
telemetry.add_collector(Box::new(PrometheusCollector::new()));

// Use in planning
let planner = ExecutionPlanner::new().with_telemetry(telemetry);
let plan = planner.plan_execution(&playbook, &inventory, &options)?;
```

### Custom Metric Collection
```rust
// Record custom metrics during planning
telemetry.record_histogram("planning.custom_metric", value, Some(&[
    ("strategy", "binary-hybrid"),
    ("host_count", &host_count.to_string()),
]));

// Record planning events
telemetry.record_event(&TelemetryEvent {
    timestamp: Utc::now(),
    event_type: EventType::OptimizationApplied,
    context: HashMap::from([
        ("optimization_type", "parallelization"),
        ("improvement", "25%"),
    ]),
    metrics: HashMap::from([
        ("before_score", 0.6),
        ("after_score", 0.8),
    ]),
});
```