use std::fs;
use std::io::Write;

use clap::Parser;
use kratos_core::clean::clean_from_report;
use kratos_core::report::parse_report_json;
use kratos_core::{KratosError, KratosResult};

use super::{canonicalize_clean_args, resolve_report_input, write_output, CommandSpec};

pub const NAME: &str = "clean";
pub const SPEC: CommandSpec = CommandSpec {
    name: NAME,
    summary: "Show deletion candidates or delete them with --apply.",
    usage: &["kratos clean [report-path-or-root] [--dry-run|--apply]"],
};

#[derive(Debug, Parser)]
#[command(disable_help_flag = true, disable_version_flag = true)]
struct CleanArgs {
    #[arg(allow_hyphen_values = true)]
    input: Option<String>,
    #[arg(long)]
    apply: bool,
    #[arg(long = "dry-run")]
    dry_run: bool,
}

pub fn run(args: &[String], stdout: &mut dyn Write) -> KratosResult<i32> {
    let args = parse_args(args)?;
    if args.apply && args.dry_run {
        return Err(KratosError::Config(
            "--apply and --dry-run cannot be used together".to_string(),
        ));
    }

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
        let mut lines = vec![
            "Kratos clean dry run.".to_string(),
            String::new(),
            format!("- Report: {}", report_path.display()),
            format!("- Candidates: {}", candidates.len()),
            String::new(),
            "Would delete:".to_string(),
        ];
        for candidate in candidates {
            lines.push(format!(
                "- {} ({}, confidence {:.2})",
                format_project_path(&report.root, &candidate.file),
                candidate.reason,
                candidate.confidence
            ));
        }
        lines.push(String::new());
        lines.push("Re-run with --apply to delete these files.".to_string());
        lines.push(format!(
            "Re-run with `kratos clean {} --apply` to delete these files.",
            report_path.display()
        ));
        write_output(stdout, &lines.join("\n"))?;
        return Ok(0);
    }

    let outcome = clean_from_report(&report, true)?;
    let mut lines = vec![
        format!("Kratos clean deleted {} file(s).", outcome.deleted_files),
        String::new(),
        format!("- Deleted: {}", outcome.deleted_files),
        format!("- Skipped: {}", outcome.skipped_files),
    ];
    if !outcome.deleted_paths.is_empty() {
        lines.push(String::new());
        lines.push("Deleted files:".to_string());
        for path in &outcome.deleted_paths {
            lines.push(format!("- {}", format_project_path(&report.root, path)));
        }
    }
    if !outcome.skipped_paths.is_empty() {
        lines.push(String::new());
        lines.push("Skipped files:".to_string());
        for path in &outcome.skipped_paths {
            lines.push(format!("- {}", format_project_path(&report.root, path)));
        }
    }
    write_output(stdout, &lines.join("\n"))?;
    Ok(0)
}

fn parse_args(args: &[String]) -> KratosResult<CleanArgs> {
    let canonical = canonicalize_clean_args(args)?;
    CleanArgs::try_parse_from(std::iter::once(NAME).chain(canonical.iter().map(String::as_str)))
        .map_err(|error| kratos_core::KratosError::Config(error.to_string().trim().to_string()))
}

fn format_project_path(root: &std::path::Path, path: &std::path::Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}
