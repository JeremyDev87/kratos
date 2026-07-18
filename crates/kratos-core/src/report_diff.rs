use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::model::{
    BrokenImportFinding, DeadExportFinding, DeletionCandidateFinding, OrphanFileFinding,
    OrphanKind, ReportV2, RouteEntrypointFinding, UnusedImportFinding,
};
use crate::report::{entrypoint_kind_to_string, export_kind_to_string, path_to_string};
use crate::{KratosError, KratosResult};

pub const FINDING_IDENTITY_VERSION: u32 = 1;

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
pub struct ReportFindingIds {
    pub broken_imports: FindingDiff<String>,
    pub orphan_files: FindingDiff<String>,
    pub dead_exports: FindingDiff<String>,
    pub unused_imports: FindingDiff<String>,
    pub route_entrypoints: FindingDiff<String>,
    pub deletion_candidates: FindingDiff<String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ReportDiff {
    pub summary: ReportDiffSummary,
    pub findings: ReportFindingDiffs,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportDiffWithIdentity {
    pub diff: ReportDiff,
    pub identity_version: u32,
    pub finding_ids: ReportFindingIds,
}

impl Default for ReportDiffWithIdentity {
    fn default() -> Self {
        Self {
            diff: ReportDiff::default(),
            identity_version: FINDING_IDENTITY_VERSION,
            finding_ids: ReportFindingIds::default(),
        }
    }
}

impl std::ops::Deref for ReportDiffWithIdentity {
    type Target = ReportDiff;

    fn deref(&self) -> &Self::Target {
        &self.diff
    }
}

pub fn diff_reports(before: &ReportV2, after: &ReportV2) -> ReportDiff {
    diff_reports_with_identity(before, after).diff
}

pub fn diff_reports_with_identity(before: &ReportV2, after: &ReportV2) -> ReportDiffWithIdentity {
    let (broken_imports, broken_import_ids) = diff_finding_lists(
        &before.findings.broken_imports,
        &after.findings.broken_imports,
        |item| broken_import_id(item, &before.root),
        |item| broken_import_id(item, &after.root),
        serialize_broken_import,
    );
    let (orphan_files, orphan_file_ids) = diff_finding_lists(
        &before.findings.orphan_files,
        &after.findings.orphan_files,
        |item| orphan_file_id(item, &before.root),
        |item| orphan_file_id(item, &after.root),
        serialize_orphan_file,
    );
    let (dead_exports, dead_export_ids) = diff_finding_lists(
        &before.findings.dead_exports,
        &after.findings.dead_exports,
        |item| dead_export_id(item, &before.root),
        |item| dead_export_id(item, &after.root),
        serialize_dead_export,
    );
    let (unused_imports, unused_import_ids) = diff_finding_lists(
        &before.findings.unused_imports,
        &after.findings.unused_imports,
        |item| unused_import_id(item, &before.root),
        |item| unused_import_id(item, &after.root),
        serialize_unused_import,
    );
    let (route_entrypoints, route_entrypoint_ids) = diff_finding_lists(
        &before.findings.route_entrypoints,
        &after.findings.route_entrypoints,
        |item| route_entrypoint_id(item, &before.root),
        |item| route_entrypoint_id(item, &after.root),
        serialize_route_entrypoint,
    );
    let (deletion_candidates, deletion_candidate_ids) = diff_finding_lists(
        &before.findings.deletion_candidates,
        &after.findings.deletion_candidates,
        |item| deletion_candidate_id(item, &before.root),
        |item| deletion_candidate_id(item, &after.root),
        serialize_deletion_candidate,
    );

    let findings = ReportFindingDiffs {
        broken_imports,
        orphan_files,
        dead_exports,
        unused_imports,
        route_entrypoints,
        deletion_candidates,
    };
    let finding_ids = ReportFindingIds {
        broken_imports: broken_import_ids,
        orphan_files: orphan_file_ids,
        dead_exports: dead_export_ids,
        unused_imports: unused_import_ids,
        route_entrypoints: route_entrypoint_ids,
        deletion_candidates: deletion_candidate_ids,
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

    ReportDiffWithIdentity {
        diff: ReportDiff { summary, findings },
        identity_version: FINDING_IDENTITY_VERSION,
        finding_ids,
    }
}

pub fn format_diff_summary(
    diff: &ReportDiff,
    before_report_path: &Path,
    after_report_path: &Path,
) -> KratosResult<String> {
    let mut lines = vec![
        "Kratos diff 완료.".to_string(),
        String::new(),
        format!("이전: {}", path_to_string(before_report_path)),
        format!("이후: {}", path_to_string(after_report_path)),
        String::new(),
    ];

    lines.extend(render_summary_line(
        "깨진 import",
        &diff.summary.broken_imports,
    ));
    lines.extend(render_summary_line("고아 파일", &diff.summary.orphan_files));
    lines.extend(render_summary_line(
        "사용되지 않는 export",
        &diff.summary.dead_exports,
    ));
    lines.extend(render_summary_line(
        "사용되지 않는 import",
        &diff.summary.unused_imports,
    ));
    lines.extend(render_summary_line(
        "라우트 진입점",
        &diff.summary.route_entrypoints,
    ));
    lines.extend(render_summary_line(
        "삭제 후보",
        &diff.summary.deletion_candidates,
    ));

    lines.push(String::new());
    lines.push(format!(
        "합계: 새로 발생 {}, 해결됨 {}, 유지됨 {}",
        diff.summary.totals.introduced, diff.summary.totals.resolved, diff.summary.totals.persisted
    ));

    if diff.summary.totals.introduced == 0
        && diff.summary.totals.resolved == 0
        && diff.summary.totals.persisted == 0
    {
        lines.push("변경된 항목이 없습니다.".to_string());
    }

    Ok(lines.join("\n"))
}

pub fn format_diff_markdown(
    diff: &ReportDiff,
    before_report_path: &Path,
    after_report_path: &Path,
) -> KratosResult<String> {
    format_diff_markdown_impl(diff, None, before_report_path, after_report_path)
}

pub fn format_diff_markdown_with_identity(
    diff: &ReportDiffWithIdentity,
    before_report_path: &Path,
    after_report_path: &Path,
) -> KratosResult<String> {
    format_diff_markdown_impl(
        &diff.diff,
        Some((diff.identity_version, &diff.finding_ids)),
        before_report_path,
        after_report_path,
    )
}

fn format_diff_markdown_impl(
    diff: &ReportDiff,
    identity: Option<(u32, &ReportFindingIds)>,
    before_report_path: &Path,
    after_report_path: &Path,
) -> KratosResult<String> {
    let mut lines = vec![
        "# Kratos Diff 결과".to_string(),
        String::new(),
        format!("- 이전: {}", path_to_string(before_report_path)),
        format!("- 이후: {}", path_to_string(after_report_path)),
    ];
    if let Some((identity_version, _)) = identity {
        lines.push(format!("- Finding identity version: {identity_version}"));
    }
    lines.push(String::new());

    push_markdown_finding_diff(
        &mut lines,
        "깨진 import",
        &diff.findings.broken_imports,
        identity.map(|(_, ids)| &ids.broken_imports),
        |item| format!("{} -> `{}`", path_to_string(&item.file), item.source),
    );
    push_markdown_finding_diff(
        &mut lines,
        "고아 파일",
        &diff.findings.orphan_files,
        identity.map(|(_, ids)| &ids.orphan_files),
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
        "사용되지 않는 export",
        &diff.findings.dead_exports,
        identity.map(|(_, ids)| &ids.dead_exports),
        |item| format!("{} -> `{}`", path_to_string(&item.file), item.export_name),
    );
    push_markdown_finding_diff(
        &mut lines,
        "사용되지 않는 import",
        &diff.findings.unused_imports,
        identity.map(|(_, ids)| &ids.unused_imports),
        |item| {
            format!(
                "{} -> `{}` (출처: `{}`)",
                path_to_string(&item.file),
                item.local,
                item.source
            )
        },
    );
    push_markdown_finding_diff(
        &mut lines,
        "라우트 진입점",
        &diff.findings.route_entrypoints,
        identity.map(|(_, ids)| &ids.route_entrypoints),
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
        "삭제 후보",
        &diff.findings.deletion_candidates,
        identity.map(|(_, ids)| &ids.deletion_candidates),
        |item| {
            format!(
                "{} ({}, 신뢰도 {})",
                path_to_string(&item.file),
                display_known_reason(&item.reason),
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
        None,
        before_report_path,
        after_report_path,
    ))
    .map_err(|error| KratosError::Json(error.to_string()))
}

pub fn format_diff_json_with_identity(
    diff: &ReportDiffWithIdentity,
    before_report_path: &Path,
    after_report_path: &Path,
) -> KratosResult<String> {
    serde_json::to_string_pretty(&report_diff_json_value(
        &diff.diff,
        Some((diff.identity_version, &diff.finding_ids)),
        before_report_path,
        after_report_path,
    ))
    .map_err(|error| KratosError::Json(error.to_string()))
}

fn report_diff_json_value(
    diff: &ReportDiff,
    identity: Option<(u32, &ReportFindingIds)>,
    before_report_path: &Path,
    after_report_path: &Path,
) -> Value {
    let mut value = json!({
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
            "brokenImports": finding_diff_to_json_optional(&diff.findings.broken_imports, identity.map(|(_, ids)| &ids.broken_imports), serialize_broken_import),
            "orphanFiles": finding_diff_to_json_optional(&diff.findings.orphan_files, identity.map(|(_, ids)| &ids.orphan_files), serialize_orphan_file),
            "deadExports": finding_diff_to_json_optional(&diff.findings.dead_exports, identity.map(|(_, ids)| &ids.dead_exports), serialize_dead_export),
            "unusedImports": finding_diff_to_json_optional(&diff.findings.unused_imports, identity.map(|(_, ids)| &ids.unused_imports), serialize_unused_import),
            "routeEntrypoints": finding_diff_to_json_optional(&diff.findings.route_entrypoints, identity.map(|(_, ids)| &ids.route_entrypoints), serialize_route_entrypoint),
            "deletionCandidates": finding_diff_to_json_optional(&diff.findings.deletion_candidates, identity.map(|(_, ids)| &ids.deletion_candidates), serialize_deletion_candidate),
        },
    });
    if let Some((identity_version, _)) = identity {
        value
            .as_object_mut()
            .expect("diff serializer must return an object")
            .insert("identityVersion".to_string(), json!(identity_version));
    }
    value
}

fn diff_finding_lists<T: Clone>(
    before: &[T],
    after: &[T],
    before_key_for_item: impl Fn(&T) -> String,
    after_key_for_item: impl Fn(&T) -> String,
    serialize_item: fn(&T) -> Value,
) -> (FindingDiff<T>, FindingDiff<String>) {
    let before_groups = group_items_by_key(before, &before_key_for_item, serialize_item);
    let after_groups = group_items_by_key(after, &after_key_for_item, serialize_item);
    let all_keys = all_group_keys(&before_groups, &after_groups);
    let mut introduced = Vec::new();
    let mut resolved = Vec::new();
    let mut persisted = Vec::new();
    let mut introduced_ids = Vec::new();
    let mut resolved_ids = Vec::new();
    let mut persisted_ids = Vec::new();

    for key in all_keys {
        let before_items = before_groups.get(&key).cloned().unwrap_or_default();
        let after_items = after_groups.get(&key).cloned().unwrap_or_default();
        let shared_count = before_items.len().min(after_items.len());

        let resolved_count = before_items.len().saturating_sub(shared_count);
        let introduced_count = after_items.len().saturating_sub(shared_count);

        persisted.extend(after_items.iter().take(shared_count).cloned());
        resolved.extend(before_items.iter().skip(shared_count).cloned());
        introduced.extend(after_items.iter().skip(shared_count).cloned());
        persisted_ids.resize(persisted_ids.len() + shared_count, key.clone());
        resolved_ids.resize(resolved_ids.len() + resolved_count, key.clone());
        introduced_ids.resize(introduced_ids.len() + introduced_count, key);
    }

    (
        FindingDiff {
            introduced,
            resolved,
            persisted,
        },
        FindingDiff {
            introduced: introduced_ids,
            resolved: resolved_ids,
            persisted: persisted_ids,
        },
    )
}

fn group_items_by_key<T: Clone>(
    items: &[T],
    key_for_item: &impl Fn(&T) -> String,
    serialize_item: fn(&T) -> Value,
) -> BTreeMap<String, Vec<T>> {
    let mut grouped = BTreeMap::new();
    for item in items {
        grouped
            .entry(key_for_item(item))
            .or_insert_with(Vec::new)
            .push(item.clone());
    }
    for group in grouped.values_mut() {
        group.sort_by_key(|item| serialize_item(item).to_string());
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

fn display_known_reason(reason: &str) -> &str {
    match reason {
        "Component-like module has no inbound references." => {
            "컴포넌트로 보이는 모듈에 참조가 없습니다."
        }
        "Route-like module is not connected to any router entry." => {
            "라우트로 보이는 모듈이 어떤 라우터 진입점에도 연결되지 않았습니다."
        }
        "Module has no inbound references and is not treated as an entrypoint." => {
            "모듈에 참조가 없고 진입점으로 취급되지 않습니다."
        }
        other => other,
    }
}

fn render_summary_line(label: &str, counts: &FindingDiffCounts) -> Vec<String> {
    vec![format!(
        "{label}: 새로 발생 {}, 해결됨 {}, 유지됨 {}",
        counts.introduced, counts.resolved, counts.persisted
    )]
}

fn push_markdown_finding_diff<T>(
    lines: &mut Vec<String>,
    title: &str,
    diff: &FindingDiff<T>,
    ids: Option<&FindingDiff<String>>,
    render: impl Fn(&T) -> String,
) {
    if diff.introduced.is_empty() && diff.resolved.is_empty() && diff.persisted.is_empty() {
        return;
    }

    lines.push(format!("## {title}"));
    lines.push(String::new());

    push_markdown_change_group(
        lines,
        "새로 발생",
        &diff.introduced,
        ids.map(|ids| ids.introduced.as_slice()),
        &render,
    );
    push_markdown_change_group(
        lines,
        "해결됨",
        &diff.resolved,
        ids.map(|ids| ids.resolved.as_slice()),
        &render,
    );
    push_markdown_change_group(
        lines,
        "유지됨",
        &diff.persisted,
        ids.map(|ids| ids.persisted.as_slice()),
        &render,
    );
}

fn push_markdown_change_group<T>(
    lines: &mut Vec<String>,
    label: &str,
    items: &[T],
    ids: Option<&[String]>,
    render: &impl Fn(&T) -> String,
) {
    if let Some(ids) = ids {
        assert_eq!(items.len(), ids.len(), "finding identity count must match");
    }
    lines.push(format!("### {label} ({})", items.len()));

    if items.is_empty() {
        lines.push("- 없음".to_string());
        lines.push(String::new());
        return;
    }

    match ids {
        Some(ids) => {
            for (item, id) in items.iter().zip(ids) {
                lines.push(format!("- `{id}` — {}", render(item)));
            }
        }
        None => {
            for item in items {
                lines.push(format!("- {}", render(item)));
            }
        }
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

fn finding_diff_to_json_optional<T>(
    diff: &FindingDiff<T>,
    ids: Option<&FindingDiff<String>>,
    serialize_item: fn(&T) -> Value,
) -> Value {
    match ids {
        Some(ids) => finding_diff_to_json(diff, ids, serialize_item),
        None => finding_diff_to_json_legacy(diff, serialize_item),
    }
}

fn finding_diff_to_json_legacy<T>(diff: &FindingDiff<T>, serialize_item: fn(&T) -> Value) -> Value {
    json!({
        "introduced": diff.introduced.iter().map(serialize_item).collect::<Vec<_>>(),
        "resolved": diff.resolved.iter().map(serialize_item).collect::<Vec<_>>(),
        "persisted": diff.persisted.iter().map(serialize_item).collect::<Vec<_>>(),
    })
}

fn finding_diff_to_json<T>(
    diff: &FindingDiff<T>,
    ids: &FindingDiff<String>,
    serialize_item: fn(&T) -> Value,
) -> Value {
    json!({
        "introduced": serialize_identified_items(&diff.introduced, &ids.introduced, serialize_item),
        "resolved": serialize_identified_items(&diff.resolved, &ids.resolved, serialize_item),
        "persisted": serialize_identified_items(&diff.persisted, &ids.persisted, serialize_item),
    })
}

fn serialize_identified_items<T>(
    items: &[T],
    ids: &[String],
    serialize_item: fn(&T) -> Value,
) -> Vec<Value> {
    assert_eq!(items.len(), ids.len(), "finding identity count must match");
    items
        .iter()
        .zip(ids)
        .map(|(item, id)| {
            let mut value = serialize_item(item);
            value
                .as_object_mut()
                .expect("finding serializer must return an object")
                .insert("id".to_string(), Value::String(id.clone()));
            value
        })
        .collect()
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
        "exportKind": export_kind_to_string(&item.export_kind),
        "reason": item.reason,
        "confidence": round_confidence(item.confidence),
        "importedByCount": item.imported_by_count,
        "usedExportNames": item.used_export_names,
        "hasNamespaceOrUnknownUsage": item.has_namespace_or_unknown_usage,
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

fn broken_import_id(item: &BrokenImportFinding, report_root: &Path) -> String {
    finding_id(
        "broken-import",
        &item.file,
        report_root,
        &[item.source.as_str(), import_kind_to_string(&item.kind)],
    )
}

fn orphan_file_id(item: &OrphanFileFinding, report_root: &Path) -> String {
    finding_id("orphan-file", &item.file, report_root, &[])
}

fn dead_export_id(item: &DeadExportFinding, report_root: &Path) -> String {
    finding_id(
        "dead-export",
        &item.file,
        report_root,
        &[item.export_name.as_str()],
    )
}

fn unused_import_id(item: &UnusedImportFinding, report_root: &Path) -> String {
    finding_id(
        "unused-import",
        &item.file,
        report_root,
        &[
            item.source.as_str(),
            item.local.as_str(),
            item.imported.as_str(),
        ],
    )
}

fn route_entrypoint_id(item: &RouteEntrypointFinding, report_root: &Path) -> String {
    finding_id(
        "route-entrypoint",
        &item.file,
        report_root,
        &[entrypoint_kind_to_string(&item.kind)],
    )
}

fn deletion_candidate_id(item: &DeletionCandidateFinding, report_root: &Path) -> String {
    finding_id("deletion-candidate", &item.file, report_root, &[])
}

fn finding_id(kind: &str, file: &Path, report_root: &Path, locator: &[&str]) -> String {
    let normalized_path = finding_file_key(file, report_root);
    let mut hasher = Sha256::new();
    hasher.update(b"kratos-finding-identity");
    hasher.update(FINDING_IDENTITY_VERSION.to_be_bytes());
    hash_identity_component(&mut hasher, kind);
    hash_identity_component(&mut hasher, &normalized_path);
    hasher.update((locator.len() as u64).to_be_bytes());
    for component in locator {
        hash_identity_component(&mut hasher, component);
    }

    format!(
        "kratos:v{}:{:x}",
        FINDING_IDENTITY_VERSION,
        hasher.finalize()
    )
}

fn hash_identity_component(hasher: &mut Sha256, component: &str) {
    hasher.update((component.len() as u64).to_be_bytes());
    hasher.update(component.as_bytes());
}

fn finding_file_key(file: &Path, report_root: &Path) -> String {
    let normalized_file = normalize_path_text(&path_to_string(file));
    let normalized_root = normalize_path_text(&path_to_string(report_root));

    if normalized_root.is_empty() {
        return format!("root:{normalized_file}");
    }
    if normalized_file == normalized_root {
        return "root:".to_string();
    }
    if let Some(relative) = normalized_file.strip_prefix(&format!("{normalized_root}/")) {
        return format!("root:{relative}");
    }

    format!("external:{normalized_file}")
}

fn normalize_path_text(path: &str) -> String {
    let replaced = path.replace('\\', "/");
    let mut components = Vec::new();
    for component in replaced.split('/') {
        match component {
            "" | "." => {}
            ".." if components.last().is_some_and(|value| *value != "..") => {
                components.pop();
            }
            other => components.push(other),
        }
    }
    components.join("/")
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

fn round_confidence(value: f32) -> f64 {
    ((value as f64) * 100.0).round() / 100.0
}
