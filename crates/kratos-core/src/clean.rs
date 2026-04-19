use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::error::{KratosError, KratosResult};
use crate::model::{ReportV2, REPORT_V2};
use crate::report::parse_report_json;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CleanOutcome {
    pub deleted_files: usize,
    pub skipped_files: usize,
}

pub fn clean_from_report_path(
    report_path: impl AsRef<Path>,
    apply: bool,
) -> KratosResult<CleanOutcome> {
    let report = load_clean_report(report_path)?;
    clean_from_report(&report, apply)
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

    if version != REPORT_V2 {
        return Err(KratosError::InvalidReportVersion {
            expected: REPORT_V2,
            found: version,
        });
    }

    parse_report_json(&raw)
}

pub fn clean_from_report(report: &ReportV2, apply: bool) -> KratosResult<CleanOutcome> {
    if report.version != REPORT_V2 {
        return Err(KratosError::InvalidReportVersion {
            expected: REPORT_V2,
            found: report.version,
        });
    }

    if !apply {
        return Ok(CleanOutcome {
            deleted_files: 0,
            skipped_files: report.findings.deletion_candidates.len(),
        });
    }

    let report_root_path = resolve_path(&report.root);
    let deletion_root = realpath_or_fallback(&report.root);
    let mut outcome = CleanOutcome::default();

    for candidate in &report.findings.deletion_candidates {
        let candidate_path = resolve_path(&candidate.file);

        if !is_within_directory(&report_root_path, &candidate_path) {
            outcome.skipped_files += 1;
            continue;
        }

        if !file_exists(&candidate_path) {
            outcome.skipped_files += 1;
            continue;
        }

        let candidate_parent_path = candidate_path
            .parent()
            .unwrap_or(report_root_path.as_path());
        let candidate_parent = realpath_or_fallback(candidate_parent_path);

        if !is_within_directory(&deletion_root, &candidate_parent) {
            outcome.skipped_files += 1;
            continue;
        }

        match std::fs::remove_file(&candidate_path) {
            Ok(()) => {
                remove_empty_directories(candidate_parent_path, &report_root_path)?;
                outcome.deleted_files += 1;
            }
            Err(error) if error.kind() == ErrorKind::NotFound => {
                outcome.skipped_files += 1;
            }
            Err(error) => return Err(error.into()),
        }
    }

    Ok(outcome)
}

fn file_exists(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok()
}

fn realpath_or_fallback(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| resolve_path(path))
}

fn remove_empty_directories(start_dir: &Path, stop_at: &Path) -> KratosResult<()> {
    let boundary = resolve_path(stop_at);
    let mut current = resolve_path(start_dir);

    while is_within_directory(&boundary, &current) && current != boundary {
        let mut entries = match std::fs::read_dir(&current) {
            Ok(entries) => entries,
            Err(_) => return Ok(()),
        };

        if entries.next().is_some() {
            return Ok(());
        }

        if std::fs::remove_dir(&current).is_err() {
            return Ok(());
        }

        let Some(parent) = current.parent() else {
            return Ok(());
        };
        current = parent.to_path_buf();
    }

    Ok(())
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
