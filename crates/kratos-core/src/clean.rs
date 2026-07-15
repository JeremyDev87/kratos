use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::ffi::{CStr, CString};
#[cfg(unix)]
use std::fs::File;
#[cfg(unix)]
use std::io::ErrorKind;
#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
#[cfg(unix)]
use std::os::unix::io::{AsRawFd, FromRawFd};
#[cfg(unix)]
use std::sync::atomic::{AtomicU64, Ordering};
#[cfg(unix)]
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

use crate::error::{KratosError, KratosResult};
use crate::fingerprint::{
    current_parent_identity as fingerprint_parent_identity, inspect_regular_file, FileSnapshot,
    CONTENT_FINGERPRINT_ALGORITHM,
};
#[cfg(unix)]
use crate::fingerprint::{directory_identity_from_file, inspect_open_regular_file};
use crate::model::{CleanCandidateFingerprint, DeletionCandidateFinding, ReportV2, REPORT_V2};
use crate::report::parse_report_json;

#[cfg(unix)]
static QUARANTINE_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CleanOutcome {
    pub deleted_files: usize,
    pub skipped_files: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CleanApplyOutcome {
    /// Compatibility count for paths moved out of the code tree. The file bytes
    /// remain under `quarantined_files`; this does not mean a physical unlink.
    pub deleted_files: usize,
    /// Confirmed current paths for retained quarantine objects. If a moved
    /// quarantine directory no longer has a trustworthy pathname, the candidate
    /// is reported through `failed_files` without fabricating a path.
    pub quarantined_files: Vec<PathBuf>,
    pub skipped_files: usize,
    pub failed_files: Vec<CleanFailure>,
}

impl From<CleanApplyOutcome> for CleanOutcome {
    fn from(value: CleanApplyOutcome) -> Self {
        Self {
            deleted_files: value.deleted_files,
            skipped_files: value.skipped_files,
        }
    }
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

pub fn clean_from_report_path_detailed(
    report_path: impl AsRef<Path>,
    apply: bool,
) -> KratosResult<CleanApplyOutcome> {
    let report = load_clean_report(report_path)?;
    clean_from_report_detailed(&report, apply)
}

pub fn clean_from_report_path_with_min_confidence(
    report_path: impl AsRef<Path>,
    min_confidence: f32,
) -> KratosResult<CleanOutcome> {
    let report = load_clean_report(report_path)?;
    clean_from_report_with_min_confidence(&report, min_confidence)
}

pub fn clean_from_report_path_with_min_confidence_detailed(
    report_path: impl AsRef<Path>,
    min_confidence: f32,
) -> KratosResult<CleanApplyOutcome> {
    let report = load_clean_report(report_path)?;
    clean_from_report_with_min_confidence_detailed(&report, min_confidence)
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
    clean_from_report_with_min_confidence_detailed(report, min_confidence).map(Into::into)
}

pub fn clean_from_report_with_min_confidence_detailed(
    report: &ReportV2,
    min_confidence: f32,
) -> KratosResult<CleanApplyOutcome> {
    let plan = plan_clean_candidates(report, min_confidence)?;
    apply_clean_plan(report, &plan)
}

pub fn clean_from_report(report: &ReportV2, apply: bool) -> KratosResult<CleanOutcome> {
    clean_from_report_detailed(report, apply).map(Into::into)
}

pub fn clean_from_report_detailed(
    report: &ReportV2,
    apply: bool,
) -> KratosResult<CleanApplyOutcome> {
    if report.version < REPORT_V2 {
        return Err(KratosError::InvalidReportVersion {
            expected: REPORT_V2,
            found: report.version,
        });
    }

    if !apply {
        return Ok(CleanApplyOutcome {
            deleted_files: 0,
            quarantined_files: Vec::new(),
            skipped_files: report.findings.deletion_candidates.len(),
            failed_files: Vec::new(),
        });
    }

    clean_from_report_with_min_confidence_detailed(report, 0.0)
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

fn apply_clean_plan(
    report: &ReportV2,
    plan: &CleanThresholdPlan,
) -> KratosResult<CleanApplyOutcome> {
    apply_clean_plan_with_hooks(report, plan, |_| {}, |_| {})
}

#[cfg(test)]
fn apply_clean_plan_with_hook<F>(
    report: &ReportV2,
    plan: &CleanThresholdPlan,
    before_quarantine: F,
) -> KratosResult<CleanApplyOutcome>
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
) -> KratosResult<CleanApplyOutcome>
where
    F: FnMut(&Path),
    G: FnMut(&Path),
{
    let mut outcome = CleanApplyOutcome {
        deleted_files: 0,
        quarantined_files: Vec::new(),
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

        match quarantine_candidate(
            &report.root,
            &candidate_path,
            &expected,
            &mut before_quarantine,
            &mut after_quarantine_verification,
        ) {
            QuarantineOutcome::Quarantined(quarantined_path) => {
                outcome.deleted_files += 1;
                outcome.quarantined_files.push(quarantined_path);
            }
            QuarantineOutcome::Skipped(retained_path) => {
                outcome.skipped_files += 1;
                if let Some(path) = retained_path {
                    outcome.quarantined_files.push(path);
                }
            }
            QuarantineOutcome::Failed {
                error,
                retained_path,
                removed_from_code_tree,
            } => {
                if removed_from_code_tree {
                    outcome.deleted_files += 1;
                }
                if let Some(path) = retained_path {
                    outcome.quarantined_files.push(path);
                }
                outcome.failed_files.push(CleanFailure {
                    file: candidate_path,
                    error,
                });
            }
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

#[cfg_attr(not(unix), allow(dead_code))]
enum QuarantineOutcome {
    Quarantined(PathBuf),
    Skipped(Option<PathBuf>),
    Failed {
        error: String,
        retained_path: Option<PathBuf>,
        removed_from_code_tree: bool,
    },
}

#[cfg(unix)]
fn quarantine_candidate<F, G>(
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
    let Some(parent_path) = candidate_path.parent() else {
        return QuarantineOutcome::Skipped(None);
    };
    if !is_safe_clean_candidate(report_root, candidate_path) {
        return QuarantineOutcome::Skipped(None);
    }

    let canonical_parent = match std::fs::canonicalize(parent_path) {
        Ok(path) => path,
        Err(_) => return QuarantineOutcome::Skipped(None),
    };
    let source_parent = match open_directory(&canonical_parent) {
        Ok(directory) => directory,
        Err(_) => return QuarantineOutcome::Skipped(None),
    };
    if directory_identity_from_file(&source_parent).as_deref()
        != Some(expected.parent_identity.as_str())
    {
        return QuarantineOutcome::Skipped(None);
    }

    let candidate_name = match cstring_file_name(candidate_path) {
        Ok(name) => name,
        Err(error) => return failed_before_move(error.to_string()),
    };
    let quarantine = match QuarantineDirectory::create(report_root) {
        Ok(directory) => directory,
        Err(error) => return failed_before_move(error.to_string()),
    };
    let quarantined_path = quarantine.path.join("candidate");

    before_move(candidate_path);
    if let Err(error) = rename_at(
        &source_parent,
        &candidate_name,
        &quarantine.directory,
        quarantine_candidate_name(),
    ) {
        return if error.kind() == ErrorKind::NotFound {
            QuarantineOutcome::Skipped(None)
        } else {
            failed_before_move(error.to_string())
        };
    }

    let source_path_valid = source_parent_path_is_unchanged(report_root, candidate_path, expected);
    let quarantine_path_valid = quarantine.path_is_pinned(report_root);
    let quarantine_file_valid = quarantine_candidate_matches(&quarantine.directory, expected);
    let verified = source_path_valid && quarantine_path_valid && quarantine_file_valid;
    if !verified {
        return fail_after_move_without_restore(
            report_root,
            &quarantine,
            &quarantined_path,
            candidate_path,
            "격리 이동 후 검증이 실패했습니다",
        );
    }

    after_quarantine_verification(&quarantined_path);

    let still_verified = source_parent_path_is_unchanged(report_root, candidate_path, expected)
        && quarantine.path_is_pinned(report_root)
        && quarantine_candidate_matches(&quarantine.directory, expected);
    if !still_verified {
        return fail_after_move_without_restore(
            report_root,
            &quarantine,
            &quarantined_path,
            candidate_path,
            "격리 검증 후 경로 또는 객체가 변경되었습니다",
        );
    }

    match std::fs::symlink_metadata(candidate_path) {
        Err(error) if error.kind() == ErrorKind::NotFound => {}
        Ok(_) => {
            return QuarantineOutcome::Failed {
                error: format!(
                    "원래 코드 경로가 다시 생성되어 격리 완료로 보고하지 않습니다. 보존 위치: {}",
                    quarantined_path.display()
                ),
                retained_path: Some(quarantined_path),
                removed_from_code_tree: false,
            };
        }
        Err(error) => {
            return QuarantineOutcome::Failed {
                error: format!(
                    "원래 코드 경로의 부재를 확인하지 못했습니다. 보존 위치: {} ({error})",
                    quarantined_path.display()
                ),
                retained_path: Some(quarantined_path),
                removed_from_code_tree: false,
            };
        }
    }

    // The verified object remains in quarantine. Portable POSIX cannot atomically
    // bind an opened-file verification to unlinking that exact directory entry
    // against a same-credential process that mutates the quarantine namespace.
    QuarantineOutcome::Quarantined(quarantined_path)
}

#[cfg(not(unix))]
fn quarantine_candidate<F, G>(
    _report_root: &Path,
    _candidate_path: &Path,
    _expected: &FileSnapshot,
    _before_move: &mut F,
    _after_quarantine_verification: &mut G,
) -> QuarantineOutcome
where
    F: FnMut(&Path),
    G: FnMut(&Path),
{
    // Platforms without descriptor-relative stable identity support never receive
    // deletion-ready evidence. Keep this final boundary fail closed as well.
    QuarantineOutcome::Skipped(None)
}

#[cfg(unix)]
struct QuarantineDirectory {
    directory: File,
    root_path: PathBuf,
    path: PathBuf,
}

#[cfg(unix)]
impl QuarantineDirectory {
    fn create(report_root: &Path) -> std::io::Result<Self> {
        let report_root_path = std::fs::canonicalize(report_root)?;
        let report_root_directory = open_directory(&report_root_path)?;
        let state_directory =
            open_or_create_directory_at(&report_root_directory, kratos_state_directory_name())?;
        let root_directory =
            open_or_create_directory_at(&state_directory, clean_quarantine_directory_name())?;
        let root_metadata = root_directory.metadata()?;
        if root_metadata.uid() != unsafe { libc::geteuid() } || root_metadata.mode() & 0o077 != 0 {
            return Err(std::io::Error::new(
                ErrorKind::PermissionDenied,
                "clean quarantine root must be owner-only",
            ));
        }
        let root_path = report_root_path.join(".kratos/clean-quarantine");

        for _ in 0..100 {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            let counter = QUARANTINE_COUNTER.fetch_add(1, Ordering::Relaxed);
            let name = CString::new(format!(
                ".kratos-clean-quarantine-{}-{nonce}-{counter}",
                std::process::id()
            ))
            .map_err(|_| std::io::Error::new(ErrorKind::InvalidInput, "invalid quarantine name"))?;

            let created =
                unsafe { libc::mkdirat(root_directory.as_raw_fd(), name.as_ptr(), 0o700) };
            if created != 0 {
                let error = std::io::Error::last_os_error();
                if error.kind() == ErrorKind::AlreadyExists {
                    continue;
                }
                return Err(error);
            }

            match open_directory_at(&root_directory, &name) {
                Ok(directory) => {
                    let path = root_path.join(std::ffi::OsStr::from_bytes(name.to_bytes()));
                    return Ok(Self {
                        directory,
                        root_path,
                        path,
                    });
                }
                Err(error) => return Err(error),
            }
        }

        Err(std::io::Error::new(
            ErrorKind::AlreadyExists,
            "could not allocate a unique clean quarantine directory",
        ))
    }

    fn path_is_pinned(&self, report_root: &Path) -> bool {
        let descriptor_identity = directory_identity_from_file(&self.directory);
        let path_metadata = std::fs::symlink_metadata(&self.path).ok();
        let path_identity = path_metadata
            .as_ref()
            .filter(|metadata| metadata.file_type().is_dir())
            .and_then(|_| fingerprint_parent_identity(&self.path));
        let expected_root = realpath_or_fallback(report_root).join(".kratos/clean-quarantine");
        let canonical_path = std::fs::canonicalize(&self.path).ok();
        descriptor_identity.is_some()
            && descriptor_identity == path_identity
            && self.root_path == expected_root
            && canonical_path
                .as_deref()
                .and_then(Path::parent)
                .is_some_and(|parent| parent == self.root_path)
    }
}

#[cfg(unix)]
fn source_parent_path_is_unchanged(
    report_root: &Path,
    candidate_path: &Path,
    expected: &FileSnapshot,
) -> bool {
    candidate_path
        .parent()
        .and_then(fingerprint_parent_identity)
        .as_deref()
        == Some(expected.parent_identity.as_str())
        && is_safe_clean_candidate(report_root, candidate_path)
}

#[cfg(unix)]
fn quarantine_candidate_matches(directory: &File, expected: &FileSnapshot) -> bool {
    open_file_at(directory, quarantine_candidate_name())
        .and_then(inspect_open_regular_file)
        .map(|(fingerprint, identity)| {
            identity == expected.identity && fingerprint == expected.fingerprint
        })
        .unwrap_or(false)
}

#[cfg(unix)]
fn quarantine_entry_exists(directory: &File) -> bool {
    let mut metadata = std::mem::MaybeUninit::<libc::stat>::uninit();
    unsafe {
        libc::fstatat(
            directory.as_raw_fd(),
            quarantine_candidate_name().as_ptr(),
            metadata.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        ) == 0
    }
}

fn failed_before_move(error: String) -> QuarantineOutcome {
    QuarantineOutcome::Failed {
        error,
        retained_path: None,
        removed_from_code_tree: false,
    }
}

#[cfg(unix)]
fn fail_after_move_without_restore(
    report_root: &Path,
    quarantine: &QuarantineDirectory,
    quarantined_path: &Path,
    candidate_path: &Path,
    reason: &str,
) -> QuarantineOutcome {
    let retained = quarantine_entry_exists(&quarantine.directory);
    let retained_path = (retained && quarantine.path_is_pinned(report_root))
        .then(|| quarantined_path.to_path_buf());
    let removed_from_code_tree = matches!(
        std::fs::symlink_metadata(candidate_path),
        Err(error) if error.kind() == ErrorKind::NotFound
    );
    let residue = if retained_path.is_some() {
        format!(
            "마지막으로 확인된 보존 위치: {}",
            quarantined_path.display()
        )
    } else if retained {
        "격리 객체는 pinned descriptor에 남아 있지만 현재 pathname을 확인할 수 없습니다".to_string()
    } else {
        "격리 entry가 더 이상 존재하지 않습니다".to_string()
    };

    QuarantineOutcome::Failed {
        error: format!(
            "{reason}. 미검증 entry는 원래 코드 경로로 자동 복원하지 않습니다. {residue}"
        ),
        retained_path,
        removed_from_code_tree,
    }
}

#[cfg(unix)]
#[allow(clippy::manual_c_str_literals)]
fn quarantine_candidate_name() -> &'static CStr {
    CStr::from_bytes_with_nul(b"candidate\0").expect("static quarantine name is valid")
}

#[cfg(unix)]
#[allow(clippy::manual_c_str_literals)]
fn kratos_state_directory_name() -> &'static CStr {
    CStr::from_bytes_with_nul(b".kratos\0").expect("static state directory name is valid")
}

#[cfg(unix)]
#[allow(clippy::manual_c_str_literals)]
fn clean_quarantine_directory_name() -> &'static CStr {
    CStr::from_bytes_with_nul(b"clean-quarantine\0").expect("static quarantine root name is valid")
}

#[cfg(unix)]
fn open_or_create_directory_at(parent: &File, name: &CStr) -> std::io::Result<File> {
    let created = unsafe { libc::mkdirat(parent.as_raw_fd(), name.as_ptr(), 0o700) };
    if created != 0 {
        let error = std::io::Error::last_os_error();
        if error.kind() != ErrorKind::AlreadyExists {
            return Err(error);
        }
    }
    open_directory_at(parent, name)
}

#[cfg(unix)]
fn cstring_file_name(path: &Path) -> std::io::Result<CString> {
    let name = path.file_name().ok_or_else(|| {
        std::io::Error::new(ErrorKind::InvalidInput, "candidate has no file name")
    })?;
    CString::new(name.as_bytes())
        .map_err(|_| std::io::Error::new(ErrorKind::InvalidInput, "candidate name contains NUL"))
}

#[cfg(unix)]
fn open_directory(path: &Path) -> std::io::Result<File> {
    let path = CString::new(path.as_os_str().as_bytes())
        .map_err(|_| std::io::Error::new(ErrorKind::InvalidInput, "directory path contains NUL"))?;
    let fd = unsafe {
        libc::open(
            path.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
        )
    };
    file_from_fd(fd)
}

#[cfg(unix)]
fn open_directory_at(parent: &File, name: &CStr) -> std::io::Result<File> {
    let fd = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
        )
    };
    file_from_fd(fd)
}

#[cfg(unix)]
fn open_file_at(parent: &File, name: &CStr) -> std::io::Result<File> {
    let fd = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
        )
    };
    file_from_fd(fd)
}

#[cfg(unix)]
fn file_from_fd(fd: libc::c_int) -> std::io::Result<File> {
    if fd < 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(unsafe { File::from_raw_fd(fd) })
    }
}

#[cfg(unix)]
fn rename_at(
    old_parent: &File,
    old_name: &CStr,
    new_parent: &File,
    new_name: &CStr,
) -> std::io::Result<()> {
    let result = unsafe {
        libc::renameat(
            old_parent.as_raw_fd(),
            old_name.as_ptr(),
            new_parent.as_raw_fd(),
            new_name.as_ptr(),
        )
    };
    zero_or_last_error(result)
}

#[cfg(unix)]
fn zero_or_last_error(result: libc::c_int) -> std::io::Result<()> {
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
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

        assert_eq!(outcome.quarantined_files.len(), 1);
        assert_eq!(outcome.skipped_files, 0);
        assert_eq!(outcome.failed_files.len(), 1);
        assert!(!file.exists());
        assert_eq!(
            std::fs::read_to_string(&outcome.quarantined_files[0])
                .expect("replacement should remain quarantined"),
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

        assert_eq!(outcome.quarantined_files.len(), 1, "{outcome:?}");
        assert_eq!(outcome.deleted_files, 2, "{outcome:?}");
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

        assert_eq!(outcome.quarantined_files.len(), 1);
        assert_eq!(outcome.skipped_files, 0);
        assert_eq!(outcome.failed_files.len(), 1);
        assert_eq!(
            std::fs::read_to_string(&outside_file).expect("outside file should remain"),
            "outside\n"
        );
        assert!(!saved_nested.join("dead.ts").exists());
    }

    #[cfg(unix)]
    #[test]
    fn parent_relocation_after_quarantine_verification_cannot_redirect_mutation() {
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
        .expect("verified quarantine deletion should fail closed");

        assert_eq!(outcome.quarantined_files.len(), 1, "{outcome:?}");
        assert_eq!(outcome.skipped_files, 0);
        assert_eq!(outcome.failed_files.len(), 1);
        assert_eq!(
            std::fs::read_to_string(&replacement).expect("replacement should remain"),
            "replacement\n"
        );
        let preserved = std::fs::read_dir(root.join(".kratos/clean-quarantine"))
            .expect("quarantine root should remain readable")
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .find(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with(".kratos-clean-quarantine-"))
            })
            .expect("quarantine should be preserved");
        assert_eq!(
            std::fs::read_to_string(preserved.join("candidate"))
                .expect("analyzed candidate should remain quarantined"),
            "analyzed\n"
        );
    }

    #[cfg(unix)]
    #[test]
    fn redirected_parent_with_matching_hardlink_fails_without_restore() {
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
        .expect("clean should retain without restoring the redirected pathname");

        assert_eq!(outcome.quarantined_files.len(), 1);
        assert_eq!(outcome.skipped_files, 0);
        assert_eq!(outcome.failed_files.len(), 1);
        assert_eq!(
            std::fs::read_to_string(&outside_candidate).expect("outside hardlink should remain"),
            "candidate\n"
        );
        assert!(!moved_candidate.exists());
    }

    #[cfg(unix)]
    #[test]
    fn quarantine_relocation_is_not_followed_for_final_mutation() {
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
        .expect("clean should retain through the pinned quarantine descriptor");

        assert_eq!(outcome.quarantined_files.len(), 0);
        assert_eq!(outcome.skipped_files, 0);
        assert_eq!(outcome.failed_files.len(), 1);
        assert!(outcome.failed_files[0]
            .error
            .contains("현재 pathname을 확인할 수 없습니다"));
        assert_eq!(
            std::fs::read_to_string(&outside_target).expect("outside target should remain"),
            "outside\n"
        );
        assert_eq!(
            std::fs::read_to_string(outside.join("moved-quarantine/candidate"))
                .expect("verified bytes remain retained in moved quarantine"),
            "candidate\n"
        );
        assert!(!candidate.exists());
    }

    #[cfg(unix)]
    #[test]
    fn matching_quarantine_entry_is_retained_without_final_unlink() {
        let root = temp_dir("clean-quarantine-entry-race");
        let candidate = root.join("candidate.ts");
        let outside = temp_dir("clean-quarantine-entry-race-outside");
        let outside_link = outside.join("candidate-link");
        std::fs::write(&candidate, "candidate\n").expect("candidate should write");
        let report = report_for_files(&root, std::slice::from_ref(&candidate));
        let plan = plan_clean_candidates(&report, 0.0).expect("plan should build");

        let outcome = apply_clean_plan_with_hooks(
            &report,
            &plan,
            |_| {},
            |quarantined_path| {
                std::fs::hard_link(quarantined_path, &outside_link)
                    .expect("outside hardlink should exist");
                std::fs::remove_file(quarantined_path).expect("quarantine entry should move");
                std::fs::hard_link(&outside_link, quarantined_path)
                    .expect("matching quarantine entry should return");
            },
        )
        .expect("retained quarantine should succeed without unlink");

        assert_eq!(outcome.quarantined_files.len(), 1);
        assert!(outcome.failed_files.is_empty());
        assert!(!candidate.exists());
        assert_eq!(
            std::fs::read_to_string(&outside_link).expect("outside link remains"),
            "candidate\n"
        );
        assert_eq!(
            std::fs::read_to_string(&outcome.quarantined_files[0])
                .expect("quarantine entry remains"),
            "candidate\n"
        );
    }

    #[cfg(unix)]
    #[test]
    fn reintroduced_original_path_is_not_reported_as_quarantined() {
        let root = temp_dir("clean-original-path-reintroduced");
        let candidate = root.join("candidate.ts");
        std::fs::write(&candidate, "candidate\n").expect("candidate should write");
        let report = report_for_files(&root, std::slice::from_ref(&candidate));
        let plan = plan_clean_candidates(&report, 0.0).expect("plan should build");

        let outcome = apply_clean_plan_with_hooks(
            &report,
            &plan,
            |_| {},
            |quarantined_path| {
                std::fs::hard_link(quarantined_path, &candidate)
                    .expect("original pathname should be reintroduced");
            },
        )
        .expect("accounting mismatch should be reported per file");

        assert_eq!(outcome.quarantined_files.len(), 1);
        assert_eq!(outcome.failed_files.len(), 1);
        assert!(candidate.exists());
        assert!(outcome.failed_files[0]
            .error
            .contains("원래 코드 경로가 다시 생성"));
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
