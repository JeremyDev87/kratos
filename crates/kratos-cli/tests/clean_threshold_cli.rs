mod support;

use kratos_core::clean::{current_file_identity, current_parent_identity};
use serde_json::json;
use sha2::{Digest, Sha256};

use support::cli::run_cli_in_dir;
use support::fs::temp_dir;

#[test]
fn clean_uses_config_threshold_and_flag_override() {
    let project_root = temp_dir("clean-threshold-cli");
    write_clean_threshold_fixture(&project_root, 0.98, 0.96, 0.75);

    let dry_run = run_cli_in_dir(&project_root, &["clean"]);
    assert!(dry_run.status.success());
    let dry_run_stdout = String::from_utf8_lossy(&dry_run.stdout);
    assert!(dry_run_stdout.contains("Kratos clean 미리보기입니다."));
    assert!(dry_run_stdout.contains("삭제 대상: 0"));
    assert!(dry_run_stdout.contains("신뢰도 기준 미달로 건너뛴 대상: 2"));
    assert!(dry_run_stdout.contains("high-confidence.ts"));
    assert!(dry_run_stdout.contains("mid-confidence.ts"));

    let overridden = run_cli_in_dir(&project_root, &["clean", "--min-confidence", "0.9"]);
    assert!(overridden.status.success());
    let overridden_stdout = String::from_utf8_lossy(&overridden.stdout);
    assert!(overridden_stdout.contains("삭제 대상: 1"));
    assert!(overridden_stdout.contains("신뢰도 기준 미달로 건너뛴 대상: 1"));
    assert!(overridden_stdout.contains("high-confidence.ts"));
    assert!(overridden_stdout.contains("신뢰도: 0.96"));
    assert!(overridden_stdout.contains("사유: high confidence candidate"));
    assert!(overridden_stdout.contains("상태: 존재함"));
    assert!(overridden_stdout.contains("미리보기:"));
    assert!(overridden_stdout.contains("export const high = true;"));
    assert!(overridden_stdout.contains("mid-confidence.ts"));

    let apply = run_cli_in_dir(
        &project_root,
        &["clean", "--apply", "--min-confidence", "0.9"],
    );
    assert!(apply.status.success());
    let apply_stdout = String::from_utf8_lossy(&apply.stdout);
    assert!(apply_stdout.contains("Kratos clean: 파일 1개를 삭제했습니다."));
    assert!(apply_stdout.contains("건너뛴 파일: 1"));
    assert!(!project_root.join("high-confidence.ts").exists());
    assert!(project_root.join("mid-confidence.ts").exists());
}

#[test]
fn clean_reports_and_skips_stale_fingerprint_candidates() {
    let project_root = temp_dir("clean-stale-fingerprint-cli");
    write_clean_threshold_fixture(&project_root, 0.98, 0.96, 0.75);
    let high_file = project_root.join("high-confidence.ts");
    std::fs::write(&high_file, "export const changed = true;\n")
        .expect("candidate should change after report generation");

    let dry_run = run_cli_in_dir(&project_root, &["clean", "--min-confidence", "0.9"]);
    assert!(dry_run.status.success());
    let dry_run_stdout = String::from_utf8_lossy(&dry_run.stdout);
    assert!(dry_run_stdout.contains("삭제 대상: 0"));
    assert!(dry_run_stdout.contains("안전 검증으로 건너뛴 대상: 1"));
    assert!(dry_run_stdout.contains("안전 상태: 스캔 후 파일 변경됨"));

    let apply = run_cli_in_dir(
        &project_root,
        &["clean", "--apply", "--min-confidence", "0.9"],
    );
    assert!(apply.status.success());
    let apply_stdout = String::from_utf8_lossy(&apply.stdout);
    assert!(apply_stdout.contains("Kratos clean: 파일 0개를 삭제했습니다."));
    assert!(apply_stdout.contains("건너뛴 파일: 2"));
    assert!(high_file.exists());
}

#[test]
fn clean_dry_run_renders_excerpts_markers_and_separate_skipped_sections() {
    let project_root = temp_dir("clean-threshold-cli-preview");
    write_clean_preview_fixture(&project_root);

    let output = run_cli_in_dir(&project_root, &["clean", "--min-confidence", "0.9"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("삭제 대상: 2"));
    assert!(stdout.contains("- src/live.ts"));
    assert!(stdout.contains("신뢰도: 0.96"));
    assert!(stdout.contains("사유: live candidate"));
    assert!(stdout.contains("상태: 존재함"));
    assert!(stdout.contains("export const live = true;"));
    assert!(stdout.contains("- src/missing.ts"));
    assert!(stdout.contains("안전 검증으로 건너뛴 대상: 1"));
    assert!(stdout.contains("안전 상태: fingerprint 없음"));
    assert!(stdout.contains("상태: 없음"));
    assert!(stdout.contains("[missing file]"));
    assert!(stdout.contains("- src/binary.bin"));
    assert!(stdout.contains("[binary file]"));
    assert!(stdout.contains("신뢰도 기준 미달로 건너뛴 대상: 1"));
    assert!(stdout.contains("src/low-confidence.ts"));
    assert!(stdout.contains("사용할 수 없어 건너뛴 대상: 1"));
    assert!(stdout.contains("outside-candidate.ts"));
}

#[test]
fn clean_rejects_out_of_range_min_confidence_values() {
    let project_root = temp_dir("clean-threshold-cli-invalid");
    write_clean_threshold_fixture(&project_root, 0.50, 0.40, 0.20);

    let output = run_cli_in_dir(&project_root, &["clean", "--min-confidence", "1.5"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("--min-confidence는 0.0 이상 1.0 이하이어야 합니다"));
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
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("thresholds must be an object when specifying thresholds.cleanMinConfidence"));
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
    let mut report: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&report_path).expect("report should read"))
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
    assert!(String::from_utf8_lossy(&output.stdout).contains("Kratos clean: 삭제 후보가 없습니다."));
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
    assert!(stdout.contains("Kratos clean 미리보기입니다."));
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
    assert!(stdout.contains("Kratos clean 미리보기입니다."));
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
        .contains("--apply는 boolean flag이거나 명시적인 boolean 값이어야 합니다"));
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
        .contains("clean은 report-path-or-root 인자를 최대 하나만 받을 수 있습니다"));
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

    let high_file = project_root.join("high-confidence.ts");
    let mid_file = project_root.join("mid-confidence.ts");
    std::fs::write(&high_file, "export const high = true;\n").expect("high file should write");
    std::fs::write(&mid_file, "export const mid = true;\n").expect("mid file should write");

    let report = json!({
        "schemaVersion": 3,
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
                    "file": high_file,
                    "reason": "high confidence candidate",
                    "confidence": high_confidence,
                    "safe": true,
                },
                {
                    "file": mid_file,
                    "reason": "mid confidence candidate",
                    "confidence": mid_confidence,
                    "safe": true,
                }
            ],
        },
        "cleanSafety": {
            "fingerprintAlgorithm": "sha256",
            "candidates": [
                {
                    "file": high_file,
                    "fingerprint": content_fingerprint(&high_file),
                    "identity": current_file_identity(&high_file),
                    "parentIdentity": current_parent_identity(&high_file),
                },
                {
                    "file": mid_file,
                    "fingerprint": content_fingerprint(&mid_file),
                    "identity": current_file_identity(&mid_file),
                    "parentIdentity": current_parent_identity(&mid_file),
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

fn content_fingerprint(path: &std::path::Path) -> String {
    let bytes = std::fs::read(path).expect("fingerprinted file should read");
    format!("{:x}", Sha256::digest(bytes))
}

fn write_clean_preview_fixture(project_root: &std::path::Path) {
    std::fs::create_dir_all(project_root.join(".kratos")).expect("report dir should exist");
    std::fs::create_dir_all(project_root.join("src")).expect("source dir should exist");

    std::fs::write(
        project_root.join("src/live.ts"),
        "export const live = true;\n",
    )
    .expect("live file should write");
    std::fs::write(project_root.join("src/binary.bin"), [0, 159, 146, 150])
        .expect("binary file should write");

    let outside_candidate = project_root.with_file_name(format!(
        "{}-outside-candidate.ts",
        project_root
            .file_name()
            .expect("project root should have file name")
            .to_string_lossy()
    ));

    let report = json!({
        "schemaVersion": 3,
        "generatedAt": "2026-04-21T00:00:00Z",
        "project": {
            "root": project_root,
            "configPath": null,
        },
        "summary": {
            "filesScanned": 4,
            "entrypoints": 0,
            "brokenImports": 0,
            "orphanFiles": 0,
            "deadExports": 0,
            "unusedImports": 0,
            "routeEntrypoints": 0,
            "deletionCandidates": 5,
        },
        "findings": {
            "brokenImports": [],
            "orphanFiles": [],
            "deadExports": [],
            "unusedImports": [],
            "routeEntrypoints": [],
            "deletionCandidates": [
                {
                    "file": project_root.join("src/live.ts"),
                    "reason": "live candidate",
                    "confidence": 0.96,
                    "safe": true,
                },
                {
                    "file": project_root.join("src/missing.ts"),
                    "reason": "missing candidate",
                    "confidence": 0.95,
                    "safe": true,
                },
                {
                    "file": project_root.join("src/binary.bin"),
                    "reason": "binary candidate",
                    "confidence": 0.94,
                    "safe": true,
                },
                {
                    "file": project_root.join("src/low-confidence.ts"),
                    "reason": "low candidate",
                    "confidence": 0.20,
                    "safe": true,
                },
                {
                    "file": outside_candidate,
                    "reason": "outside candidate",
                    "confidence": 0.93,
                    "safe": true,
                }
            ],
        },
        "cleanSafety": {
            "fingerprintAlgorithm": "sha256",
            "candidates": [
                {
                    "file": project_root.join("src/live.ts"),
                    "fingerprint": content_fingerprint(&project_root.join("src/live.ts")),
                    "identity": current_file_identity(&project_root.join("src/live.ts")),
                    "parentIdentity": current_parent_identity(&project_root.join("src/live.ts")),
                },
                {
                    "file": project_root.join("src/missing.ts"),
                    "fingerprint": null,
                    "identity": null,
                    "parentIdentity": null,
                },
                {
                    "file": project_root.join("src/binary.bin"),
                    "fingerprint": content_fingerprint(&project_root.join("src/binary.bin")),
                    "identity": current_file_identity(&project_root.join("src/binary.bin")),
                    "parentIdentity": current_parent_identity(&project_root.join("src/binary.bin")),
                },
                {
                    "file": project_root.join("src/low-confidence.ts"),
                    "fingerprint": null,
                    "identity": null,
                    "parentIdentity": null,
                },
                {
                    "file": outside_candidate,
                    "fingerprint": null,
                    "identity": null,
                    "parentIdentity": null,
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
