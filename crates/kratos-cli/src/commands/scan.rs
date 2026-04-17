use kratos_core::{KratosError, KratosResult};

use super::CommandSpec;

pub const NAME: &str = "scan";
pub const SPEC: CommandSpec = CommandSpec {
    name: NAME,
    summary: "Analyze a codebase and save the latest report.",
    usage: &["kratos scan [root] [--output path] [--json]"],
};

pub fn run(_args: &[String]) -> KratosResult<i32> {
    Err(KratosError::not_implemented("kratos-cli::scan"))
}
