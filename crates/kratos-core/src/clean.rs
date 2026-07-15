use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

use crate::error::{KratosError, KratosResult};
use crate::fingerprint::{
    current_parent_identity as fingerprint_parent_identity, inspect_regular_file, FileSnapshot,
    CONTENT_FINGERPRINT_ALGORITHM,
};
use crate::model::{CleanCandidateFingerprint, DeletionCandidateFinding, ReportV2, REPORT_V2};
use crate::report::parse_report_json;

static QUARANTINE_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CleanOutcome {
    pub deleted_files: usize,
    pub skipped_files: usize,
    pub failed_files: Vec<CleanFailure>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CleanFailure {
    pub file: PathBuf,
    pub error: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CleanThresholdPlan {
    pub deletion_targets: Vec<DeletionCandidateFinding>,
    pub threshold_skipped_targets: Vec<DeletionCandidateFinding>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum CleanSafetyStatus {
    #[default]
    Ready,
    PathOutsideRoot,
    DuplicateCandidate,
    UnsafeFlag,
    UnsupportedFingerprintAlgorithm,
    MissingFingerprint,
    MissingIdentity,
    DuplicateFingerprint,
    FingerprintUnavailable,
    FingerprintMismatch,
    IdentityMismatch,
}

pub fn clean_from_report_path(
    report_path: impl AsRef<Path>,
    apply: bool,
) -> KratosResult<CleanOutcome> {
    let report = load_clean_report(report_path)?;
    clean_from_report(&report, apply)
}

pub fn clean_from_report_path_with_min_confidence(
    report_path: impl AsRef<Path>,
    min_confidence: f32,
) -> KratosResult<CleanOutcome> {
    let report = load_clean_report(report_path)?;
    clean_from_report_with_min_confidence(&report, min_confidence)
}

pub fn load_clean_report(report_path: impl AsRef<Path>) -> KratosResult<ReportV2> {
    let raw = std::fs::read_to_string(report_path)?;
    let value: Value =
        serde_json::from_str(&raw).map_err(|error| KratosError::Json(error.to_string()))?;
    let version = value
        .get("schemaVersion")
        .or_else(|| value.get("version"))
        .and_then(Value::as_u64)
        .ok_or_else(|| KratosError::Json("Report is missing schemaVersion/version".to_string()))?
        as u32;

    if version < REPORT_V2 {
        return Err(KratosError::InvalidReportVersion {
            expected: REPORT_V2,
            found: version,
        });
    }

    parse_report_json(&raw)
}

pub fn plan_clean_candidates(
    report: &ReportV2,
    min_confidence: f32,
) -> KratosResult<CleanThresholdPlan> {
    validate_clean_threshold_inputs(report, min_confidence)?;

    let mut plan = CleanThresholdPlan::default();

    for candidate in &report.findings.deletion_candidates {
        if candidate.confidence >= min_confidence {
            plan.deletion_targets.push(candidate.clone());
        } else {
            plan.threshold_skipped_targets.push(candidate.clone());
        }
    }

    Ok(plan)
}

pub fn clean_from_report_with_min_confidence(
    report: &ReportV2,
    min_confidence: f32,
) -> KratosResult<CleanOutcome> {
    let plan = plan_clean_candidates(report, min_confidence)?;
    apply_clean_plan(report, &plan)
}

pub fn clean_from_report(report: &ReportV2, apply: bool) -> KratosResult<CleanOutcome> {
    if report.version < REPORT_V2 {
        return Err(KratosError::InvalidReportVersion {
            expected: REPORT_V2,
            found: report.version,
        });
    }

    if !apply {
        return Ok(CleanOutcome {
            deleted_files: 0,
            skipped_files: report.findings.deletion_candidates.len(),
            failed_files: Vec::new(),
        });
    }

    clean_from_report_with_min_confidence(report, 0.0)
}

#[doc(hidden)]
pub fn current_file_identity(path: &Path) -> Option<String> {
    inspect_regular_file(path)
        .ok()
        .map(|snapshot| snapshot.identity)
}

#[doc(hidden)]
pub fn current_parent_identity(path: &Path) -> Option<String> {
    fingerprint_parent_identity(path.parent()?)
}

pub(crate) fn is_safe_clean_candidate(report_root: &Path, candidate_path: &Path) -> bool {
    let report_root_path = resolve_path(report_root);
    let candidate_path = resolve_path(candidate_path);

    if !is_within_directory(&report_root_path, &candidate_path) {
        return false;
    }

    let deletion_root = realpath_or_fallback(report_root);
    let candidate_parent_path = candidate_path
        .parent()
        .unwrap_or(report_root_path.as_path());
    let candidate_parent = realpath_or_fallback(candidate_parent_path);

    is_within_directory(&deletion_root, &candidate_parent)
}

pub fn clean_candidate_safety_status(
    report: &ReportV2,
    candidate: &DeletionCandidateFinding,
) -> CleanSafetyStatus {
    if !is_safe_clean_candidate(&report.root, &candidate.file) {
        return CleanSafetyStatus::PathOutsideRoot;
    }

    let candidate_path = resolve_path(&candidate.file);
    if report
        .findings
        .deletion_candidates
        .iter()
        .filter(|item| resolve_path(&item.file) == candidate_path)
        .count()
        != 1
    {
        return CleanSafetyStatus::DuplicateCandidate;
    }
    if !candidate.safe {
        return CleanSafetyStatus::UnsafeFlag;
    }

    let mut fingerprints = report
        .clean_safety
        .candidates
        .iter()
        .filter(|entry| resolve_path(&entry.file) == candidate_path);
    let Some(entry) = fingerprints.next() else {
        return CleanSafetyStatus::MissingFingerprint;
    };
    if fingerprints.next().is_some() {
        return CleanSafetyStatus::DuplicateFingerprint;
    }
    let Some(expected_fingerprint) = entry.fingerprint.as_deref() else {
        return CleanSafetyStatus::MissingFingerprint;
    };
    let Some(expected_identity) = entry.identity.as_deref() else {
        return CleanSafetyStatus::MissingIdentity;
    };
    let Some(expected_parent_identity) = entry.parent_identity.as_deref() else {
        return CleanSafetyStatus::MissingIdentity;
    };
    if report.clean_safety.fingerprint_algorithm != CONTENT_FINGERPRINT_ALGORITHM {
        return CleanSafetyStatus::UnsupportedFingerprintAlgorithm;
    }
    let Ok(actual) = inspect_regular_file(&candidate_path) else {
        return CleanSafetyStatus::FingerprintUnavailable;
    };

    if actual.identity != expected_identity || actual.parent_identity != expected_parent_identity {
        CleanSafetyStatus::IdentityMismatch
    } else if actual.fingerprint != expected_fingerprint {
        CleanSafetyStatus::FingerprintMismatch
    } else {
        CleanSafetyStatus::Ready
    }
}

fn validate_clean_threshold_inputs(report: &ReportV2, min_confidence: f32) -> KratosResult<()> {
    if report.version < REPORT_V2 {
        return Err(KratosError::InvalidReportVersion {
            expected: REPORT_V2,
            found: report.version,
        });
    }

    if !min_confidence.is_finite() || !(0.0..=1.0).contains(&min_confidence) {
        return Err(KratosError::Config(
            "--min-confidence must be between 0.0 and 1.0".to_string(),
        ));
    }

    Ok(())
}

fn apply_clean_plan(report: &ReportV2, plan: &CleanThresholdPlan) -> KratosResult<CleanOutcome> {
    apply_clean_plan_with_hooks(report, plan, |_| {}, |_| {})
}

#[cfg(test)]
fn apply_clean_plan_with_hook<F>(
    report: &ReportV2,
    plan: &CleanThresholdPlan,
    before_quarantine: F,
) -> KratosResult<CleanOutcome>
where
    F: FnMut(&Path),
{
    apply_clean_plan_with_hooks(report, plan, before_quarantine, |_| {})
}

fn apply_clean_plan_with_hooks<F, G>(
    report: &ReportV2,
    plan: &CleanThresholdPlan,
    mut before_quarantine: F,
    mut after_quarantine_verification: G,
) -> KratosResult<CleanOutcome>
where
    F: FnMut(&Path),
    G: FnMut(&Path),
{
    let mut outcome = CleanOutcome {
        deleted_files: 0,
        skipped_files: plan.threshold_skipped_targets.len(),
        failed_files: Vec::new(),
    };

    for candidate in &plan.deletion_targets {
        let candidate_path = resolve_path(&candidate.file);

        if clean_candidate_safety_status(report, candidate) != CleanSafetyStatus::Ready {
            outcome.skipped_files += 1;
            continue;
        }
        let Some(expected) =
            unique_safety_entry(report, &candidate_path).and_then(snapshot_from_entry)
        else {
            outcome.skipped_files += 1;
            continue;
        };

        match quarantine_and_delete(
            &report.root,
            &candidate_path,
            &expected,
            &mut before_quarantine,
            &mut after_quarantine_verification,
        ) {
            QuarantineOutcome::Deleted => {
                outcome.deleted_files += 1;
            }
            QuarantineOutcome::Skipped => outcome.skipped_files += 1,
            QuarantineOutcome::Failed(error) => outcome.failed_files.push(CleanFailure {
                file: candidate_path,
                error,
            }),
        }
    }

    Ok(outcome)
}

fn unique_safety_entry<'a>(
    report: &'a ReportV2,
    candidate_path: &Path,
) -> Option<&'a CleanCandidateFingerprint> {
    let mut entries = report
        .clean_safety
        .candidates
        .iter()
        .filter(|entry| resolve_path(&entry.file) == candidate_path);
    let entry = entries.next()?;
    entries.next().is_none().then_some(entry)
}

fn snapshot_from_entry(entry: &CleanCandidateFingerprint) -> Option<FileSnapshot> {
    Some(FileSnapshot {
        fingerprint: entry.fingerprint.clone()?,
        identity: entry.identity.clone()?,
        parent_identity: entry.parent_identity.clone()?,
    })
}

enum QuarantineOutcome {
    Deleted,
    Skipped,
    Failed(String),
}

fn quarantine_and_delete<F, G>(
    report_root: &Path,
    candidate_path: &Path,
    expected: &FileSnapshot,
    before_move: &mut F,
    after_quarantine_verification: &mut G,
) -> QuarantineOutcome
where
    F: FnMut(&Path),
    G: FnMut(&Path),
{
    let quarantine_dir = match create_quarantine_dir(report_root) {
        Ok(path) => path,
        Err(error) => return QuarantineOutcome::Failed(error.to_string()),
    };
    let quarantined_path = quarantine_dir.join("candidate");
    let source_parent_matches = candidate_path
        .parent()
        .and_then(fingerprint_parent_identity)
        .as_deref()
        == Some(expected.parent_identity.as_str());
    if !source_parent_matches || !is_safe_clean_candidate(report_root, candidate_path) {
        let _ = std::fs::remove_dir(&quarantine_dir);
        return QuarantineOutcome::Skipped;
    }

    before_move(candidate_path);
    let source_parent_matches_after_hook = candidate_path
        .parent()
        .and_then(fingerprint_parent_identity)
        .as_deref()
        == Some(expected.parent_identity.as_str());
    if !source_parent_matches_after_hook || !is_safe_clean_candidate(report_root, candidate_path) {
        let _ = std::fs::remove_dir(&quarantine_dir);
        return QuarantineOutcome::Skipped;
    }

    if let Err(error) = std::fs::rename(candidate_path, &quarantined_path) {
        let _ = std::fs::remove_dir(&quarantine_dir);
        return if error.kind() == ErrorKind::NotFound {
            QuarantineOutcome::Skipped
        } else {
            QuarantineOutcome::Failed(error.to_string())
        };
    }

    let source_parent_matches = candidate_path
        .parent()
        .and_then(fingerprint_parent_identity)
        .as_deref()
        == Some(expected.parent_identity.as_str());
    let verified = source_parent_matches
        && is_safe_clean_candidate(report_root, candidate_path)
        && is_safe_clean_candidate(report_root, &quarantined_path)
        && inspect_regular_file(&quarantined_path)
            .map(|actual| {
                actual.identity == expected.identity && actual.fingerprint == expected.fingerprint
            })
            .unwrap_or(false);

    if !verified {
        return match restore_quarantined_file(
            report_root,
            &quarantined_path,
            candidate_path,
            expected,
        ) {
            Ok(()) => {
                let _ = std::fs::remove_dir(&quarantine_dir);
                QuarantineOutcome::Skipped
            }
            Err(error) => QuarantineOutcome::Failed(format!(
                "검증 실패 파일을 복원하지 못했습니다. 보존 위치: {} ({error})",
                quarantined_path.display()
            )),
        };
    }

    after_quarantine_verification(&quarantined_path);

    let quarantine_still_verified = is_safe_clean_candidate(report_root, &quarantined_path)
        && inspect_regular_file(&quarantined_path)
            .map(|actual| {
                actual.identity == expected.identity && actual.fingerprint == expected.fingerprint
            })
            .unwrap_or(false);
    if !quarantine_still_verified {
        return QuarantineOutcome::Failed(format!(
            "검증된 quarantine 경로가 변경되어 삭제하지 않았습니다. 보존 위치: {}",
            quarantined_path.display()
        ));
    }

    match std::fs::remove_file(&quarantined_path) {
        Ok(()) => {
            let _ = std::fs::remove_dir(&quarantine_dir);
            QuarantineOutcome::Deleted
        }
        Err(delete_error) => match restore_quarantined_file(
            report_root,
            &quarantined_path,
            candidate_path,
            expected,
        ) {
            Ok(()) => {
                let _ = std::fs::remove_dir(&quarantine_dir);
                QuarantineOutcome::Failed(delete_error.to_string())
            }
            Err(restore_error) => QuarantineOutcome::Failed(format!(
                "삭제와 복원에 실패했습니다. 보존 위치: {} (삭제: {delete_error}; 복원: {restore_error})",
                quarantined_path.display()
            )),
        },
    }
}

fn create_quarantine_dir(parent: &Path) -> std::io::Result<PathBuf> {
    for _ in 0..100 {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let counter = QUARANTINE_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = parent.join(format!(
            ".kratos-clean-quarantine-{}-{nonce}-{counter}",
            std::process::id()
        ));
        match create_private_directory(&path) {
            Ok(()) => return Ok(path),
            Err(error) if error.kind() == ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }

    Err(std::io::Error::new(
        ErrorKind::AlreadyExists,
        "could not allocate a unique clean quarantine directory",
    ))
}

#[cfg(unix)]
fn create_private_directory(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::DirBuilderExt;

    std::fs::DirBuilder::new().mode(0o700).create(path)
}

#[cfg(not(unix))]
fn create_private_directory(path: &Path) -> std::io::Result<()> {
    std::fs::create_dir(path)
}

fn restore_quarantined_file(
    report_root: &Path,
    quarantined_path: &Path,
    original_path: &Path,
    expected: &FileSnapshot,
) -> std::io::Result<()> {
    let parent_matches = original_path
        .parent()
        .and_then(fingerprint_parent_identity)
        .as_deref()
        == Some(expected.parent_identity.as_str());
    if !parent_matches || !is_safe_clean_candidate(report_root, original_path) {
        return Err(std::io::Error::other(
            "candidate parent changed or escaped report root during restore",
        ));
    }
    std::fs::hard_link(quarantined_path, original_path)?;
    std::fs::remove_file(quarantined_path)
}

fn realpath_or_fallback(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| resolve_path(path))
}

fn is_within_directory(root: &Path, candidate: &Path) -> bool {
    let normalized_root = resolve_path(root);
    let normalized_candidate = resolve_path(candidate);

    normalized_candidate == normalized_root || normalized_candidate.starts_with(&normalized_root)
}

fn resolve_path(path: &Path) -> PathBuf {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    };

    normalize_path(absolute)
}

fn normalize_path(path: PathBuf) -> PathBuf {
    let mut normalized = PathBuf::new();

    for component in path.components() {
        match component {
            std::path::Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            std::path::Component::RootDir => normalized.push(component.as_os_str()),
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                let can_pop = matches!(
                    normalized.components().next_back(),
                    Some(std::path::Component::Normal(_))
                );

                if can_pop {
                    normalized.pop();
                } else {
                    normalized.push("..");
                }
            }
            std::path::Component::Normal(segment) => normalized.push(segment),
        }
    }

    if normalized.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        normalized
    }
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;
    use crate::model::{CleanSafetyManifest, FindingSet};

    #[test]
    fn quarantine_rechecks_the_exact_file_after_precheck() {
        let root = temp_dir("clean-quarantine-recheck");
        let file = root.join("dead.ts");
        std::fs::write(&file, "original\n").expect("fixture should write");
        let report = report_for_files(&root, std::slice::from_ref(&file));
        let plan = plan_clean_candidates(&report, 0.0).expect("plan should build");

        let outcome = apply_clean_plan_with_hook(&report, &plan, |path| {
            std::fs::remove_file(path).expect("original should be replaceable");
            std::fs::write(path, "replacement\n").expect("replacement should write");
        })
        .expect("clean should stay fail closed");

        assert_eq!(outcome.deleted_files, 0);
        assert_eq!(outcome.skipped_files, 1);
        assert!(outcome.failed_files.is_empty());
        assert_eq!(
            std::fs::read_to_string(&file).expect("replacement should remain"),
            "replacement\n"
        );
    }

    #[test]
    fn partial_failure_preserves_successful_delete_accounting() {
        let root = temp_dir("clean-partial-accounting");
        let first = root.join("first/dead.ts");
        let second = root.join("second/dead.ts");
        std::fs::create_dir_all(first.parent().expect("first parent"))
            .expect("parent should exist");
        std::fs::create_dir_all(second.parent().expect("second parent"))
            .expect("parent should exist");
        std::fs::write(&first, "first\n").expect("first should write");
        std::fs::write(&second, "second\n").expect("second should write");
        let report = report_for_files(&root, &[first.clone(), second.clone()]);
        let plan = plan_clean_candidates(&report, 0.0).expect("plan should build");

        let mut verification_count = 0;
        let outcome = apply_clean_plan_with_hooks(
            &report,
            &plan,
            |_| {},
            |path| {
                verification_count += 1;
                if verification_count == 2 {
                    std::fs::remove_file(path).expect("quarantined second should be removable");
                }
            },
        )
        .expect("per-file failure should be reported in the outcome");

        assert_eq!(outcome.deleted_files, 1);
        assert_eq!(outcome.skipped_files, 0);
        assert_eq!(outcome.failed_files.len(), 1);
        assert_eq!(outcome.failed_files[0].file, second);
        assert!(!first.exists());
    }

    #[cfg(unix)]
    #[test]
    fn mutable_parent_symlink_race_does_not_delete_outside_file() {
        let root = temp_dir("clean-parent-race");
        let nested = root.join("nested");
        let saved_nested = root.join("saved-nested");
        let candidate = nested.join("dead.ts");
        let outside = temp_dir("clean-parent-race-outside");
        let outside_file = outside.join("dead.ts");
        std::fs::create_dir_all(&nested).expect("nested should exist");
        std::fs::write(&candidate, "analyzed\n").expect("candidate should write");
        std::fs::write(&outside_file, "outside\n").expect("outside should write");
        let report = report_for_files(&root, std::slice::from_ref(&candidate));
        let plan = plan_clean_candidates(&report, 0.0).expect("plan should build");

        let outcome = apply_clean_plan_with_hook(&report, &plan, |_| {
            std::fs::rename(&nested, &saved_nested).expect("nested should move");
            std::os::unix::fs::symlink(&outside, &nested).expect("parent symlink should install");
        })
        .expect("clean should stay fail closed");

        assert_eq!(outcome.deleted_files, 0);
        assert_eq!(outcome.skipped_files, 1);
        assert!(outcome.failed_files.is_empty());
        assert_eq!(
            std::fs::read_to_string(&outside_file).expect("outside file should remain"),
            "outside\n"
        );
        assert!(saved_nested.join("dead.ts").exists());
    }

    #[cfg(unix)]
    #[test]
    fn parent_relocation_after_quarantine_verification_cannot_redirect_deletion() {
        let root = temp_dir("clean-post-verification-parent-race");
        let nested = root.join("nested");
        let candidate = nested.join("dead.ts");
        let outside = temp_dir("clean-post-verification-parent-race-outside");
        let moved_nested = outside.join("nested");
        let replacement = moved_nested.join("dead.ts");
        std::fs::create_dir_all(&nested).expect("nested should exist");
        std::fs::write(&candidate, "analyzed\n").expect("candidate should write");
        let report = report_for_files(&root, std::slice::from_ref(&candidate));
        let plan = plan_clean_candidates(&report, 0.0).expect("plan should build");

        let outcome = apply_clean_plan_with_hooks(
            &report,
            &plan,
            |_| {},
            |_| {
                std::fs::rename(&nested, &moved_nested).expect("empty parent should move");
                std::os::unix::fs::symlink(&moved_nested, &nested)
                    .expect("redirecting symlink should install");
                std::fs::write(&replacement, "replacement\n")
                    .expect("replacement should write outside root");
            },
        )
        .expect("verified quarantine deletion should complete");

        assert_eq!(outcome.deleted_files, 1);
        assert_eq!(outcome.skipped_files, 0);
        assert!(outcome.failed_files.is_empty());
        assert_eq!(
            std::fs::read_to_string(&replacement).expect("replacement should remain"),
            "replacement\n"
        );
    }

    #[cfg(unix)]
    #[test]
    fn redirected_parent_with_matching_hardlink_is_skipped_without_deletion() {
        let root = temp_dir("clean-parent-hardlink-race");
        let parent = root.join("nested");
        let moved_parent = root.join("moved-nested");
        let outside = temp_dir("clean-parent-hardlink-race-outside");
        let candidate = parent.join("candidate.ts");
        let moved_candidate = moved_parent.join("candidate.ts");
        let outside_candidate = outside.join("candidate.ts");
        std::fs::create_dir_all(&parent).expect("parent should exist");
        std::fs::write(&candidate, "candidate\n").expect("candidate should write");
        let report = report_for_files(&root, std::slice::from_ref(&candidate));
        let plan = plan_clean_candidates(&report, 0.0).expect("plan should build");

        let outcome = apply_clean_plan_with_hook(&report, &plan, |_| {
            std::fs::rename(&parent, &moved_parent).expect("parent should move");
            std::fs::hard_link(&moved_candidate, &outside_candidate)
                .expect("matching external hardlink should exist");
            std::os::unix::fs::symlink(&outside, &parent)
                .expect("redirecting parent symlink should install");
        })
        .expect("clean should restore the redirected pathname");

        assert_eq!(outcome.deleted_files, 0);
        assert_eq!(outcome.skipped_files, 1);
        assert!(outcome.failed_files.is_empty());
        assert_eq!(
            std::fs::read_to_string(&outside_candidate).expect("outside hardlink should remain"),
            "candidate\n"
        );
        assert!(moved_candidate.exists());
    }

    #[cfg(unix)]
    #[test]
    fn quarantine_relocation_is_not_followed_for_final_delete() {
        let root = temp_dir("clean-quarantine-race");
        let candidate = root.join("candidate.ts");
        let outside = temp_dir("clean-quarantine-race-outside");
        let outside_target = outside.join("candidate");
        std::fs::write(&candidate, "candidate\n").expect("candidate should write");
        std::fs::write(&outside_target, "outside\n").expect("outside target should write");
        let report = report_for_files(&root, std::slice::from_ref(&candidate));
        let plan = plan_clean_candidates(&report, 0.0).expect("plan should build");

        let outcome = apply_clean_plan_with_hooks(
            &report,
            &plan,
            |_| {},
            |quarantined_path| {
                let quarantine_dir = quarantined_path.parent().expect("quarantine parent");
                let moved_quarantine = outside.join("moved-quarantine");
                std::fs::rename(quarantine_dir, &moved_quarantine).expect("quarantine should move");
                std::os::unix::fs::symlink(&outside, quarantine_dir)
                    .expect("redirecting quarantine symlink should install");
            },
        )
        .expect("clean should report quarantine relocation as failure");

        assert_eq!(outcome.deleted_files, 0);
        assert_eq!(outcome.skipped_files, 0);
        assert_eq!(outcome.failed_files.len(), 1);
        assert_eq!(
            std::fs::read_to_string(&outside_target).expect("outside target should remain"),
            "outside\n"
        );
        assert!(outside.join("moved-quarantine/candidate").exists());
    }

    #[cfg(unix)]
    #[test]
    fn restore_refuses_a_parent_symlink_escape() {
        let root = temp_dir("clean-restore-race");
        let nested = root.join("nested");
        let moved_nested = temp_dir("clean-restore-race-moved");
        let candidate = nested.join("candidate.ts");
        let quarantined = root.join("quarantine-candidate");
        std::fs::create_dir_all(&nested).expect("nested should exist");
        std::fs::write(&candidate, "candidate\n").expect("candidate should write");
        std::fs::write(&quarantined, "candidate\n").expect("quarantine should write");
        let expected = inspect_regular_file(&candidate).expect("snapshot should exist");
        std::fs::rename(&nested, moved_nested.join("nested"))
            .expect("candidate parent should move");
        std::os::unix::fs::symlink(moved_nested.join("nested"), &nested)
            .expect("redirecting parent symlink should install");

        let result = restore_quarantined_file(&root, &quarantined, &candidate, &expected);
        assert!(result.is_err());
        assert!(quarantined.exists());
        assert!(moved_nested.join("nested/candidate.ts").exists());
    }

    fn report_for_files(root: &Path, files: &[PathBuf]) -> ReportV2 {
        let mut findings = FindingSet::default();
        let mut safety_candidates = Vec::new();
        for file in files {
            let snapshot = inspect_regular_file(file).expect("fixture should have evidence");
            findings.deletion_candidates.push(DeletionCandidateFinding {
                file: file.clone(),
                reason: "test".to_string(),
                confidence: 1.0,
                safe: true,
            });
            safety_candidates.push(CleanCandidateFingerprint {
                file: file.clone(),
                fingerprint: Some(snapshot.fingerprint),
                identity: Some(snapshot.identity),
                parent_identity: Some(snapshot.parent_identity),
            });
        }

        let mut report = ReportV2::new(root.to_path_buf());
        report.findings = findings;
        report.clean_safety = CleanSafetyManifest {
            fingerprint_algorithm: CONTENT_FINGERPRINT_ALGORITHM.to_string(),
            candidates: safety_candidates,
        };
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
}
