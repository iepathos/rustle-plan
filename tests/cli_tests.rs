use anyhow::Result;
use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use std::fs;
use tempfile::TempDir;

fn create_test_rustle_output() -> String {
    r#"{
        "metadata": {
            "file_path": "/tmp/test.yml",
            "created_at": "2024-01-01T00:00:00Z",
            "checksum": "abc123"
        },
        "plays": [
            {
                "name": "Test Play",
                "hosts": ["host1", "host2"],
                "tasks": [
                    {
                        "id": "task1",
                        "name": "Test task",
                        "module": "shell",
                        "args": {
                            "cmd": "echo hello"
                        },
                        "dependencies": [],
                        "tags": ["test"],
                        "when": null,
                        "notify": []
                    }
                ],
                "handlers": [],
                "vars": {}
            }
        ],
        "variables": {},
        "inventory": {
            "hosts": ["host1", "host2"],
            "groups": {},
            "vars": {}
        }
    }"#
    .to_string()
}

#[test]
fn test_cli_help() {
    let mut cmd = Command::cargo_bin("rustle-plan").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Generate optimized execution plans",
        ));
}

#[test]
fn test_cli_version() {
    let mut cmd = Command::cargo_bin("rustle-plan").unwrap();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn test_basic_execution_from_stdin() {
    let mut cmd = Command::cargo_bin("rustle-plan").unwrap();
    cmd.write_stdin(create_test_rustle_output())
        .assert()
        .success();
}

#[test]
fn test_execution_from_file() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let input_file = temp_dir.path().join("input.json");
    fs::write(&input_file, create_test_rustle_output())?;

    let mut cmd = Command::cargo_bin("rustle-plan").unwrap();
    cmd.arg(input_file.to_str().unwrap()).assert().success();

    Ok(())
}

#[test]
fn test_stdin_with_dash() {
    let mut cmd = Command::cargo_bin("rustle-plan").unwrap();
    cmd.arg("-")
        .write_stdin(create_test_rustle_output())
        .assert()
        .success();
}

#[test]
fn test_list_tasks() {
    let mut cmd = Command::cargo_bin("rustle-plan").unwrap();
    cmd.arg("--list-tasks")
        .write_stdin(create_test_rustle_output())
        .assert()
        .success()
        .stdout(predicate::str::contains("Planned tasks:"))
        .stdout(predicate::str::contains("Test task"));
}

#[test]
fn test_list_hosts() {
    let mut cmd = Command::cargo_bin("rustle-plan").unwrap();
    cmd.arg("--list-hosts")
        .write_stdin(create_test_rustle_output())
        .assert()
        .success()
        .stdout(predicate::str::contains("Target hosts:"))
        .stdout(predicate::str::contains("host1"))
        .stdout(predicate::str::contains("host2"));
}

#[test]
fn test_list_binaries() {
    let mut cmd = Command::cargo_bin("rustle-plan").unwrap();
    cmd.arg("--list-binaries")
        .arg("--force-binary")
        .write_stdin(create_test_rustle_output())
        .assert()
        .success()
        .stdout(predicate::str::contains("Binary deployments:"));
}

#[test]
fn test_dry_run() {
    let mut cmd = Command::cargo_bin("rustle-plan").unwrap();
    cmd.arg("--dry-run")
        .write_stdin(create_test_rustle_output())
        .assert()
        .success();
}

#[test]
fn test_dry_run_with_time_estimates() {
    let mut cmd = Command::cargo_bin("rustle-plan").unwrap();
    cmd.arg("--dry-run")
        .arg("--estimate-time")
        .write_stdin(create_test_rustle_output())
        .assert()
        .success();
}

#[test]
fn test_json_output_format() {
    let mut cmd = Command::cargo_bin("rustle-plan").unwrap();
    cmd.arg("--output")
        .arg("json")
        .write_stdin(create_test_rustle_output())
        .assert()
        .success()
        .stdout(predicate::str::is_match(r#"\{[\s\S]*\}"#).unwrap());
}

#[test]
fn test_binary_output_format() {
    let mut cmd = Command::cargo_bin("rustle-plan").unwrap();
    cmd.arg("--output")
        .arg("binary")
        .write_stdin(create_test_rustle_output())
        .assert()
        .success();
}

#[test]
fn test_dot_output_requires_visualize() {
    let mut cmd = Command::cargo_bin("rustle-plan").unwrap();
    cmd.arg("--output")
        .arg("dot")
        .write_stdin(create_test_rustle_output())
        .assert()
        .failure()
        .stdout(predicate::str::contains("DOT output requires --visualize"));
}

#[test]
fn test_dot_visualization() {
    let mut cmd = Command::cargo_bin("rustle-plan").unwrap();
    cmd.arg("--output")
        .arg("dot")
        .arg("--visualize")
        .write_stdin(create_test_rustle_output())
        .assert()
        .success()
        .stdout(predicate::str::contains("digraph execution_plan"));
}

#[test]
fn test_limit_hosts() {
    let mut cmd = Command::cargo_bin("rustle-plan").unwrap();
    cmd.arg("--limit")
        .arg("host1")
        .write_stdin(create_test_rustle_output())
        .assert()
        .success();
}

#[test]
fn test_tags_filter() {
    let mut cmd = Command::cargo_bin("rustle-plan").unwrap();
    cmd.arg("--tags")
        .arg("test")
        .write_stdin(create_test_rustle_output())
        .assert()
        .success();
}

#[test]
fn test_skip_tags() {
    let mut cmd = Command::cargo_bin("rustle-plan").unwrap();
    cmd.arg("--skip-tags")
        .arg("skip")
        .write_stdin(create_test_rustle_output())
        .assert()
        .success();
}

#[test]
fn test_check_mode() {
    let mut cmd = Command::cargo_bin("rustle-plan").unwrap();
    cmd.arg("--check")
        .write_stdin(create_test_rustle_output())
        .assert()
        .success();
}

#[test]
fn test_diff_mode() {
    let mut cmd = Command::cargo_bin("rustle-plan").unwrap();
    cmd.arg("--diff")
        .write_stdin(create_test_rustle_output())
        .assert()
        .success();
}

#[test]
fn test_forks_option() {
    let mut cmd = Command::cargo_bin("rustle-plan").unwrap();
    cmd.arg("--forks")
        .arg("10")
        .write_stdin(create_test_rustle_output())
        .assert()
        .success();
}

#[test]
fn test_serial_option() {
    let mut cmd = Command::cargo_bin("rustle-plan").unwrap();
    cmd.arg("--serial")
        .arg("2")
        .write_stdin(create_test_rustle_output())
        .assert()
        .success();
}

#[test]
fn test_all_strategies() {
    let strategies = [
        "linear",
        "rolling",
        "free",
        "host-pinned",
        "binary-hybrid",
        "binary-only",
    ];

    for strategy in &strategies {
        let mut cmd = Command::cargo_bin("rustle-plan").unwrap();
        cmd.arg("--strategy")
            .arg(strategy)
            .write_stdin(create_test_rustle_output())
            .assert()
            .success();
    }
}

#[test]
fn test_binary_threshold() {
    let mut cmd = Command::cargo_bin("rustle-plan").unwrap();
    cmd.arg("--binary-threshold")
        .arg("10")
        .write_stdin(create_test_rustle_output())
        .assert()
        .success();
}

#[test]
fn test_force_binary() {
    let mut cmd = Command::cargo_bin("rustle-plan").unwrap();
    cmd.arg("--force-binary")
        .write_stdin(create_test_rustle_output())
        .assert()
        .success();
}

#[test]
fn test_force_ssh() {
    let mut cmd = Command::cargo_bin("rustle-plan").unwrap();
    cmd.arg("--force-ssh")
        .write_stdin(create_test_rustle_output())
        .assert()
        .success();
}

#[test]
fn test_optimize_flag() {
    let mut cmd = Command::cargo_bin("rustle-plan").unwrap();
    cmd.arg("--optimize")
        .write_stdin(create_test_rustle_output())
        .assert()
        .success();
}

#[test]
fn test_verbose_output() {
    let mut cmd = Command::cargo_bin("rustle-plan").unwrap();
    cmd.arg("--verbose")
        .arg("--dry-run")
        .write_stdin(create_test_rustle_output())
        .assert()
        .success();
}

#[test]
fn test_invalid_input() {
    let mut cmd = Command::cargo_bin("rustle-plan").unwrap();
    cmd.write_stdin("invalid json")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Failed to parse JSON"));
}

#[test]
fn test_missing_file() {
    let mut cmd = Command::cargo_bin("rustle-plan").unwrap();
    cmd.arg("/nonexistent/file.json")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Failed to read playbook file"));
}

#[test]
fn test_output_is_valid_json() {
    let mut cmd = Command::cargo_bin("rustle-plan").unwrap();
    let output = cmd
        .write_stdin(create_test_rustle_output())
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let _: Value = serde_json::from_str(&stdout).expect("Output should be valid JSON");
}

#[test]
fn test_hosts_as_string_in_play() {
    let input = r#"{
        "metadata": {
            "file_path": "/tmp/test.yml",
            "created_at": "2024-01-01T00:00:00Z",
            "checksum": "abc123"
        },
        "plays": [
            {
                "name": "Test Play",
                "hosts": "all",
                "tasks": [
                    {
                        "id": "task1",
                        "name": "Test task",
                        "module": "shell",
                        "args": {"cmd": "echo hello"},
                        "dependencies": [],
                        "tags": [],
                        "when": null,
                        "notify": []
                    }
                ],
                "handlers": [],
                "vars": {}
            }
        ],
        "variables": {},
        "inventory": {
            "hosts": ["host1"],
            "groups": {},
            "vars": {}
        }
    }"#;

    let mut cmd = Command::cargo_bin("rustle-plan").unwrap();
    cmd.write_stdin(input).assert().success();
}

#[test]
fn test_no_inventory_uses_default() {
    let input = r#"{
        "metadata": {
            "file_path": "/tmp/test.yml",
            "created_at": "2024-01-01T00:00:00Z",
            "checksum": "abc123"
        },
        "plays": [
            {
                "name": "Test Play",
                "hosts": ["localhost"],
                "tasks": [
                    {
                        "id": "task1",
                        "name": "Test task",
                        "module": "debug",
                        "args": {"msg": "hello"},
                        "dependencies": [],
                        "tags": [],
                        "when": null,
                        "notify": []
                    }
                ],
                "handlers": [],
                "vars": {}
            }
        ],
        "variables": {}
    }"#;

    let mut cmd = Command::cargo_bin("rustle-plan").unwrap();
    cmd.write_stdin(input).assert().success();
}

#[test]
fn test_combined_options() {
    let mut cmd = Command::cargo_bin("rustle-plan").unwrap();
    cmd.arg("--strategy")
        .arg("rolling")
        .arg("--serial")
        .arg("3")
        .arg("--check")
        .arg("--optimize")
        .arg("--tags")
        .arg("test")
        .arg("--forks")
        .arg("20")
        .write_stdin(create_test_rustle_output())
        .assert()
        .success();
}

#[test]
fn test_estimate_time_in_json_output() {
    let mut cmd = Command::cargo_bin("rustle-plan").unwrap();
    cmd.arg("--estimate-time")
        .write_stdin(create_test_rustle_output())
        .assert()
        .success();
}
