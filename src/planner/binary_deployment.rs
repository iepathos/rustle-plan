use crate::planner::error::PlanError;
use crate::types::*;
use std::collections::HashMap;
use std::time::Duration;

pub struct BinaryDeploymentPlanner {
    _compilation_cache: CompilationCache,
    _target_profiles: HashMap<String, TargetProfile>,
}

#[derive(Debug, Clone)]
pub struct CompilationCache {
    _cached_builds: HashMap<String, CachedBuild>,
}

#[derive(Debug, Clone)]
pub struct CachedBuild {
    pub checksum: String,
    pub compilation_time: Duration,
    pub binary_size: u64,
}

#[derive(Debug, Clone)]
pub struct TargetProfile {
    pub arch: String,
    pub os: String,
    pub compilation_time_multiplier: f32,
}

impl BinaryDeploymentPlanner {
    pub fn new() -> Self {
        let mut target_profiles = HashMap::new();

        // Add common target profiles
        target_profiles.insert(
            "x86_64-linux".to_string(),
            TargetProfile {
                arch: "x86_64".to_string(),
                os: "linux".to_string(),
                compilation_time_multiplier: 1.0,
            },
        );

        target_profiles.insert(
            "aarch64-linux".to_string(),
            TargetProfile {
                arch: "aarch64".to_string(),
                os: "linux".to_string(),
                compilation_time_multiplier: 1.2,
            },
        );

        Self {
            _compilation_cache: CompilationCache {
                _cached_builds: HashMap::new(),
            },
            _target_profiles: target_profiles,
        }
    }

    pub fn plan_deployments(
        &self,
        tasks: &[TaskPlan],
        hosts: &[String],
        threshold: u32,
    ) -> Result<Vec<BinaryDeployment>, PlanError> {
        self.plan_deployments_with_inventory(tasks, hosts, threshold, None)
    }

    pub fn plan_deployments_with_inventory(
        &self,
        tasks: &[TaskPlan],
        hosts: &[String],
        threshold: u32,
        inventory: Option<&ParsedInventory>,
    ) -> Result<Vec<BinaryDeployment>, PlanError> {
        // Group tasks by compatibility and suitability
        let task_groups = self.analyze_task_groups(tasks)?;

        let mut deployments = Vec::new();

        for group in task_groups {
            if group.tasks.len() >= threshold as usize {
                let decision = self.should_use_binary(&group, threshold);

                match decision {
                    BinaryDeploymentDecision::Deploy { .. } => {
                        let deployment = self.create_binary_deployment(&group, hosts, inventory)?;
                        deployments.push(deployment);
                    }
                    BinaryDeploymentDecision::Skip { .. } => {
                        // Skip this group
                        continue;
                    }
                }
            }
        }

        // Optimize deployment grouping
        self.optimize_binary_deployments(&mut deployments)?;

        Ok(deployments)
    }

    pub fn analyze_task_groups(&self, tasks: &[TaskPlan]) -> Result<Vec<TaskGroup>, PlanError> {
        let mut groups = Vec::new();
        let mut ungrouped_tasks: Vec<&TaskPlan> = tasks.iter().collect();

        while !ungrouped_tasks.is_empty() {
            let seed_task = ungrouped_tasks.remove(0);
            let mut group = TaskGroup {
                id: format!("group_{}", groups.len()),
                tasks: vec![seed_task.clone()],
                hosts: seed_task.hosts.clone(),
                modules: vec![seed_task.module.clone()],
                network_operations: self.count_network_operations(seed_task),
            };

            // Find compatible tasks
            ungrouped_tasks.retain(|&task| {
                if self.is_binary_compatible(seed_task, task)
                    && self.has_host_overlap(&group.hosts, &task.hosts)
                {
                    group.tasks.push(task.clone());
                    group.modules.push(task.module.clone());
                    group.network_operations += self.count_network_operations(task);
                    false // Remove from ungrouped
                } else {
                    true // Keep in ungrouped
                }
            });

            // Only include groups that would benefit from binary deployment
            if group.network_operations > 3 {
                groups.push(group);
            }
        }

        Ok(groups)
    }

    pub fn should_use_binary(
        &self,
        task_group: &TaskGroup,
        threshold: u32,
    ) -> BinaryDeploymentDecision {
        // Check if group meets minimum threshold
        if (task_group.tasks.len() as u32) < threshold {
            return BinaryDeploymentDecision::Skip {
                reason: format!(
                    "Task group has {} tasks, below threshold of {}",
                    task_group.tasks.len(),
                    threshold
                ),
            };
        }

        // Check if all modules are binary-compatible
        for module in &task_group.modules {
            if !self.is_module_binary_compatible(module) {
                return BinaryDeploymentDecision::Skip {
                    reason: format!("Module '{module}' is not binary-compatible"),
                };
            }
        }

        // Calculate estimated benefit
        let ssh_operations = task_group.network_operations;
        let binary_operations = 2; // Upload binary + execute
        let estimated_benefit =
            (ssh_operations as f32 - binary_operations as f32) / ssh_operations as f32;

        if estimated_benefit > 0.5 {
            BinaryDeploymentDecision::Deploy {
                reason: format!(
                    "Binary deployment reduces network operations from {} to {} ({}% improvement)",
                    ssh_operations,
                    binary_operations,
                    (estimated_benefit * 100.0) as u32
                ),
                estimated_benefit,
            }
        } else {
            BinaryDeploymentDecision::Skip {
                reason: "Insufficient network overhead reduction".to_string(),
            }
        }
    }

    fn create_binary_deployment(
        &self,
        group: &TaskGroup,
        hosts: &[String],
        inventory: Option<&ParsedInventory>,
    ) -> Result<BinaryDeployment, PlanError> {
        let deployment_hosts: Vec<String> = hosts
            .iter()
            .filter(|host| group.hosts.contains(host))
            .cloned()
            .collect();

        let embedded_data = self.create_embedded_data(group)?;
        let estimated_size = self.estimate_binary_size(group)?;

        Ok(BinaryDeployment {
            deployment_id: group.id.clone(),
            target_hosts: deployment_hosts.clone(),
            binary_name: format!("rustle-runner-{}", group.id),
            tasks: group.tasks.iter().map(|t| t.task_id.clone()).collect(),
            modules: group.modules.clone(),
            embedded_data,
            execution_mode: BinaryExecutionMode::Controller,
            estimated_size,
            compilation_requirements: self
                .create_compilation_requirements(&deployment_hosts, inventory)?,
        })
    }

    fn create_embedded_data(&self, group: &TaskGroup) -> Result<BinaryEmbeddedData, PlanError> {
        Ok(BinaryEmbeddedData {
            execution_plan: self.serialize_group_plan(group)?,
            static_files: self.extract_static_files(&group.tasks)?,
            variables: self.extract_variables(&group.tasks)?,
            facts_required: self.extract_fact_dependencies(&group.tasks)?,
        })
    }

    fn serialize_group_plan(&self, group: &TaskGroup) -> Result<String, PlanError> {
        // Create a minimal execution plan for this group
        let mini_plan = serde_json::json!({
            "group_id": group.id,
            "tasks": group.tasks,
            "hosts": group.hosts,
        });

        Ok(serde_json::to_string(&mini_plan)?)
    }

    fn extract_static_files(&self, tasks: &[TaskPlan]) -> Result<Vec<EmbeddedFile>, PlanError> {
        let mut files = Vec::new();

        for task in tasks {
            if task.module == "copy" || task.module == "template" {
                if let Some(src) = task.args.get("src").and_then(|v| v.as_str()) {
                    if let Some(dest) = task.args.get("dest").and_then(|v| v.as_str()) {
                        files.push(EmbeddedFile {
                            src_path: src.to_string(),
                            dest_path: dest.to_string(),
                            checksum: "placeholder-checksum".to_string(), // Would calculate real checksum
                            size: 1024, // Would calculate real size
                        });
                    }
                }
            }
        }

        Ok(files)
    }

    fn extract_variables(
        &self,
        tasks: &[TaskPlan],
    ) -> Result<HashMap<String, serde_json::Value>, PlanError> {
        let mut variables = HashMap::new();

        // Extract variables referenced in task arguments
        for task in tasks {
            for (key, value) in &task.args {
                if let Some(str_value) = value.as_str() {
                    if str_value.contains("{{") && str_value.contains("}}") {
                        // This is a template variable - in a real implementation,
                        // we would parse and extract the variable names
                        variables.insert(key.clone(), value.clone());
                    }
                }
            }
        }

        Ok(variables)
    }

    fn extract_fact_dependencies(&self, tasks: &[TaskPlan]) -> Result<Vec<String>, PlanError> {
        let mut facts = Vec::new();

        // Common facts that tasks might need
        for task in tasks {
            match task.module.as_str() {
                "package" => {
                    facts.push("ansible_pkg_mgr".to_string());
                    facts.push("ansible_os_family".to_string());
                }
                "service" => {
                    facts.push("ansible_service_mgr".to_string());
                }
                "file" | "copy" | "template" => {
                    facts.push("ansible_user_uid".to_string());
                    facts.push("ansible_user_gid".to_string());
                }
                _ => {}
            }
        }

        facts.sort();
        facts.dedup();
        Ok(facts)
    }

    fn estimate_binary_size(&self, group: &TaskGroup) -> Result<u64, PlanError> {
        // Base binary size (Rust runtime + our code)
        let base_size = 5 * 1024 * 1024; // 5MB

        // Add size for embedded data
        let embedded_size = group.tasks.len() as u64 * 1024; // 1KB per task

        // Add size for static files
        let static_file_size = group
            .tasks
            .iter()
            .filter(|t| t.module == "copy" || t.module == "template")
            .count() as u64
            * 10
            * 1024; // 10KB per file

        Ok(base_size + embedded_size + static_file_size)
    }

    fn create_compilation_requirements(
        &self,
        target_hosts: &[String],
        inventory: Option<&ParsedInventory>,
    ) -> Result<CompilationRequirements, PlanError> {
        // Try to determine target architecture from host facts
        let (target_arch, target_os) = if let Some(inventory) = inventory {
            self.determine_target_from_facts(target_hosts, inventory)
        } else {
            // Fallback to default values if no inventory/facts available
            ("x86_64".to_string(), "linux".to_string())
        };

        // Check if cross-compilation is needed
        let current_arch = std::env::consts::ARCH;
        let current_os = std::env::consts::OS;
        let cross_compilation = target_arch != current_arch || target_os != current_os;

        Ok(CompilationRequirements {
            target_arch,
            target_os,
            rust_version: "1.70.0".to_string(),
            cross_compilation,
            static_linking: true,
        })
    }

    fn determine_target_from_facts(
        &self,
        target_hosts: &[String],
        inventory: &ParsedInventory,
    ) -> (String, String) {
        // Use the first target host with facts available
        for host in target_hosts {
            if let Some(facts) = inventory.host_facts.get(host) {
                let arch = facts
                    .get("ansible_architecture")
                    .and_then(|v| v.as_str())
                    .map(|arch| match arch {
                        "aarch64" => "aarch64",
                        "arm64" => "aarch64",
                        "x86_64" => "x86_64",
                        "i386" | "i686" => "i686",
                        _ => "x86_64", // default fallback
                    })
                    .unwrap_or("x86_64")
                    .to_string();

                let os = facts
                    .get("ansible_system")
                    .and_then(|v| v.as_str())
                    .map(|system| match system {
                        "Darwin" => "macos",
                        "Linux" => "linux",
                        "Windows" => "windows",
                        _ => "linux", // default fallback
                    })
                    .unwrap_or("linux")
                    .to_string();

                return (arch, os);
            }
        }

        // Fallback if no facts found
        ("x86_64".to_string(), "linux".to_string())
    }

    fn optimize_binary_deployments(
        &self,
        deployments: &mut Vec<BinaryDeployment>,
    ) -> Result<(), PlanError> {
        // Sort by estimated benefit (larger deployments first)
        deployments.sort_by(|a, b| b.estimated_size.cmp(&a.estimated_size));

        // Remove duplicate deployments for the same hosts
        deployments.dedup_by(|a, b| a.target_hosts == b.target_hosts);

        Ok(())
    }

    pub fn estimate_compilation_time(
        &self,
        deployments: &[BinaryDeployment],
    ) -> Result<Duration, PlanError> {
        let base_compilation_time = Duration::from_secs(30); // Base Rust compilation time
        let per_task_time = Duration::from_millis(100); // Additional time per task

        let total_tasks: usize = deployments.iter().map(|d| d.tasks.len()).sum();
        let compilation_overhead = per_task_time * total_tasks as u32;

        Ok(base_compilation_time + compilation_overhead)
    }

    fn count_network_operations(&self, task: &TaskPlan) -> u32 {
        match task.module.as_str() {
            "copy" | "template" | "fetch" => 2, // Upload + command
            "package" | "service" => 1,         // Command only
            "shell" | "command" => 1,           // Command only
            _ => 1,
        }
    }

    fn is_binary_compatible(&self, task1: &TaskPlan, task2: &TaskPlan) -> bool {
        let compatible_modules = ["file", "copy", "template", "shell", "package", "service"];
        let interactive_modules = ["pause", "prompt"];

        compatible_modules.contains(&task1.module.as_str())
            && compatible_modules.contains(&task2.module.as_str())
            && !interactive_modules.contains(&task1.module.as_str())
            && !interactive_modules.contains(&task2.module.as_str())
            && task1.risk_level != RiskLevel::Critical
            && task2.risk_level != RiskLevel::Critical
    }

    fn has_host_overlap(&self, hosts1: &[String], hosts2: &[String]) -> bool {
        hosts1.iter().any(|h| hosts2.contains(h))
    }

    fn is_module_binary_compatible(&self, module: &str) -> bool {
        let compatible_modules = [
            "file", "copy", "template", "shell", "command", "package", "service", "user", "group",
            "cron",
        ];

        compatible_modules.contains(&module)
    }
}

impl Default for BinaryDeploymentPlanner {
    fn default() -> Self {
        Self::new()
    }
}
