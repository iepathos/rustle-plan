use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use rustle_plan::{ExecutionPlanner, ExecutionStrategy, PlanningOptions};
use std::io::{self, Read};
use std::path::PathBuf;
use tracing::{error, info};

#[derive(Parser)]
#[command(
    name = "rustle-plan",
    about = "Generate optimized execution plans from parsed playbooks",
    long_about = "The rustle-plan tool takes parsed playbooks and generates optimized execution plans with binary deployment strategies. It analyzes task dependencies, determines parallelization opportunities, and produces detailed execution graphs."
)]
struct Cli {
    /// Path to parsed playbook file (or stdin if -)
    #[arg(value_name = "PARSED_PLAYBOOK")]
    playbook: Option<PathBuf>,

    /// Limit execution to specific hosts
    #[arg(short, long, value_name = "PATTERN")]
    limit: Option<String>,

    /// Only run tasks with these tags
    #[arg(short, long, value_name = "TAGS")]
    tags: Vec<String>,

    /// Skip tasks with these tags
    #[arg(long, value_name = "TAGS")]
    skip_tags: Vec<String>,

    /// Execution strategy
    #[arg(short, long, value_enum, default_value = "binary-hybrid")]
    strategy: StrategyArg,

    /// Number of hosts to run at once
    #[arg(long, value_name = "NUM")]
    serial: Option<u32>,

    /// Maximum parallel processes
    #[arg(long, default_value = "50")]
    forks: u32,

    /// Check mode (don't make changes)
    #[arg(short, long)]
    check: bool,

    /// Show file differences
    #[arg(long)]
    diff: bool,

    /// Minimum tasks to justify binary compilation
    #[arg(long, default_value = "5")]
    binary_threshold: u32,

    /// Force binary deployment for all suitable tasks
    #[arg(long)]
    force_binary: bool,

    /// Force SSH execution (disable binary deployment)
    #[arg(long)]
    force_ssh: bool,

    /// List all planned tasks
    #[arg(long)]
    list_tasks: bool,

    /// List all target hosts
    #[arg(long)]
    list_hosts: bool,

    /// List planned binary deployments
    #[arg(long)]
    list_binaries: bool,

    /// Generate execution graph visualization
    #[arg(long)]
    visualize: bool,

    /// Output format
    #[arg(short, long, value_enum, default_value = "json")]
    output: OutputFormat,

    /// Enable execution optimizations
    #[arg(long)]
    optimize: bool,

    /// Include execution time estimates
    #[arg(long)]
    estimate_time: bool,

    /// Plan but don't output execution plan
    #[arg(long)]
    dry_run: bool,

    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,
}

#[derive(ValueEnum, Clone)]
enum StrategyArg {
    Linear,
    Rolling,
    Free,
    HostPinned,
    BinaryHybrid,
    BinaryOnly,
}

impl From<StrategyArg> for ExecutionStrategy {
    fn from(strategy: StrategyArg) -> Self {
        match strategy {
            StrategyArg::Linear => ExecutionStrategy::Linear,
            StrategyArg::Rolling => ExecutionStrategy::Rolling { batch_size: 5 },
            StrategyArg::Free => ExecutionStrategy::Free,
            StrategyArg::HostPinned => ExecutionStrategy::HostPinned,
            StrategyArg::BinaryHybrid => ExecutionStrategy::BinaryHybrid,
            StrategyArg::BinaryOnly => ExecutionStrategy::BinaryOnly,
        }
    }
}

#[derive(ValueEnum, Clone)]
enum OutputFormat {
    Json,
    Binary,
    Dot,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize tracing
    let level = if cli.verbose {
        tracing::Level::DEBUG
    } else {
        tracing::Level::INFO
    };

    tracing_subscriber::fmt().with_max_level(level).init();

    // Read playbook input
    let playbook_content = if let Some(ref path) = cli.playbook {
        if path.as_os_str() == "-" {
            let mut content = String::new();
            io::stdin()
                .read_to_string(&mut content)
                .context("Failed to read playbook from stdin")?;
            content
        } else {
            std::fs::read_to_string(path)
                .with_context(|| format!("Failed to read playbook file: {}", path.display()))?
        }
    } else {
        let mut content = String::new();
        io::stdin()
            .read_to_string(&mut content)
            .context("Failed to read playbook from stdin")?;
        content
    };

    // Parse the combined output from rustle-parse (includes both playbook and inventory)
    let (parsed_playbook, parsed_inventory) = parse_rustle_output(&playbook_content)?;

    // Create planning options
    let planning_options = PlanningOptions {
        limit: cli.limit,
        tags: cli.tags,
        skip_tags: cli.skip_tags,
        check_mode: cli.check,
        diff_mode: cli.diff,
        forks: cli.forks,
        serial: cli.serial,
        strategy: cli.strategy.into(),
        binary_threshold: cli.binary_threshold,
        force_binary: cli.force_binary,
        force_ssh: cli.force_ssh,
    };

    // Create execution planner
    let planner = ExecutionPlanner::new()
        .with_strategy(planning_options.strategy.clone())
        .with_forks(cli.forks)
        .with_optimization(cli.optimize)
        .with_check_mode(cli.check)
        .with_binary_threshold(cli.binary_threshold);

    info!("Planning execution for playbook");

    // Generate execution plan
    let execution_plan = planner
        .plan_execution(&parsed_playbook, &parsed_inventory, &planning_options)
        .context("Failed to generate execution plan")?;

    // Handle different output modes
    if cli.list_tasks {
        list_tasks(&execution_plan);
        return Ok(());
    }

    if cli.list_hosts {
        list_hosts(&execution_plan);
        return Ok(());
    }

    if cli.list_binaries {
        list_binary_deployments(&execution_plan);
        return Ok(());
    }

    if cli.dry_run {
        info!("Dry run completed successfully");
        if cli.estimate_time {
            if let Some(duration) = execution_plan.estimated_duration {
                println!("Estimated execution time: {duration:?}");
            }
            if let Some(compilation_time) = execution_plan.estimated_compilation_time {
                println!("Estimated compilation time: {compilation_time:?}");
            }
        }
        return Ok(());
    }

    // Output execution plan
    match cli.output {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&execution_plan)
                .context("Failed to serialize execution plan to JSON")?;
            println!("{json}");
        }
        OutputFormat::Binary => {
            // For binary output, we could use a more compact serialization format
            let binary = serde_json::to_vec(&execution_plan)
                .context("Failed to serialize execution plan to binary")?;
            io::stdout()
                .write_all(&binary)
                .context("Failed to write binary output")?;
        }
        OutputFormat::Dot => {
            if cli.visualize {
                generate_dot_visualization(&execution_plan)?;
            } else {
                error!("DOT output requires --visualize flag");
                std::process::exit(1);
            }
        }
    }

    Ok(())
}

fn deserialize_hosts<'de, D>(deserializer: D) -> std::result::Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;
    
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrVec {
        String(String),
        Vec(Vec<String>),
    }
    
    match StringOrVec::deserialize(deserializer)? {
        StringOrVec::String(s) => Ok(vec![s]),
        StringOrVec::Vec(v) => Ok(v),
    }
}

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

    let parsed: RustleParseOutput =
        serde_json::from_str(content).context("Failed to parse JSON from rustle-parse")?;

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
    }
}

fn list_tasks(plan: &rustle_plan::ExecutionPlan) {
    println!("Planned tasks:");
    for (play_idx, play) in plan.plays.iter().enumerate() {
        println!("  Play {}: {}", play_idx + 1, play.name);
        for batch in &play.batches {
            for task in &batch.tasks {
                println!("    - {} ({})", task.name, task.task_id);
            }
        }
    }
}

fn list_hosts(plan: &rustle_plan::ExecutionPlan) {
    println!("Target hosts:");
    for host in &plan.hosts {
        println!("  - {host}");
    }
}

fn list_binary_deployments(plan: &rustle_plan::ExecutionPlan) {
    println!("Binary deployments:");
    for deployment in &plan.binary_deployments {
        println!(
            "  - {} ({})",
            deployment.binary_name, deployment.deployment_id
        );
        println!("    Hosts: {}", deployment.target_hosts.join(", "));
        println!("    Tasks: {}", deployment.tasks.len());
        println!("    Estimated size: {} bytes", deployment.estimated_size);
    }
}

fn generate_dot_visualization(plan: &rustle_plan::ExecutionPlan) -> Result<()> {
    println!("digraph execution_plan {{");
    println!("  rankdir=TB;");
    println!("  node [shape=box];");

    for (play_idx, play) in plan.plays.iter().enumerate() {
        println!("  subgraph cluster_{play_idx} {{");
        println!("    label=\"{}\";", play.name);

        for batch in &play.batches {
            for task in &batch.tasks {
                println!("    \"{}\" [label=\"{}\"];", task.task_id, task.name);

                for dep in &task.dependencies {
                    println!("    \"{}\" -> \"{}\";", dep, task.task_id);
                }
            }
        }

        println!("  }}");
    }

    println!("}}");
    Ok(())
}

use std::io::Write;
