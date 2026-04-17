mod commands;

use std::env;
use std::process;

use kratos_core::KratosResult;

fn main() {
    let exit_code = match run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("Kratos (Rust preview) failed: {error}");
            1
        }
    };

    if exit_code != 0 {
        process::exit(exit_code);
    }
}

fn run() -> KratosResult<i32> {
    let mut args = env::args().skip(1);
    let Some(command) = args.next() else {
        println!("{}", commands::format_root_help());
        return Ok(0);
    };
    let argv: Vec<String> = args.collect();

    match command.as_str() {
        "--help" | "-h" | "help" => {
            println!("{}", commands::format_root_help());
            Ok(0)
        }
        other => commands::dispatch(other, &argv),
    }
}
