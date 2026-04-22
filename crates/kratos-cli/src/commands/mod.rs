pub mod clean;
pub mod diff;
pub mod report;
pub mod scan;

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};

use kratos_core::{KratosError, KratosResult};

#[derive(Clone, Copy)]
pub struct CommandSpec {
    pub name: &'static str,
    pub summary: &'static str,
    pub usage: &'static [&'static str],
}

pub const COMMANDS: &[CommandSpec] = &[scan::SPEC, report::SPEC, diff::SPEC, clean::SPEC];
pub const DEFAULT_REPORT_RELATIVE_PATH: &str = ".kratos/latest-report.json";

pub fn dispatch(command: &str, args: &[String], stdout: &mut dyn Write) -> KratosResult<i32> {
    match command {
        scan::NAME => dispatch_command(scan::SPEC, scan::run, args, stdout),
        report::NAME => dispatch_command(report::SPEC, report::run, args, stdout),
        diff::NAME => dispatch_command(diff::SPEC, diff::run, args, stdout),
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
    let parsed = parse_cli_options(raw_args, &["output"], &["json"]);
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

    Ok(args)
}

pub fn canonicalize_report_args(raw_args: &[String]) -> Vec<String> {
    let parsed = parse_cli_options(raw_args, &["format"], &[]);
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

    args
}

pub fn canonicalize_diff_args(raw_args: &[String]) -> Vec<String> {
    let parsed = parse_cli_options(raw_args, &["format"], &[]);
    let mut args = Vec::new();

    if let Some(before) = parsed.positionals.first() {
        args.push(before.clone());
    }

    if let Some(after) = parsed.positionals.get(1) {
        args.push(after.clone());
    }

    if let Some(value) = parsed.flags.get("format").and_then(flag_value_as_string) {
        if !value.is_empty() {
            args.push("--format".to_string());
            args.push(value.to_string());
        }
    }

    args
}

pub fn canonicalize_clean_args(raw_args: &[String]) -> Vec<String> {
    let parsed = parse_cli_options(raw_args, &[], &["apply"]);
    let mut args = Vec::new();

    if let Some(input) = parsed.positionals.first() {
        args.push(input.clone());
    }

    if is_enabled_boolean_flag(parsed.flags.get("apply")) {
        args.push("--apply".to_string());
    }

    args
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
