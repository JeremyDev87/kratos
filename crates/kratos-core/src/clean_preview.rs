use std::fs::File;
use std::io::{BufRead, BufReader, ErrorKind};
use std::path::{Path, PathBuf};

use crate::clean::{is_safe_clean_candidate, plan_clean_candidates};
use crate::model::{DeletionCandidateFinding, ReportV2};
use crate::KratosResult;

pub const BINARY_PREVIEW_MARKER: &str = "[binary file]";
pub const MISSING_PREVIEW_MARKER: &str = "[missing file]";
pub const UNREADABLE_PREVIEW_MARKER: &str = "[preview unavailable]";
const MAX_PREVIEW_LINES: usize = 20;
const MAX_PREVIEW_BYTES: usize = 16 * 1024;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CleanPreviewItem {
    pub file: PathBuf,
    pub relative_path: String,
    pub reason: String,
    pub confidence: f32,
    pub exists: bool,
    pub preview_excerpt: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CleanPreviewPlan {
    pub items: Vec<CleanPreviewItem>,
    pub deletion_target_paths: Vec<PathBuf>,
    pub threshold_skipped_targets: Vec<DeletionCandidateFinding>,
    pub unavailable_targets: Vec<DeletionCandidateFinding>,
}

pub fn build_clean_preview(
    report: &ReportV2,
    min_confidence: f32,
) -> KratosResult<CleanPreviewPlan> {
    let threshold_plan = plan_clean_candidates(report, min_confidence)?;
    let mut items = Vec::new();
    let mut unavailable_targets = Vec::new();

    for candidate in &threshold_plan.deletion_targets {
        if !is_safe_clean_candidate(&report.root, &candidate.file) {
            unavailable_targets.push(candidate.clone());
            continue;
        }

        items.push(build_preview_item(candidate, &report.root));
    }

    items.sort_by(|left, right| {
        right
            .confidence
            .total_cmp(&left.confidence)
            .then_with(|| left.relative_path.cmp(&right.relative_path))
    });

    Ok(CleanPreviewPlan {
        deletion_target_paths: items.iter().map(|item| item.file.clone()).collect(),
        items,
        threshold_skipped_targets: threshold_plan.threshold_skipped_targets,
        unavailable_targets,
    })
}

fn build_preview_item(
    candidate: &DeletionCandidateFinding,
    report_root: &Path,
) -> CleanPreviewItem {
    let (exists, preview_excerpt) = read_preview_excerpt(&candidate.file);

    CleanPreviewItem {
        file: candidate.file.clone(),
        relative_path: to_project_relative_path(&candidate.file, report_root),
        reason: candidate.reason.clone(),
        confidence: candidate.confidence,
        exists,
        preview_excerpt,
    }
}

fn read_preview_excerpt(path: &Path) -> (bool, String) {
    if let Ok(metadata) = std::fs::symlink_metadata(path) {
        if metadata.file_type().is_symlink() {
            return read_symlink_preview_excerpt(path);
        }
    }

    match File::open(path) {
        Ok(file) => match preview_from_reader(BufReader::new(file)) {
            Ok(preview_excerpt) => (true, preview_excerpt),
            Err(_) => (
                std::fs::symlink_metadata(path).is_ok(),
                UNREADABLE_PREVIEW_MARKER.to_string(),
            ),
        },
        Err(error) if error.kind() == ErrorKind::NotFound => {
            (false, MISSING_PREVIEW_MARKER.to_string())
        }
        Err(_) => (
            std::fs::symlink_metadata(path).is_ok(),
            UNREADABLE_PREVIEW_MARKER.to_string(),
        ),
    }
}

fn read_symlink_preview_excerpt(path: &Path) -> (bool, String) {
    if std::fs::symlink_metadata(path).is_ok() {
        (true, UNREADABLE_PREVIEW_MARKER.to_string())
    } else {
        (false, MISSING_PREVIEW_MARKER.to_string())
    }
}

fn preview_from_reader<R: BufRead>(mut reader: R) -> std::io::Result<String> {
    let mut excerpt = Vec::new();
    let mut lines = 0usize;

    while lines < MAX_PREVIEW_LINES && excerpt.len() < MAX_PREVIEW_BYTES {
        let buffer = reader.fill_buf()?;
        if buffer.is_empty() {
            break;
        }

        let remaining = MAX_PREVIEW_BYTES - excerpt.len();
        let candidate = &buffer[..buffer.len().min(remaining)];

        let consumed = match candidate.iter().position(|byte| *byte == b'\n') {
            Some(index) => {
                lines += 1;
                index + 1
            }
            None => candidate.len(),
        };

        if candidate[..consumed].contains(&0) {
            return Ok(BINARY_PREVIEW_MARKER.to_string());
        }

        excerpt.extend_from_slice(&candidate[..consumed]);
        reader.consume(consumed);
    }

    match String::from_utf8(excerpt) {
        Ok(text) => Ok(text
            .lines()
            .take(MAX_PREVIEW_LINES)
            .collect::<Vec<_>>()
            .join("\n")),
        Err(error) => {
            let utf8_error = error.utf8_error();

            if utf8_error.error_len().is_none() {
                let valid_prefix = error.into_bytes();
                let text = std::str::from_utf8(&valid_prefix[..utf8_error.valid_up_to()])
                    .expect("valid UTF-8 prefix should decode");
                Ok(text
                    .lines()
                    .take(MAX_PREVIEW_LINES)
                    .collect::<Vec<_>>()
                    .join("\n"))
            } else {
                Ok(BINARY_PREVIEW_MARKER.to_string())
            }
        }
    }
}

fn to_project_relative_path(file: &Path, report_root: &Path) -> String {
    file.strip_prefix(report_root)
        .map(path_to_forward_slashes)
        .unwrap_or_else(|_| path_to_forward_slashes(file))
}

fn path_to_forward_slashes(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
