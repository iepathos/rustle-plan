# Target Architecture Detection for Binary Deployment

## Feature Summary

This feature enhances rustle-plan to include target OS and architecture information in the execution plan output. Since standard Ansible inventories don't contain system facts (OS/architecture), this feature implements a flexible approach using sensible defaults with multiple override mechanisms. This enables rustle-deploy to compile appropriate binaries for deployment targets without making assumptions about the target systems.

## Goals & Requirements

### Functional Requirements
- Provide sensible default target architecture (x86_64-linux)
- Support multiple override mechanisms for specifying targets:
  - Command-line flags
  - Inventory variables
  - Configuration file
  - Fact cache integration (optional)
- Include compilation target details in each BinaryDeploymentPlan
- Support multiple architecture targets within a single execution plan
- Support per-host and per-group target specifications
- Maintain backward compatibility with existing execution plan format

### Non-Functional Requirements
- Zero performance impact on execution plan generation
- Clear error messages when target detection fails
- Support for common target architectures (x86_64, aarch64, arm, etc.)
- Support for major operating systems (Linux, Darwin/macOS, Windows)
- Extensible design for adding new target platforms

### Success Criteria
- Rustle-deploy can successfully compile binaries for specified targets
- Each binary deployment includes accurate target architecture information
- Mixed architecture deployments are properly handled
- Default behavior works correctly when no target is specified
- Override mechanisms work at all levels (global, group, host)

## API/Interface Design

### Updated CompilationRequirements Structure
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilationRequirements {
    // Primary fields (preferred by rustle-deploy)
    pub target_arch: String,        // Architecture: x86_64, aarch64, arm, etc.
    pub target_os: String,          // OS: linux, darwin, windows
    
    // Optional explicit target triple
    pub target_triple: Option<String>,  // e.g., x86_64-unknown-linux-gnu
    
    // Existing fields
    pub rust_version: String,
    pub cross_compilation: bool,
    pub static_linking: bool,
}
```

### New Target Resolution Module
```rust
// src/planner/target_resolution.rs
pub struct TargetResolver {
    default_target: TargetInfo,
    global_override: Option<TargetInfo>,
    inventory_overrides: HashMap<String, TargetInfo>,
    fact_cache: Option<FactCache>,
}

#[derive(Debug, Clone)]
pub struct TargetInfo {
    pub arch: String,
    pub os: String,
    pub triple: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TargetOverrides {
    pub global_arch: Option<String>,
    pub global_os: Option<String>,
    pub fact_cache_path: Option<PathBuf>,
}

impl TargetResolver {
    pub fn new(inventory: &ParsedInventory, overrides: TargetOverrides) -> Self;
    pub fn resolve_target_for_hosts(&self, hosts: &[String]) -> Result<TargetInfo, PlanError>;
    pub fn normalize_architecture(&self, arch: &str) -> String;
    pub fn normalize_os(&self, os: &str) -> String;
    pub fn build_target_triple(&self, arch: &str, os: &str) -> Option<String>;
}
```

### Integration with BinaryDeploymentPlanner
```rust
impl BinaryDeploymentPlanner {
    // Updated to accept target resolver
    pub fn create_compilation_requirements(
        &self, 
        hosts: &[String],
        target_resolver: &TargetResolver
    ) -> Result<CompilationRequirements, PlanError>;
}
```

### Command-line Interface Updates
```rust
// Add to rustle-plan CLI arguments
#[derive(Parser)]
struct Args {
    // ... existing args ...
    
    /// Target architecture for binary compilation (default: x86_64)
    #[arg(long, env = "RUSTLE_TARGET_ARCH")]
    target_arch: Option<String>,
    
    /// Target OS for binary compilation (default: linux)
    #[arg(long, env = "RUSTLE_TARGET_OS")]
    target_os: Option<String>,
    
    /// Path to Ansible fact cache for automatic target detection
    #[arg(long, env = "RUSTLE_FACT_CACHE")]
    fact_cache: Option<PathBuf>,
}
```

## File and Package Structure

### New Files
- `src/planner/target_resolution.rs` - Core target resolution logic
- `src/planner/fact_cache.rs` - Optional fact cache integration
- `tests/target_resolution_tests.rs` - Unit tests for target resolution

### Modified Files
- `src/planner/binary_deployment.rs` - Update to use target resolution
- `src/types/plan.rs` - Update CompilationRequirements structure  
- `src/planner/mod.rs` - Export target resolution module
- `src/planner/execution_plan.rs` - Pass target resolver to binary planner
- `src/bin/rustle-plan.rs` - Add CLI arguments for target specification

## Implementation Details

### Step 1: Target Resolution Priority

The target resolver will use the following priority order:
1. **Host-specific variables** in inventory (highest priority)
   - `host1 ansible_architecture=aarch64 ansible_system=Linux`
2. **Group variables** in inventory
   - `[webservers:vars]`
   - `ansible_architecture=x86_64`
3. **Command-line overrides**
   - `--target-arch x86_64 --target-os linux`
4. **Fact cache** (if available and path provided)
   - Read from Ansible's JSON fact cache
5. **Default values** (lowest priority)
   - Architecture: `x86_64`
   - OS: `linux`

### Step 2: Create Target Resolution Module
```rust
// src/planner/target_resolution.rs
use std::collections::HashMap;
use std::path::PathBuf;
use crate::types::*;
use crate::planner::error::PlanError;

impl TargetResolver {
    pub fn new(inventory: &ParsedInventory, overrides: TargetOverrides) -> Self {
        let mut inventory_overrides = HashMap::new();
        
        // Extract target info from inventory variables
        for host in &inventory.hosts {
            if let Some(host_vars) = inventory.vars.get(host) {
                if let Some(target) = Self::extract_target_from_vars(host_vars) {
                    inventory_overrides.insert(host.clone(), target);
                }
            }
        }
        
        // Also check group variables
        for (group, hosts) in &inventory.groups {
            if let Some(group_vars) = inventory.vars.get(group) {
                if let Some(target) = Self::extract_target_from_vars(group_vars) {
                    // Apply to all hosts in group that don't have host-specific overrides
                    for host in hosts {
                        inventory_overrides.entry(host.clone()).or_insert(target.clone());
                    }
                }
            }
        }
        
        let global_override = match (overrides.global_arch, overrides.global_os) {
            (Some(arch), Some(os)) => Some(TargetInfo {
                arch,
                os,
                triple: None,
            }),
            _ => None,
        };
        
        let fact_cache = overrides.fact_cache_path
            .and_then(|path| FactCache::load(&path).ok());
        
        Self {
            inventory_overrides,
            global_override,
            fact_cache,
            default_target: TargetInfo {
                arch: "x86_64".to_string(),
                os: "linux".to_string(),
                triple: None,
            },
        }
    }
    
    fn extract_target_from_vars(vars: &serde_json::Value) -> Option<TargetInfo> {
        let arch = vars.get("ansible_architecture")
            .or_else(|| vars.get("ansible_machine"))
            .and_then(|v| v.as_str());
            
        let os = vars.get("ansible_system")
            .or_else(|| vars.get("ansible_os_family"))
            .and_then(|v| v.as_str());
            
        match (arch, os) {
            (Some(a), Some(o)) => Some(TargetInfo {
                arch: a.to_string(),
                os: o.to_string(),
                triple: None,
            }),
            _ => None,
        }
    }
    
    pub fn resolve_target_for_hosts(&self, hosts: &[String]) -> Result<TargetInfo, PlanError> {
        // Priority order for resolution
        let mut target_counts: HashMap<String, (TargetInfo, usize)> = HashMap::new();
        
        for host in hosts {
            let target = self.resolve_target_for_host(host)?;
            let key = format!("{}-{}", target.arch, target.os);
            
            target_counts.entry(key)
                .and_modify(|(_, count)| *count += 1)
                .or_insert((target, 1));
        }
        
        // If no hosts specified, use global override or default
        if hosts.is_empty() {
            return Ok(self.global_override.clone()
                .unwrap_or_else(|| self.default_target.clone()));
        }
        
        // Return the most common target among the hosts
        let (_, (target, _)) = target_counts.into_iter()
            .max_by_key(|(_, (_, count))| *count)
            .ok_or_else(|| PlanError::ValidationError("No targets resolved".to_string()))?;
            
        Ok(target)
    }
    
    fn resolve_target_for_host(&self, host: &str) -> Result<TargetInfo, PlanError> {
        // Priority order:
        // 1. Host-specific inventory override
        if let Some(target) = self.inventory_overrides.get(host) {
            return Ok(self.normalize_target(target.clone()));
        }
        
        // 2. Fact cache (if available)
        if let Some(fact_cache) = &self.fact_cache {
            if let Some(target) = fact_cache.get_target_for_host(host) {
                return Ok(self.normalize_target(target));
            }
        }
        
        // 3. Global override
        if let Some(target) = &self.global_override {
            return Ok(self.normalize_target(target.clone()));
        }
        
        // 4. Default
        Ok(self.default_target.clone())
    }
    
    fn normalize_target(&self, mut target: TargetInfo) -> TargetInfo {
        target.arch = self.normalize_architecture(&target.arch);
        target.os = self.normalize_os(&target.os);
        target.triple = self.build_target_triple(&target.arch, &target.os);
        target
    }
    
    pub fn normalize_architecture(&self, arch: &str) -> String {
        match arch.to_lowercase().as_str() {
            "x86_64" | "amd64" => "x86_64".to_string(),
            "aarch64" | "arm64" => "aarch64".to_string(),
            "armv7l" | "armv7" => "armv7".to_string(),
            "i686" | "i386" => "i686".to_string(),
            _ => arch.to_lowercase(),
        }
    }
    
    pub fn normalize_os(&self, os: &str) -> String {
        match os.to_lowercase().as_str() {
            "linux" => "linux".to_string(),
            "darwin" | "macos" | "osx" => "darwin".to_string(),
            "windows" | "win32" => "windows".to_string(),
            "freebsd" => "freebsd".to_string(),
            _ => os.to_lowercase(),
        }
    }
    
    pub fn build_target_triple(&self, arch: &str, os: &str) -> Option<String> {
        match (arch, os) {
            ("x86_64", "linux") => Some("x86_64-unknown-linux-gnu".to_string()),
            ("aarch64", "linux") => Some("aarch64-unknown-linux-gnu".to_string()),
            ("x86_64", "darwin") => Some("x86_64-apple-darwin".to_string()),
            ("aarch64", "darwin") => Some("aarch64-apple-darwin".to_string()),
            ("x86_64", "windows") => Some("x86_64-pc-windows-msvc".to_string()),
            ("i686", "linux") => Some("i686-unknown-linux-gnu".to_string()),
            ("armv7", "linux") => Some("armv7-unknown-linux-gnueabihf".to_string()),
            _ => None,
        }
    }
}
```

### Step 3: Fact Cache Integration (Optional)
```rust
// src/planner/fact_cache.rs
use std::path::Path;
use std::collections::HashMap;
use serde_json::Value;

pub struct FactCache {
    facts: HashMap<String, Value>,
}

impl FactCache {
    pub fn load(path: &Path) -> Result<Self, PlanError> {
        // Support JSON fact cache format
        let content = std::fs::read_to_string(path)?;
        let facts: HashMap<String, Value> = serde_json::from_str(&content)?;
        Ok(Self { facts })
    }
    
    pub fn get_target_for_host(&self, host: &str) -> Option<TargetInfo> {
        let host_facts = self.facts.get(host)?;
        
        let arch = host_facts.get("ansible_facts")
            .and_then(|f| f.get("ansible_architecture"))
            .or_else(|| host_facts.get("ansible_architecture"))
            .and_then(|v| v.as_str());
            
        let os = host_facts.get("ansible_facts")
            .and_then(|f| f.get("ansible_system"))
            .or_else(|| host_facts.get("ansible_system"))
            .and_then(|v| v.as_str());
            
        match (arch, os) {
            (Some(a), Some(o)) => Some(TargetInfo {
                arch: a.to_string(),
                os: o.to_string(),
                triple: None,
            }),
            _ => None,
        }
    }
}
```

### Step 4: Update BinaryDeploymentPlanner
```rust
// Updates to src/planner/binary_deployment.rs
impl BinaryDeploymentPlanner {
    fn create_binary_deployment(
        &self,
        group: &TaskGroup,
        hosts: &[String],
        target_resolver: &TargetResolver,
    ) -> Result<BinaryDeployment, PlanError> {
        let deployment_hosts: Vec<String> = hosts
            .iter()
            .filter(|host| group.hosts.contains(host))
            .cloned()
            .collect();

        let embedded_data = self.create_embedded_data(group)?;
        let estimated_size = self.estimate_binary_size(group)?;
        
        // Use target detector for compilation requirements
        let compilation_requirements = self.create_compilation_requirements(
            &deployment_hosts,
            target_resolver
        )?;

        Ok(BinaryDeployment {
            deployment_id: group.id.clone(),
            target_hosts: deployment_hosts,
            binary_name: format!("rustle-runner-{}", group.id),
            tasks: group.tasks.iter().map(|t| t.task_id.clone()).collect(),
            modules: group.modules.clone(),
            embedded_data,
            execution_mode: BinaryExecutionMode::Controller,
            estimated_size,
            compilation_requirements,
        })
    }
    
    fn create_compilation_requirements(
        &self,
        hosts: &[String],
        target_resolver: &TargetResolver,
    ) -> Result<CompilationRequirements, PlanError> {
        let target_info = target_resolver.detect_target_for_hosts(hosts)?;
        
        Ok(CompilationRequirements {
            target_arch: target_info.arch,
            target_os: target_info.os,
            target_triple: target_info.triple,
            rust_version: "1.70.0".to_string(),
            cross_compilation: self.requires_cross_compilation(&target_info),
            static_linking: true,
        })
    }
    
    fn requires_cross_compilation(&self, target: &TargetInfo) -> bool {
        // Detect if cross-compilation is needed based on current host
        #[cfg(target_arch = "x86_64")]
        let host_arch = "x86_64";
        #[cfg(target_arch = "aarch64")]
        let host_arch = "aarch64";
        #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
        let host_arch = "unknown";
        
        #[cfg(target_os = "linux")]
        let host_os = "linux";
        #[cfg(target_os = "macos")]
        let host_os = "darwin";
        #[cfg(target_os = "windows")]
        let host_os = "windows";
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        let host_os = "unknown";
        
        target.arch != host_arch || target.os != host_os
    }
}
```

### Step 5: Update ExecutionPlanBuilder
```rust
// Updates to src/planner/execution_plan.rs
use crate::planner::target_resolution::{TargetResolver, TargetOverrides};

impl ExecutionPlanBuilder {
    pub fn build(&self, playbook: ParsedPlaybook, inventory: ParsedInventory) -> Result<ExecutionPlan, PlanError> {
        // Create target resolver from inventory and CLI options
        let target_overrides = TargetOverrides {
            global_arch: self.options.target_arch.clone(),
            global_os: self.options.target_os.clone(),
            fact_cache_path: self.options.fact_cache_path.clone(),
        };
        let target_resolver = TargetResolver::new(&inventory, target_overrides);
        
        // ... existing code ...
        
        // Pass target resolver to binary deployment planning
        let binary_deployments = if self.options.force_ssh {
            Vec::new()
        } else {
            self.binary_planner.plan_deployments_with_resolver(
                &all_tasks,
                &all_hosts,
                self.options.binary_threshold,
                &target_resolver,
            )?
        };
        
        // ... rest of implementation
    }
}
```

## Testing Strategy

### Unit Tests
```rust
// tests/target_resolution_tests.rs
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_architecture_normalization() {
        let resolver = TargetResolver::new(&ParsedInventory::default(), TargetOverrides::default());
        
        assert_eq!(resolver.normalize_architecture("x86_64"), "x86_64");
        assert_eq!(resolver.normalize_architecture("amd64"), "x86_64");
        assert_eq!(resolver.normalize_architecture("ARM64"), "aarch64");
        assert_eq!(resolver.normalize_architecture("armv7l"), "armv7");
    }
    
    #[test]
    fn test_os_normalization() {
        let resolver = TargetResolver::new(&ParsedInventory::default(), TargetOverrides::default());
        
        assert_eq!(resolver.normalize_os("Linux"), "linux");
        assert_eq!(resolver.normalize_os("Darwin"), "darwin");
        assert_eq!(resolver.normalize_os("MacOS"), "darwin");
        assert_eq!(resolver.normalize_os("Windows"), "windows");
    }
    
    #[test]
    fn test_target_triple_generation() {
        let resolver = TargetResolver::new(&ParsedInventory::default(), TargetOverrides::default());
        
        assert_eq!(
            resolver.build_target_triple("x86_64", "linux"),
            Some("x86_64-unknown-linux-gnu".to_string())
        );
        
        assert_eq!(
            resolver.build_target_triple("aarch64", "darwin"),
            Some("aarch64-apple-darwin".to_string())
        );
    }
    
    #[test]
    fn test_inventory_target_resolution() {
        let mut inventory = ParsedInventory {
            hosts: vec!["host1".to_string(), "host2".to_string()],
            groups: HashMap::new(),
            vars: HashMap::new(),
        };
        
        // Add host-specific variables
        inventory.vars.insert("host1".to_string(), serde_json::json!({
            "ansible_architecture": "x86_64",
            "ansible_system": "Linux"
        }));
        
        inventory.vars.insert("host2".to_string(), serde_json::json!({
            "ansible_machine": "aarch64",
            "ansible_os_family": "Darwin"
        }));
        
        let resolver = TargetResolver::new(&inventory, TargetOverrides::default());
        
        let target1 = resolver.resolve_target_for_hosts(&["host1".to_string()]).unwrap();
        assert_eq!(target1.arch, "x86_64");
        assert_eq!(target1.os, "linux");
        
        let target2 = resolver.resolve_target_for_hosts(&["host2".to_string()]).unwrap();
        assert_eq!(target2.arch, "aarch64");
        assert_eq!(target2.os, "darwin");
    }
    
    #[test]
    fn test_cli_override_priority() {
        let inventory = ParsedInventory {
            hosts: vec!["host1".to_string()],
            groups: HashMap::new(),
            vars: HashMap::from([(
                "host1".to_string(),
                serde_json::json!({
                    "ansible_architecture": "x86_64",
                    "ansible_system": "Linux"
                })
            )]),
        };
        
        let overrides = TargetOverrides {
            global_arch: Some("aarch64".to_string()),
            global_os: Some("darwin".to_string()),
            fact_cache_path: None,
        };
        
        let resolver = TargetResolver::new(&inventory, overrides);
        
        // Host-specific variables should take priority over CLI overrides
        let target = resolver.resolve_target_for_hosts(&["host1".to_string()]).unwrap();
        assert_eq!(target.arch, "x86_64");
        assert_eq!(target.os, "linux");
        
        // For hosts without specific variables, CLI override should apply
        let target = resolver.resolve_target_for_hosts(&["unknown_host".to_string()]).unwrap();
        assert_eq!(target.arch, "aarch64");
        assert_eq!(target.os, "darwin");
    }
}
```

### Integration Tests
- Test with real inventory files containing target variables
- Verify execution plans include correct target information  
- Test priority order of different override mechanisms
- Test mixed architecture deployments
- Test fact cache integration when available

## Edge Cases & Error Handling

### Missing Target Information
- When no target information is provided via any mechanism, use default x86_64-linux
- Log warnings when using defaults
- Include source of target information in output (e.g., "from inventory", "from CLI", "default")

### Mixed Architecture Groups
- When a task group targets hosts with different architectures:
  - Option 1: Create separate binary deployments for each architecture
  - Option 2: Select the most common architecture and log warnings
  - Option 3: Fall back to SSH for mixed groups

### Unknown Architectures
- Maintain a mapping of known architecture aliases
- Log warnings for unrecognized architectures
- Pass through unrecognized values rather than failing

### Cross-Compilation Detection
- Automatically detect when cross-compilation is needed
- Set the `cross_compilation` flag appropriately
- Include controller architecture in metadata for debugging

### Inventory Examples
Users can specify target architectures in their inventory:
```ini
# Per-host specification
[webservers]
web1 ansible_host=192.168.1.10 ansible_architecture=x86_64 ansible_system=Linux
web2 ansible_host=192.168.1.11 ansible_architecture=aarch64 ansible_system=Linux

# Group-level specification  
[databases:vars]
ansible_architecture=x86_64
ansible_system=Linux

# Mixed architectures
[edge_devices]
edge1 ansible_host=10.0.1.1 ansible_architecture=armv7 ansible_system=Linux
edge2 ansible_host=10.0.1.2 ansible_architecture=aarch64 ansible_system=Linux
```

## Dependencies

### External Dependencies
- No new external crates required
- Uses existing serde for JSON parsing

### Internal Dependencies
- Depends on inventory parsing module
- Integrates with binary deployment planner
- Uses existing error types

## Configuration

### New Configuration Options
```rust
// Add to PlanningOptions
pub struct PlanningOptions {
    // ... existing fields ...
    
    // New fields for target specification
    pub target_arch: Option<String>,
    pub target_os: Option<String>,
    pub fact_cache_path: Option<PathBuf>,
}
```

### Environment Variables
- `RUSTLE_TARGET_ARCH` - Override default architecture
- `RUSTLE_TARGET_OS` - Override default OS
- `RUSTLE_FACT_CACHE` - Path to Ansible fact cache

## Documentation

### API Documentation
```rust
/// Resolves target architecture and OS for binary compilation.
/// 
/// This module provides a flexible system for determining compilation
/// targets using multiple sources: inventory variables, CLI arguments,
/// fact caches, and defaults.
/// 
/// # Priority Order
/// 
/// 1. Host-specific inventory variables
/// 2. Group inventory variables  
/// 3. CLI overrides (--target-arch, --target-os)
/// 4. Fact cache (if available)
/// 5. Default (x86_64-linux)
/// 
/// # Examples
/// 
/// ```
/// let inventory = ParsedInventory::from_file("inventory.yml")?;
/// let overrides = TargetOverrides {
///     global_arch: Some("aarch64".to_string()),
///     global_os: Some("linux".to_string()),
///     fact_cache_path: None,
/// };
/// let resolver = TargetResolver::new(&inventory, overrides);
/// 
/// let target = resolver.resolve_target_for_hosts(&["web01", "web02"])?;
/// println!("Target: {}-{}", target.arch, target.os);
/// ```
pub struct TargetResolver { ... }
```

### User Documentation

#### README Addition
```markdown
## Specifying Target Architectures

Since Rustle compiles binaries ahead of time (unlike Ansible which runs Python at runtime), 
you need to specify the target architecture and OS for your deployment hosts.

### Methods to Specify Targets

1. **Inventory Variables** (Recommended)
   ```ini
   [webservers]
   web1 ansible_host=192.168.1.10 ansible_architecture=x86_64 ansible_system=Linux
   web2 ansible_host=192.168.1.11 ansible_architecture=aarch64 ansible_system=Linux
   
   [databases:vars]
   ansible_architecture=x86_64
   ansible_system=Linux
   ```

2. **Command Line**
   ```bash
   rustle-plan --target-arch x86_64 --target-os linux playbook.yml inventory.ini
   ```

3. **Environment Variables**
   ```bash
   export RUSTLE_TARGET_ARCH=aarch64
   export RUSTLE_TARGET_OS=linux
   rustle-plan playbook.yml inventory.ini
   ```

4. **Fact Cache** (Advanced)
   ```bash
   # First gather facts
   ansible all -m setup --tree /tmp/facts
   
   # Then use the fact cache
   rustle-plan --fact-cache /tmp/facts playbook.yml inventory.ini
   ```

### Supported Targets

| Architecture | OS | Target Triple |
|-------------|-----|---------------|
| x86_64 | linux | x86_64-unknown-linux-gnu |
| aarch64 | linux | aarch64-unknown-linux-gnu |
| x86_64 | darwin | x86_64-apple-darwin |
| aarch64 | darwin | aarch64-apple-darwin |
| x86_64 | windows | x86_64-pc-windows-msvc |
| armv7 | linux | armv7-unknown-linux-gnueabihf |

### Default Behavior

If no target is specified, rustle-plan defaults to `x86_64-linux`.
```

### Migration Guide
- Existing execution plans remain compatible
- New fields are added without breaking changes
- Rustle-deploy will use new fields when available