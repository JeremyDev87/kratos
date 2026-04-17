use crate::error::{KratosError, KratosResult};
use crate::model::ReportV2;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CleanOutcome {
    pub deleted_files: usize,
    pub skipped_files: usize,
}

pub fn clean_from_report(_report: &ReportV2, _apply: bool) -> KratosResult<CleanOutcome> {
    Err(KratosError::not_implemented("clean::clean_from_report"))
}
