mod commands;

use std::io::{self, Write};

pub fn run_cli(args: &[String]) -> i32 {
    let mut stdout = io::stdout();
    let mut stderr = io::stderr();
    run_cli_with_io(args, &mut stdout, &mut stderr)
}

pub fn run_cli_with_io(args: &[String], stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    match run(args, stdout, stderr) {
        Ok(code) => code,
        Err(error) => {
            let _ = writeln!(stderr, "Kratos 실행 실패: {error}");
            1
        }
    }
}

fn run(
    args: &[String],
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> kratos_core::KratosResult<i32> {
    let Some(command) = args.first() else {
        commands::write_output(stdout, &commands::format_root_help())?;
        return Ok(0);
    };

    let argv = &args[1..];

    match command.as_str() {
        "--version" | "-V" => {
            commands::write_output(stdout, &format!("kratos {}", package_version()))?;
            Ok(0)
        }
        "--help" | "-h" | "help" => {
            commands::write_output(stdout, &commands::format_root_help())?;
            Ok(0)
        }
        other if commands::is_known_command(other) => commands::dispatch(other, argv, stdout),
        other => {
            commands::write_output(stderr, &commands::format_unknown_command(other))?;
            Ok(1)
        }
    }
}

fn package_version() -> String {
    serde_json::from_str::<serde_json::Value>(include_str!("../../../package.json"))
        .ok()
        .and_then(|manifest| {
            manifest
                .get("version")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        })
        .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string())
}
