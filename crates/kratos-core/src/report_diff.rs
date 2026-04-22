use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde_json::{json, Value};

use crate::model::{
    BrokenImportFinding, DeadExportFinding, DeletionCandidateFinding, OrphanFileFinding,
    OrphanKind, ReportV2, RouteEntrypointFinding, UnusedImportFinding,
};
use crate::report::{entrypoint_kind_to_string, path_to_string};
use crate::{KratosError, KratosResult};

#[derive(Clone, Debug, PartialEq)]
pub struct FindingDiff<T> {
    pub introduced: Vec<T>,
    pub resolved: Vec<T>,
    pub persisted: Vec<T>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FindingDiffCounts {
    pub introduced: usize,
    pub resolved: usize,
    pub persisted: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ReportDiffSummary {
    pub broken_imports: FindingDiffCounts,
    pub orphan_files: FindingDiffCounts,
    pub dead_exports: FindingDiffCounts,
    pub unused_imports: FindingDiffCounts,
    pub route_entrypoints: FindingDiffCounts,
    pub deletion_candidates: FindingDiffCounts,
    pub totals: FindingDiffCounts,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ReportFindingDiffs {
    pub broken_imports: FindingDiff<BrokenImportFinding>,
    pub orphan_files: FindingDiff<OrphanFileFinding>,
    pub dead_exports: FindingDiff<DeadExportFinding>,
    pub unused_imports: FindingDiff<UnusedImportFinding>,
    pub route_entrypoints: FindingDiff<RouteEntrypointFinding>,
    pub deletion_candidates: FindingDiff<DeletionCandidateFinding>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ReportDiff {
    pub summary: ReportDiffSummary,
    pub findings: ReportFindingDiffs,
}

pub fn diff_reports(before: &ReportV2, after: &ReportV2) -> ReportDiff {
    let findings = ReportFindingDiffs {
        broken_imports: diff_finding_lists(
            &before.findings.broken_imports,
            &after.findings.broken_imports,
            |item| broken_import_key(item, &before.root),
            |item| broken_import_key(item, &after.root),
        ),
        orphan_files: diff_finding_lists(
            &before.findings.orphan_files,
            &after.findings.orphan_files,
            |item| orphan_file_key(item, &before.root),
            |item| orphan_file_key(item, &after.root),
        ),
        dead_exports: diff_finding_lists(
            &before.findings.dead_exports,
            &after.findings.dead_exports,
            |item| dead_export_key(item, &before.root),
            |item| dead_export_key(item, &after.root),
        ),
        unused_imports: diff_finding_lists(
            &before.findings.unused_imports,
            &after.findings.unused_imports,
            |item| unused_import_key(item, &before.root),
            |item| unused_import_key(item, &after.root),
        ),
        route_entrypoints: diff_finding_lists(
            &before.findings.route_entrypoints,
            &after.findings.route_entrypoints,
            |item| route_entrypoint_key(item, &before.root),
            |item| route_entrypoint_key(item, &after.root),
        ),
        deletion_candidates: diff_finding_lists(
            &before.findings.deletion_candidates,
            &after.findings.deletion_candidates,
            |item| deletion_candidate_key(item, &before.root),
            |item| deletion_candidate_key(item, &after.root),
        ),
    };

    let summary = ReportDiffSummary {
        broken_imports: findings.broken_imports.counts(),
        orphan_files: findings.orphan_files.counts(),
        dead_exports: findings.dead_exports.counts(),
        unused_imports: findings.unused_imports.counts(),
        route_entrypoints: findings.route_entrypoints.counts(),
        deletion_candidates: findings.deletion_candidates.counts(),
        totals: FindingDiffCounts {
            introduced: findings.broken_imports.introduced.len()
                + findings.orphan_files.introduced.len()
                + findings.dead_exports.introduced.len()
                + findings.unused_imports.introduced.len()
                + findings.route_entrypoints.introduced.len()
                + findings.deletion_candidates.introduced.len(),
            resolved: findings.broken_imports.resolved.len()
                + findings.orphan_files.resolved.len()
                + findings.dead_exports.resolved.len()
                + findings.unused_imports.resolved.len()
                + findings.route_entrypoints.resolved.len()
                + findings.deletion_candidates.resolved.len(),
            persisted: findings.broken_imports.persisted.len()
                + findings.orphan_files.persisted.len()
                + findings.dead_exports.persisted.len()
                + findings.unused_imports.persisted.len()
                + findings.route_entrypoints.persisted.len()
                + findings.deletion_candidates.persisted.len(),
        },
    };

    ReportDiff { summary, findings }
}

pub fn format_diff_summary(
    diff: &ReportDiff,
    before_report_path: &Path,
    after_report_path: &Path,
) -> KratosResult<String> {
    let mut lines = vec![
        "Kratos diff complete.".to_string(),
        String::new(),
        format!("Before: {}", path_to_string(before_report_path)),
        format!("After: {}", path_to_string(after_report_path)),
        String::new(),
    ];

    lines.extend(render_summary_line(
        "Broken imports",
        &diff.summary.broken_imports,
    ));
    lines.extend(render_summary_line(
        "Orphan files",
        &diff.summary.orphan_files,
    ));
    lines.extend(render_summary_line(
        "Dead exports",
        &diff.summary.dead_exports,
    ));
    lines.extend(render_summary_line(
        "Unused imports",
        &diff.summary.unused_imports,
    ));
    lines.extend(render_summary_line(
        "Route entrypoints",
        &diff.summary.route_entrypoints,
    ));
    lines.extend(render_summary_line(
        "Deletion candidates",
        &diff.summary.deletion_candidates,
    ));

    lines.push(String::new());
    lines.push(format!(
        "Totals: introduced {}, resolved {}, persisted {}",
        diff.summary.totals.introduced, diff.summary.totals.resolved, diff.summary.totals.persisted
    ));

    if diff.summary.totals.introduced == 0
        && diff.summary.totals.resolved == 0
        && diff.summary.totals.persisted == 0
    {
        lines.push("No finding changes.".to_string());
    }

    Ok(lines.join("\n"))
}

pub fn format_diff_markdown(
    diff: &ReportDiff,
    before_report_path: &Path,
    after_report_path: &Path,
) -> KratosResult<String> {
    let mut lines = vec![
        "# Kratos Diff".to_string(),
        String::new(),
        format!("- Before: {}", path_to_string(before_report_path)),
        format!("- After: {}", path_to_string(after_report_path)),
        String::new(),
    ];

    push_markdown_finding_diff(
        &mut lines,
        "Broken imports",
        &diff.findings.broken_imports,
        |item| format!("{} -> `{}`", path_to_string(&item.file), item.source),
    );
    push_markdown_finding_diff(
        &mut lines,
        "Orphan files",
        &diff.findings.orphan_files,
        |item| {
            format!(
                "{} ({})",
                path_to_string(&item.file),
                orphan_kind_to_string(&item.kind)
            )
        },
    );
    push_markdown_finding_diff(
        &mut lines,
        "Dead exports",
        &diff.findings.dead_exports,
        |item| format!("{} -> `{}`", path_to_string(&item.file), item.export_name),
    );
    push_markdown_finding_diff(
        &mut lines,
        "Unused imports",
        &diff.findings.unused_imports,
        |item| {
            format!(
                "{} -> `{}` from `{}`",
                path_to_string(&item.file),
                item.local,
                item.source
            )
        },
    );
    push_markdown_finding_diff(
        &mut lines,
        "Route entrypoints",
        &diff.findings.route_entrypoints,
        |item| {
            format!(
                "{} ({})",
                path_to_string(&item.file),
                entrypoint_kind_to_string(&item.kind)
            )
        },
    );
    push_markdown_finding_diff(
        &mut lines,
        "Deletion candidates",
        &diff.findings.deletion_candidates,
        |item| {
            format!(
                "{} ({}, confidence {})",
                path_to_string(&item.file),
                item.reason,
                item.confidence
            )
        },
    );

    if lines.last().is_some_and(|line| !line.is_empty()) {
        lines.push(String::new());
    }

    Ok(lines.join("\n"))
}

pub fn format_diff_json(
    diff: &ReportDiff,
    before_report_path: &Path,
    after_report_path: &Path,
) -> KratosResult<String> {
    serde_json::to_string_pretty(&report_diff_json_value(
        diff,
        before_report_path,
        after_report_path,
    ))
    .map_err(|error| KratosError::Json(error.to_string()))
}

fn report_diff_json_value(
    diff: &ReportDiff,
    before_report_path: &Path,
    after_report_path: &Path,
) -> Value {
    json!({
        "before": {
            "path": path_to_string(before_report_path),
        },
        "after": {
            "path": path_to_string(after_report_path),
        },
        "summary": {
            "brokenImports": counts_to_json(&diff.summary.broken_imports),
            "orphanFiles": counts_to_json(&diff.summary.orphan_files),
            "deadExports": counts_to_json(&diff.summary.dead_exports),
            "unusedImports": counts_to_json(&diff.summary.unused_imports),
            "routeEntrypoints": counts_to_json(&diff.summary.route_entrypoints),
            "deletionCandidates": counts_to_json(&diff.summary.deletion_candidates),
            "totals": counts_to_json(&diff.summary.totals),
        },
        "findings": {
            "brokenImports": finding_diff_to_json(&diff.findings.broken_imports, serialize_broken_import),
            "orphanFiles": finding_diff_to_json(&diff.findings.orphan_files, serialize_orphan_file),
            "deadExports": finding_diff_to_json(&diff.findings.dead_exports, serialize_dead_export),
            "unusedImports": finding_diff_to_json(&diff.findings.unused_imports, serialize_unused_import),
            "routeEntrypoints": finding_diff_to_json(&diff.findings.route_entrypoints, serialize_route_entrypoint),
            "deletionCandidates": finding_diff_to_json(&diff.findings.deletion_candidates, serialize_deletion_candidate),
        },
    })
}

fn diff_finding_lists<T: Clone>(
    before: &[T],
    after: &[T],
    before_key_for_item: impl Fn(&T) -> String,
    after_key_for_item: impl Fn(&T) -> String,
) -> FindingDiff<T> {
    let before_groups = group_items_by_key(before, &before_key_for_item);
    let after_groups = group_items_by_key(after, &after_key_for_item);
    let all_keys = all_group_keys(&before_groups, &after_groups);
    let mut introduced = Vec::new();
    let mut resolved = Vec::new();
    let mut persisted = Vec::new();

    for key in all_keys {
        let before_items = before_groups.get(&key).cloned().unwrap_or_default();
        let after_items = after_groups.get(&key).cloned().unwrap_or_default();
        let shared_count = before_items.len().min(after_items.len());

        persisted.extend(after_items.iter().take(shared_count).cloned());
        resolved.extend(before_items.iter().skip(shared_count).cloned());
        introduced.extend(after_items.iter().skip(shared_count).cloned());
    }

    FindingDiff {
        introduced,
        resolved,
        persisted,
    }
}

fn group_items_by_key<T: Clone>(
    items: &[T],
    key_for_item: &impl Fn(&T) -> String,
) -> BTreeMap<String, Vec<T>> {
    let mut grouped = BTreeMap::new();
    for item in items {
        grouped
            .entry(key_for_item(item))
            .or_insert_with(Vec::new)
            .push(item.clone());
    }
    grouped
}

fn all_group_keys<T>(
    before_groups: &BTreeMap<String, Vec<T>>,
    after_groups: &BTreeMap<String, Vec<T>>,
) -> BTreeSet<String> {
    before_groups
        .keys()
        .chain(after_groups.keys())
        .cloned()
        .collect()
}

fn render_summary_line(label: &str, counts: &FindingDiffCounts) -> Vec<String> {
    vec![format!(
        "{label}: introduced {}, resolved {}, persisted {}",
        counts.introduced, counts.resolved, counts.persisted
    )]
}

fn push_markdown_finding_diff<T>(
    lines: &mut Vec<String>,
    title: &str,
    diff: &FindingDiff<T>,
    render: impl Fn(&T) -> String,
) {
    if diff.introduced.is_empty() && diff.resolved.is_empty() && diff.persisted.is_empty() {
        return;
    }

    lines.push(format!("## {title}"));
    lines.push(String::new());

    push_markdown_change_group(lines, "Introduced", &diff.introduced, &render);
    push_markdown_change_group(lines, "Resolved", &diff.resolved, &render);
    push_markdown_change_group(lines, "Persisted", &diff.persisted, &render);
}

fn push_markdown_change_group<T>(
    lines: &mut Vec<String>,
    label: &str,
    items: &[T],
    render: &impl Fn(&T) -> String,
) {
    lines.push(format!("### {label} ({})", items.len()));

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

impl<T> FindingDiff<T> {
    pub fn counts(&self) -> FindingDiffCounts {
        FindingDiffCounts {
            introduced: self.introduced.len(),
            resolved: self.resolved.len(),
            persisted: self.persisted.len(),
        }
    }
}

impl<T> Default for FindingDiff<T> {
    fn default() -> Self {
        Self {
            introduced: Vec::new(),
            resolved: Vec::new(),
            persisted: Vec::new(),
        }
    }
}

fn finding_diff_to_json<T>(diff: &FindingDiff<T>, serialize_item: fn(&T) -> Value) -> Value {
    json!({
        "introduced": diff.introduced.iter().map(serialize_item).collect::<Vec<_>>(),
        "resolved": diff.resolved.iter().map(serialize_item).collect::<Vec<_>>(),
        "persisted": diff.persisted.iter().map(serialize_item).collect::<Vec<_>>(),
    })
}

fn counts_to_json(counts: &FindingDiffCounts) -> Value {
    json!({
        "introduced": counts.introduced,
        "resolved": counts.resolved,
        "persisted": counts.persisted,
    })
}

fn serialize_broken_import(item: &BrokenImportFinding) -> Value {
    json!({
        "file": path_to_string(&item.file),
        "source": item.source,
        "kind": import_kind_to_string(&item.kind),
    })
}

fn serialize_orphan_file(item: &OrphanFileFinding) -> Value {
    json!({
        "file": path_to_string(&item.file),
        "kind": orphan_kind_to_string(&item.kind),
        "reason": item.reason,
        "confidence": round_confidence(item.confidence),
    })
}

fn serialize_dead_export(item: &DeadExportFinding) -> Value {
    json!({
        "file": path_to_string(&item.file),
        "exportName": item.export_name,
    })
}

fn serialize_unused_import(item: &UnusedImportFinding) -> Value {
    json!({
        "file": path_to_string(&item.file),
        "source": item.source,
        "local": item.local,
        "imported": item.imported,
    })
}

fn serialize_route_entrypoint(item: &RouteEntrypointFinding) -> Value {
    json!({
        "file": path_to_string(&item.file),
        "kind": entrypoint_kind_to_string(&item.kind),
    })
}

fn serialize_deletion_candidate(item: &DeletionCandidateFinding) -> Value {
    json!({
        "file": path_to_string(&item.file),
        "reason": item.reason,
        "confidence": round_confidence(item.confidence),
        "safe": item.safe,
    })
}

fn broken_import_key(item: &BrokenImportFinding, report_root: &Path) -> String {
    format!(
        "{}|{}|{}",
        finding_file_key(&item.file, report_root),
        item.source,
        import_kind_to_string(&item.kind)
    )
}

fn orphan_file_key(item: &OrphanFileFinding, report_root: &Path) -> String {
    format!(
        "{}|{}|{}|{}",
        finding_file_key(&item.file, report_root),
        orphan_kind_to_string(&item.kind),
        item.reason,
        item.confidence.to_bits()
    )
}

fn dead_export_key(item: &DeadExportFinding, report_root: &Path) -> String {
    format!("{}|{}", finding_file_key(&item.file, report_root), item.export_name)
}

fn unused_import_key(item: &UnusedImportFinding, report_root: &Path) -> String {
    format!(
        "{}|{}|{}|{}",
        finding_file_key(&item.file, report_root),
        item.source,
        item.local,
        item.imported
    )
}

fn route_entrypoint_key(item: &RouteEntrypointFinding, report_root: &Path) -> String {
    format!(
        "{}|{}",
        finding_file_key(&item.file, report_root),
        entrypoint_kind_to_string(&item.kind)
    )
}

fn deletion_candidate_key(item: &DeletionCandidateFinding, report_root: &Path) -> String {
    format!(
        "{}|{}|{}|{}",
        finding_file_key(&item.file, report_root),
        item.reason,
        item.confidence.to_bits(),
        item.safe
    )
}

fn finding_file_key(file: &Path, report_root: &Path) -> String {
    file.strip_prefix(report_root)
        .map(path_to_string)
        .unwrap_or_else(|_| path_to_string(file))
}

fn import_kind_to_string(kind: &crate::model::ImportKind) -> &'static str {
    match kind {
        crate::model::ImportKind::Static => "static",
        crate::model::ImportKind::SideEffect => "side-effect",
        crate::model::ImportKind::Reexport => "reexport",
        crate::model::ImportKind::ReexportAll => "reexport-all",
        crate::model::ImportKind::ReexportNamespace => "reexport-namespace",
        crate::model::ImportKind::Require => "require",
        crate::model::ImportKind::Dynamic => "dynamic",
        crate::model::ImportKind::Unknown => "unknown",
    }
}

fn orphan_kind_to_string(kind: &OrphanKind) -> &'static str {
    match kind {
        OrphanKind::Module => "orphan-module",
        OrphanKind::Component => "orphan-component",
        OrphanKind::RouteModule => "orphan-route-module",
    }
}

fn round_confidence(value: f32) -> f32 {
    (value * 100.0).round() / 100.0
}
