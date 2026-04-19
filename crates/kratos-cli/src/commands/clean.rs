use std::fs;
use std::io::Write;

use clap::Parser;
use kratos_core::clean::clean_from_report;
use kratos_core::report::parse_report_json;
use kratos_core::KratosResult;

use super::{canonicalize_clean_args, resolve_report_input, write_output, CommandSpec};

pub const NAME: &str = "clean";
pub const SPEC: CommandSpec = CommandSpec {
    name: NAME,
    summary: "Show deletion candidates or delete them with --apply.",
    usage: &["kratos clean [report-path-or-root] [--apply]"],
};

#[derive(Debug, Parser)]
#[command(disable_help_flag = true, disable_version_flag = true)]
struct CleanArgs {
    #[arg(allow_hyphen_values = true)]
    input: Option<String>,
    #[arg(long)]
    apply: bool,
}

pub fn run(args: &[String], stdout: &mut dyn Write) -> KratosResult<i32> {
    let args = parse_args(args)?;
    let cwd = std::env::current_dir()?;
    let report_path = resolve_report_input(args.input.as_deref(), &cwd);
    let raw = fs::read_to_string(&report_path)?;
    let report = parse_report_json(&raw)?;
    let candidates = &report.findings.deletion_candidates;

    if candidates.is_empty() {
        write_output(stdout, "Kratos clean found no deletion candidates.")?;
        return Ok(0);
    }

    if !args.apply {
        let mut lines = vec!["Kratos clean dry run.".to_string(), String::new()];
        for candidate in candidates {
            lines.push(format!(
                "- {} ({})",
                candidate.file.display(),
                candidate.reason
            ));
        }
        lines.push(String::new());
        lines.push("Re-run with --apply to delete these files.".to_string());
        write_output(stdout, &lines.join("\n"))?;
        return Ok(0);
    }

    let outcome = clean_from_report(&report, true)?;
    write_output(
        stdout,
        &format!("Kratos clean deleted {} file(s).", outcome.deleted_files),
    )?;
    Ok(0)
}

fn parse_args(args: &[String]) -> KratosResult<CleanArgs> {
    let canonical = canonicalize_clean_args(args);
    CleanArgs::try_parse_from(std::iter::once(NAME).chain(canonical.iter().map(String::as_str)))
        .map_err(|error| kratos_core::KratosError::Config(error.to_string().trim().to_string()))
}
