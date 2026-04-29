use std::path::Path;

use std::collections::{BTreeMap, BTreeSet};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::config::load_project_config;
use crate::discover::collect_source_files;
use crate::entrypoints::detect_entrypoint_kind;
use crate::error::KratosResult;
use crate::model::{
    BrokenImportFinding, DeadExportFinding, DeletionCandidateFinding, EntrypointKind, ExportRecord,
    FindingSet, ImportKind, ImportSpecifierKind, ImportUsageRecord, ModuleRecord,
    OrphanFileFinding, OrphanKind, ProjectConfig, ReportV2, ResolvedImportRecord,
    RouteEntrypointFinding, SummaryCounts, UnusedImportFinding,
};
use crate::parser::parse_module_source;
use crate::resolve::{resolve_import_target, unresolved_import};
use crate::suppressions::{apply_suppressions, load_project_suppressions};

const DEFAULT_CONFIG_FILENAME: &str = "kratos.config.json";
const NEXT_APP_COMPONENT_DEFAULT_STEMS: &[&str] = &[
    "page",
    "layout",
    "loading",
    "error",
    "not-found",
    "global-not-found",
    "forbidden",
    "unauthorized",
    "default",
    "template",
    "global-error",
];
const NEXT_APP_METADATA_DEFAULT_STEMS: &[&str] = &[
    "robots",
    "sitemap",
    "icon",
    "apple-icon",
    "opengraph-image",
    "twitter-image",
    "manifest",
];
const NEXT_APP_METADATA_STEMS: &[&str] = &["page", "layout", "global-not-found"];
const NEXT_APP_IMAGE_METADATA_STEMS: &[&str] =
    &["icon", "apple-icon", "opengraph-image", "twitter-image"];
const NEXT_APP_OPEN_GRAPH_IMAGE_STEMS: &[&str] = &["opengraph-image", "twitter-image"];
const NEXT_APP_ICON_IMAGE_STEMS: &[&str] = &["icon", "apple-icon"];
const NEXT_APP_STATIC_PARAMS_STEMS: &[&str] = &["page", "layout", "route"];
const NEXT_APP_SEGMENT_CONFIG_STEMS: &[&str] = &[
    "page",
    "layout",
    "route",
    "robots",
    "sitemap",
    "icon",
    "apple-icon",
    "opengraph-image",
    "twitter-image",
    "manifest",
];

pub fn analyze_project(root: &Path) -> KratosResult<ReportV2> {
    let config = load_project_config(root)?;
    analyze_with_config(&config)
}

pub fn analyze_with_config(config: &ProjectConfig) -> KratosResult<ReportV2> {
    let files = collect_source_files(config)?;
    let mut modules = BTreeMap::new();
    let mut pure_barrel_files = BTreeSet::new();

    for file_path in files {
        let source = std::fs::read_to_string(&file_path)?;
        let parsed = parse_module_source(&file_path, &source)?;
        let entrypoint_kind = detect_entrypoint_kind(&file_path, config)?;
        let is_pure_barrel = parsed.is_pure_reexport_barrel;

        modules.insert(
            file_path.clone(),
            ModuleRecord {
                file_path: file_path.clone(),
                relative_path: to_project_path(&file_path, &config.root),
                entrypoint_kind,
                imports: parsed.imports,
                exports: dedupe_exports(parsed.exports),
                unused_imports: parsed.unused_imports,
                resolved_imports: Vec::new(),
                importers: Vec::new(),
                imported_by: Vec::new(),
                imported_by_count: 0,
                import_count: 0,
                export_count: 0,
            },
        );

        if is_pure_barrel {
            pure_barrel_files.insert(file_path);
        }
    }

    let mut broken_imports = Vec::new();
    let mut route_entrypoints = Vec::new();
    let module_paths = modules.keys().cloned().collect::<Vec<_>>();

    for module_path in module_paths {
        let Some(module) = modules.get(&module_path) else {
            continue;
        };

        if let Some(kind) = &module.entrypoint_kind {
            if matches!(
                kind,
                crate::model::EntrypointKind::NextAppRoute
                    | crate::model::EntrypointKind::NextPagesRoute
            ) {
                route_entrypoints.push(RouteEntrypointFinding {
                    file: module.file_path.clone(),
                    kind: kind.clone(),
                });
            }
        }

        let imports = module.imports.clone();
        let mut resolved_imports = Vec::new();
        let mut importer_updates = Vec::new();

        for entry in imports {
            let resolution = resolve_import_target(&entry.source, &module_path, config)?;

            match resolution.kind {
                crate::model::ImportResolutionKind::Source => {
                    let Some(target_path) = resolution.path else {
                        continue;
                    };

                    if !modules.contains_key(&target_path) {
                        continue;
                    }

                    resolved_imports.push(ResolvedImportRecord {
                        kind: entry.kind.clone(),
                        source: entry.source.clone(),
                        target: target_path.clone(),
                        specifiers: entry.specifiers.clone(),
                    });
                    importer_updates.push((
                        target_path,
                        module_path.clone(),
                        ImportUsageRecord {
                            file_path: module_path.clone(),
                            kind: entry.kind,
                            specifiers: entry.specifiers,
                        },
                    ));
                }
                crate::model::ImportResolutionKind::MissingInternal => {
                    let unresolved = unresolved_import(entry.source);
                    broken_imports.push(BrokenImportFinding {
                        file: module_path.clone(),
                        source: unresolved.source,
                        kind: entry.kind,
                    });
                }
                crate::model::ImportResolutionKind::Asset
                | crate::model::ImportResolutionKind::External => {}
            }
        }

        if let Some(module) = modules.get_mut(&module_path) {
            module.resolved_imports = resolved_imports;
            module.import_count = module.resolved_imports.len();
        }

        for (target_path, importer_path, usage) in importer_updates {
            if let Some(target_module) = modules.get_mut(&target_path) {
                push_unique_path(&mut target_module.imported_by, importer_path);
                target_module.imported_by_count = target_module.imported_by.len();
                target_module.importers.push(usage);
            }
        }
    }

    let mut orphan_files = Vec::new();
    let mut dead_exports = Vec::new();
    let mut unused_imports = Vec::new();
    let mut deletion_candidates = Vec::new();

    for module in modules.values() {
        for entry in &module.unused_imports {
            unused_imports.push(UnusedImportFinding {
                file: module.file_path.clone(),
                source: entry.source.clone(),
                local: entry.local.clone(),
                imported: entry.imported.clone(),
            });
        }

        let is_pure_barrel = pure_barrel_files.contains(&module.file_path);

        if module.imported_by.is_empty() && module.entrypoint_kind.is_none() && !is_pure_barrel {
            let classification = classify_orphan(&module.relative_path);

            orphan_files.push(OrphanFileFinding {
                file: module.file_path.clone(),
                kind: classification.kind.clone(),
                reason: classification.reason.clone(),
                confidence: classification.confidence,
            });
            deletion_candidates.push(DeletionCandidateFinding {
                file: module.file_path.clone(),
                reason: classification.reason,
                confidence: classification.confidence,
                safe: true,
            });
        }

        let export_usage = summarize_export_usage(module);
        let should_skip_dead_exports = should_skip_dead_exports(module.entrypoint_kind.as_ref())
            || export_usage.uses_namespace
            || export_usage.uses_unknown
            || is_pure_barrel;

        if !should_skip_dead_exports {
            for exported in &module.exports {
                if exported.name == "*"
                    || export_usage.used_names.contains(&exported.name)
                    || is_framework_consumed_export(
                        module.entrypoint_kind.as_ref(),
                        module.relative_path.as_str(),
                        exported.name.as_str(),
                    )
                {
                    continue;
                }

                dead_exports.push(DeadExportFinding {
                    file: module.file_path.clone(),
                    export_name: exported.name.clone(),
                });
            }
        }
    }

    let mut findings = FindingSet {
        broken_imports: broken_imports,
        orphan_files,
        dead_exports,
        unused_imports,
        route_entrypoints,
        deletion_candidates,
    };
    let suppressions = load_project_suppressions(config);
    let suppressed_findings = apply_suppressions(&mut findings, &suppressions);

    let mut report = ReportV2::new(config.root.clone());
    report.generated_at = Some(current_timestamp());
    report.config_path = config.config_path.clone().or_else(|| {
        let candidate = config.root.join(DEFAULT_CONFIG_FILENAME);
        candidate.exists().then_some(candidate)
    });
    report.summary = SummaryCounts {
        files_scanned: modules.len(),
        entrypoints: modules
            .values()
            .filter(|module| module.entrypoint_kind.is_some())
            .count(),
        broken_imports: findings.broken_imports.len(),
        orphan_files: findings.orphan_files.len(),
        dead_exports: findings.dead_exports.len(),
        unused_imports: findings.unused_imports.len(),
        route_entrypoints: findings.route_entrypoints.len(),
        deletion_candidates: findings.deletion_candidates.len(),
        suppressed_findings,
    };
    report.findings = findings;
    report.modules = modules
        .into_values()
        .map(|mut module| {
            module.imported_by_count = module.imported_by.len();
            module.import_count = module.resolved_imports.len();
            module.export_count = module.exports.len();
            module
        })
        .collect();
    Ok(report)
}

fn dedupe_exports(exports: Vec<ExportRecord>) -> Vec<ExportRecord> {
    let mut seen = BTreeSet::new();
    let mut deduped = Vec::new();

    for entry in exports {
        let key = format!("{:?}:{}", entry.kind, entry.name);
        if seen.insert(key) {
            deduped.push(entry);
        }
    }

    deduped
}

fn summarize_export_usage(module: &ModuleRecord) -> ExportUsage {
    let mut used_names = BTreeSet::new();
    let mut uses_namespace = false;
    let mut uses_unknown = false;

    for importer in &module.importers {
        if importer.kind == ImportKind::Dynamic && importer.specifiers.is_empty() {
            uses_unknown = true;
            continue;
        }

        for specifier in &importer.specifiers {
            match specifier.kind {
                ImportSpecifierKind::Namespace => uses_namespace = true,
                ImportSpecifierKind::Unknown => uses_unknown = true,
                ImportSpecifierKind::Default | ImportSpecifierKind::Named => {
                    if let Some(imported) = &specifier.imported {
                        used_names.insert(imported.clone());
                    }
                }
            }
        }
    }

    ExportUsage {
        used_names,
        uses_namespace,
        uses_unknown,
    }
}

fn classify_orphan(relative_path: &str) -> OrphanClassification {
    let normalized = relative_path.to_ascii_lowercase();
    let file_name = relative_path.rsplit('/').next().unwrap_or(relative_path);

    if normalized.contains("/components/") || file_name.starts_with(char::is_uppercase) {
        return OrphanClassification {
            kind: OrphanKind::Component,
            reason: "Component-like module has no inbound references.".to_string(),
            confidence: 0.92,
        };
    }

    if normalized.contains("/routes/") || is_route_like_file_name(file_name) {
        return OrphanClassification {
            kind: OrphanKind::RouteModule,
            reason: "Route-like module is not connected to any router entry.".to_string(),
            confidence: 0.84,
        };
    }

    OrphanClassification {
        kind: OrphanKind::Module,
        reason: "Module has no inbound references and is not treated as an entrypoint.".to_string(),
        confidence: 0.88,
    }
}

fn should_skip_dead_exports(entrypoint_kind: Option<&EntrypointKind>) -> bool {
    matches!(
        entrypoint_kind,
        Some(
            EntrypointKind::UserEntry
                | EntrypointKind::PackageEntry
                | EntrypointKind::AppEntry
                | EntrypointKind::ToolingEntry
                | EntrypointKind::FrameworkEntry
        )
    )
}

fn is_framework_consumed_export(
    entrypoint_kind: Option<&EntrypointKind>,
    relative_path: &str,
    export_name: &str,
) -> bool {
    match entrypoint_kind {
        Some(EntrypointKind::NextAppRoute) => {
            is_next_app_framework_consumed_export(relative_path, export_name)
        }
        Some(EntrypointKind::NextPagesRoute) => matches!(
            export_name,
            "default"
                | "config"
                | "getInitialProps"
                | "getStaticProps"
                | "getStaticPaths"
                | "getServerSideProps"
        ),
        _ => false,
    }
}

fn is_next_app_framework_consumed_export(relative_path: &str, export_name: &str) -> bool {
    let Some(stem) = route_file_stem(relative_path) else {
        return false;
    };

    match export_name {
        "default" => {
            matches_stem(stem, NEXT_APP_COMPONENT_DEFAULT_STEMS)
                || matches_stem(stem, NEXT_APP_METADATA_DEFAULT_STEMS)
        }
        "metadata" | "generateMetadata" | "viewport" | "generateViewport" => {
            matches_stem(stem, NEXT_APP_METADATA_STEMS)
        }
        "generateImageMetadata" => matches_stem(stem, NEXT_APP_IMAGE_METADATA_STEMS),
        "generateSitemaps" => stem == "sitemap",
        "alt" => matches_stem(stem, NEXT_APP_OPEN_GRAPH_IMAGE_STEMS),
        "size" | "contentType" => {
            matches_stem(stem, NEXT_APP_OPEN_GRAPH_IMAGE_STEMS)
                || matches_stem(stem, NEXT_APP_ICON_IMAGE_STEMS)
        }
        "generateStaticParams" => matches_stem(stem, NEXT_APP_STATIC_PARAMS_STEMS),
        "revalidate" | "dynamic" | "dynamicParams" | "fetchCache" | "runtime"
        | "preferredRegion" | "maxDuration" => matches_stem(stem, NEXT_APP_SEGMENT_CONFIG_STEMS),
        "GET" | "POST" | "PUT" | "PATCH" | "DELETE" | "HEAD" | "OPTIONS" => stem == "route",
        _ => false,
    }
}

fn route_file_stem(relative_path: &str) -> Option<&str> {
    let file_name = relative_path.rsplit('/').next().unwrap_or(relative_path);
    let (stem, extension) = file_name.rsplit_once('.')?;

    if stem.is_empty() || extension.is_empty() || stem.contains('.') || extension.contains('.') {
        return None;
    }

    Some(stem)
}

fn matches_stem(stem: &str, candidates: &[&str]) -> bool {
    candidates.iter().any(|candidate| *candidate == stem)
}

fn is_route_like_file_name(file_name: &str) -> bool {
    let base_name = file_name
        .rsplit_once('.')
        .map(|(stem, _)| stem)
        .unwrap_or(file_name);

    base_name
        .split(|character: char| !character.is_ascii_alphanumeric())
        .any(|token| token.eq_ignore_ascii_case("page") || token.eq_ignore_ascii_case("route"))
}

fn push_unique_path(paths: &mut Vec<std::path::PathBuf>, path: std::path::PathBuf) {
    if !paths.iter().any(|existing| existing == &path) {
        paths.push(path);
    }
}

fn to_project_path(file_path: &Path, root: &Path) -> String {
    file_path
        .strip_prefix(root)
        .unwrap_or(file_path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn current_timestamp() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format_unix_timestamp(now.as_secs(), now.subsec_millis())
}

fn format_unix_timestamp(seconds: u64, millis: u32) -> String {
    let days = (seconds / 86_400) as i64;
    let seconds_of_day = seconds % 86_400;
    let hour = seconds_of_day / 3_600;
    let minute = (seconds_of_day % 3_600) / 60;
    let second = seconds_of_day % 60;
    let (year, month, day) = civil_from_days(days);

    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{millis:03}Z")
}

fn civil_from_days(days_since_epoch: i64) -> (i32, u32, u32) {
    let z = days_since_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = (year_of_era + era * 400) as i32;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };

    if month <= 2 {
        year += 1;
    }

    (year, month as u32, day as u32)
}

struct ExportUsage {
    used_names: BTreeSet<String>,
    uses_namespace: bool,
    uses_unknown: bool,
}

struct OrphanClassification {
    kind: OrphanKind,
    reason: String,
    confidence: f32,
}

#[cfg(test)]
mod tests {
    use super::{classify_orphan, is_framework_consumed_export, should_skip_dead_exports};
    use crate::model::{EntrypointKind, OrphanKind};

    #[test]
    fn classify_orphan_avoids_route_false_positives_from_substrings() {
        assert_eq!(
            classify_orphan("src/lib/package.ts").kind,
            OrphanKind::Module
        );
        assert_eq!(
            classify_orphan("src/lib/router.ts").kind,
            OrphanKind::Module
        );
        assert_eq!(
            classify_orphan("src/routes/account.ts").kind,
            OrphanKind::RouteModule
        );
        assert_eq!(
            classify_orphan("src/features/user.page.tsx").kind,
            OrphanKind::RouteModule
        );
    }

    #[test]
    fn next_route_entrypoints_only_skip_framework_consumed_exports() {
        assert!(is_framework_consumed_export(
            Some(&EntrypointKind::NextAppRoute),
            "app/page.tsx",
            "default"
        ));
        assert!(is_framework_consumed_export(
            Some(&EntrypointKind::NextAppRoute),
            "app/page.tsx",
            "generateMetadata"
        ));
        assert!(is_framework_consumed_export(
            Some(&EntrypointKind::NextAppRoute),
            "app/opengraph-image.tsx",
            "generateImageMetadata"
        ));
        assert!(is_framework_consumed_export(
            Some(&EntrypointKind::NextAppRoute),
            "app/opengraph-image.tsx",
            "alt"
        ));
        assert!(is_framework_consumed_export(
            Some(&EntrypointKind::NextAppRoute),
            "app/opengraph-image.tsx",
            "contentType"
        ));
        assert!(is_framework_consumed_export(
            Some(&EntrypointKind::NextAppRoute),
            "app/icon.tsx",
            "size"
        ));
        assert!(is_framework_consumed_export(
            Some(&EntrypointKind::NextAppRoute),
            "app/icon.tsx",
            "runtime"
        ));
        assert!(is_framework_consumed_export(
            Some(&EntrypointKind::NextAppRoute),
            "app/sitemap.ts",
            "generateSitemaps"
        ));
        assert!(is_framework_consumed_export(
            Some(&EntrypointKind::NextAppRoute),
            "app/global-not-found.tsx",
            "metadata"
        ));
        assert!(is_framework_consumed_export(
            Some(&EntrypointKind::NextAppRoute),
            "app/forbidden.tsx",
            "default"
        ));
        assert!(is_framework_consumed_export(
            Some(&EntrypointKind::NextAppRoute),
            "app/api/route.ts",
            "GET"
        ));
        assert!(is_framework_consumed_export(
            Some(&EntrypointKind::NextAppRoute),
            "app/api/route.ts",
            "dynamic"
        ));
        assert!(is_framework_consumed_export(
            Some(&EntrypointKind::NextPagesRoute),
            "pages/index.tsx",
            "getStaticProps"
        ));
        assert!(is_framework_consumed_export(
            Some(&EntrypointKind::NextPagesRoute),
            "pages/index.tsx",
            "getInitialProps"
        ));
        assert!(!is_framework_consumed_export(
            Some(&EntrypointKind::NextAppRoute),
            "app/page.tsx",
            "GET"
        ));
        assert!(!is_framework_consumed_export(
            Some(&EntrypointKind::NextAppRoute),
            "app/loading.tsx",
            "dynamic"
        ));
        assert!(!is_framework_consumed_export(
            Some(&EntrypointKind::NextAppRoute),
            "app/page.tsx",
            "alt"
        ));
        assert!(!is_framework_consumed_export(
            Some(&EntrypointKind::NextAppRoute),
            "app/page.tsx",
            "helper"
        ));
        assert!(!should_skip_dead_exports(Some(
            &EntrypointKind::NextAppRoute
        )));
        assert!(should_skip_dead_exports(Some(&EntrypointKind::AppEntry)));
    }
}
