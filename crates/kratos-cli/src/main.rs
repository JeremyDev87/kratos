mod commands;

use std::env;
use std::io::{self, Write};
use std::process;

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let mut stdout = io::stdout();
    let mut stderr = io::stderr();
    let exit_code = run_cli_with_io(&args, &mut stdout, &mut stderr);
    process::exit(exit_code);
}

pub fn run_cli(args: &[String]) -> i32 {
    let mut stdout = io::stdout();
    let mut stderr = io::stderr();
    run_cli_with_io(args, &mut stdout, &mut stderr)
}

fn run_cli_with_io(args: &[String], stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    match run(args, stdout, stderr) {
        Ok(code) => code,
        Err(error) => {
            let _ = writeln!(stderr, "Kratos failed: {error}");
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
