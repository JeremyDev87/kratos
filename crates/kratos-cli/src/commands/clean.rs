use std::fs;
use std::io::Write;

use kratos_core::clean::{clean_from_report_with_min_confidence, plan_clean_candidates};
use kratos_core::config::load_clean_min_confidence;
use kratos_core::report::parse_report_json;
use kratos_core::KratosResult;

use super::{parse_cli_options, resolve_report_input, write_output, CommandSpec, ParsedFlagValue};

pub const NAME: &str = "clean";
pub const SPEC: CommandSpec = CommandSpec {
    name: NAME,
    summary: "Show deletion candidates or delete them with --apply.",
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
        write_output(stdout, "Kratos clean found no deletion candidates.")?;
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
            "Kratos clean deleted {} file(s).\nskipped_files: {}",
            outcome.deleted_files, outcome.skipped_files
        ),
    )?;
    Ok(0)
}

fn parse_args(args: &[String]) -> KratosResult<CleanArgs> {
    let parsed = parse_cli_options(args, &["min-confidence"], &["apply"]);
    if parsed.positionals.len() > 1 {
        return Err(kratos_core::KratosError::Config(
            "clean accepts at most one report-path-or-root argument".to_string(),
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
            "--apply must be a boolean flag or an explicit boolean value".to_string(),
        )),
    }
}

fn parse_min_confidence(value: &ParsedFlagValue) -> KratosResult<f32> {
    let raw = match value {
        ParsedFlagValue::Present => {
            return Err(kratos_core::KratosError::Config(
                "--min-confidence requires a value".to_string(),
            ))
        }
        ParsedFlagValue::Value(raw) => raw.trim(),
    };

    if raw.is_empty() {
        return Err(kratos_core::KratosError::Config(
            "--min-confidence requires a value".to_string(),
        ));
    }

    let parsed = raw.parse::<f32>().map_err(|_| {
        kratos_core::KratosError::Config("--min-confidence must be between 0.0 and 1.0".to_string())
    })?;

    if !parsed.is_finite() || !(0.0..=1.0).contains(&parsed) {
        return Err(kratos_core::KratosError::Config(
            "--min-confidence must be between 0.0 and 1.0".to_string(),
        ));
    }

    Ok(parsed)
}

fn format_clean_plan(plan: &kratos_core::clean::CleanThresholdPlan) -> String {
    let mut lines = vec![
        "Kratos clean dry run.".to_string(),
        String::new(),
        format!("Deletion targets: {}", plan.deletion_targets.len()),
    ];

    for candidate in &plan.deletion_targets {
        lines.push(format_candidate_line(candidate));
    }

    if !plan.threshold_skipped_targets.is_empty() {
        lines.push(String::new());
        lines.push(format!(
            "Threshold-skipped targets: {}",
            plan.threshold_skipped_targets.len()
        ));

        for candidate in &plan.threshold_skipped_targets {
            lines.push(format_candidate_line(candidate));
        }
    }

    lines.push(String::new());
    lines.push("Re-run with --apply to delete these files.".to_string());
    lines.join("\n")
}

fn format_candidate_line(candidate: &kratos_core::model::DeletionCandidateFinding) -> String {
    format!(
        "- {} (confidence {:.2}, {})",
        candidate.file.display(),
        candidate.confidence,
        candidate.reason
    )
}
