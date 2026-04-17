use kratos_core::{KratosError, KratosResult};

use super::CommandSpec;

pub const NAME: &str = "report";
pub const SPEC: CommandSpec = CommandSpec {
    name: NAME,
    summary: "Print a saved report in summary, json, or markdown form.",
    usage: &["kratos report [report-path-or-root] [--format summary|json|md]"],
};

pub fn run(_args: &[String]) -> KratosResult<i32> {
    Err(KratosError::not_implemented("kratos-cli::report"))
}
