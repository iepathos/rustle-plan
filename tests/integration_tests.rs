use rustle_plan::*;
use std::collections::HashMap;

#[test]
fn test_basic_execution_planning() {
    let planner = ExecutionPlanner::new();

    // Create a simple parsed playbook
    let parsed_playbook = ParsedPlaybook {
        name: "test-playbook".to_string(),
        plays: vec![ParsedPlay {
            name: "Test Play".to_string(),
            hosts: vec!["all".to_string()],
            tasks: vec![
                ParsedTask {
                    id: "task-1".to_string(),
                    name: "Install package".to_string(),
                    module: "package".to_string(),
                    args: {
                        let mut args = HashMap::new();
                        args.insert(
                            "name".to_string(),
                            serde_json::Value::String("nginx".to_string()),
                        );
                        args.insert(
                            "state".to_string(),
                            serde_json::Value::String("present".to_string()),
                        );
                        args
                    },
                    dependencies: vec![],
                    tags: vec!["install".to_string()],
                    when: None,
                    notify: vec!["restart nginx".to_string()],
                },
                ParsedTask {
                    id: "task-2".to_string(),
                    name: "Start service".to_string(),
                    module: "service".to_string(),
                    args: {
                        let mut args = HashMap::new();
                        args.insert(
                            "name".to_string(),
                            serde_json::Value::String("nginx".to_string()),
                        );
                        args.insert(
                            "state".to_string(),
                            serde_json::Value::String("started".to_string()),
                        );
                        args
                    },
                    dependencies: vec!["task-1".to_string()],
                    tags: vec!["service".to_string()],
                    when: None,
                    notify: vec![],
                },
            ],
            handlers: vec![],
            vars: HashMap::new(),
        }],
        vars: HashMap::new(),
    };

    // Create a simple inventory
    let parsed_inventory = ParsedInventory {
        hosts: vec!["server1".to_string(), "server2".to_string()],
        groups: HashMap::new(),
        vars: HashMap::new(),
    };

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

    // Plan execution
    let result = planner.plan_execution(&parsed_playbook, &parsed_inventory, &planning_options);

    if let Err(ref e) = result {
        println!("Planning error: {e:?}");
    }
    assert!(result.is_ok(), "Planning should succeed");

    let execution_plan = result.unwrap();

    // Verify basic properties
    assert_eq!(execution_plan.plays.len(), 1, "Should have one play");
    assert_eq!(execution_plan.total_tasks, 2, "Should have two tasks");
    assert_eq!(execution_plan.hosts.len(), 2, "Should target two hosts");

    let play = &execution_plan.plays[0];
    assert_eq!(play.name, "Test Play");
    assert!(
        !play.batches.is_empty(),
        "Play should have execution batches"
    );
}

#[test]
fn test_binary_deployment_planning() {
    let planner = ExecutionPlanner::new()
        .with_strategy(ExecutionStrategy::BinaryHybrid)
        .with_binary_threshold(1); // Lower threshold for testing

    // Create tasks suitable for binary deployment
    let tasks = vec![
        TaskPlan {
            task_id: "task-1".to_string(),
            name: "Copy file".to_string(),
            module: "copy".to_string(),
            args: HashMap::new(),
            hosts: vec!["server1".to_string(), "server2".to_string()],
            dependencies: vec![],
            conditions: vec![],
            tags: vec![],
            notify: vec![],
            execution_order: 0,
            can_run_parallel: true,
            estimated_duration: Some(std::time::Duration::from_secs(2)),
            risk_level: RiskLevel::Medium,
        },
        TaskPlan {
            task_id: "task-2".to_string(),
            name: "Run shell command".to_string(),
            module: "shell".to_string(),
            args: HashMap::new(),
            hosts: vec!["server1".to_string(), "server2".to_string()],
            dependencies: vec![],
            conditions: vec![],
            tags: vec![],
            notify: vec![],
            execution_order: 1,
            can_run_parallel: true,
            estimated_duration: Some(std::time::Duration::from_secs(3)),
            risk_level: RiskLevel::Medium,
        },
    ];

    let hosts = vec!["server1".to_string(), "server2".to_string()];

    let result = planner.plan_binary_deployments(&tasks, &hosts);
    assert!(result.is_ok(), "Binary deployment planning should succeed");

    let deployments = result.unwrap();
    // With a low threshold, we should get at least one deployment
    // (actual behavior depends on suitability analysis)
    println!("Binary deployments: {}", deployments.len());
}

#[test]
fn test_dependency_analysis() {
    let tasks = vec![
        ParsedTask {
            id: "task-1".to_string(),
            name: "Install package".to_string(),
            module: "package".to_string(),
            args: HashMap::new(),
            dependencies: vec![],
            tags: vec![],
            when: None,
            notify: vec![],
        },
        ParsedTask {
            id: "task-2".to_string(),
            name: "Start service".to_string(),
            module: "service".to_string(),
            args: HashMap::new(),
            dependencies: vec!["task-1".to_string()],
            tags: vec![],
            when: None,
            notify: vec![],
        },
    ];

    let analyzer = DependencyAnalyzer::new();
    let result = analyzer.analyze(&tasks);

    assert!(result.is_ok(), "Dependency analysis should succeed");

    let graph = result.unwrap();
    assert!(
        graph.has_path("task-1", "task-2"),
        "Should have path from task-1 to task-2"
    );
    assert!(
        !graph.has_path("task-2", "task-1"),
        "Should not have path from task-2 to task-1"
    );
}

#[test]
fn test_task_estimation() {
    let estimator = TaskEstimator::new();

    let task = ParsedTask {
        id: "test-task".to_string(),
        name: "Install package".to_string(),
        module: "package".to_string(),
        args: HashMap::new(),
        dependencies: vec![],
        tags: vec![],
        when: None,
        notify: vec![],
    };

    let duration = estimator.estimate_task_duration(&task);
    assert!(duration.is_some(), "Should provide duration estimate");

    let duration = duration.unwrap();
    assert!(duration.as_secs() > 0, "Duration should be positive");

    // Package operations should take longer than simple operations
    assert!(
        duration.as_secs() >= 10,
        "Package operations should take at least 10 seconds"
    );
}
