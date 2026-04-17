use kratos_core::{KratosError, KratosResult};

use super::CommandSpec;

pub const NAME: &str = "clean";
pub const SPEC: CommandSpec = CommandSpec {
    name: NAME,
    summary: "Show deletion candidates or delete them with --apply.",
    usage: &["kratos clean [report-path-or-root] [--apply]"],
};

pub fn run(_args: &[String]) -> KratosResult<i32> {
    Err(KratosError::not_implemented("kratos-cli::clean"))
}
