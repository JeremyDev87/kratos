use std::path::Path;

use crate::model::ReportV2;
use crate::report::{entrypoint_kind_to_string, path_to_string};
use crate::KratosResult;

pub fn format_summary_report(
    report: &ReportV2,
    report_path: &Path,
    title: &str,
) -> KratosResult<String> {
    let impact = impact_summary(report);
    let mut lines = vec![
        title.to_string(),
        String::new(),
        format!("Impact: {}", impact.headline),
        format!("Best next move: {}", impact.best_next_move),
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
    append_next_steps(&mut lines, report, report_path);

    append_preview(
        &mut lines,
        "Broken imports",
        &report.findings.broken_imports,
        |item| format!("{} -> {}", display_path(report, &item.file), item.source),
    );
    append_deletion_preview(&mut lines, report);
    append_preview(
        &mut lines,
        "Orphan files",
        &report.findings.orphan_files,
        |item| display_path(report, &item.file),
    );
    append_preview(
        &mut lines,
        "Dead exports",
        &report.findings.dead_exports,
        |item| format!("{}#{}", display_path(report, &item.file), item.export_name),
    );
    append_preview(
        &mut lines,
        "Unused imports",
        &report.findings.unused_imports,
        |item| {
            format!(
                "{} -> {} from {}",
                display_path(report, &item.file),
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
                display_path(report, &item.file),
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
        format!("> {}", impact_summary(report).headline),
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

    push_markdown_impact(&mut lines, report, report_path);
    push_markdown_section(
        &mut lines,
        "Broken imports",
        &report.findings.broken_imports,
        |item| format!("{} -> `{}`", display_path(report, &item.file), item.source),
    );
    push_markdown_section(
        &mut lines,
        "Orphan files",
        &report.findings.orphan_files,
        |item| format!("{} ({})", display_path(report, &item.file), item.reason),
    );
    push_markdown_section(
        &mut lines,
        "Dead exports",
        &report.findings.dead_exports,
        |item| {
            format!(
                "{} -> `{}`",
                display_path(report, &item.file),
                item.export_name
            )
        },
    );
    push_markdown_section(
        &mut lines,
        "Unused imports",
        &report.findings.unused_imports,
        |item| {
            format!(
                "{} -> `{}` from `{}`",
                display_path(report, &item.file),
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
                display_path(report, &item.file),
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
                display_path(report, &item.file),
                item.reason,
                item.confidence
            )
        },
    );

    Ok(lines.join("\n"))
}

struct ImpactSummary {
    headline: String,
    best_next_move: String,
}

fn impact_summary(report: &ReportV2) -> ImpactSummary {
    let breakages = report.summary.broken_imports;
    let cleanup_candidates = report.summary.deletion_candidates;
    let dead_exports = report.summary.dead_exports;
    let unused_imports = report.summary.unused_imports;
    let actionable_total = breakages + cleanup_candidates + dead_exports + unused_imports;

    let headline = if actionable_total == 0 {
        format!(
            "No actionable findings across {}.",
            plural(
                report.summary.files_scanned,
                "scanned file",
                "scanned files"
            )
        )
    } else {
        let mut parts = Vec::new();
        if breakages > 0 {
            parts.push(plural(breakages, "broken import", "broken imports"));
        }
        if cleanup_candidates > 0 {
            parts.push(plural(
                cleanup_candidates,
                "cleanup candidate",
                "cleanup candidates",
            ));
        }
        if dead_exports > 0 {
            parts.push(plural(dead_exports, "dead export", "dead exports"));
        }
        if unused_imports > 0 {
            parts.push(plural(unused_imports, "unused import", "unused imports"));
        }

        format!(
            "{}: {}.",
            plural(
                actionable_total,
                "actionable finding",
                "actionable findings"
            ),
            parts.join(", ")
        )
    };

    let best_next_move = if breakages > 0 {
        "Fix broken imports before deleting files.".to_string()
    } else if cleanup_candidates > 0 {
        "Run the clean preview and remove high-confidence dead files.".to_string()
    } else if dead_exports > 0 {
        "Prune dead exports or confirm they are intentionally public API.".to_string()
    } else if unused_imports > 0 {
        "Remove unused imports to shrink noisy dependencies.".to_string()
    } else {
        "Keep the JSON report as a baseline for future diffs.".to_string()
    };

    ImpactSummary {
        headline,
        best_next_move,
    }
}

fn plural(count: usize, singular: &str, plural: &str) -> String {
    if count == 1 {
        format!("1 {singular}")
    } else {
        format!("{count} {plural}")
    }
}

fn append_next_steps(lines: &mut Vec<String>, report: &ReportV2, report_path: &Path) {
    let report_arg = shell_arg(report_path);

    lines.push(String::new());
    lines.push("Next steps:".to_string());

    if report.summary.deletion_candidates > 0 {
        lines.push(format!("- Preview cleanup: kratos clean {report_arg}"));
    }
    lines.push(format!(
        "- Shareable markdown: kratos report {report_arg} --format md"
    ));
}

fn push_markdown_impact(lines: &mut Vec<String>, report: &ReportV2, report_path: &Path) {
    let impact = impact_summary(report);
    let report_arg = shell_arg(report_path);

    lines.push("## Impact".to_string());
    lines.push(String::new());
    lines.push(format!("- {}", impact.headline));
    lines.push(format!("- Best next move: {}", impact.best_next_move));
    if report.summary.deletion_candidates > 0 {
        lines.push(format!("- Preview cleanup: `kratos clean {report_arg}`"));
    }
    lines.push(format!(
        "- Refresh markdown: `kratos report {report_arg} --format md`"
    ));
    lines.push(String::new());
}

fn append_deletion_preview(lines: &mut Vec<String>, report: &ReportV2) {
    if report.findings.deletion_candidates.is_empty() {
        return;
    }

    lines.push(String::new());
    lines.push("Top cleanup candidates:".to_string());

    for item in report.findings.deletion_candidates.iter().take(5) {
        lines.push(format!(
            "- {} (confidence {}, {})",
            display_path(report, &item.file),
            item.confidence,
            item.reason
        ));
    }

    if report.findings.deletion_candidates.len() > 5 {
        lines.push(format!(
            "- ...and {} more",
            report.findings.deletion_candidates.len() - 5
        ));
    }
}

fn display_path(report: &ReportV2, path: &Path) -> String {
    match path.strip_prefix(&report.root) {
        Ok(relative) if !relative.as_os_str().is_empty() => path_to_string(relative),
        _ => path_to_string(path),
    }
}

fn shell_arg(path: &Path) -> String {
    let raw = path_to_string(path);
    if raw
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || "/._-".contains(character))
    {
        raw
    } else {
        format!("'{}'", raw.replace('\'', "'\\''"))
    }
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
