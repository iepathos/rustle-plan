# rustle-plan

[![CI](https://github.com/iepathos/rustle-plan/actions/workflows/ci.yml/badge.svg)](https://github.com/iepathos/rustle-plan/actions/workflows/ci.yml)
[![Security](https://github.com/iepathos/rustle-plan/actions/workflows/security.yml/badge.svg)](https://github.com/iepathos/rustle-plan/actions/workflows/security.yml)
[![Release](https://github.com/iepathos/rustle-plan/actions/workflows/release.yml/badge.svg)](https://github.com/iepathos/rustle-plan/actions/workflows/release.yml)

A specialized execution planner for the Rustle automation ecosystem that generates optimized execution plans with binary deployment strategies. This tool takes parsed playbooks from `rustle-parse` and produces detailed execution plans that optimize for minimal network overhead through intelligent parallelization and binary deployment planning.

## 🚀 Features

- **Optimized execution planning** from parsed playbooks with dependency analysis
- **Binary deployment strategies** to minimize network round-trips and maximize performance
- **Intelligent parallelization** with automatic detection of parallel execution opportunities
- **Multiple execution strategies** including linear, rolling, free, and binary-hybrid modes
- **Comprehensive dependency analysis** with circular dependency detection
- **Execution time estimation** including binary compilation overhead
- **Host filtering and task selection** with tag-based filtering support
- **Execution graph visualization** for dependency analysis and optimization verification

## 📦 Installation

### From Source

```bash
git clone <repository-url> rustle-plan
cd rustle-plan
cargo build --release
```

The binary will be available at `target/release/rustle-plan`.

### Development Setup

```bash
# Install development dependencies
rustup component add rustfmt clippy
cargo install cargo-watch cargo-tarpaulin cargo-audit

# Run in development mode
cargo run -- --help
```

## 🛠️ Usage

### Basic Usage

```bash
# Generate execution plan from parsed playbook
rustle-plan parsed_playbook.json

# Plan from rustle-parse output (includes embedded inventory)
rustle-plan rustle_parse_output.json

# Use binary-hybrid strategy for optimal performance
rustle-plan --strategy binary-hybrid --optimize parsed_playbook.json

# Pipeline with rustle-parse
rustle-parse playbook.yml | rustle-plan --strategy rolling
```

### Planning Options

```bash
# Host and task filtering
rustle-plan --limit "web*" --tags "deploy,config" parsed_playbook.json

# Execution strategies
rustle-plan --strategy linear parsed_playbook.json        # Sequential execution
rustle-plan --strategy rolling --serial 5 parsed_playbook.json  # Rolling updates
rustle-plan --strategy free parsed_playbook.json          # Maximum parallelization
rustle-plan --strategy binary-hybrid parsed_playbook.json # Optimal binary deployment

# Binary deployment control
rustle-plan --force-binary parsed_playbook.json          # Force binary for all suitable tasks
rustle-plan --force-ssh parsed_playbook.json             # Disable binary deployment
rustle-plan --binary-threshold 3 parsed_playbook.json    # Custom threshold for binary grouping
```

### Analysis and Inspection

```bash
# List planned tasks and execution order
rustle-plan --list-tasks parsed_playbook.json

# List target hosts
rustle-plan --list-hosts parsed_playbook.json

# Show planned binary deployments
rustle-plan --list-binaries parsed_playbook.json

# Generate dependency graph visualization
rustle-plan --visualize -o dot parsed_playbook.json > execution_graph.dot

# Dry run with time estimates
rustle-plan --dry-run --estimate-time parsed_playbook.json
```

### Performance Optimization

```bash
# Enable all optimizations
rustle-plan --optimize --strategy binary-hybrid parsed_playbook.json

# Custom parallelism settings
rustle-plan --forks 100 --serial 10 parsed_playbook.json

# Check mode planning (no changes)
rustle-plan --check --diff parsed_playbook.json
```

## 📋 Command Line Reference

```
rustle-plan [OPTIONS] [PARSED_PLAYBOOK]

Arguments:
  [PARSED_PLAYBOOK]  Path to parsed playbook file (or stdin if -)

Options:
  -l, --limit <PATTERN>             Limit execution to specific hosts
  -t, --tags <TAGS>                 Only run tasks with these tags
      --skip-tags <TAGS>            Skip tasks with these tags
  -s, --strategy <STRATEGY>         Execution strategy [default: binary-hybrid]
      --serial <NUM>                Number of hosts to run at once
      --forks <NUM>                 Maximum parallel processes [default: 50]
  -c, --check                       Check mode (don't make changes)
      --diff                        Show file differences
      --binary-threshold <NUM>      Minimum tasks for binary compilation [default: 5]
      --force-binary                Force binary deployment for all suitable tasks
      --force-ssh                   Force SSH execution (disable binary deployment)
      --list-tasks                  List all planned tasks
      --list-hosts                  List all target hosts
      --list-binaries               List planned binary deployments
      --visualize                   Generate execution graph visualization
  -o, --output <FORMAT>             Output format [default: json]
      --optimize                    Enable execution optimizations
      --estimate-time               Include execution time estimates
      --dry-run                     Plan but don't output execution plan
  -v, --verbose                     Enable verbose output
  -h, --help                        Print help
  -V, --version                     Print version

Execution Strategies:
  linear         Sequential execution across all hosts
  rolling        Rolling updates with configurable batch sizes
  free           Maximum parallelization within dependency constraints
  host-pinned    Pin tasks to specific hosts for locality
  binary-hybrid  Intelligent mix of binary deployment and SSH execution
  binary-only    Force binary deployment where possible

Output Formats:
  json           Structured JSON execution plan (default)
  binary         Compact binary format for efficient storage
  dot            Graphviz DOT format for visualization
```

## 📁 Input Format

`rustle-plan` expects JSON input from `rustle-parse` that includes both playbook and inventory data. The input format includes:

```json
{
  "metadata": {
    "file_path": "example_playbook.yml",
    "version": null,
    "created_at": "2025-07-11T01:15:00.000000Z",
    "checksum": "d48e92ff5b2b8cd603041d0d6a56a9c4674696e8e3c7601a6c526e6a37adea50"
  },
  "plays": [
    {
      "name": "Configure web servers",
      "hosts": ["all"],
      "tasks": [
        {
          "id": "task-1",
          "name": "Install nginx",
          "module": "package",
          "args": {
            "name": "nginx",
            "state": "present"
          },
          "dependencies": [],
          "tags": ["install"],
          "when": null,
          "notify": ["restart nginx"]
        }
      ],
      "handlers": [
        {
          "id": "handler-1",
          "name": "restart nginx",
          "module": "service",
          "args": {
            "name": "nginx",
            "state": "restarted"
          },
          "when": null
        }
      ],
      "vars": {}
    }
  ],
  "variables": {},
  "inventory": {
    "hosts": ["server1", "server2", "server3"],
    "groups": {
      "webservers": ["server1", "server2"],
      "databases": ["server3"]
    },
    "vars": {
      "nginx_version": "1.20.2"
    }
  },
  "facts_required": false,
  "vault_ids": []
}
```

## 📊 Output Format

The tool outputs detailed execution plans in JSON format by default:

```json
{
  "metadata": {
    "created_at": "2025-07-11T01:08:23.589337Z",
    "rustle_plan_version": "0.1.0",
    "playbook_hash": "785bab203c116d67dcacce745579fcbd",
    "inventory_hash": "653a2ee28be80c8e41d5251b3923d10d",
    "planning_options": {
      "strategy": "BinaryHybrid",
      "forks": 50,
      "binary_threshold": 5
    }
  },
  "plays": [
    {
      "play_id": "play-0",
      "name": "Configure web servers",
      "strategy": "BinaryHybrid",
      "hosts": ["localhost"],
      "batches": [
        {
          "batch_id": "binary-batch",
          "hosts": ["localhost"],
          "tasks": [
            {
              "task_id": "task-1",
              "name": "Install nginx",
              "module": "package",
              "execution_order": 0,
              "can_run_parallel": false,
              "estimated_duration": {"secs": 66, "nanos": 0},
              "risk_level": "High"
            }
          ],
          "parallel_groups": []
        }
      ]
    }
  ],
  "binary_deployments": [],
  "total_tasks": 3,
  "estimated_duration": {"secs": 31, "nanos": 960000000},
  "parallelism_score": 0.33333334,
  "network_efficiency_score": 0.2
}
```

## 🔍 Planning Features

### Dependency Analysis
- **Explicit dependencies**: Respects task dependencies specified in playbooks
- **Implicit dependencies**: Detects file-based and service-package dependencies
- **Circular dependency detection**: Prevents invalid execution plans
- **Cross-play dependencies**: Handles dependencies between different plays

### Binary Deployment Optimization
- **Task grouping**: Groups compatible tasks for binary deployment
- **Compilation planning**: Determines when binary compilation is beneficial
- **Network optimization**: Reduces SSH round-trips through binary execution
- **Module compatibility**: Analyzes which modules can be statically linked

### Parallelization Analysis
- **Parallel group detection**: Identifies tasks that can run simultaneously
- **Resource contention**: Prevents conflicts between parallel tasks
- **Host-based parallelization**: Optimizes execution across multiple hosts
- **Execution order optimization**: Minimizes overall execution time

### Execution Strategies

#### Linear Strategy
- Sequential execution across all hosts
- Minimal parallelization for maximum safety
- Best for sensitive operations and debugging

#### Rolling Strategy
- Batch-based execution with configurable batch sizes
- Ideal for zero-downtime deployments
- Configurable failure thresholds

#### Free Strategy
- Maximum parallelization within dependency constraints
- Optimal for performance when safety allows
- Advanced resource management

#### Binary-Hybrid Strategy
- Intelligent mix of binary deployment and SSH execution
- Automatic binary vs SSH decision making
- Optimal balance of performance and compatibility

## 🧪 Testing

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run integration tests only
cargo test --test integration_tests

# Generate code coverage
cargo tarpaulin --out Html
```

### Test with Example Data

```bash
# Test with example playbook
cargo run -- example_playbook.json --list-tasks

# Test pipeline integration
echo '{"name":"test","plays":[],"vars":{}}' | cargo run -- --dry-run

# Test different strategies
cargo run -- example_playbook.json --strategy rolling --serial 2
```

## 🔧 Development

### Project Structure

```
rustle-plan/
├── src/
│   ├── bin/
│   │   └── rustle-plan.rs        # CLI binary entry point
│   ├── planner/
│   │   ├── mod.rs                # Planner module exports
│   │   ├── execution_plan.rs     # Core planning logic
│   │   ├── binary_deployment.rs  # Binary deployment planning
│   │   ├── dependency.rs         # Dependency analysis
│   │   ├── optimization.rs       # Execution optimization
│   │   ├── strategy.rs           # Execution strategies
│   │   ├── condition.rs          # Conditional execution
│   │   ├── estimation.rs         # Time estimation
│   │   ├── validation.rs         # Plan validation
│   │   ├── graph.rs              # Dependency graphs
│   │   ├── suitability.rs        # Binary suitability analysis
│   │   └── error.rs              # Error types
│   ├── types/
│   │   ├── mod.rs                # Type exports
│   │   ├── plan.rs               # Plan data structures
│   │   └── strategy.rs           # Strategy definitions
│   └── lib.rs                    # Library exports
├── tests/
│   ├── integration_tests.rs      # Integration test suite
│   └── planner/                  # Unit tests for planner modules
├── specs/                        # Specification documents
├── example_playbook.json         # Example input for testing
├── Cargo.toml                    # Project manifest
└── README.md                     # This file
```

### Key Dependencies

- **serde** & **serde_json** - JSON serialization and parsing
- **clap** - Command-line argument parsing
- **petgraph** - Dependency graph analysis
- **anyhow** & **thiserror** - Error handling
- **tracing** - Structured logging
- **chrono** - Date and time handling
- **uuid** - Unique identifier generation

## 🎯 Roadmap

### Current Status ✅

- [x] Basic execution plan generation
- [x] Dependency analysis with cycle detection
- [x] Multiple execution strategies
- [x] Binary deployment planning framework
- [x] CLI interface with comprehensive options
- [x] JSON input/output support
- [x] Integration with rustle-parse
- [x] Execution time estimation
- [x] Parallelization analysis

### Near Term 🔄

- [ ] Advanced binary deployment optimization
- [ ] Rolling update strategy improvements
- [ ] Enhanced visualization output
- [ ] Performance benchmarking suite
- [ ] Configuration file support

### Future Enhancements 🔮

- [ ] Real-time plan adaptation
- [ ] Machine learning optimization
- [ ] Advanced resource modeling
- [ ] Integration with monitoring systems
- [ ] Cloud-native execution strategies

## 🤝 Integration with Rustle Ecosystem

`rustle-plan` is designed to work seamlessly with other Rustle tools:

```bash
# Complete pipeline example
rustle-parse playbook.yml \
  | rustle-plan --strategy binary-hybrid --optimize \
  | rustle-deploy \
  | rustle-exec
```

### Input Compatibility
- **rustle-parse**: Consumes combined JSON output from rustle-parse including embedded inventory data

### Output Integration
- **rustle-deploy**: Execution plans used by rustle-deploy for binary compilation
- **rustle-exec**: Plans consumed by rustle-exec for execution
- **Monitoring**: Plan metadata for execution monitoring and analytics

## 📄 Specifications

This implementation follows [Specification 010: Rustle Plan Tool](specs/010-rustle-plan.md). See the specs directory for detailed requirements and design decisions.

## 🤝 Contributing

1. Follow the guidelines in `CLAUDE.md`
2. Ensure all tests pass: `cargo test`
3. Run formatters: `cargo fmt`
4. Check lints: `cargo clippy`
5. Update documentation as needed

## 📝 License

GPL-3.0 License - see [LICENSE](LICENSE) file for details.

---

Built with ❤️ in Rust for high-performance automation planning.