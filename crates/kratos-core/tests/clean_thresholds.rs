use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use kratos_core::clean::{clean_from_report_with_min_confidence, plan_clean_candidates};
use kratos_core::model::{DeletionCandidateFinding, ReportV2};

#[test]
fn plan_clean_candidates_splits_deletion_targets_and_threshold_skips() {
    let temp_root = temp_dir("clean-threshold-plan");
    let report = report_with_candidates(
        &temp_root,
        &[
            ("src/high.ts", 0.95),
            ("src/medium.ts", 0.80),
            ("src/low.ts", 0.25),
        ],
    );

    let plan = plan_clean_candidates(&report, 0.8).expect("plan should build");

    assert_eq!(plan.deletion_targets.len(), 2);
    assert_eq!(plan.threshold_skipped_targets.len(), 1);
    assert_eq!(plan.deletion_targets[0].file, temp_root.join("src/high.ts"));
    assert_eq!(
        plan.deletion_targets[1].file,
        temp_root.join("src/medium.ts")
    );
    assert_eq!(
        plan.threshold_skipped_targets[0].file,
        temp_root.join("src/low.ts")
    );
}

#[test]
fn clean_from_report_with_min_confidence_skips_low_confidence_targets() {
    let temp_root = temp_dir("clean-threshold-apply");
    let report = report_with_candidates(&temp_root, &[("src/high.ts", 0.96), ("src/low.ts", 0.40)]);

    let outcome =
        clean_from_report_with_min_confidence(&report, 0.9).expect("clean should succeed");

    assert_eq!(outcome.deleted_files, 1);
    assert_eq!(outcome.skipped_files, 1);
    assert!(!temp_root.join("src/high.ts").exists());
    assert!(temp_root.join("src/low.ts").exists());
}

#[test]
fn clean_from_report_with_min_confidence_rejects_invalid_thresholds() {
    let temp_root = temp_dir("clean-threshold-invalid");
    let report = report_with_candidates(&temp_root, &[("src/high.ts", 0.96)]);

    let error = clean_from_report_with_min_confidence(&report, 1.1)
        .expect_err("threshold should be rejected");
    assert!(
        error
            .to_string()
            .contains("--min-confidence must be between 0.0 and 1.0"),
        "unexpected error: {error}"
    );
}

fn report_with_candidates(root: &Path, candidates: &[(&str, f32)]) -> ReportV2 {
    let mut report = ReportV2::new(root.to_path_buf());
    std::fs::create_dir_all(root).expect("report root should exist");

    for (relative, _) in candidates {
        let path = root.join(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("candidate parent should exist");
        }
        std::fs::write(&path, "export const dead = true;\n").expect("candidate file should write");
    }

    report.findings.deletion_candidates = candidates
        .iter()
        .map(|(relative, confidence)| DeletionCandidateFinding {
            file: root.join(relative),
            reason: format!("{} candidate", relative),
            confidence: *confidence,
            safe: true,
        })
        .collect();

    report
}

fn temp_dir(label: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be valid")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("kratos-core-{label}-{unique}"));
    std::fs::create_dir_all(&path).expect("temp dir should be created");
    path
}
