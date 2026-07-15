use std::fs;
use std::io::Write;
use std::path::Path;

use kratos_core::clean::{clean_from_report_with_min_confidence, CleanSafetyStatus};
use kratos_core::clean_preview::{build_clean_preview, CleanPreviewItem, CleanPreviewPlan};
use kratos_core::config::load_clean_min_confidence;
use kratos_core::model::DeletionCandidateFinding;
use kratos_core::report::parse_report_json;
use kratos_core::report_format::display_known_reason;
use kratos_core::KratosResult;

use super::{parse_cli_options, resolve_report_input, write_output, CommandSpec, ParsedFlagValue};

pub const NAME: &str = "clean";
pub const SPEC: CommandSpec = CommandSpec {
    name: NAME,
    summary: "삭제 후보를 표시하거나 --apply로 삭제합니다.",
    usage: &["kratos clean [report-path-or-root] [--apply] [--min-confidence value]"],
};

#[derive(Debug, Default)]
struct CleanArgs {
    input: Option<String>,
    apply: bool,
    min_confidence: Option<f32>,
}

pub fn run(args: &[String], stdout: &mut dyn Write) -> KratosResult<i32> {
    let args = parse_args(args)?;
    let cwd = std::env::current_dir()?;
    let report_path = resolve_report_input(args.input.as_deref(), &cwd);
    let raw = fs::read_to_string(&report_path)?;
    let report = parse_report_json(&raw)?;

    if report.findings.deletion_candidates.is_empty() {
        write_output(stdout, "Kratos clean: 삭제 후보가 없습니다.")?;
        return Ok(0);
    }

    let min_confidence = match args.min_confidence {
        Some(value) => value,
        None => load_clean_min_confidence(&report.root)?,
    };

    if !args.apply {
        let plan = build_clean_preview(&report, min_confidence)?;
        write_output(stdout, &format_clean_preview_plan(&plan, &report.root))?;
        return Ok(0);
    }

    let outcome = clean_from_report_with_min_confidence(&report, min_confidence)?;
    let mut output = format!(
        "Kratos clean: 파일 {}개를 삭제했습니다.\n건너뛴 파일: {}\n실패한 파일: {}",
        outcome.deleted_files,
        outcome.skipped_files,
        outcome.failed_files.len()
    );
    for failure in &outcome.failed_files {
        output.push_str(&format!(
            "\n- {}: {}",
            relative_path(&failure.file, &report.root),
            failure.error
        ));
    }
    write_output(stdout, &output)?;
    Ok(if outcome.failed_files.is_empty() {
        0
    } else {
        1
    })
}

fn parse_args(args: &[String]) -> KratosResult<CleanArgs> {
    let parsed = parse_cli_options(args, &["min-confidence"], &["apply"]);
    if parsed.positionals.len() > 1 {
        return Err(kratos_core::KratosError::Config(
            "clean은 report-path-or-root 인자를 최대 하나만 받을 수 있습니다".to_string(),
        ));
    }

    let mut cleaned = CleanArgs {
        input: parsed.positionals.first().cloned(),
        apply: parse_apply_flag(parsed.flags.get("apply"))?,
        min_confidence: None,
    };

    if let Some(value) = parsed.flags.get("min-confidence") {
        cleaned.min_confidence = Some(parse_min_confidence(value)?);
    }

    Ok(cleaned)
}

fn parse_apply_flag(value: Option<&ParsedFlagValue>) -> KratosResult<bool> {
    match value {
        Some(ParsedFlagValue::Present) => Ok(true),
        Some(ParsedFlagValue::Value(raw)) => parse_explicit_boolean(raw),
        None => Ok(false),
    }
}

fn parse_explicit_boolean(raw: &str) -> KratosResult<bool> {
    let normalized = raw.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "" => Ok(false),
        "true" | "1" | "yes" | "on" => Ok(true),
        "false" | "0" | "no" | "off" => Ok(false),
        _ => Err(kratos_core::KratosError::Config(
            "--apply는 boolean flag이거나 명시적인 boolean 값이어야 합니다".to_string(),
        )),
    }
}

fn parse_min_confidence(value: &ParsedFlagValue) -> KratosResult<f32> {
    let raw = match value {
        ParsedFlagValue::Present => {
            return Err(kratos_core::KratosError::Config(
                "--min-confidence에는 값이 필요합니다".to_string(),
            ))
        }
        ParsedFlagValue::Value(raw) => raw.trim(),
    };

    if raw.is_empty() {
        return Err(kratos_core::KratosError::Config(
            "--min-confidence에는 값이 필요합니다".to_string(),
        ));
    }

    let parsed = raw.parse::<f32>().map_err(|_| {
        kratos_core::KratosError::Config(
            "--min-confidence는 0.0 이상 1.0 이하이어야 합니다".to_string(),
        )
    })?;

    if !parsed.is_finite() || !(0.0..=1.0).contains(&parsed) {
        return Err(kratos_core::KratosError::Config(
            "--min-confidence는 0.0 이상 1.0 이하이어야 합니다".to_string(),
        ));
    }

    Ok(parsed)
}

fn format_clean_preview_plan(plan: &CleanPreviewPlan, report_root: &Path) -> String {
    let mut lines = vec![
        "Kratos clean 미리보기입니다.".to_string(),
        String::new(),
        format!("삭제 대상: {}", plan.deletion_target_paths.len()),
    ];

    for item in plan
        .items
        .iter()
        .filter(|item| item.safety_status == CleanSafetyStatus::Ready)
    {
        lines.extend(format_preview_item(item));
    }

    let safety_skipped = plan
        .items
        .iter()
        .filter(|item| item.safety_status != CleanSafetyStatus::Ready)
        .collect::<Vec<_>>();
    if !safety_skipped.is_empty() {
        lines.push(String::new());
        lines.push(format!(
            "안전 검증으로 건너뛴 대상: {}",
            safety_skipped.len()
        ));
        for item in safety_skipped {
            lines.extend(format_preview_item(item));
        }
    }

    if !plan.threshold_skipped_targets.is_empty() {
        lines.push(String::new());
        lines.push(format!(
            "신뢰도 기준 미달로 건너뛴 대상: {}",
            plan.threshold_skipped_targets.len()
        ));

        for candidate in &plan.threshold_skipped_targets {
            lines.push(format_candidate_line(candidate, report_root));
        }
    }

    if !plan.unavailable_targets.is_empty() {
        lines.push(String::new());
        lines.push(format!(
            "사용할 수 없어 건너뛴 대상: {}",
            plan.unavailable_targets.len()
        ));

        for candidate in &plan.unavailable_targets {
            lines.push(format_candidate_line(candidate, report_root));
        }
    }

    lines.push(String::new());
    lines.push("삭제하려면 --apply로 다시 실행하세요.".to_string());
    lines.join("\n")
}

fn format_preview_item(item: &CleanPreviewItem) -> Vec<String> {
    let exists_state = if item.exists { "존재함" } else { "없음" };
    let mut lines = vec![
        format!("- {}", item.relative_path),
        format!("  신뢰도: {:.2}", item.confidence),
        format!("  사유: {}", display_known_reason(&item.reason)),
        format!(
            "  안전 상태: {}",
            display_safety_status(&item.safety_status)
        ),
        format!("  상태: {exists_state}"),
        "  미리보기:".to_string(),
    ];

    if item.preview_excerpt.is_empty() {
        lines.push("    [empty file]".to_string());
    } else {
        lines.extend(
            item.preview_excerpt
                .lines()
                .map(|line| format!("    {line}")),
        );
    }

    lines
}

fn display_safety_status(status: &CleanSafetyStatus) -> &'static str {
    match status {
        CleanSafetyStatus::Ready => "검증됨",
        CleanSafetyStatus::PathOutsideRoot => "프로젝트 루트 밖 경로",
        CleanSafetyStatus::DuplicateCandidate => "삭제 후보 경로 중복",
        CleanSafetyStatus::UnsafeFlag => "safe=false",
        CleanSafetyStatus::UnsupportedFingerprintAlgorithm => "지원하지 않는 fingerprint 알고리즘",
        CleanSafetyStatus::MissingFingerprint => "fingerprint 없음",
        CleanSafetyStatus::MissingIdentity => "파일 identity 없음",
        CleanSafetyStatus::DuplicateFingerprint => "fingerprint 중복",
        CleanSafetyStatus::FingerprintUnavailable => "현재 fingerprint 확인 불가",
        CleanSafetyStatus::FingerprintMismatch => "스캔 후 파일 변경됨",
        CleanSafetyStatus::IdentityMismatch => "스캔 후 파일 변경됨",
    }
}

fn format_candidate_line(candidate: &DeletionCandidateFinding, report_root: &Path) -> String {
    format!(
        "- {} (신뢰도 {:.2}, {})",
        relative_path(&candidate.file, report_root),
        candidate.confidence,
        display_known_reason(&candidate.reason)
    )
}

fn relative_path(file: &Path, report_root: &Path) -> String {
    file.strip_prefix(report_root)
        .map(path_to_forward_slashes)
        .unwrap_or_else(|_| path_to_forward_slashes(file))
}

fn path_to_forward_slashes(path: impl AsRef<Path>) -> String {
    path.as_ref().to_string_lossy().replace('\\', "/")
}
