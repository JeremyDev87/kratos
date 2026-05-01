use std::fs;
use std::io::Write;

use kratos_core::clean::{clean_from_report_with_min_confidence, plan_clean_candidates};
use kratos_core::config::load_clean_min_confidence;
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
        let plan = plan_clean_candidates(&report, min_confidence)?;
        write_output(stdout, &format_clean_plan(&plan))?;
        return Ok(0);
    }

    let outcome = clean_from_report_with_min_confidence(&report, min_confidence)?;
    write_output(
        stdout,
        &format!(
            "Kratos clean: 파일 {}개를 삭제했습니다.\n건너뛴 파일: {}",
            outcome.deleted_files, outcome.skipped_files
        ),
    )?;
    Ok(0)
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

fn format_clean_plan(plan: &kratos_core::clean::CleanThresholdPlan) -> String {
    let mut lines = vec![
        "Kratos clean 미리보기입니다.".to_string(),
        String::new(),
        format!("삭제 대상: {}", plan.deletion_targets.len()),
    ];

    for candidate in &plan.deletion_targets {
        lines.push(format_candidate_line(candidate));
    }

    if !plan.threshold_skipped_targets.is_empty() {
        lines.push(String::new());
        lines.push(format!(
            "신뢰도 기준 미달로 건너뛴 대상: {}",
            plan.threshold_skipped_targets.len()
        ));

        for candidate in &plan.threshold_skipped_targets {
            lines.push(format_candidate_line(candidate));
        }
    }

    lines.push(String::new());
    lines.push("삭제하려면 --apply로 다시 실행하세요.".to_string());
    lines.join("\n")
}

fn format_candidate_line(candidate: &kratos_core::model::DeletionCandidateFinding) -> String {
    format!(
        "- {} (신뢰도 {:.2}, {})",
        candidate.file.display(),
        candidate.confidence,
        display_known_reason(&candidate.reason)
    )
}
