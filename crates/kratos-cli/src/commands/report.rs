use std::fs;
use std::io::Write;

use clap::Parser;
use kratos_core::report::parse_report_json;
use kratos_core::report_format::{format_markdown_report, format_summary_report};
use kratos_core::{KratosError, KratosResult};
use serde_json::Value;

use super::{
    canonicalize_report_args, fail_on_exit_code, parse_fail_on, render_fail_on_message,
    resolve_report_input, write_output, CommandSpec,
};

pub const NAME: &str = "report";
pub const SPEC: CommandSpec = CommandSpec {
    name: NAME,
    summary: "Print a saved report in summary, json, or markdown form.",
    usage: &["kratos report [report-path-or-root] [--format summary|json|md] [--fail-on kinds]"],
};

#[derive(Debug, Parser)]
#[command(disable_help_flag = true, disable_version_flag = true)]
struct ReportArgs {
    #[arg(allow_hyphen_values = true)]
    input: Option<String>,
    #[arg(long, allow_hyphen_values = true)]
    format: Option<String>,
    #[arg(long = "fail-on", allow_hyphen_values = true)]
    fail_on: Option<String>,
}

pub fn run(args: &[String], stdout: &mut dyn Write) -> KratosResult<i32> {
    let args = parse_args(args)?;
    let cwd = std::env::current_dir()?;
    let report_path = resolve_report_input(args.input.as_deref(), &cwd);
    let raw = fs::read_to_string(&report_path)?;
    let raw_json: Value =
        serde_json::from_str(&raw).map_err(|error| KratosError::Json(error.to_string()))?;
    let format = args.format.as_deref().unwrap_or("summary");
    let fail_on = parse_fail_on(args.fail_on.as_deref())?;
    let mut exit_code = 0;

    match format {
        "summary" => {
            let report = parse_report_json(&raw)?;
            write_output(
                stdout,
                &format_summary_report(&report, &report_path, "Kratos report.")?,
            )?;
            if let Some(message) = render_fail_on_message(&report, &fail_on, false) {
                write_output(stdout, &message)?;
            }
            exit_code = fail_on_exit_code(&report, &fail_on);
        }
        "json" => write_output(
            stdout,
            &serde_json::to_string_pretty(&raw_json)
                .map_err(|error| KratosError::Json(error.to_string()))?,
        )?,
        "md" => {
            let report = parse_report_json(&raw)?;
            write_output(stdout, &format_markdown_report(&report, &report_path)?)?;
            if let Some(message) = render_fail_on_message(&report, &fail_on, true) {
                write_output(stdout, &message)?;
            }
            exit_code = fail_on_exit_code(&report, &fail_on);
        }
        other => {
            return Err(KratosError::Config(format!(
                "Invalid report format: {other}"
            )))
        }
    }

    if format == "json" && !fail_on.is_empty() {
        let report = parse_report_json(&raw)?;
        exit_code = fail_on_exit_code(&report, &fail_on);
    }

    Ok(exit_code)
}

fn parse_args(args: &[String]) -> KratosResult<ReportArgs> {
    let canonical = canonicalize_report_args(args)?;
    ReportArgs::try_parse_from(std::iter::once(NAME).chain(canonical.iter().map(String::as_str)))
        .map_err(|error| KratosError::Config(error.to_string().trim().to_string()))
}
