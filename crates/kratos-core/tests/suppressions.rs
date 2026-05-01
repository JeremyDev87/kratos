use std::path::{Path, PathBuf};

use kratos_core::analyze::analyze_project;
use kratos_core::model::{
    BrokenImportFinding, DeadExportFinding, DeletionCandidateFinding, EntrypointKind, FindingSet,
    ImportKind, OrphanFileFinding, OrphanKind, RouteEntrypointFinding, UnusedImportFinding,
};
use kratos_core::report::{parse_report_json, serialize_report_pretty};
use kratos_core::report_format::{format_markdown_report, format_summary_report};
use kratos_core::suppressions::{
    apply_suppressions, load_project_suppressions, SuppressionKind, SuppressionRule,
    SuppressionSource,
};

#[test]
fn load_project_suppressions_merges_config_and_generated_rules() {
    let project = TestProject::new("suppression-loading");
    project.write(
        "kratos.config.json",
        r#"{
  "suppressions": [
    {
      "kind": "brokenImport",
      "file": "src/index.ts",
      "source": "./missing",
      "reason": "config rule"
    }
  ]
}
"#,
    );
    project.write(
        ".kratos/suppressions.json",
        r#"{
  "suppressions": [
    {
      "kind": "deadExport",
      "file": "src/feature.ts",
      "export": "helper",
      "reason": "generated rule"
    },
    {
      "kind": "deadExport",
      "file": "../escape.ts",
      "reason": "ignored"
    }
  ]
}
"#,
    );

    let config = kratos_core::config::load_project_config(project.root()).expect("config loads");
    let suppressions = load_project_suppressions(&config);

    assert_eq!(suppressions.len(), 2);
    assert_eq!(suppressions[0].origin, SuppressionSource::Config);
    assert_eq!(suppressions[1].origin, SuppressionSource::Generated);
    assert_eq!(suppressions[0].file, project.root().join("src/index.ts"));
    assert_eq!(suppressions[1].file, project.root().join("src/feature.ts"));
}

#[test]
fn apply_suppressions_matches_exact_fields_and_counts_each_finding_once() {
    let root = project_root("suppression-match");
    let mut findings = FindingSet {
        broken_imports: vec![
            BrokenImportFinding {
                file: root.join("src/index.ts"),
                source: "./missing-a".to_string(),
                kind: ImportKind::Static,
            },
            BrokenImportFinding {
                file: root.join("src/index.ts"),
                source: "./missing-b".to_string(),
                kind: ImportKind::Static,
            },
        ],
        orphan_files: vec![OrphanFileFinding {
            file: root.join("src/orphan.ts"),
            kind: OrphanKind::Module,
            reason: "orphan".to_string(),
            confidence: 0.9,
        }],
        dead_exports: vec![
            DeadExportFinding {
                file: root.join("src/dead.ts"),
                export_name: "default".to_string(),
            },
            DeadExportFinding {
                file: root.join("src/dead.ts"),
                export_name: "helper".to_string(),
            },
        ],
        unused_imports: vec![
            UnusedImportFinding {
                file: root.join("src/unused.ts"),
                source: "./lib".to_string(),
                local: "lib".to_string(),
                imported: "lib".to_string(),
            },
            UnusedImportFinding {
                file: root.join("src/unused.ts"),
                source: "./lib".to_string(),
                local: "other".to_string(),
                imported: "other".to_string(),
            },
        ],
        route_entrypoints: vec![RouteEntrypointFinding {
            file: root.join("src/route.ts"),
            kind: EntrypointKind::NextAppRoute,
        }],
        deletion_candidates: vec![DeletionCandidateFinding {
            file: root.join("src/delete.ts"),
            reason: "delete".to_string(),
            confidence: 0.8,
            safe: true,
        }],
    };

    let suppressions = vec![
        SuppressionRule {
            kind: SuppressionKind::BrokenImport,
            file: root.join("src/index.ts"),
            source: Some("./missing-a".to_string()),
            local: None,
            export: None,
            reason: "config".to_string(),
            origin: SuppressionSource::Config,
        },
        SuppressionRule {
            kind: SuppressionKind::BrokenImport,
            file: root.join("src/index.ts"),
            source: Some("./missing-a".to_string()),
            local: None,
            export: None,
            reason: "generated".to_string(),
            origin: SuppressionSource::Generated,
        },
        SuppressionRule {
            kind: SuppressionKind::OrphanFile,
            file: root.join("src/orphan.ts"),
            source: None,
            local: None,
            export: None,
            reason: "config".to_string(),
            origin: SuppressionSource::Config,
        },
        SuppressionRule {
            kind: SuppressionKind::DeadExport,
            file: root.join("src/dead.ts"),
            source: None,
            local: None,
            export: Some("default".to_string()),
            reason: "config".to_string(),
            origin: SuppressionSource::Config,
        },
        SuppressionRule {
            kind: SuppressionKind::UnusedImport,
            file: root.join("src/unused.ts"),
            source: Some("./lib".to_string()),
            local: Some("lib".to_string()),
            export: None,
            reason: "config".to_string(),
            origin: SuppressionSource::Config,
        },
        SuppressionRule {
            kind: SuppressionKind::DeletionCandidate,
            file: root.join("src/delete.ts"),
            source: None,
            local: None,
            export: None,
            reason: "config".to_string(),
            origin: SuppressionSource::Config,
        },
    ];

    let suppressed = apply_suppressions(&mut findings, &suppressions);

    assert_eq!(suppressed, 5);
    assert_eq!(findings.broken_imports.len(), 1);
    assert_eq!(findings.broken_imports[0].source, "./missing-b");
    assert!(findings.orphan_files.is_empty());
    assert_eq!(findings.dead_exports.len(), 1);
    assert_eq!(findings.dead_exports[0].export_name, "helper");
    assert_eq!(findings.unused_imports.len(), 1);
    assert_eq!(findings.unused_imports[0].local, "other");
    assert_eq!(findings.route_entrypoints.len(), 1);
    assert!(findings.deletion_candidates.is_empty());
}

#[test]
fn analyze_project_surfaces_suppressed_findings_in_summary_and_formatters() {
    let project = TestProject::new("suppression-analysis");
    project.write(
        "kratos.config.json",
        r#"{
  "suppressions": [
    {
      "kind": "brokenImport",
      "file": "src/index.ts",
      "source": "./missing",
      "reason": "config rule"
    }
  ]
}
"#,
    );
    project.write(
        ".kratos/suppressions.json",
        r#"{
  "suppressions": [
    {
      "kind": "deadExport",
      "file": "src/feature.ts",
      "export": "helper",
      "reason": "generated rule"
    },
    {
      "kind": "deletionCandidate",
      "file": "src/feature.ts",
      "reason": "generated rule"
    }
  ]
}
"#,
    );
    project.write(
        "src/index.ts",
        "import './missing';\nexport const main = true;\n",
    );
    project.write("src/feature.ts", "export const helper = true;\n");

    let report = analyze_project(project.root()).expect("analysis should succeed");
    let report_path = project.root().join(".kratos/latest-report.json");
    let summary = format_summary_report(&report, &report_path, "Kratos scan complete.")
        .expect("summary should format");
    let markdown = format_markdown_report(&report, &report_path).expect("markdown should format");
    let serialized = serialize_report_pretty(&report).expect("report should serialize");
    let parsed = parse_report_json(&serialized).expect("serialized report should parse");

    assert_eq!(report.findings.broken_imports.len(), 0);
    assert_eq!(report.findings.dead_exports.len(), 0);
    assert_eq!(report.findings.deletion_candidates.len(), 0);
    assert_eq!(report.summary.suppressed_findings, 3);
    assert_eq!(parsed.summary.suppressed_findings, 3);
    assert!(summary.contains("숨김 처리된 항목: 3"));
    assert!(markdown.contains("- 숨김 처리된 항목: 3"));
    assert!(
        serialized.contains("\"suppressedFindings\": 3"),
        "expected optional suppressed findings field in serialized report"
    );
}

fn project_root(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "kratos-{label}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time should move forward")
            .as_nanos()
    ))
}

struct TestProject {
    root: PathBuf,
}

impl TestProject {
    fn new(label: &str) -> Self {
        let root = project_root(label);
        std::fs::create_dir_all(&root).expect("temp project should be created");
        Self { root }
    }

    fn root(&self) -> &Path {
        &self.root
    }

    fn write(&self, relative: &str, contents: &str) {
        let path = self.root.join(relative);

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("parent directories should exist");
        }

        std::fs::write(path, contents).expect("file should be written");
    }
}

impl Drop for TestProject {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
