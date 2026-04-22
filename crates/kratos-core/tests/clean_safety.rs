use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use kratos_core::clean::{clean_from_report, clean_from_report_path};
use kratos_core::error::KratosError;
use kratos_core::model::{DeletionCandidateFinding, ReportV2};
use kratos_core::report::serialize_report_pretty;

#[test]
fn clean_rejects_deletion_candidates_outside_report_root() {
    let temp_root = temp_dir("clean-outside-root");
    let report_root = temp_root.join("app");
    let outside_root = temp_root.join("application");
    let outside_file = outside_root.join("should-not-delete.ts");

    std::fs::create_dir_all(&report_root).expect("report root should exist");
    std::fs::create_dir_all(&outside_root).expect("outside root should exist");
    std::fs::write(&outside_file, "export const keep = true;\n").expect("outside file writes");

    let report = report_with_candidate(&report_root, &outside_file);
    let outcome = clean_from_report(&report, true).expect("clean should succeed");

    assert!(outside_file.exists());
    assert_eq!(outcome.deleted_files, 0);
    assert_eq!(outcome.skipped_files, 1);
}

#[test]
fn clean_rejects_symlink_escape_candidates() {
    let temp_root = temp_dir("clean-symlink-escape");
    let report_root = temp_root.join("app");
    let outside_root = temp_root.join("outside");
    let outside_file = outside_root.join("target.ts");
    let symlink_path = report_root.join("link");

    std::fs::create_dir_all(&report_root).expect("report root should exist");
    std::fs::create_dir_all(&outside_root).expect("outside root should exist");
    std::fs::write(&outside_file, "export const keep = true;\n").expect("outside file writes");
    symlink_dir(&outside_root, &symlink_path);

    let report = report_with_candidate(&report_root, &symlink_path.join("target.ts"));
    let outcome = clean_from_report(&report, true).expect("clean should succeed");

    assert!(outside_file.exists());
    assert_eq!(outcome.deleted_files, 0);
    assert_eq!(outcome.skipped_files, 1);
}

#[test]
fn clean_deletes_dangling_symlink_candidates() {
    let temp_root = temp_dir("clean-dangling-symlink");
    let report_root = temp_root.join("app");
    let dangling_link = report_root.join("dangling.ts");

    std::fs::create_dir_all(&report_root).expect("report root should exist");
    symlink_file(Path::new("missing-target.ts"), &dangling_link);

    let report = report_with_candidate(&report_root, &dangling_link);
    let outcome = clean_from_report(&report, true).expect("clean should delete dangling symlinks");

    assert!(
        std::fs::symlink_metadata(&dangling_link).is_err(),
        "dangling symlink should be deleted"
    );
    assert_eq!(outcome.deleted_files, 1);
    assert_eq!(outcome.skipped_files, 0);
}

#[test]
fn clean_deletes_live_symlink_candidates_without_touching_targets() {
    let temp_root = temp_dir("clean-live-symlink");
    let report_root = temp_root.join("app");
    let outside_root = temp_root.join("outside");
    let outside_file = outside_root.join("target.ts");
    let symlink_path = report_root.join("linked.ts");

    std::fs::create_dir_all(&report_root).expect("report root should exist");
    std::fs::create_dir_all(&outside_root).expect("outside root should exist");
    std::fs::write(&outside_file, "export const keep = true;\n").expect("outside file writes");
    symlink_file(&outside_file, &symlink_path);

    let report = report_with_candidate(&report_root, &symlink_path);
    let outcome = clean_from_report(&report, true).expect("clean should delete symlink entries");

    assert!(
        std::fs::symlink_metadata(&symlink_path).is_err(),
        "symlink entry should be deleted"
    );
    assert!(outside_file.exists(), "target file should remain untouched");
    assert_eq!(outcome.deleted_files, 1);
    assert_eq!(outcome.skipped_files, 0);
}

#[test]
fn clean_allows_symlinked_project_root_and_removes_empty_directories() {
    let temp_root = temp_dir("clean-symlink-root");
    let real_root = temp_root.join("real-app");
    let symlink_root = temp_root.join("linked-app");
    let nested_dir = real_root.join("orphan");
    let dead_file = nested_dir.join("dead.ts");

    std::fs::create_dir_all(&nested_dir).expect("nested dir should exist");
    std::fs::write(&dead_file, "export const dead = true;\n").expect("dead file writes");
    symlink_dir(&real_root, &symlink_root);

    let report = report_with_candidate(&symlink_root, &symlink_root.join("orphan/dead.ts"));
    let outcome = clean_from_report(&report, true).expect("clean should succeed");

    assert!(!dead_file.exists());
    assert!(!nested_dir.exists());
    assert_eq!(outcome.deleted_files, 1);
    assert_eq!(outcome.skipped_files, 0);
}

#[test]
fn clean_ignores_cleanup_failures_after_successful_delete() {
    let temp_root = temp_dir("clean-best-effort-cleanup");
    let report_root = temp_root.join("app");
    let real_nested_dir = report_root.join("real-nested");
    let symlink_nested_dir = report_root.join("symlink-nested");
    let dead_file = real_nested_dir.join("dead.ts");

    std::fs::create_dir_all(&real_nested_dir).expect("real nested dir should exist");
    std::fs::write(&dead_file, "export const dead = true;\n").expect("dead file writes");
    symlink_dir(&real_nested_dir, &symlink_nested_dir);

    let report = report_with_candidate(&report_root, &symlink_nested_dir.join("dead.ts"));
    let outcome = clean_from_report(&report, true).expect("clean should stay best-effort");

    assert!(!dead_file.exists());
    assert_eq!(outcome.deleted_files, 1);
    assert_eq!(outcome.skipped_files, 0);
}

#[test]
fn clean_from_report_path_accepts_future_schema_reports_when_shape_is_compatible() {
    let temp_root = temp_dir("clean-future-schema");
    let report_root = temp_root.join("app");
    let dead_file = report_root.join("dead.ts");
    let report_path = report_root.join(".kratos/latest-report.json");

    std::fs::create_dir_all(report_path.parent().expect("report dir should exist"))
        .expect("report dir should exist");
    std::fs::write(&dead_file, "export const dead = true;\n").expect("dead file writes");
    std::fs::write(
        &report_path,
        format!(
            "{{\"schemaVersion\":3,\"generatedAt\":\"2026-04-21T00:00:00Z\",\"project\":{{\"root\":\"{}\",\"configPath\":null}},\"summary\":{{\"filesScanned\":1,\"entrypoints\":0,\"brokenImports\":0,\"orphanFiles\":0,\"deadExports\":0,\"unusedImports\":0,\"routeEntrypoints\":0,\"deletionCandidates\":1}},\"findings\":{{\"brokenImports\":[],\"orphanFiles\":[],\"deadExports\":[],\"unusedImports\":[],\"routeEntrypoints\":[],\"deletionCandidates\":[{{\"file\":\"{}\",\"reason\":\"test\",\"confidence\":1.0,\"safe\":true}}]}},\"graph\":{{\"modules\":[]}}}}",
            report_root.display(),
            dead_file.display(),
        ),
    )
    .expect("report writes");

    let outcome =
        clean_from_report_path(&report_path, true).expect("future-schema clean should work");

    assert!(!dead_file.exists());
    assert_eq!(outcome.deleted_files, 1);
    assert_eq!(outcome.skipped_files, 0);
}

#[test]
fn clean_from_report_path_rejects_legacy_v1_reports() {
    let temp_root = temp_dir("clean-invalid-version");
    let report_path = temp_root.join("latest-report.json");

    std::fs::create_dir_all(&temp_root).expect("temp root should exist");
    std::fs::write(
        &report_path,
        format!(
            "{{\"version\":1,\"root\":\"{}\",\"findings\":{{\"deletionCandidates\":[]}}}}",
            temp_root.display()
        ),
    )
    .expect("report writes");

    let error =
        clean_from_report_path(&report_path, true).expect_err("v1 reports should be rejected");

    match error {
        KratosError::InvalidReportVersion { expected, found } => {
            assert_eq!(expected, 2);
            assert_eq!(found, 1);
        }
        other => panic!("expected invalid report version error, got {other}"),
    }
}

#[test]
fn clean_from_report_rejects_reports_older_than_v2() {
    let temp_root = temp_dir("clean-report-version-floor");
    let report_root = temp_root.join("app");
    let dead_file = report_root.join("dead.ts");

    std::fs::create_dir_all(&report_root).expect("report root should exist");
    std::fs::write(&dead_file, "export const dead = true;\n").expect("dead file writes");

    let mut report = report_with_candidate(&report_root, &dead_file);
    report.version = 1;

    let error = clean_from_report(&report, true).expect_err("older reports should be rejected");

    match error {
        KratosError::InvalidReportVersion { expected, found } => {
            assert_eq!(expected, 2);
            assert_eq!(found, 1);
        }
        other => panic!("expected invalid report version error, got {other}"),
    }
}

#[test]
fn clean_from_report_path_reads_v2_report_and_deletes_candidate() {
    let temp_root = temp_dir("clean-report-path-v2");
    let report_root = temp_root.join("app");
    let dead_file = report_root.join("dead.ts");
    let report_path = report_root.join(".kratos/latest-report.json");

    std::fs::create_dir_all(report_path.parent().expect("report dir should exist"))
        .expect("report dir should exist");
    std::fs::write(&dead_file, "export const dead = true;\n").expect("dead file writes");

    let report = report_with_candidate(&report_root, &dead_file);
    let serialized = serialize_report_pretty(&report).expect("report should serialize");
    std::fs::write(&report_path, serialized).expect("report should write");

    let outcome =
        clean_from_report_path(&report_path, true).expect("clean_from_report_path should work");

    assert!(!dead_file.exists());
    assert_eq!(outcome.deleted_files, 1);
    assert_eq!(outcome.skipped_files, 0);
}

fn report_with_candidate(root: &Path, candidate: &Path) -> ReportV2 {
    let mut report = ReportV2::new(root.to_path_buf());
    report
        .findings
        .deletion_candidates
        .push(DeletionCandidateFinding {
            file: candidate.to_path_buf(),
            reason: "test".to_string(),
            confidence: 1.0,
            safe: true,
        });
    report
}

fn temp_dir(label: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be valid")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("kratos-{label}-{unique}"));
    std::fs::create_dir_all(&path).expect("temp dir should be created");
    path
}

#[cfg(unix)]
fn symlink_dir(target: &Path, link: &Path) {
    std::os::unix::fs::symlink(target, link).expect("directory symlink should be created");
}

#[cfg(unix)]
fn symlink_file(target: &Path, link: &Path) {
    std::os::unix::fs::symlink(target, link).expect("file symlink should be created");
}

#[cfg(windows)]
fn symlink_dir(target: &Path, link: &Path) {
    std::os::windows::fs::symlink_dir(target, link).expect("directory symlink should be created");
}

#[cfg(windows)]
fn symlink_file(target: &Path, link: &Path) {
    std::os::windows::fs::symlink_file(target, link).expect("file symlink should be created");
}
