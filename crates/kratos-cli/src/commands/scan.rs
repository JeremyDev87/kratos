use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use clap::Parser;
use kratos_core::analyze::analyze_project;
use kratos_core::report::serialize_report_pretty;
use kratos_core::report_format::format_summary_report;
use kratos_core::{KratosError, KratosResult};

use super::{
    canonicalize_scan_args, fail_on_exit_code, parse_fail_on, render_fail_on_message,
    resolve_input_path, write_output, CommandSpec,
    DEFAULT_REPORT_RELATIVE_PATH,
};

pub const NAME: &str = "scan";
pub const SPEC: CommandSpec = CommandSpec {
    name: NAME,
    summary: "Analyze a codebase and save the latest report.",
    usage: &["kratos scan [root] [--output path] [--json] [--fail-on kinds]"],
};

#[derive(Debug, Parser)]
#[command(disable_help_flag = true, disable_version_flag = true)]
struct ScanArgs {
    #[arg(allow_hyphen_values = true)]
    root: Option<String>,
    #[arg(long, allow_hyphen_values = true)]
    output: Option<String>,
    #[arg(long)]
    json: bool,
    #[arg(long = "fail-on", allow_hyphen_values = true)]
    fail_on: Option<String>,
}

pub fn run(args: &[String], stdout: &mut dyn Write) -> KratosResult<i32> {
    let args = parse_args(args)?;
    let cwd = std::env::current_dir()?;
    let root = match args.root.as_deref() {
        Some(raw) => resolve_input_path(&cwd, raw),
        None => cwd,
    };
    let output_path = resolve_output_path(&root, args.output.as_deref());
    let report = analyze_project(&root)?;
    let serialized = serialize_report_pretty(&report)?;
    let fail_on = parse_fail_on(args.fail_on.as_deref())?;

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&output_path, format!("{serialized}\n"))?;

    if args.json {
        write_output(stdout, &serialized)?;
        return Ok(fail_on_exit_code(&report, &fail_on));
    }

    write_output(
        stdout,
        &format_summary_report(&report, &output_path, "Kratos scan complete.")?,
    )?;

    if let Some(message) = render_fail_on_message(&report, &fail_on, false) {
        write_output(stdout, &message)?;
    }

    Ok(fail_on_exit_code(&report, &fail_on))
}

fn parse_args(args: &[String]) -> KratosResult<ScanArgs> {
    let canonical = canonicalize_scan_args(args)?;
    ScanArgs::try_parse_from(std::iter::once(NAME).chain(canonical.iter().map(String::as_str)))
        .map_err(|error| KratosError::Config(error.to_string().trim().to_string()))
}

fn resolve_output_path(root: &Path, output_flag: Option<&str>) -> PathBuf {
    match output_flag {
        Some(raw) => resolve_input_path(root, raw),
        None => resolve_input_path(root, DEFAULT_REPORT_RELATIVE_PATH),
    }
}
