mod support;

use serde_json::json;

use support::cli::run_cli_in_dir;
use support::fs::temp_dir;

#[test]
fn clean_uses_config_threshold_and_flag_override() {
    let project_root = temp_dir("clean-threshold-cli");
    write_clean_threshold_fixture(&project_root, 0.98, 0.96, 0.75);

    let dry_run = run_cli_in_dir(&project_root, &["clean"]);
    assert!(dry_run.status.success());
    let dry_run_stdout = String::from_utf8_lossy(&dry_run.stdout);
    assert!(dry_run_stdout.contains("Kratos clean dry run."));
    assert!(dry_run_stdout.contains("Deletion targets: 0"));
    assert!(dry_run_stdout.contains("Threshold-skipped targets: 2"));
    assert!(dry_run_stdout.contains("high-confidence.ts"));
    assert!(dry_run_stdout.contains("mid-confidence.ts"));

    let overridden = run_cli_in_dir(&project_root, &["clean", "--min-confidence", "0.9"]);
    assert!(overridden.status.success());
    let overridden_stdout = String::from_utf8_lossy(&overridden.stdout);
    assert!(overridden_stdout.contains("Deletion targets: 1"));
    assert!(overridden_stdout.contains("Threshold-skipped targets: 1"));
    assert!(overridden_stdout.contains("high-confidence.ts"));
    assert!(overridden_stdout.contains("mid-confidence.ts"));

    let apply = run_cli_in_dir(
        &project_root,
        &["clean", "--apply", "--min-confidence", "0.9"],
    );
    assert!(apply.status.success());
    let apply_stdout = String::from_utf8_lossy(&apply.stdout);
    assert!(apply_stdout.contains("Kratos clean deleted 1 file(s)."));
    assert!(apply_stdout.contains("skipped_files: 1"));
    assert!(!project_root.join("high-confidence.ts").exists());
    assert!(project_root.join("mid-confidence.ts").exists());
}

#[test]
fn clean_rejects_out_of_range_min_confidence_values() {
    let project_root = temp_dir("clean-threshold-cli-invalid");
    write_clean_threshold_fixture(&project_root, 0.50, 0.40, 0.20);

    let output = run_cli_in_dir(&project_root, &["clean", "--min-confidence", "1.5"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("--min-confidence must be between 0.0 and 1.0"));
}

#[test]
fn clean_rejects_invalid_thresholds_config_shape() {
    let project_root = temp_dir("clean-threshold-cli-invalid-config-shape");
    write_clean_threshold_fixture(&project_root, 0.50, 0.40, 0.20);
    std::fs::write(
        project_root.join("kratos.config.json"),
        "{\n  \"thresholds\": []\n}\n",
    )
    .expect("config should write");

    let output = run_cli_in_dir(&project_root, &["clean"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains(
        "thresholds must be an object when specifying thresholds.cleanMinConfidence"
    ));
}

#[test]
fn clean_noop_ignores_invalid_thresholds_config_shape() {
    let project_root = temp_dir("clean-threshold-cli-noop-invalid-config-shape");
    write_clean_threshold_fixture(&project_root, 0.50, 0.40, 0.20);
    std::fs::write(
        project_root.join("kratos.config.json"),
        "{\n  \"thresholds\": []\n}\n",
    )
    .expect("config should write");

    let report_path = project_root.join(".kratos/latest-report.json");
    let mut report: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&report_path).expect("report should read"),
    )
    .expect("report should parse");
    report["summary"]["deletionCandidates"] = json!(0);
    report["findings"]["deletionCandidates"] = json!([]);
    std::fs::write(
        &report_path,
        serde_json::to_string_pretty(&report).expect("report should serialize"),
    )
    .expect("report should write");

    let output = run_cli_in_dir(&project_root, &["clean"]);
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Kratos clean found no deletion candidates."));
}

#[test]
fn clean_apply_false_stays_dry_run() {
    let project_root = temp_dir("clean-threshold-cli-apply-false");
    write_clean_threshold_fixture(&project_root, 0.50, 0.40, 0.20);

    let output = run_cli_in_dir(
        &project_root,
        &["clean", "--apply=false", "--min-confidence", "0.3"],
    );
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Kratos clean dry run."));
    assert!(project_root.join("high-confidence.ts").exists());
    assert!(project_root.join("mid-confidence.ts").exists());
}

#[test]
fn clean_apply_empty_string_stays_dry_run() {
    let project_root = temp_dir("clean-threshold-cli-apply-empty");
    write_clean_threshold_fixture(&project_root, 0.50, 0.40, 0.20);

    let output = run_cli_in_dir(
        &project_root,
        &["clean", "--apply=", "--min-confidence", "0.3"],
    );
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Kratos clean dry run."));
    assert!(project_root.join("high-confidence.ts").exists());
    assert!(project_root.join("mid-confidence.ts").exists());
}

#[test]
fn clean_rejects_invalid_apply_value() {
    let project_root = temp_dir("clean-threshold-cli-apply-invalid");
    write_clean_threshold_fixture(&project_root, 0.50, 0.40, 0.20);

    let output = run_cli_in_dir(
        &project_root,
        &["clean", "--apply=maybe", "--min-confidence", "0.3"],
    );
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("--apply must be a boolean flag or an explicit boolean value"));
    assert!(project_root.join("high-confidence.ts").exists());
    assert!(project_root.join("mid-confidence.ts").exists());
}

#[test]
fn clean_rejects_surplus_positionals() {
    let project_root = temp_dir("clean-threshold-cli-surplus-positionals");
    write_clean_threshold_fixture(&project_root, 0.50, 0.40, 0.20);

    let report_path = project_root.join(".kratos/latest-report.json");
    let output = run_cli_in_dir(
        &project_root,
        &[
            "clean",
            report_path.to_str().expect("path should be utf8"),
            "extra",
        ],
    );
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("clean accepts at most one report-path-or-root argument"));
}

#[test]
fn clean_rejects_missing_threshold_key_when_thresholds_is_present() {
    let project_root = temp_dir("clean-threshold-cli-missing-threshold-key");
    write_clean_threshold_fixture(&project_root, 0.50, 0.40, 0.20);
    std::fs::write(
        project_root.join("kratos.config.json"),
        "{\n  \"thresholds\": {}\n}\n",
    )
    .expect("config should write");

    let output = run_cli_in_dir(&project_root, &["clean"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("thresholds.cleanMinConfidence is required when thresholds is present"));
}

fn write_clean_threshold_fixture(
    project_root: &std::path::Path,
    config_threshold: f32,
    high_confidence: f32,
    mid_confidence: f32,
) {
    std::fs::create_dir_all(project_root.join(".kratos")).expect("report dir should exist");
    std::fs::create_dir_all(project_root.join("src")).expect("source dir should exist");
    std::fs::write(
        project_root.join("kratos.config.json"),
        serde_json::to_string_pretty(&json!({
            "thresholds": {
                "cleanMinConfidence": config_threshold,
            }
        }))
        .expect("config should serialize"),
    )
    .expect("config should write");

    std::fs::write(
        project_root.join("high-confidence.ts"),
        "export const high = true;\n",
    )
    .expect("high file should write");
    std::fs::write(
        project_root.join("mid-confidence.ts"),
        "export const mid = true;\n",
    )
    .expect("mid file should write");

    let report = json!({
        "schemaVersion": 2,
        "generatedAt": "2026-04-21T00:00:00Z",
        "project": {
            "root": project_root,
            "configPath": project_root.join("kratos.config.json"),
        },
        "summary": {
            "filesScanned": 2,
            "entrypoints": 0,
            "brokenImports": 0,
            "orphanFiles": 0,
            "deadExports": 0,
            "unusedImports": 0,
            "routeEntrypoints": 0,
            "deletionCandidates": 2,
        },
        "findings": {
            "brokenImports": [],
            "orphanFiles": [],
            "deadExports": [],
            "unusedImports": [],
            "routeEntrypoints": [],
            "deletionCandidates": [
                {
                    "file": project_root.join("high-confidence.ts"),
                    "reason": "high confidence candidate",
                    "confidence": high_confidence,
                    "safe": true,
                },
                {
                    "file": project_root.join("mid-confidence.ts"),
                    "reason": "mid confidence candidate",
                    "confidence": mid_confidence,
                    "safe": true,
                }
            ],
        },
        "graph": {
            "modules": [],
        },
    });

    std::fs::write(
        project_root.join(".kratos/latest-report.json"),
        serde_json::to_string_pretty(&report).expect("report should serialize"),
    )
    .expect("report should write");
}
