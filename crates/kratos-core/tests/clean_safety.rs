use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use kratos_core::clean::{
    clean_candidate_safety_status, clean_from_report, clean_from_report_path,
    current_file_identity, current_parent_identity, CleanSafetyStatus,
};
use kratos_core::clean_preview::build_clean_preview;
use kratos_core::error::KratosError;
use kratos_core::model::{CleanCandidateFingerprint, DeletionCandidateFinding, ReportV2};
use kratos_core::report::serialize_report_pretty;
use sha2::{Digest, Sha256};

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
fn clean_skips_dangling_symlink_candidates_without_fingerprints() {
    let temp_root = temp_dir("clean-dangling-symlink");
    let report_root = temp_root.join("app");
    let dangling_link = report_root.join("dangling.ts");

    std::fs::create_dir_all(&report_root).expect("report root should exist");
    symlink_file(Path::new("missing-target.ts"), &dangling_link);

    let report = report_with_candidate(&report_root, &dangling_link);
    let outcome = clean_from_report(&report, true).expect("clean should fail closed");

    assert!(
        std::fs::symlink_metadata(&dangling_link).is_ok(),
        "dangling symlink should remain"
    );
    assert_eq!(outcome.deleted_files, 0);
    assert_eq!(outcome.skipped_files, 1);
}

#[test]
fn clean_skips_live_symlink_candidates_without_touching_targets() {
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
    let outcome = clean_from_report(&report, true).expect("clean should fail closed");

    assert!(
        std::fs::symlink_metadata(&symlink_path).is_ok(),
        "symlink entry should remain"
    );
    assert!(outside_file.exists(), "target file should remain untouched");
    assert_eq!(outcome.deleted_files, 0);
    assert_eq!(outcome.skipped_files, 1);
}

#[test]
fn clean_skips_direct_symlink_even_with_forged_matching_evidence() {
    let temp_root = temp_dir("clean-forged-symlink-evidence");
    let report_root = temp_root.join("app");
    let outside_root = temp_root.join("outside");
    let outside_file = outside_root.join("keep.ts");
    let symlink_path = report_root.join("linked.ts");

    std::fs::create_dir_all(&report_root).expect("report root should exist");
    std::fs::create_dir_all(&outside_root).expect("outside root should exist");
    std::fs::write(&outside_file, "export const keep = true;\n").expect("target should write");
    symlink_file(&outside_file, &symlink_path);

    let mut report = report_with_candidate(&report_root, &symlink_path);
    report.findings.deletion_candidates[0].safe = true;
    report.clean_safety.candidates[0].fingerprint = regular_file_fingerprint(&outside_file);
    report.clean_safety.candidates[0].identity = current_file_identity(&outside_file);

    let outcome = clean_from_report(&report, true).expect("clean should fail closed");

    assert!(std::fs::symlink_metadata(&symlink_path).is_ok());
    assert!(outside_file.exists());
    assert_eq!(outcome.deleted_files, 0);
    assert_eq!(outcome.skipped_files, 1);
}

#[test]
fn clean_allows_symlinked_project_root_without_removing_parent_directories() {
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
    assert!(nested_dir.exists());
    assert_eq!(outcome.deleted_files, 1);
    assert_eq!(outcome.skipped_files, 0);
}

#[test]
fn clean_deletes_through_root_contained_symlink_parent_without_parent_cleanup() {
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
    let report = report_with_candidate(&report_root, &dead_file);
    let serialized = serialize_report_pretty(&report).expect("report should serialize");
    let mut value: serde_json::Value =
        serde_json::from_str(&serialized).expect("report JSON should parse");
    value["schemaVersion"] = serde_json::json!(4);
    std::fs::write(
        &report_path,
        serde_json::to_string_pretty(&value).expect("future report should serialize"),
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
fn clean_from_report_path_reads_current_report_and_deletes_unchanged_candidate() {
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

#[test]
fn clean_skips_candidate_when_content_changed_after_report() {
    let temp_root = temp_dir("clean-stale-content");
    let report_root = temp_root.join("app");
    let dead_file = report_root.join("dead.ts");

    std::fs::create_dir_all(&report_root).expect("report root should exist");
    std::fs::write(&dead_file, "export const dead = true;\n").expect("dead file writes");
    let report = report_with_candidate(&report_root, &dead_file);
    std::fs::write(&dead_file, "export const nowUsed = true;\n").expect("file should change");

    let outcome = clean_from_report(&report, true).expect("clean should fail closed");

    assert!(dead_file.exists());
    assert_eq!(outcome.deleted_files, 0);
    assert_eq!(outcome.skipped_files, 1);
}

#[test]
fn clean_skips_candidate_recreated_at_the_same_path() {
    let temp_root = temp_dir("clean-recreated-content");
    let report_root = temp_root.join("app");
    let dead_file = report_root.join("dead.ts");

    std::fs::create_dir_all(&report_root).expect("report root should exist");
    std::fs::write(&dead_file, "export const old = true;\n").expect("old file writes");
    let report = report_with_candidate(&report_root, &dead_file);
    std::fs::remove_file(&dead_file).expect("old file should delete");
    std::fs::write(&dead_file, "export const replacement = true;\n")
        .expect("replacement file writes");

    let outcome = clean_from_report(&report, true).expect("clean should fail closed");

    assert!(dead_file.exists());
    assert_eq!(outcome.deleted_files, 0);
    assert_eq!(outcome.skipped_files, 1);
}

#[test]
fn clean_skips_same_content_file_recreated_at_the_same_path() {
    let temp_root = temp_dir("clean-recreated-same-content");
    let report_root = temp_root.join("app");
    let dead_file = report_root.join("dead.ts");
    let content = "export const dead = true;\n";

    std::fs::create_dir_all(&report_root).expect("report root should exist");
    std::fs::write(&dead_file, content).expect("old file writes");
    let report = report_with_candidate(&report_root, &dead_file);
    std::fs::remove_file(&dead_file).expect("old file should delete");
    std::fs::write(&dead_file, content).expect("same-content replacement should write");

    let outcome = clean_from_report(&report, true).expect("clean should fail closed");

    assert!(dead_file.exists());
    assert_eq!(outcome.deleted_files, 0);
    assert_eq!(outcome.skipped_files, 1);
}

#[test]
fn clean_skips_safe_false_even_with_a_matching_fingerprint() {
    let temp_root = temp_dir("clean-safe-false");
    let report_root = temp_root.join("app");
    let dead_file = report_root.join("dead.ts");

    std::fs::create_dir_all(&report_root).expect("report root should exist");
    std::fs::write(&dead_file, "export const dead = true;\n").expect("dead file writes");
    let mut report = report_with_candidate(&report_root, &dead_file);
    report.findings.deletion_candidates[0].safe = false;

    let outcome = clean_from_report(&report, true).expect("clean should fail closed");

    assert!(dead_file.exists());
    assert_eq!(outcome.deleted_files, 0);
    assert_eq!(outcome.skipped_files, 1);
}

#[test]
fn clean_skips_schema_v2_candidate_without_fingerprint_evidence() {
    let temp_root = temp_dir("clean-schema-v2-no-fingerprint");
    let report_root = temp_root.join("app");
    let dead_file = report_root.join("dead.ts");

    std::fs::create_dir_all(&report_root).expect("report root should exist");
    std::fs::write(&dead_file, "export const dead = true;\n").expect("dead file writes");
    let mut report = report_with_candidate(&report_root, &dead_file);
    report.version = 2;
    report.clean_safety = Default::default();

    let outcome = clean_from_report(&report, true).expect("clean should fail closed");

    assert!(dead_file.exists());
    assert_eq!(outcome.deleted_files, 0);
    assert_eq!(outcome.skipped_files, 1);
}

#[test]
fn clean_skips_missing_file_after_report_generation() {
    let temp_root = temp_dir("clean-missing-after-report");
    let report_root = temp_root.join("app");
    let dead_file = report_root.join("dead.ts");

    std::fs::create_dir_all(&report_root).expect("report root should exist");
    std::fs::write(&dead_file, "export const dead = true;\n").expect("dead file writes");
    let report = report_with_candidate(&report_root, &dead_file);
    std::fs::remove_file(&dead_file).expect("candidate should be removable before clean");

    let outcome = clean_from_report(&report, true).expect("clean should fail closed");

    assert!(!dead_file.exists());
    assert_eq!(outcome.deleted_files, 0);
    assert_eq!(outcome.skipped_files, 1);
}

#[test]
fn clean_skips_candidate_replaced_by_a_non_regular_file() {
    let temp_root = temp_dir("clean-non-regular-after-report");
    let report_root = temp_root.join("app");
    let dead_file = report_root.join("dead.ts");

    std::fs::create_dir_all(&report_root).expect("report root should exist");
    std::fs::write(&dead_file, "export const dead = true;\n").expect("dead file writes");
    let report = report_with_candidate(&report_root, &dead_file);
    std::fs::remove_file(&dead_file).expect("candidate should be removable before replacement");
    std::fs::create_dir(&dead_file).expect("directory replacement should be created");

    let outcome = clean_from_report(&report, true).expect("clean should fail closed");

    assert!(dead_file.is_dir());
    assert_eq!(outcome.deleted_files, 0);
    assert_eq!(outcome.skipped_files, 1);
}

#[test]
fn clean_skips_unsupported_fingerprint_algorithm() {
    let temp_root = temp_dir("clean-unsupported-fingerprint");
    let report_root = temp_root.join("app");
    let dead_file = report_root.join("dead.ts");

    std::fs::create_dir_all(&report_root).expect("report root should exist");
    std::fs::write(&dead_file, "export const dead = true;\n").expect("dead file writes");
    let mut report = report_with_candidate(&report_root, &dead_file);
    report.clean_safety.fingerprint_algorithm = "sha512".to_string();

    let outcome = clean_from_report(&report, true).expect("clean should fail closed");

    assert!(dead_file.exists());
    assert_eq!(outcome.deleted_files, 0);
    assert_eq!(outcome.skipped_files, 1);
}

#[test]
fn clean_skips_duplicate_fingerprint_entries() {
    let temp_root = temp_dir("clean-duplicate-fingerprint");
    let report_root = temp_root.join("app");
    let dead_file = report_root.join("dead.ts");

    std::fs::create_dir_all(&report_root).expect("report root should exist");
    std::fs::write(&dead_file, "export const dead = true;\n").expect("dead file writes");
    let mut report = report_with_candidate(&report_root, &dead_file);
    report
        .clean_safety
        .candidates
        .push(report.clean_safety.candidates[0].clone());

    assert_eq!(
        clean_candidate_safety_status(&report, &report.findings.deletion_candidates[0]),
        CleanSafetyStatus::DuplicateFingerprint
    );

    let outcome = clean_from_report(&report, true).expect("clean should fail closed");

    assert!(dead_file.exists());
    assert_eq!(outcome.deleted_files, 0);
    assert_eq!(outcome.skipped_files, 1);
}

#[test]
fn duplicate_normalized_deletion_candidates_fail_closed_in_preview_and_apply() {
    let temp_root = temp_dir("clean-duplicate-candidate");
    let report_root = temp_root.join("app");
    let dead_file = report_root.join("src/dead.ts");
    std::fs::create_dir_all(dead_file.parent().expect("parent should exist"))
        .expect("parent should exist");
    std::fs::write(&dead_file, "dead\n").expect("candidate should write");
    let mut report = report_with_candidate(&report_root, &dead_file);
    let mut alias = report.findings.deletion_candidates[0].clone();
    alias.file = report_root.join("src/../src/dead.ts");
    report.findings.deletion_candidates.push(alias);

    let preview = build_clean_preview(&report, 0.0).expect("preview should build");
    assert!(preview.deletion_target_paths.is_empty());
    assert_eq!(preview.items.len(), 2);
    assert!(preview
        .items
        .iter()
        .all(|item| item.safety_status == CleanSafetyStatus::DuplicateCandidate));

    let outcome = clean_from_report(&report, true).expect("apply should fail closed");
    assert!(dead_file.exists());
    assert_eq!(outcome.deleted_files, 0);
    assert_eq!(outcome.skipped_files, 2);
    assert!(outcome.failed_files.is_empty());
}

fn report_with_candidate(root: &Path, candidate: &Path) -> ReportV2 {
    let mut report = ReportV2::new(root.to_path_buf());
    let fingerprint = regular_file_fingerprint(candidate);
    report
        .findings
        .deletion_candidates
        .push(DeletionCandidateFinding {
            file: candidate.to_path_buf(),
            reason: "test".to_string(),
            confidence: 1.0,
            safe: fingerprint.is_some(),
        });
    report
        .clean_safety
        .candidates
        .push(CleanCandidateFingerprint {
            file: candidate.to_path_buf(),
            fingerprint,
            identity: current_file_identity(candidate),
            parent_identity: current_parent_identity(candidate),
        });
    report
}

fn regular_file_fingerprint(path: &Path) -> Option<String> {
    let metadata = std::fs::symlink_metadata(path).ok()?;
    if !metadata.file_type().is_file() {
        return None;
    }
    let bytes = std::fs::read(path).ok()?;
    Some(format!("{:x}", Sha256::digest(bytes)))
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
