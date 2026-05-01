mod support;

use std::path::{Path, PathBuf};

use serde_json::Value;
use support::cli::run_cli_in_dir;

#[test]
fn diff_accepts_project_roots_and_report_paths() {
    let before_root = temp_dir("diff-before-root");
    let after_root = temp_dir("diff-after-root");
    let before_report_path = before_root.join(".kratos/latest-report.json");
    let after_report_path = after_root.join("custom-report.json");

    write_report(&before_report_path, &before_root, "./missing-a");
    write_report(&after_report_path, &after_root, "./missing-b");

    let output = run_cli_in_dir(
        &before_root,
        &[
            "diff",
            before_root.to_str().expect("before root should be utf8"),
            after_report_path
                .to_str()
                .expect("after report should be utf8"),
            "--format",
            "md",
        ],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("# Kratos Diff 결과"));
    assert!(stdout.contains(&format!("이전: {}", before_report_path.display())));
    assert!(stdout.contains(&format!("이후: {}", after_report_path.display())));
    assert!(stdout.contains("## 깨진 import"));
    assert!(stdout.contains("### 새로 발생 (1)"));
    assert!(stdout.contains("### 해결됨 (1)"));
    assert!(stdout.contains("./missing-b"));
}

#[test]
fn diff_defaults_to_summary_and_rejects_invalid_format() {
    let before_root = temp_dir("diff-default-before");
    let after_root = temp_dir("diff-default-after");
    let before_report_path = before_root.join(".kratos/latest-report.json");
    let after_report_path = after_root.join(".kratos/latest-report.json");

    write_report(&before_report_path, &before_root, "./missing-a");
    write_report(&after_report_path, &after_root, "./missing-b");

    let summary_output = run_cli_in_dir(
        &before_root,
        &[
            "diff",
            before_root.to_str().expect("before root should be utf8"),
            after_root.to_str().expect("after root should be utf8"),
            "--format",
        ],
    );
    assert!(summary_output.status.success());
    let stdout = String::from_utf8_lossy(&summary_output.stdout);
    assert!(stdout.contains("Kratos diff 완료."));
    assert!(stdout.contains("깨진 import: 새로 발생 1, 해결됨 1, 유지됨 1"));
    assert!(stdout.contains("합계: 새로 발생 1, 해결됨 1, 유지됨 1"));

    let invalid_output = run_cli_in_dir(
        &before_root,
        &[
            "diff",
            before_root.to_str().expect("before root should be utf8"),
            after_report_path
                .to_str()
                .expect("after report should be utf8"),
            "--format",
            "bogus",
        ],
    );
    assert_eq!(invalid_output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&invalid_output.stderr)
        .contains("Kratos failed: Config error: Invalid diff format: bogus"));
}

#[test]
fn diff_json_reports_full_shape() {
    let before_root = temp_dir("diff-json-before");
    let after_root = temp_dir("diff-json-after");
    let before_report_path = before_root.join(".kratos/latest-report.json");
    let after_report_path = after_root.join(".kratos/latest-report.json");

    write_report(&before_report_path, &before_root, "./missing-a");
    write_report(&after_report_path, &after_root, "./missing-b");

    let output = run_cli_in_dir(
        &before_root,
        &[
            "diff",
            before_root.to_str().expect("before root should be utf8"),
            after_root.to_str().expect("after root should be utf8"),
            "--format",
            "json",
        ],
    );

    assert!(output.status.success());
    let value: Value =
        serde_json::from_slice(&output.stdout).expect("diff output should parse as json");
    assert_eq!(
        value["before"]["path"],
        Value::from(before_report_path.to_string_lossy().to_string())
    );
    assert_eq!(
        value["after"]["path"],
        Value::from(after_report_path.to_string_lossy().to_string())
    );
    assert_eq!(
        value["summary"]["brokenImports"]["introduced"],
        Value::from(1)
    );
    assert_eq!(
        value["findings"]["brokenImports"]["introduced"][0]["source"],
        Value::from("./missing-b")
    );
}

fn write_report(path: &Path, root: &Path, broken_source: &str) {
    let report = serde_json::json!({
        "schemaVersion": 2,
        "generatedAt": "2026-04-22T00:00:00Z",
        "project": {
            "root": root.to_string_lossy(),
            "configPath": null,
        },
        "summary": {
            "filesScanned": 1,
            "entrypoints": 0,
            "brokenImports": 1,
            "orphanFiles": 0,
            "deadExports": 0,
            "unusedImports": 0,
            "routeEntrypoints": 0,
            "deletionCandidates": 0,
        },
        "findings": {
            "brokenImports": [{
                "file": root.join("src/shared.ts").to_string_lossy(),
                "source": "./shared",
                "kind": "static",
            }, {
                "file": root.join("src/index.ts").to_string_lossy(),
                "source": broken_source,
                "kind": "static",
            }],
            "orphanFiles": [],
            "deadExports": [],
            "unusedImports": [],
            "routeEntrypoints": [],
            "deletionCandidates": [],
        },
        "graph": {
            "modules": [{
                "file": root.join("src/index.ts").to_string_lossy(),
                "relativePath": "src/index.ts",
                "entrypointKind": null,
                "importedByCount": 0,
                "importCount": 0,
                "exportCount": 0,
            }],
        },
    });

    let body = serde_json::to_string_pretty(&report).expect("report should serialize");
    std::fs::create_dir_all(path.parent().expect("report should have a parent"))
        .expect("report directory should create");
    std::fs::write(path, format!("{body}\n")).expect("report should write");
}

fn temp_dir(label: &str) -> PathBuf {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time should be valid")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("kratos-cli-{label}-{unique}"));
    std::fs::create_dir_all(&path).expect("temp dir should be created");
    path
}
