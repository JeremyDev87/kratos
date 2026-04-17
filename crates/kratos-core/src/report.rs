use std::path::Path;

use crate::error::{KratosError, KratosResult};
use crate::model::{ReportV2, REPORT_V2};

pub fn validate_report_version(report: &ReportV2) -> KratosResult<()> {
    if report.version != REPORT_V2 {
        return Err(KratosError::InvalidReportVersion {
            expected: REPORT_V2,
            found: report.version,
        });
    }

    Ok(())
}

pub fn serialize_report_pretty(_report: &ReportV2) -> KratosResult<String> {
    Err(KratosError::not_implemented(
        "report::serialize_report_pretty",
    ))
}

pub fn parse_report_json(_raw: &str) -> KratosResult<ReportV2> {
    Err(KratosError::not_implemented("report::parse_report_json"))
}

pub fn format_summary_report(_report: &ReportV2, _report_path: &Path) -> KratosResult<String> {
    Err(KratosError::not_implemented(
        "report::format_summary_report",
    ))
}

pub fn format_markdown_report(_report: &ReportV2, _report_path: &Path) -> KratosResult<String> {
    Err(KratosError::not_implemented(
        "report::format_markdown_report",
    ))
}
