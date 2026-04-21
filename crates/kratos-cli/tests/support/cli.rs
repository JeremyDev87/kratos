use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use super::fs::repo_root;

pub fn run_cli(args: &[&str]) -> Output {
    run_cli_in_dir(&repo_root(), args)
}

pub fn run_cli_in_dir(cwd: &Path, args: &[&str]) -> Output {
    Command::new(cli_binary())
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("cli command should run")
}

pub fn cli_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_kratos-cli"))
}
