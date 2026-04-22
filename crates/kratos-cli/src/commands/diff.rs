use std::fs;
use std::io::Write;

use clap::Parser;
use kratos_core::report::parse_report_json;
use kratos_core::report_diff::{
    diff_reports, format_diff_json, format_diff_markdown, format_diff_summary,
};
use kratos_core::{KratosError, KratosResult};

use super::{canonicalize_diff_args, resolve_report_input, write_output, CommandSpec};

pub const NAME: &str = "diff";
pub const SPEC: CommandSpec = CommandSpec {
    name: NAME,
    summary: "Compare two saved reports.",
    usage: &[
        "kratos diff [before-report-path-or-root] [after-report-path-or-root] [--format summary|json|md]",
    ],
};

#[derive(Debug, Parser)]
#[command(disable_help_flag = true, disable_version_flag = true)]
struct DiffArgs {
    #[arg(allow_hyphen_values = true)]
    before: Option<String>,
    #[arg(allow_hyphen_values = true)]
    after: Option<String>,
    #[arg(long, allow_hyphen_values = true)]
    format: Option<String>,
}

pub fn run(args: &[String], stdout: &mut dyn Write) -> KratosResult<i32> {
    let args = parse_args(args)?;
    let cwd = std::env::current_dir()?;
    let before_report_path = resolve_report_input(args.before.as_deref(), &cwd);
    let after_report_path = resolve_report_input(args.after.as_deref(), &cwd);
    let before_raw = fs::read_to_string(&before_report_path)?;
    let after_raw = fs::read_to_string(&after_report_path)?;
    let before = parse_report_json(&before_raw)?;
    let after = parse_report_json(&after_raw)?;
    let diff = diff_reports(&before, &after);
    let format = args.format.as_deref().unwrap_or("summary");

    match format {
        "summary" => write_output(
            stdout,
            &format_diff_summary(&diff, &before_report_path, &after_report_path)?,
        )?,
        "json" => write_output(
            stdout,
            &format_diff_json(&diff, &before_report_path, &after_report_path)?,
        )?,
        "md" => write_output(
            stdout,
            &format_diff_markdown(&diff, &before_report_path, &after_report_path)?,
        )?,
        other => return Err(KratosError::Config(format!("Invalid diff format: {other}"))),
    }

    Ok(0)
}

fn parse_args(args: &[String]) -> KratosResult<DiffArgs> {
    let canonical = canonicalize_diff_args(args);
    DiffArgs::try_parse_from(std::iter::once(NAME).chain(canonical.iter().map(String::as_str)))
        .map_err(|error| KratosError::Config(error.to_string().trim().to_string()))
}
