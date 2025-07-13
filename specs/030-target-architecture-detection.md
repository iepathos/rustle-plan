# Target Architecture Detection for Binary Deployment

## Feature Summary

This feature enhances rustle-plan to automatically detect and include target OS and architecture information in the execution plan output. This enables rustle-deploy to compile appropriate binaries for deployment targets without making assumptions about the target systems. The feature analyzes inventory host information and determines the correct compilation targets (OS and architecture) for each binary deployment plan.

## Goals & Requirements

### Functional Requirements
- Detect target OS and architecture from inventory host information
- Include compilation target details in each BinaryDeploymentPlan
- Support multiple architecture targets within a single execution plan
- Provide fallback defaults when target information cannot be determined
- Maintain backward compatibility with existing execution plan format

### Non-Functional Requirements
- Zero performance impact on execution plan generation
- Clear error messages when target detection fails
- Support for common target architectures (x86_64, aarch64, arm, etc.)
- Support for major operating systems (Linux, Darwin/macOS, Windows)
- Extensible design for adding new target platforms

### Success Criteria
- Rustle-deploy can successfully compile binaries for detected targets
- Each binary deployment includes accurate target architecture information
- Mixed architecture deployments are properly handled
- Fallback behavior works correctly when detection fails

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

### New Target Detection Module
```rust
// src/planner/target_detection.rs
pub struct TargetDetector {
    inventory_facts: HashMap<String, HostFacts>,
    default_target: TargetInfo,
}

#[derive(Debug, Clone)]
pub struct TargetInfo {
    pub arch: String,
    pub os: String,
    pub triple: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HostFacts {
    pub ansible_architecture: Option<String>,
    pub ansible_machine: Option<String>,
    pub ansible_system: Option<String>,
    pub ansible_os_family: Option<String>,
    pub ansible_distribution: Option<String>,
}

impl TargetDetector {
    pub fn new(inventory: &ParsedInventory) -> Self;
    pub fn detect_target_for_hosts(&self, hosts: &[String]) -> Result<TargetInfo, PlanError>;
    pub fn normalize_architecture(&self, arch: &str) -> String;
    pub fn normalize_os(&self, os: &str) -> String;
    pub fn build_target_triple(&self, arch: &str, os: &str) -> Option<String>;
}
```

### Integration with BinaryDeploymentPlanner
```rust
impl BinaryDeploymentPlanner {
    // Updated to accept target detector
    pub fn create_compilation_requirements(
        &self, 
        hosts: &[String],
        target_detector: &TargetDetector
    ) -> Result<CompilationRequirements, PlanError>;
}
```

## File and Package Structure

### New Files
- `src/planner/target_detection.rs` - Core target detection logic
- `tests/target_detection_tests.rs` - Unit tests for target detection

### Modified Files
- `src/planner/binary_deployment.rs` - Update to use target detection
- `src/types/plan.rs` - Update CompilationRequirements structure
- `src/planner/mod.rs` - Export target detection module
- `src/planner/execution_plan.rs` - Pass inventory data to binary planner

## Implementation Details

### Step 1: Create Target Detection Module
```rust
// src/planner/target_detection.rs
use std::collections::HashMap;
use crate::types::*;
use crate::planner::error::PlanError;

impl TargetDetector {
    pub fn new(inventory: &ParsedInventory) -> Self {
        let inventory_facts = Self::extract_host_facts(inventory);
        
        Self {
            inventory_facts,
            default_target: TargetInfo {
                arch: "x86_64".to_string(),
                os: "linux".to_string(),
                triple: None,
            },
        }
    }
    
    fn extract_host_facts(inventory: &ParsedInventory) -> HashMap<String, HostFacts> {
        let mut facts = HashMap::new();
        
        // Extract facts from inventory variables
        for host in &inventory.hosts {
            if let Some(host_vars) = inventory.vars.get(host) {
                if let Ok(host_facts) = serde_json::from_value::<HostFacts>(host_vars.clone()) {
                    facts.insert(host.clone(), host_facts);
                }
            }
        }
        
        facts
    }
    
    pub fn detect_target_for_hosts(&self, hosts: &[String]) -> Result<TargetInfo, PlanError> {
        // Collect all unique targets for the given hosts
        let mut targets = HashMap::new();
        
        for host in hosts {
            if let Some(facts) = self.inventory_facts.get(host) {
                let arch = self.detect_architecture(facts);
                let os = self.detect_os(facts);
                let key = format!("{}-{}", arch, os);
                
                targets.entry(key).or_insert(TargetInfo {
                    arch: arch.clone(),
                    os: os.clone(),
                    triple: self.build_target_triple(&arch, &os),
                });
            }
        }
        
        // If no targets detected, use default
        if targets.is_empty() {
            return Ok(self.default_target.clone());
        }
        
        // If multiple targets, select the most common one
        // In a real implementation, might want to handle this differently
        let (_, target) = targets.into_iter()
            .max_by_key(|(_, _)| 1) // Simplified - would count occurrences
            .unwrap();
            
        Ok(target)
    }
    
    fn detect_architecture(&self, facts: &HostFacts) -> String {
        // Try ansible_architecture first, then ansible_machine
        let raw_arch = facts.ansible_architecture.as_ref()
            .or(facts.ansible_machine.as_ref())
            .map(|s| s.as_str())
            .unwrap_or("x86_64");
            
        self.normalize_architecture(raw_arch)
    }
    
    fn detect_os(&self, facts: &HostFacts) -> String {
        // Try ansible_system first, then ansible_os_family
        let raw_os = facts.ansible_system.as_ref()
            .or(facts.ansible_os_family.as_ref())
            .map(|s| s.as_str())
            .unwrap_or("Linux");
            
        self.normalize_os(raw_os)
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

### Step 2: Update BinaryDeploymentPlanner
```rust
// Updates to src/planner/binary_deployment.rs
impl BinaryDeploymentPlanner {
    fn create_binary_deployment(
        &self,
        group: &TaskGroup,
        hosts: &[String],
        target_detector: &TargetDetector,
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
            target_detector
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
        target_detector: &TargetDetector,
    ) -> Result<CompilationRequirements, PlanError> {
        let target_info = target_detector.detect_target_for_hosts(hosts)?;
        
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

### Step 3: Update ExecutionPlanBuilder
```rust
// Updates to src/planner/execution_plan.rs
use crate::planner::target_detection::TargetDetector;

impl ExecutionPlanBuilder {
    pub fn build(&self, playbook: ParsedPlaybook, inventory: ParsedInventory) -> Result<ExecutionPlan, PlanError> {
        // Create target detector from inventory
        let target_detector = TargetDetector::new(&inventory);
        
        // ... existing code ...
        
        // Pass target detector to binary deployment planning
        let binary_deployments = if self.options.force_ssh {
            Vec::new()
        } else {
            self.binary_planner.plan_deployments_with_detector(
                &all_tasks,
                &all_hosts,
                self.options.binary_threshold,
                &target_detector,
            )?
        };
        
        // ... rest of implementation
    }
}
```

## Testing Strategy

### Unit Tests
```rust
// tests/target_detection_tests.rs
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_architecture_normalization() {
        let detector = TargetDetector::new(&ParsedInventory::default());
        
        assert_eq!(detector.normalize_architecture("x86_64"), "x86_64");
        assert_eq!(detector.normalize_architecture("amd64"), "x86_64");
        assert_eq!(detector.normalize_architecture("ARM64"), "aarch64");
        assert_eq!(detector.normalize_architecture("armv7l"), "armv7");
    }
    
    #[test]
    fn test_os_normalization() {
        let detector = TargetDetector::new(&ParsedInventory::default());
        
        assert_eq!(detector.normalize_os("Linux"), "linux");
        assert_eq!(detector.normalize_os("Darwin"), "darwin");
        assert_eq!(detector.normalize_os("MacOS"), "darwin");
        assert_eq!(detector.normalize_os("Windows"), "windows");
    }
    
    #[test]
    fn test_target_triple_generation() {
        let detector = TargetDetector::new(&ParsedInventory::default());
        
        assert_eq!(
            detector.build_target_triple("x86_64", "linux"),
            Some("x86_64-unknown-linux-gnu".to_string())
        );
        
        assert_eq!(
            detector.build_target_triple("aarch64", "darwin"),
            Some("aarch64-apple-darwin".to_string())
        );
    }
    
    #[test]
    fn test_host_target_detection() {
        let mut inventory = ParsedInventory {
            hosts: vec!["host1".to_string(), "host2".to_string()],
            groups: HashMap::new(),
            vars: HashMap::new(),
        };
        
        // Add host facts
        inventory.vars.insert("host1".to_string(), serde_json::json!({
            "ansible_architecture": "x86_64",
            "ansible_system": "Linux"
        }));
        
        inventory.vars.insert("host2".to_string(), serde_json::json!({
            "ansible_machine": "aarch64",
            "ansible_os_family": "Darwin"
        }));
        
        let detector = TargetDetector::new(&inventory);
        
        let target1 = detector.detect_target_for_hosts(&["host1".to_string()]).unwrap();
        assert_eq!(target1.arch, "x86_64");
        assert_eq!(target1.os, "linux");
        
        let target2 = detector.detect_target_for_hosts(&["host2".to_string()]).unwrap();
        assert_eq!(target2.arch, "aarch64");
        assert_eq!(target2.os, "darwin");
    }
}
```

### Integration Tests
- Test with real inventory files containing various architectures
- Verify execution plans include correct target information
- Test fallback behavior with missing inventory facts
- Test mixed architecture deployments

## Edge Cases & Error Handling

### Missing Inventory Facts
- When host facts are not available, fall back to default x86_64-linux
- Log warnings when using defaults
- Include detection confidence in output

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
- Include host architecture in metadata for debugging

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
    
    // New fields
    pub target_detection: TargetDetectionMode,
    pub default_target_arch: Option<String>,
    pub default_target_os: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TargetDetectionMode {
    Auto,        // Automatic detection from inventory
    Manual,      // Use provided defaults
    Disabled,    // Always use x86_64-linux
}
```

### Environment Variables
- `RUSTLE_PLAN_DEFAULT_ARCH` - Override default architecture
- `RUSTLE_PLAN_DEFAULT_OS` - Override default OS
- `RUSTLE_PLAN_TARGET_DETECTION` - Set detection mode

## Documentation

### API Documentation
```rust
/// Detects target architecture and OS from inventory host information.
/// 
/// This module analyzes Ansible facts in the inventory to determine
/// the appropriate compilation targets for binary deployments.
/// 
/// # Examples
/// 
/// ```
/// let inventory = ParsedInventory::from_file("inventory.yml")?;
/// let detector = TargetDetector::new(&inventory);
/// 
/// let target = detector.detect_target_for_hosts(&["web01", "web02"])?;
/// println!("Target: {}-{}", target.arch, target.os);
/// ```
pub struct TargetDetector { ... }
```

### User Documentation
- Add section to README explaining target detection
- Document supported architectures and OS combinations
- Provide examples of inventory facts format
- Explain fallback behavior and configuration options

### Migration Guide
- Existing execution plans remain compatible
- New fields are added without breaking changes
- Rustle-deploy will use new fields when available