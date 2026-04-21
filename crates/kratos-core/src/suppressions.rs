use std::path::{Path, PathBuf};

use crate::config::{resolve_config_path, resolve_path};
use crate::jsonc::JsonValue;
use crate::model::{
    BrokenImportFinding, DeadExportFinding, DeletionCandidateFinding, FindingSet,
    OrphanFileFinding, ProjectConfig, UnusedImportFinding,
};

const GENERATED_SUPPRESSIONS_PATH: &str = ".kratos/suppressions.json";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SuppressionSource {
    Config,
    Generated,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SuppressionKind {
    BrokenImport,
    OrphanFile,
    DeadExport,
    UnusedImport,
    DeletionCandidate,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SuppressionRule {
    pub kind: SuppressionKind,
    pub file: PathBuf,
    pub source: Option<String>,
    pub local: Option<String>,
    pub export: Option<String>,
    pub reason: String,
    pub origin: SuppressionSource,
}

pub fn parse_suppression_rules(
    root: &Path,
    value: Option<&JsonValue>,
    origin: SuppressionSource,
) -> Vec<SuppressionRule> {
    let Some(entries) = value.and_then(JsonValue::as_array) else {
        return Vec::new();
    };

    entries
        .iter()
        .filter_map(|entry| parse_suppression_rule(root, entry, origin))
        .collect()
}

pub fn load_generated_suppressions(root: &Path) -> Vec<SuppressionRule> {
    let path = resolve_config_path(root, GENERATED_SUPPRESSIONS_PATH);
    let Ok(content) = std::fs::read_to_string(path) else {
        return Vec::new();
    };

    let Ok(value) = crate::jsonc::parse_loose_json(&content) else {
        return Vec::new();
    };

    parse_suppression_rules(
        root,
        value.get("suppressions"),
        SuppressionSource::Generated,
    )
}

pub fn load_project_suppressions(config: &ProjectConfig) -> Vec<SuppressionRule> {
    let mut suppressions = config.suppressions.clone();
    suppressions.extend(load_generated_suppressions(&config.root));
    suppressions
}

pub fn apply_suppressions(findings: &mut FindingSet, suppressions: &[SuppressionRule]) -> usize {
    let mut suppressed = 0;

    suppressed += filter_broken_imports(&mut findings.broken_imports, suppressions);
    suppressed += filter_orphan_files(&mut findings.orphan_files, suppressions);
    suppressed += filter_dead_exports(&mut findings.dead_exports, suppressions);
    suppressed += filter_unused_imports(&mut findings.unused_imports, suppressions);
    suppressed += filter_deletion_candidates(&mut findings.deletion_candidates, suppressions);

    suppressed
}

fn parse_suppression_rule(
    root: &Path,
    value: &JsonValue,
    origin: SuppressionSource,
) -> Option<SuppressionRule> {
    let object = value.as_object()?;

    let kind = match object.get("kind").and_then(JsonValue::as_str) {
        Some("brokenImport") => SuppressionKind::BrokenImport,
        Some("orphanFile") => SuppressionKind::OrphanFile,
        Some("deadExport") => SuppressionKind::DeadExport,
        Some("unusedImport") => SuppressionKind::UnusedImport,
        Some("deletionCandidate") => SuppressionKind::DeletionCandidate,
        _ => return None,
    };

    let reason = read_required_non_empty_string(object.get("reason"))?;
    let file = read_root_relative_path(root, object.get("file").and_then(JsonValue::as_str)?)?;
    let source = read_optional_exact_string(object.get("source"))?;
    let local = read_optional_exact_string(object.get("local"))?;
    let export = read_optional_exact_string(object.get("export"))?;

    Some(SuppressionRule {
        kind,
        file,
        source,
        local,
        export,
        reason,
        origin,
    })
}

fn read_root_relative_path(root: &Path, raw: &str) -> Option<PathBuf> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }

    let raw_path = Path::new(raw);
    if raw_path.is_absolute() {
        return None;
    }

    let resolved = resolve_path(root, raw);
    let Ok(relative) = resolved.strip_prefix(root) else {
        return None;
    };

    if relative.as_os_str().is_empty() {
        return None;
    }

    Some(resolved)
}

fn read_required_non_empty_string(value: Option<&JsonValue>) -> Option<String> {
    let value = value?.as_str()?.trim().to_string();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

fn read_optional_exact_string(value: Option<&JsonValue>) -> Option<Option<String>> {
    let Some(value) = value else {
        return Some(None);
    };

    match value {
        JsonValue::Null => Some(None),
        JsonValue::String(value) => {
            let value = value.trim().to_string();
            if value.is_empty() {
                None
            } else {
                Some(Some(value))
            }
        }
        _ => None,
    }
}

fn filter_broken_imports(
    findings: &mut Vec<BrokenImportFinding>,
    suppressions: &[SuppressionRule],
) -> usize {
    let before = findings.len();
    findings.retain(|item| !matches_broken_import(item, suppressions));
    before - findings.len()
}

fn filter_orphan_files(
    findings: &mut Vec<OrphanFileFinding>,
    suppressions: &[SuppressionRule],
) -> usize {
    let before = findings.len();
    findings.retain(|item| !matches_orphan_file(item, suppressions));
    before - findings.len()
}

fn filter_dead_exports(
    findings: &mut Vec<DeadExportFinding>,
    suppressions: &[SuppressionRule],
) -> usize {
    let before = findings.len();
    findings.retain(|item| !matches_dead_export(item, suppressions));
    before - findings.len()
}

fn filter_unused_imports(
    findings: &mut Vec<UnusedImportFinding>,
    suppressions: &[SuppressionRule],
) -> usize {
    let before = findings.len();
    findings.retain(|item| !matches_unused_import(item, suppressions));
    before - findings.len()
}

fn filter_deletion_candidates(
    findings: &mut Vec<DeletionCandidateFinding>,
    suppressions: &[SuppressionRule],
) -> usize {
    let before = findings.len();
    findings.retain(|item| !matches_deletion_candidate(item, suppressions));
    before - findings.len()
}

fn matches_broken_import(item: &BrokenImportFinding, suppressions: &[SuppressionRule]) -> bool {
    suppressions.iter().any(|suppression| {
        suppression.kind == SuppressionKind::BrokenImport
            && suppression.file == item.file
            && suppression
                .source
                .as_ref()
                .map_or(true, |source| source == &item.source)
    })
}

fn matches_orphan_file(item: &OrphanFileFinding, suppressions: &[SuppressionRule]) -> bool {
    suppressions.iter().any(|suppression| {
        suppression.kind == SuppressionKind::OrphanFile && suppression.file == item.file
    })
}

fn matches_dead_export(item: &DeadExportFinding, suppressions: &[SuppressionRule]) -> bool {
    suppressions.iter().any(|suppression| {
        suppression.kind == SuppressionKind::DeadExport
            && suppression.file == item.file
            && suppression
                .export
                .as_ref()
                .map_or(true, |export| export == &item.export_name)
    })
}

fn matches_unused_import(item: &UnusedImportFinding, suppressions: &[SuppressionRule]) -> bool {
    suppressions.iter().any(|suppression| {
        suppression.kind == SuppressionKind::UnusedImport
            && suppression.file == item.file
            && suppression
                .source
                .as_ref()
                .map_or(true, |source| source == &item.source)
            && suppression
                .local
                .as_ref()
                .map_or(true, |local| local == &item.local)
    })
}

fn matches_deletion_candidate(
    item: &DeletionCandidateFinding,
    suppressions: &[SuppressionRule],
) -> bool {
    suppressions.iter().any(|suppression| {
        suppression.kind == SuppressionKind::DeletionCandidate && suppression.file == item.file
    })
}
