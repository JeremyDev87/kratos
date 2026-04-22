pub mod clean;
pub mod report;
pub mod scan;

use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::path::{Path, PathBuf};

use kratos_core::{KratosError, KratosResult, ReportV2};

#[derive(Clone, Copy)]
pub struct CommandSpec {
    pub name: &'static str,
    pub summary: &'static str,
    pub usage: &'static [&'static str],
}

pub const COMMANDS: &[CommandSpec] = &[scan::SPEC, report::SPEC, clean::SPEC];
pub const DEFAULT_REPORT_RELATIVE_PATH: &str = ".kratos/latest-report.json";

pub fn dispatch(command: &str, args: &[String], stdout: &mut dyn Write) -> KratosResult<i32> {
    match command {
        scan::NAME => dispatch_command(scan::SPEC, scan::run, args, stdout),
        report::NAME => dispatch_command(report::SPEC, report::run, args, stdout),
        clean::NAME => dispatch_command(clean::SPEC, clean::run, args, stdout),
        _ => Err(KratosError::Config(format!("Unknown command: {command}"))),
    }
}

pub fn is_known_command(command: &str) -> bool {
    COMMANDS.iter().any(|entry| entry.name == command)
}

pub fn format_root_help() -> String {
    let mut lines = vec![
        "Kratos".to_string(),
        "Destroy dead code ruthlessly.".to_string(),
        String::new(),
        "Usage:".to_string(),
    ];

    for command in COMMANDS {
        for usage in command.usage {
            lines.push(format!("  {usage}"));
        }
    }

    lines.extend([String::new(), "Commands:".to_string()]);

    let max_name_length = COMMANDS
        .iter()
        .map(|command| command.name.len())
        .max()
        .unwrap_or(0);
    for command in COMMANDS {
        lines.push(format!(
            "  {:<width$}  {}",
            command.name,
            command.summary,
            width = max_name_length
        ));
    }

    lines.join("\n")
}

pub fn format_command_help(spec: CommandSpec) -> String {
    let mut lines = vec![
        "Kratos".to_string(),
        "Destroy dead code ruthlessly.".to_string(),
        String::new(),
        format!("{} command", spec.name),
        spec.summary.to_string(),
        String::new(),
        "Usage:".to_string(),
    ];

    for usage in spec.usage {
        lines.push(format!("  {usage}"));
    }

    lines.push(String::new());
    lines.push("Run `kratos --help` to see every command.".to_string());
    lines.join("\n")
}

pub fn format_unknown_command(command: &str) -> String {
    format!("Unknown command: {command}\n\n{}", format_root_help())
}

fn dispatch_command(
    spec: CommandSpec,
    runner: fn(&[String], &mut dyn Write) -> KratosResult<i32>,
    args: &[String],
    stdout: &mut dyn Write,
) -> KratosResult<i32> {
    if should_show_help(args) {
        write_output(stdout, &format_command_help(spec))?;
        return Ok(0);
    }

    runner(args, stdout)
}

fn should_show_help(args: &[String]) -> bool {
    args.iter().any(|arg| arg == "--help") || (args.len() == 1 && args[0] == "-h")
}

pub fn write_output(stream: &mut dyn Write, content: &str) -> KratosResult<()> {
    stream.write_all(content.as_bytes())?;
    if !content.ends_with('\n') {
        stream.write_all(b"\n")?;
    }
    Ok(())
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ParsedCliOptions {
    pub positionals: Vec<String>,
    pub flags: BTreeMap<String, ParsedFlagValue>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParsedFlagValue {
    Present,
    Value(String),
}

pub fn parse_cli_options(
    args: &[String],
    value_flags: &[&str],
    boolean_flags: &[&str],
) -> ParsedCliOptions {
    let mut parsed = ParsedCliOptions::default();
    let mut index = 0;

    while index < args.len() {
        let token = &args[index];

        if !token.starts_with("--") {
            parsed.positionals.push(token.clone());
            index += 1;
            continue;
        }

        let without_prefix = &token[2..];
        let mut segments = without_prefix.splitn(2, '=');
        let name = segments.next().unwrap_or_default().to_string();
        let is_boolean_flag = boolean_flags.iter().any(|flag| *flag == name);
        let expects_value = value_flags.iter().any(|flag| *flag == name);

        if let Some(inline_value) = segments.next() {
            parsed
                .flags
                .insert(name, ParsedFlagValue::Value(inline_value.to_string()));
            index += 1;
            continue;
        }

        if is_boolean_flag {
            parsed.flags.insert(name, ParsedFlagValue::Present);
            index += 1;
            continue;
        }

        let next = args.get(index + 1);
        if expects_value {
            if let Some(next_token) = next {
                if !next_token.starts_with("--") {
                    parsed
                        .flags
                        .insert(name, ParsedFlagValue::Value(next_token.clone()));
                    index += 2;
                    continue;
                }
            }

            parsed.flags.insert(name, ParsedFlagValue::Present);
            index += 1;
            continue;
        }

        if let Some(next_token) = next {
            if !next_token.starts_with("--") {
                parsed
                    .flags
                    .insert(name, ParsedFlagValue::Value(next_token.clone()));
                index += 2;
                continue;
            }
        }

        parsed.flags.insert(name, ParsedFlagValue::Present);
        index += 1;
    }

    parsed
}

pub fn canonicalize_scan_args(raw_args: &[String]) -> KratosResult<Vec<String>> {
    let parsed = parse_cli_options(raw_args, &["output", "fail-on"], &["json"]);
    let mut args = Vec::new();

    if let Some(root) = parsed.positionals.first() {
        args.push(root.clone());
    }

    if let Some(value) = parsed.flags.get("output").and_then(flag_value_as_string) {
        if !value.is_empty() {
            args.push("--output".to_string());
            args.push(value.to_string());
        }
    } else if matches!(parsed.flags.get("output"), Some(ParsedFlagValue::Present)) {
        return Err(KratosError::Config(
            "--output requires a path value".to_string(),
        ));
    }

    if is_enabled_boolean_flag(parsed.flags.get("json")) {
        args.push("--json".to_string());
    }

    if let Some(value) = required_value_flag(parsed.flags.get("fail-on"), "--fail-on")? {
        args.push("--fail-on".to_string());
        args.push(value.to_string());
    }

    Ok(args)
}

pub fn canonicalize_report_args(raw_args: &[String]) -> KratosResult<Vec<String>> {
    let parsed = parse_cli_options(raw_args, &["format", "fail-on"], &[]);
    let mut args = Vec::new();

    if let Some(input) = parsed.positionals.first() {
        args.push(input.clone());
    }

    if let Some(value) = parsed.flags.get("format").and_then(flag_value_as_string) {
        if !value.is_empty() {
            args.push("--format".to_string());
            args.push(value.to_string());
        }
    }

    if let Some(value) = required_value_flag(parsed.flags.get("fail-on"), "--fail-on")? {
        args.push("--fail-on".to_string());
        args.push(value.to_string());
    }

    Ok(args)
}

pub fn canonicalize_clean_args(raw_args: &[String]) -> KratosResult<Vec<String>> {
    let parsed = parse_cli_options(raw_args, &[], &["apply", "dry-run"]);
    let mut args = Vec::new();

    if let Some(input) = parsed.positionals.first() {
        args.push(input.clone());
    }

    if is_enabled_boolean_flag(parsed.flags.get("apply")) {
        args.push("--apply".to_string());
    }

    if is_enabled_boolean_flag(parsed.flags.get("dry-run")) {
        args.push("--dry-run".to_string());
    }

    Ok(args)
}

pub fn resolve_report_input(input: Option<&str>, cwd: &Path) -> PathBuf {
    match input {
        None => normalize_path(cwd.join(DEFAULT_REPORT_RELATIVE_PATH)),
        Some(raw) => {
            let absolute = resolve_input_path(cwd, raw);
            if absolute.to_string_lossy().ends_with(".json") {
                absolute
            } else {
                normalize_path(absolute.join(DEFAULT_REPORT_RELATIVE_PATH))
            }
        }
    }
}

pub fn resolve_input_path(base: &Path, input: &str) -> PathBuf {
    let path = Path::new(input);
    if path.is_absolute() {
        normalize_path(path.to_path_buf())
    } else {
        normalize_path(base.join(path))
    }
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

fn flag_value_as_string(value: &ParsedFlagValue) -> Option<&str> {
    match value {
        ParsedFlagValue::Present => None,
        ParsedFlagValue::Value(raw) => Some(raw.as_str()),
    }
}

fn is_enabled_boolean_flag(value: Option<&ParsedFlagValue>) -> bool {
    match value {
        Some(ParsedFlagValue::Present) => true,
        Some(ParsedFlagValue::Value(raw)) => !raw.is_empty(),
        None => false,
    }
}

fn required_value_flag<'a>(
    value: Option<&'a ParsedFlagValue>,
    flag_name: &str,
) -> KratosResult<Option<&'a str>> {
    match value {
        None => Ok(None),
        Some(ParsedFlagValue::Present) => Err(KratosError::Config(format!(
            "{flag_name} requires a value"
        ))),
        Some(ParsedFlagValue::Value(raw)) if raw.trim().is_empty() => Err(KratosError::Config(
            format!("{flag_name} requires a non-empty value"),
        )),
        Some(ParsedFlagValue::Value(raw)) => Ok(Some(raw.as_str())),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FailOnKind {
    BrokenImports,
    OrphanFiles,
    DeadExports,
    UnusedImports,
    DeletionCandidates,
}

pub fn parse_fail_on(raw: Option<&str>) -> KratosResult<Vec<FailOnKind>> {
    let Some(raw) = raw else {
        return Ok(Vec::new());
    };

    let normalized = raw.trim();
    if normalized.is_empty() || normalized.eq_ignore_ascii_case("none") {
        return Ok(Vec::new());
    }

    let mut kinds = BTreeSet::new();

    for token in normalized.split(',').map(str::trim).filter(|token| !token.is_empty()) {
        match token.to_ascii_lowercase().as_str() {
            "any" | "all" | "findings" => {
                kinds.extend([
                    FailOnKind::BrokenImports,
                    FailOnKind::OrphanFiles,
                    FailOnKind::DeadExports,
                    FailOnKind::UnusedImports,
                    FailOnKind::DeletionCandidates,
                ]);
            }
            "broken-import" | "broken-imports" | "broken_imports" => {
                kinds.insert(FailOnKind::BrokenImports);
            }
            "orphan-file" | "orphan-files" | "orphan_files" => {
                kinds.insert(FailOnKind::OrphanFiles);
            }
            "dead-export" | "dead-exports" | "dead_exports" => {
                kinds.insert(FailOnKind::DeadExports);
            }
            "unused-import" | "unused-imports" | "unused_imports" => {
                kinds.insert(FailOnKind::UnusedImports);
            }
            "deletion-candidate" | "deletion-candidates" | "deletion_candidates" => {
                kinds.insert(FailOnKind::DeletionCandidates);
            }
            other => {
                return Err(KratosError::Config(format!(
                    "Invalid --fail-on value: {other}"
                )))
            }
        }
    }

    Ok(kinds.into_iter().collect())
}

pub fn fail_on_exit_code(report: &ReportV2, fail_on: &[FailOnKind]) -> i32 {
    if gated_findings(report, fail_on).is_empty() {
        0
    } else {
        2
    }
}

pub fn render_fail_on_message(
    report: &ReportV2,
    fail_on: &[FailOnKind],
    markdown: bool,
) -> Option<String> {
    let failures = gated_findings(report, fail_on);
    if failures.is_empty() {
        return None;
    }

    let matched = failures
        .iter()
        .map(|(kind, count)| format!("{}: {}", fail_on_label(*kind), count))
        .collect::<Vec<_>>()
        .join(", ");

    if markdown {
        Some(format!(
            "## Gate Status\n\n- Result: failed\n- Matched findings: {matched}\n"
        ))
    } else {
        Some(format!("Gate status: failed\nMatched findings: {matched}"))
    }
}

fn gated_findings(report: &ReportV2, fail_on: &[FailOnKind]) -> Vec<(FailOnKind, usize)> {
    fail_on
        .iter()
        .copied()
        .filter_map(|kind| {
            let count = fail_on_count(report, kind);
            (count > 0).then_some((kind, count))
        })
        .collect()
}

fn fail_on_count(report: &ReportV2, kind: FailOnKind) -> usize {
    match kind {
        FailOnKind::BrokenImports => report.findings.broken_imports.len(),
        FailOnKind::OrphanFiles => report.findings.orphan_files.len(),
        FailOnKind::DeadExports => report.findings.dead_exports.len(),
        FailOnKind::UnusedImports => report.findings.unused_imports.len(),
        FailOnKind::DeletionCandidates => report.findings.deletion_candidates.len(),
    }
}

fn fail_on_label(kind: FailOnKind) -> &'static str {
    match kind {
        FailOnKind::BrokenImports => "broken imports",
        FailOnKind::OrphanFiles => "orphan files",
        FailOnKind::DeadExports => "dead exports",
        FailOnKind::UnusedImports => "unused imports",
        FailOnKind::DeletionCandidates => "deletion candidates",
    }
}
