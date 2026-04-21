use std::path::Path;

use crate::model::ReportV2;
use crate::report::{entrypoint_kind_to_string, path_to_string};
use crate::KratosResult;

pub fn format_summary_report(
    report: &ReportV2,
    report_path: &Path,
    title: &str,
) -> KratosResult<String> {
    let mut lines = vec![
        title.to_string(),
        String::new(),
        format!("Root: {}", path_to_string(&report.root)),
        format!("Files scanned: {}", report.summary.files_scanned),
        format!("Entrypoints: {}", report.summary.entrypoints),
        format!("Broken imports: {}", report.summary.broken_imports),
        format!("Orphan files: {}", report.summary.orphan_files),
        format!("Dead exports: {}", report.summary.dead_exports),
        format!("Unused imports: {}", report.summary.unused_imports),
        format!("Route entrypoints: {}", report.summary.route_entrypoints),
        format!(
            "Deletion candidates: {}",
            report.summary.deletion_candidates
        ),
    ];
    if report.summary.suppressed_findings > 0 {
        lines.push(format!(
            "Suppressed findings: {}",
            report.summary.suppressed_findings
        ));
    }
    lines.push(String::new());
    lines.push(format!("Saved report: {}", path_to_string(report_path)));

    append_preview(
        &mut lines,
        "Broken imports",
        &report.findings.broken_imports,
        |item| format!("{} -> {}", path_to_string(&item.file), item.source),
    );
    append_preview(
        &mut lines,
        "Orphan files",
        &report.findings.orphan_files,
        |item| path_to_string(&item.file),
    );
    append_preview(
        &mut lines,
        "Dead exports",
        &report.findings.dead_exports,
        |item| format!("{}#{}", path_to_string(&item.file), item.export_name),
    );
    append_preview(
        &mut lines,
        "Route entrypoints",
        &report.findings.route_entrypoints,
        |item| {
            format!(
                "{} ({})",
                path_to_string(&item.file),
                entrypoint_kind_to_string(&item.kind)
            )
        },
    );

    Ok(lines.join("\n"))
}

pub fn format_markdown_report(report: &ReportV2, report_path: &Path) -> KratosResult<String> {
    let mut lines = vec![
        "# Kratos Report".to_string(),
        String::new(),
        format!(
            "- Generated: {}",
            report.generated_at.as_deref().unwrap_or("undefined")
        ),
        format!("- Root: {}", path_to_string(&report.root)),
        format!("- Report: {}", path_to_string(report_path)),
        String::new(),
        "## Summary".to_string(),
        String::new(),
        format!("- Files scanned: {}", report.summary.files_scanned),
        format!("- Entrypoints: {}", report.summary.entrypoints),
        format!("- Broken imports: {}", report.summary.broken_imports),
        format!("- Orphan files: {}", report.summary.orphan_files),
        format!("- Dead exports: {}", report.summary.dead_exports),
        format!("- Unused imports: {}", report.summary.unused_imports),
        format!("- Route entrypoints: {}", report.summary.route_entrypoints),
        format!(
            "- Deletion candidates: {}",
            report.summary.deletion_candidates
        ),
    ];
    if report.summary.suppressed_findings > 0 {
        lines.push(format!(
            "- Suppressed findings: {}",
            report.summary.suppressed_findings
        ));
    }
    lines.push(String::new());

    push_markdown_section(
        &mut lines,
        "Broken imports",
        &report.findings.broken_imports,
        |item| format!("{} -> `{}`", path_to_string(&item.file), item.source),
    );
    push_markdown_section(
        &mut lines,
        "Orphan files",
        &report.findings.orphan_files,
        |item| format!("{} ({})", path_to_string(&item.file), item.reason),
    );
    push_markdown_section(
        &mut lines,
        "Dead exports",
        &report.findings.dead_exports,
        |item| format!("{} -> `{}`", path_to_string(&item.file), item.export_name),
    );
    push_markdown_section(
        &mut lines,
        "Unused imports",
        &report.findings.unused_imports,
        |item| {
            format!(
                "{} -> `{}` from `{}`",
                path_to_string(&item.file),
                item.local,
                item.source
            )
        },
    );
    push_markdown_section(
        &mut lines,
        "Route entrypoints",
        &report.findings.route_entrypoints,
        |item| {
            format!(
                "{} ({})",
                path_to_string(&item.file),
                entrypoint_kind_to_string(&item.kind)
            )
        },
    );
    push_markdown_section(
        &mut lines,
        "Deletion candidates",
        &report.findings.deletion_candidates,
        |item| {
            format!(
                "{} ({}, confidence {})",
                path_to_string(&item.file),
                item.reason,
                item.confidence
            )
        },
    );

    Ok(lines.join("\n"))
}

fn append_preview<T>(
    lines: &mut Vec<String>,
    label: &str,
    items: &[T],
    render: impl Fn(&T) -> String,
) {
    if items.is_empty() {
        return;
    }

    lines.push(String::new());
    lines.push(format!("{label}:"));

    for item in items.iter().take(5) {
        lines.push(format!("- {}", render(item)));
    }

    if items.len() > 5 {
        lines.push(format!("- ...and {} more", items.len() - 5));
    }
}

fn push_markdown_section<T>(
    lines: &mut Vec<String>,
    title: &str,
    items: &[T],
    render: impl Fn(&T) -> String,
) {
    lines.push(format!("## {title}"));
    lines.push(String::new());

    if items.is_empty() {
        lines.push("- None".to_string());
        lines.push(String::new());
        return;
    }

    for item in items {
        lines.push(format!("- {}", render(item)));
    }

    lines.push(String::new());
}
