use std::path::Path;

use crate::model::ReportV2;
use crate::report::{entrypoint_kind_to_string, import_kind_to_string, path_to_string};
use crate::KratosResult;

pub fn format_summary_report(
    report: &ReportV2,
    report_path: &Path,
    title: &str,
) -> KratosResult<String> {
    let mut lines = vec![
        title.to_string(),
        String::new(),
        "Project:".to_string(),
        format!("- Root: {}", path_to_string(&report.root)),
        format!(
            "- Generated: {}",
            report.generated_at.as_deref().unwrap_or("undefined")
        ),
        format!("- Saved report: {}", path_to_string(report_path)),
    ];
    if let Some(config_path) = &report.config_path {
        lines.push(format!("- Config: {}", format_project_path(report, config_path)));
    }
    lines.extend([
        String::new(),
        "Finding counts:".to_string(),
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
    ]);
    if report.summary.suppressed_findings > 0 {
        lines.push(format!(
            "Suppressed findings: {}",
            report.summary.suppressed_findings
        ));
    }
    lines.push(String::new());
    lines.push("Findings preview:".to_string());

    append_preview(
        &mut lines,
        "Broken imports",
        &report.findings.broken_imports,
        |item| {
            format!(
                "{} -> {} ({})",
                format_project_path(report, &item.file),
                item.source,
                import_kind_to_string(&item.kind)
            )
        },
    );
    append_preview(
        &mut lines,
        "Orphan files",
        &report.findings.orphan_files,
        |item| {
            format!(
                "{} [{:.2}] {}",
                format_project_path(report, &item.file),
                item.confidence,
                item.reason
            )
        },
    );
    append_preview(
        &mut lines,
        "Dead exports",
        &report.findings.dead_exports,
        |item| format!("{}#{}", format_project_path(report, &item.file), item.export_name),
    );
    append_preview(
        &mut lines,
        "Unused imports",
        &report.findings.unused_imports,
        |item| {
            format!(
                "{} -> {} from {}",
                format_project_path(report, &item.file),
                item.local,
                item.source
            )
        },
    );
    append_preview(
        &mut lines,
        "Route entrypoints",
        &report.findings.route_entrypoints,
        |item| {
            format!(
                "{} ({})",
                format_project_path(report, &item.file),
                entrypoint_kind_to_string(&item.kind)
            )
        },
    );
    append_preview(
        &mut lines,
        "Deletion candidates",
        &report.findings.deletion_candidates,
        |item| {
            format!(
                "{} [{:.2}] {}",
                format_project_path(report, &item.file),
                item.confidence,
                item.reason
            )
        },
    );
    lines.push(String::new());
    lines.push("Next steps:".to_string());
    lines.push(format!(
        "- kratos report {} --format md",
        format_shell_argument(&path_to_string(report_path))
    ));
    lines.push(format!(
        "- kratos clean {}",
        format_shell_argument(&path_to_string(report_path))
    ));

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
    ];
    if let Some(config_path) = &report.config_path {
        lines.push(format!("- Config: {}", format_project_path(report, config_path)));
    }
    lines.extend([
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
    ]);
    if report.summary.suppressed_findings > 0 {
        lines.push(format!(
            "- Suppressed findings: {}",
            report.summary.suppressed_findings
        ));
    }
    lines.push(String::new());
    lines.push("## Next Steps".to_string());
    lines.push(String::new());
    lines.push(format!(
        "- {}",
        format_markdown_inline_code(&format!(
            "kratos report {} --format md",
            format_shell_argument(&path_to_string(report_path))
        ))
    ));
    lines.push(format!(
        "- {}",
        format_markdown_inline_code(&format!(
            "kratos clean {}",
            format_shell_argument(&path_to_string(report_path))
        ))
    ));
    lines.push(String::new());

    push_markdown_section(
        &mut lines,
        &format!("Broken imports ({})", report.findings.broken_imports.len()),
        &report.findings.broken_imports,
        |item| {
            format!(
                "{} -> `{}` ({})",
                format_project_path(report, &item.file),
                item.source,
                import_kind_to_string(&item.kind)
            )
        },
    );
    push_markdown_section(
        &mut lines,
        &format!("Orphan files ({})", report.findings.orphan_files.len()),
        &report.findings.orphan_files,
        |item| {
            format!(
                "{} ({}, confidence {:.2})",
                format_project_path(report, &item.file),
                item.reason,
                item.confidence
            )
        },
    );
    push_markdown_section(
        &mut lines,
        &format!("Dead exports ({})", report.findings.dead_exports.len()),
        &report.findings.dead_exports,
        |item| {
            format!(
                "{} -> `{}`",
                format_project_path(report, &item.file),
                item.export_name
            )
        },
    );
    push_markdown_section(
        &mut lines,
        &format!("Unused imports ({})", report.findings.unused_imports.len()),
        &report.findings.unused_imports,
        |item| {
            format!(
                "{} -> `{}` from `{}`",
                format_project_path(report, &item.file),
                item.local,
                item.source
            )
        },
    );
    push_markdown_section(
        &mut lines,
        &format!("Route entrypoints ({})", report.findings.route_entrypoints.len()),
        &report.findings.route_entrypoints,
        |item| {
            format!(
                "{} ({})",
                format_project_path(report, &item.file),
                entrypoint_kind_to_string(&item.kind)
            )
        },
    );
    push_markdown_section(
        &mut lines,
        &format!(
            "Deletion candidates ({})",
            report.findings.deletion_candidates.len()
        ),
        &report.findings.deletion_candidates,
        |item| {
            format!(
                "{} ({}, confidence {:.2}, safe {})",
                format_project_path(report, &item.file),
                item.reason,
                item.confidence,
                item.safe
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

fn format_project_path(report: &ReportV2, path: &Path) -> String {
    path.strip_prefix(&report.root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn format_shell_argument(value: &str) -> String {
    if value.is_empty() {
        "''".to_string()
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}

fn format_markdown_inline_code(value: &str) -> String {
    let delimiter_width = value
        .split(|character| character != '`')
        .map(str::len)
        .max()
        .unwrap_or(0)
        + 1;
    let delimiter = "`".repeat(delimiter_width);

    if value.starts_with('`') || value.ends_with('`') {
        format!("{delimiter} {value} {delimiter}")
    } else {
        format!("{delimiter}{value}{delimiter}")
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
