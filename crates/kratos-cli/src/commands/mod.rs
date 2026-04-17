pub mod clean;
pub mod report;
pub mod scan;

use kratos_core::{KratosError, KratosResult};

#[derive(Clone, Copy)]
pub struct CommandSpec {
    pub name: &'static str,
    pub summary: &'static str,
    pub usage: &'static [&'static str],
}

pub const COMMANDS: &[CommandSpec] = &[scan::SPEC, report::SPEC, clean::SPEC];

pub fn dispatch(command: &str, args: &[String]) -> KratosResult<i32> {
    match command {
        scan::NAME => dispatch_command(scan::SPEC, scan::run, args),
        report::NAME => dispatch_command(report::SPEC, report::run, args),
        clean::NAME => dispatch_command(clean::SPEC, clean::run, args),
        _ => Err(KratosError::Config(format!("Unknown command: {command}"))),
    }
}

pub fn format_root_help() -> String {
    let mut lines = vec![
        "Kratos (Rust preview)".to_string(),
        "Destroy dead code ruthlessly.".to_string(),
        String::new(),
        "Usage:".to_string(),
        "  kratos <command> [options]".to_string(),
        String::new(),
        "Commands:".to_string(),
    ];

    for command in COMMANDS {
        lines.push(format!("  {:<7} {}", command.name, command.summary));
    }

    lines.push(String::new());
    lines.push("Use `kratos <command> --help` for command details.".to_string());
    lines.join("\n")
}

pub fn format_command_help(spec: CommandSpec) -> String {
    let mut lines = vec![format!("kratos {}", spec.name), spec.summary.to_string(), String::new()];

    for usage in spec.usage {
        lines.push(format!("Usage: {usage}"));
    }

    lines.join("\n")
}

fn dispatch_command(
    spec: CommandSpec,
    runner: fn(&[String]) -> KratosResult<i32>,
    args: &[String],
) -> KratosResult<i32> {
    if should_show_help(args) {
        println!("{}", format_command_help(spec));
        return Ok(0);
    }

    runner(args)
}

fn should_show_help(args: &[String]) -> bool {
    args.iter().any(|arg| arg == "--help") || (args.len() == 1 && args[0] == "-h")
}
