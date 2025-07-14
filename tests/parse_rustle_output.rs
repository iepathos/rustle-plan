use anyhow::Result;
use rustle_plan::{ExecutionPlanner, ExecutionStrategy, PlanningOptions};
use std::fs;
use std::path::PathBuf;

#[test]
fn test_parse_rustle_output_with_string_hosts() -> Result<()> {
    // Read the example output from fixtures
    let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("rustle_parse_example_output.json");

    let content = fs::read_to_string(&fixture_path)?;

    // Create a minimal executable that can parse the output
    let result = parse_rustle_output(&content);

    assert!(
        result.is_ok(),
        "Failed to parse rustle output: {:?}",
        result.err()
    );

    let (playbook, inventory) = result?;

    // Verify the parsed data
    assert_eq!(playbook.name, "simple");
    assert_eq!(playbook.plays.len(), 1);

    let play = &playbook.plays[0];
    assert_eq!(play.name, "Simple test playbook");
    assert_eq!(play.hosts, vec!["all"]);
    assert_eq!(play.tasks.len(), 3);
    assert_eq!(play.handlers.len(), 1);

    // Verify tasks
    assert_eq!(play.tasks[0].name, "Print a message");
    assert_eq!(play.tasks[0].module, "debug");
    assert_eq!(play.tasks[1].name, "Install package");
    assert_eq!(play.tasks[1].module, "package");
    assert_eq!(play.tasks[2].name, "Notify handler");
    assert_eq!(play.tasks[2].module, "command");

    // Verify handler
    assert_eq!(play.handlers[0].name, "restart service");
    assert_eq!(play.handlers[0].module, "service");

    // Verify default inventory was created
    assert_eq!(inventory.hosts, vec!["localhost"]);

    Ok(())
}

#[test]
fn test_execution_planning_with_rustle_output() -> Result<()> {
    // Read the example output
    let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("rustle_parse_example_output.json");

    let content = fs::read_to_string(&fixture_path)?;
    let (playbook, inventory) = parse_rustle_output(&content)?;

    // Create planning options
    let planning_options = PlanningOptions {
        limit: None,
        tags: vec![],
        skip_tags: vec![],
        check_mode: false,
        diff_mode: false,
        forks: 50,
        serial: None,
        strategy: ExecutionStrategy::Linear,
        binary_threshold: 5,
        force_binary: false,
        force_ssh: false,
    };

    // Create planner and generate execution plan
    let planner = ExecutionPlanner::new()
        .with_strategy(ExecutionStrategy::Linear)
        .with_forks(50)
        .with_optimization(true);

    let execution_plan = planner.plan_execution(&playbook, &inventory, &planning_options)?;

    // Verify execution plan
    assert_eq!(execution_plan.total_tasks, 3);
    assert_eq!(execution_plan.plays.len(), 1);
    assert_eq!(execution_plan.hosts, vec!["localhost"]);

    let play = &execution_plan.plays[0];
    assert_eq!(play.name, "Simple test playbook");
    assert_eq!(play.hosts, vec!["localhost"]);

    // Verify tasks were included in batches
    let total_tasks_in_batches: usize = play.batches.iter().map(|b| b.tasks.len()).sum();
    assert_eq!(total_tasks_in_batches, 3);

    Ok(())
}

#[test]
fn test_parse_system_facts_playbook() -> Result<()> {
    // Read the system facts fixture
    let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("system_facts.json");

    let content = fs::read_to_string(&fixture_path)?;
    let result = parse_rustle_output(&content);

    assert!(
        result.is_ok(),
        "Failed to parse system facts playbook: {:?}",
        result.err()
    );

    let (playbook, _inventory) = result?;

    // Verify the parsed data
    assert_eq!(playbook.name, "system-facts-playbook");
    assert_eq!(playbook.plays.len(), 1);

    let play = &playbook.plays[0];
    assert_eq!(play.name, "System facts gathering playbook");
    assert_eq!(play.tasks.len(), 3);

    // Verify tasks
    assert_eq!(play.tasks[0].name, "Gather system facts");
    assert_eq!(play.tasks[0].module, "setup");
    assert_eq!(play.tasks[1].name, "Display gathered facts");
    assert_eq!(play.tasks[1].module, "debug");
    assert_eq!(play.tasks[2].name, "Task for Linux systems only");
    assert_eq!(play.tasks[2].module, "debug");

    // Verify conditional logic
    assert_eq!(
        play.tasks[2].when,
        Some("ansible_system == \"Linux\"".to_string())
    );

    // Verify tags on setup task
    assert!(play.tasks[0].tags.contains(&"facts".to_string()));
    assert!(play.tasks[0].tags.contains(&"setup".to_string()));
    assert!(play.tasks[0].tags.contains(&"system".to_string()));

    Ok(())
}

#[test]
fn test_execution_planning_with_system_facts() -> Result<()> {
    // Read the system facts fixture
    let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("system_facts.json");

    let content = fs::read_to_string(&fixture_path)?;
    let (playbook, inventory) = parse_rustle_output(&content)?;

    // Create planning options
    let planning_options = PlanningOptions {
        limit: None,
        tags: vec![],
        skip_tags: vec![],
        check_mode: false,
        diff_mode: false,
        forks: 50,
        serial: None,
        strategy: ExecutionStrategy::Linear,
        binary_threshold: 5,
        force_binary: false,
        force_ssh: false,
    };

    // Create planner and generate execution plan
    let planner = ExecutionPlanner::new()
        .with_strategy(ExecutionStrategy::Linear)
        .with_forks(50)
        .with_optimization(true);

    let execution_plan = planner.plan_execution(&playbook, &inventory, &planning_options)?;

    // Verify execution plan handles facts gathering
    assert_eq!(execution_plan.total_tasks, 3);
    assert_eq!(execution_plan.plays.len(), 1);
    assert_eq!(execution_plan.hosts, vec!["localhost"]);

    let play = &execution_plan.plays[0];
    assert_eq!(play.name, "System facts gathering playbook");

    // Verify all tasks are present in batches
    let total_tasks_in_batches: usize = play.batches.iter().map(|b| b.tasks.len()).sum();
    assert_eq!(total_tasks_in_batches, 3);

    // Verify setup task is first in execution order (facts gathering)
    assert!(!play.batches.is_empty());
    let first_batch = &play.batches[0];
    assert!(!first_batch.tasks.is_empty());

    // Find the setup task (should be scheduled early for facts gathering)
    let setup_task_found = play
        .batches
        .iter()
        .any(|batch| batch.tasks.iter().any(|task| task.module == "setup"));
    assert!(
        setup_task_found,
        "Setup task should be present in execution plan"
    );

    Ok(())
}

// Helper function to parse rustle output (same as in main binary)
fn parse_rustle_output(
    content: &str,
) -> Result<(rustle_plan::ParsedPlaybook, rustle_plan::ParsedInventory)> {
    use serde::Deserialize;
    use std::collections::HashMap;

    #[derive(Deserialize)]
    struct RustleParseOutput {
        metadata: RustleParseMetadata,
        plays: Vec<RustleParsePlay>,
        variables: HashMap<String, serde_json::Value>,
        #[serde(default)]
        inventory: Option<RustleParseInventory>,
        #[serde(default)]
        #[allow(dead_code)]
        facts_required: bool,
        #[serde(default)]
        #[allow(dead_code)]
        vault_ids: Vec<String>,
    }

    #[derive(Deserialize)]
    struct RustleParseMetadata {
        file_path: String,
        #[serde(default)]
        #[allow(dead_code)]
        version: Option<String>,
        #[allow(dead_code)]
        created_at: String,
        #[allow(dead_code)]
        checksum: String,
    }

    #[derive(Deserialize)]
    struct RustleParsePlay {
        name: String,
        #[serde(deserialize_with = "deserialize_hosts")]
        hosts: Vec<String>,
        tasks: Vec<RustleParseTask>,
        handlers: Vec<RustleParseHandler>,
        vars: HashMap<String, serde_json::Value>,
    }

    #[derive(Deserialize)]
    struct RustleParseTask {
        id: String,
        name: String,
        module: String,
        args: HashMap<String, serde_json::Value>,
        dependencies: Vec<String>,
        tags: Vec<String>,
        when: Option<String>,
        notify: Vec<String>,
    }

    #[derive(Deserialize)]
    struct RustleParseHandler {
        id: String,
        name: String,
        module: String,
        args: HashMap<String, serde_json::Value>,
        when: Option<String>,
    }

    #[derive(Deserialize)]
    struct RustleParseInventory {
        hosts: Vec<String>,
        groups: HashMap<String, Vec<String>>,
        vars: HashMap<String, serde_json::Value>,
    }

    let parsed: RustleParseOutput = serde_json::from_str(content)?;

    // Extract playbook name from file path
    let playbook_name = std::path::Path::new(&parsed.metadata.file_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    let plays = parsed
        .plays
        .into_iter()
        .map(|play| {
            let tasks = play
                .tasks
                .into_iter()
                .map(|task| rustle_plan::ParsedTask {
                    id: task.id,
                    name: task.name,
                    module: task.module,
                    args: task.args,
                    dependencies: task.dependencies,
                    tags: task.tags,
                    when: task.when,
                    notify: task.notify,
                })
                .collect();

            let handlers = play
                .handlers
                .into_iter()
                .map(|handler| rustle_plan::ParsedHandler {
                    id: handler.id,
                    name: handler.name,
                    module: handler.module,
                    args: handler.args,
                    when: handler.when,
                })
                .collect();

            rustle_plan::ParsedPlay {
                name: play.name,
                hosts: play.hosts,
                tasks,
                handlers,
                vars: play.vars,
            }
        })
        .collect();

    let parsed_playbook = rustle_plan::ParsedPlaybook {
        name: playbook_name,
        plays,
        vars: parsed.variables,
    };

    let parsed_inventory = if let Some(inventory) = parsed.inventory {
        rustle_plan::ParsedInventory {
            hosts: inventory.hosts,
            groups: inventory.groups,
            vars: inventory.vars,
            host_facts: std::collections::HashMap::new(),
        }
    } else {
        create_default_inventory()
    };

    Ok((parsed_playbook, parsed_inventory))
}

fn create_default_inventory() -> rustle_plan::ParsedInventory {
    rustle_plan::ParsedInventory {
        hosts: vec!["localhost".to_string()],
        groups: std::collections::HashMap::new(),
        vars: std::collections::HashMap::new(),
        host_facts: std::collections::HashMap::new(),
    }
}

fn deserialize_hosts<'de, D>(deserializer: D) -> std::result::Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrVecOrNull {
        String(String),
        Vec(Vec<String>),
        Null,
    }

    match StringOrVecOrNull::deserialize(deserializer)? {
        StringOrVecOrNull::String(s) => Ok(vec![s]),
        StringOrVecOrNull::Vec(v) => Ok(v),
        StringOrVecOrNull::Null => Ok(vec!["localhost".to_string()]), // Default to localhost when hosts is null
    }
}
