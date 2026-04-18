use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use kratos_core::analyze::analyze_project;
use kratos_core::report::{
    format_markdown_report, format_summary_report, parse_report_json, serialize_report_pretty,
    validate_report_version,
};
use serde_json::Value;

#[test]
fn report_v2_matches_parity_fixture_outputs() {
    let repo_root = repo_root();
    let demo_root = repo_root.join("fixtures/demo-app");
    let report_path = demo_root.join(".kratos/latest-report.json");
    let report = analyze_project(&demo_root).expect("demo app should analyze");

    let summary = normalize_text(
        &format_summary_report(&report, &report_path).expect("summary should format"),
        &demo_root,
        &report_path,
        report.generated_at.as_deref(),
    );
    let markdown = normalize_text(
        &format_markdown_report(&report, &report_path).expect("markdown should format"),
        &demo_root,
        &report_path,
        report.generated_at.as_deref(),
    );

    let expected_summary =
        std::fs::read_to_string(repo_root.join("fixtures/parity/demo-app/report-summary.txt"))
            .expect("summary fixture should exist");
    let expected_markdown =
        std::fs::read_to_string(repo_root.join("fixtures/parity/demo-app/report-markdown.md"))
            .expect("markdown fixture should exist");

    assert_eq!(summary.trim_end(), expected_summary.trim_end());
    assert_eq!(markdown.trim_end(), expected_markdown.trim_end());
}

#[test]
fn report_v2_serializes_schema_version_2_and_roundtrips_core_fields() {
    let repo_root = repo_root();
    let demo_root = repo_root.join("fixtures/demo-app");
    let report = analyze_project(&demo_root).expect("demo app should analyze");
    let serialized = serialize_report_pretty(&report).expect("report should serialize");
    let parsed = parse_report_json(&serialized).expect("serialized report should parse");
    validate_report_version(&parsed).expect("v2 report should validate");

    assert_eq!(parsed.version, 2);
    assert_eq!(parsed.root, demo_root);
    assert_eq!(parsed.config_path, report.config_path);
    assert_eq!(parsed.summary, report.summary);
    assert_eq!(
        parsed.findings.broken_imports,
        report.findings.broken_imports
    );
    assert_eq!(parsed.findings.dead_exports, report.findings.dead_exports);
    assert_eq!(
        parsed.findings.unused_imports,
        report.findings.unused_imports
    );
    assert_eq!(
        parsed.findings.route_entrypoints,
        report.findings.route_entrypoints
    );
    assert_eq!(parsed.findings.orphan_files, report.findings.orphan_files);
    assert_eq!(
        parsed.findings.deletion_candidates,
        report.findings.deletion_candidates
    );
    assert_eq!(
        parsed
            .modules
            .iter()
            .map(|module| (
                &module.file_path,
                &module.relative_path,
                &module.entrypoint_kind,
                module.imported_by_count,
                module.import_count,
                module.export_count,
            ))
            .collect::<Vec<_>>(),
        report
            .modules
            .iter()
            .map(|module| (
                &module.file_path,
                &module.relative_path,
                &module.entrypoint_kind,
                module.imported_by_count,
                module.import_count,
                module.export_count,
            ))
            .collect::<Vec<_>>()
    );

    let serialized_value: Value =
        serde_json::from_str(&serialized).expect("serialized report should be valid JSON");
    let fixture_value: Value = serde_json::from_str(
        &std::fs::read_to_string(repo_root.join("fixtures/parity/demo-app/latest-report.v1.json"))
            .expect("report fixture should exist"),
    )
    .expect("fixture JSON should parse");

    let normalized = normalize_json(
        serialized_value,
        &demo_root,
        report.generated_at.as_deref().unwrap_or_default(),
    );

    assert_eq!(normalized["schemaVersion"], Value::from(2));
    assert_eq!(normalized["generatedAt"], Value::from("<GENERATED_AT>"));
    assert_eq!(normalized["project"]["root"], Value::from("<ROOT>"));
    assert_eq!(normalized["project"]["configPath"], Value::Null);
    assert_eq!(normalized["summary"], fixture_value["summary"]);
    assert_eq!(
        normalized["findings"]["brokenImports"],
        fixture_value["findings"]["brokenImports"]
    );
    assert_eq!(
        strip_orphan_confidence(&normalized["findings"]["orphanFiles"]),
        fixture_value["findings"]["orphanFiles"]
    );
    assert_eq!(
        normalized["findings"]["deadExports"],
        fixture_value["findings"]["deadExports"]
    );
    assert_eq!(
        normalized["findings"]["unusedImports"],
        fixture_value["findings"]["unusedImports"]
    );
    assert_eq!(
        normalized["findings"]["routeEntrypoints"],
        fixture_value["findings"]["routeEntrypoints"]
    );
    assert_eq!(
        normalized["findings"]["deletionCandidates"],
        fixture_value["findings"]["deletionCandidates"]
    );
    assert_eq!(normalized["graph"]["modules"], fixture_value["modules"]);

    let reparsed_serialized =
        serialize_report_pretty(&parsed).expect("parsed report should serialize again");
    let reparsed_value: Value =
        serde_json::from_str(&reparsed_serialized).expect("reparsed JSON should parse");
    let normalized_reparsed = normalize_json(
        reparsed_value,
        &demo_root,
        parsed.generated_at.as_deref().unwrap_or_default(),
    );
    assert_eq!(
        normalized_reparsed["graph"]["modules"],
        normalized["graph"]["modules"]
    );
}

#[test]
fn report_v2_rejects_truncated_required_sections() {
    let missing_sections = r#"{
      "schemaVersion": 2,
      "project": {
        "root": "/tmp/kratos"
      }
    }"#;
    let missing_module_file = r#"{
      "schemaVersion": 2,
      "generatedAt": "2026-04-18T00:00:00Z",
      "project": {
        "root": "/tmp/kratos",
        "configPath": null
      },
      "summary": {
        "filesScanned": 1,
        "entrypoints": 0,
        "brokenImports": 0,
        "orphanFiles": 0,
        "deadExports": 0,
        "unusedImports": 0,
        "routeEntrypoints": 0,
        "deletionCandidates": 0
      },
      "findings": {
        "brokenImports": [],
        "orphanFiles": [],
        "deadExports": [],
        "unusedImports": [],
        "routeEntrypoints": [],
        "deletionCandidates": []
      },
      "graph": {
        "modules": [
          {
            "relativePath": "src/index.ts",
            "entrypointKind": null,
            "importedByCount": 0,
            "importCount": 0,
            "exportCount": 0
          }
        ]
      }
    }"#;

    let missing_sections_error =
        parse_report_json(missing_sections).expect_err("missing sections should fail");
    let missing_module_file_error =
        parse_report_json(missing_module_file).expect_err("missing module file should fail");

    assert!(
        missing_sections_error.to_string().contains("summary"),
        "expected summary error, got: {missing_sections_error}"
    );
    assert!(
        missing_module_file_error
            .to_string()
            .contains("graph.modules[0].file"),
        "expected module file error, got: {missing_module_file_error}"
    );
}

#[test]
fn report_v2_rejects_invalid_metadata_field_types() {
    let invalid_generated_at = r#"{
      "schemaVersion": 2,
      "generatedAt": true,
      "project": {
        "root": "/tmp/kratos",
        "configPath": null
      },
      "summary": {
        "filesScanned": 0,
        "entrypoints": 0,
        "brokenImports": 0,
        "orphanFiles": 0,
        "deadExports": 0,
        "unusedImports": 0,
        "routeEntrypoints": 0,
        "deletionCandidates": 0
      },
      "findings": {
        "brokenImports": [],
        "orphanFiles": [],
        "deadExports": [],
        "unusedImports": [],
        "routeEntrypoints": [],
        "deletionCandidates": []
      },
      "graph": {
        "modules": []
      }
    }"#;
    let invalid_config_path = r#"{
      "schemaVersion": 2,
      "generatedAt": "2026-04-19T00:00:00Z",
      "project": {
        "root": "/tmp/kratos",
        "configPath": { "bad": true }
      },
      "summary": {
        "filesScanned": 0,
        "entrypoints": 0,
        "brokenImports": 0,
        "orphanFiles": 0,
        "deadExports": 0,
        "unusedImports": 0,
        "routeEntrypoints": 0,
        "deletionCandidates": 0
      },
      "findings": {
        "brokenImports": [],
        "orphanFiles": [],
        "deadExports": [],
        "unusedImports": [],
        "routeEntrypoints": [],
        "deletionCandidates": []
      },
      "graph": {
        "modules": []
      }
    }"#;

    let generated_at_error =
        parse_report_json(invalid_generated_at).expect_err("invalid generatedAt should fail");
    let config_path_error =
        parse_report_json(invalid_config_path).expect_err("invalid configPath should fail");

    assert!(
        generated_at_error.to_string().contains("generatedAt"),
        "expected generatedAt error, got: {generated_at_error}"
    );
    assert!(
        config_path_error.to_string().contains("project.configPath"),
        "expected configPath error, got: {config_path_error}"
    );
}

#[test]
fn report_v2_rejects_invalid_required_enum_values() {
    let invalid_import_kind = r#"{
      "schemaVersion": 2,
      "generatedAt": "2026-04-19T00:00:00Z",
      "project": {
        "root": "/tmp/kratos",
        "configPath": null
      },
      "summary": {
        "filesScanned": 0,
        "entrypoints": 0,
        "brokenImports": 1,
        "orphanFiles": 0,
        "deadExports": 0,
        "unusedImports": 0,
        "routeEntrypoints": 0,
        "deletionCandidates": 0
      },
      "findings": {
        "brokenImports": [
          {
            "file": "/tmp/kratos/src/main.ts",
            "source": "shared/missing",
            "kind": "bogus"
          }
        ],
        "orphanFiles": [],
        "deadExports": [],
        "unusedImports": [],
        "routeEntrypoints": [],
        "deletionCandidates": []
      },
      "graph": {
        "modules": []
      }
    }"#;
    let invalid_orphan_kind = r#"{
      "schemaVersion": 2,
      "generatedAt": "2026-04-19T00:00:00Z",
      "project": {
        "root": "/tmp/kratos",
        "configPath": null
      },
      "summary": {
        "filesScanned": 0,
        "entrypoints": 0,
        "brokenImports": 0,
        "orphanFiles": 1,
        "deadExports": 0,
        "unusedImports": 0,
        "routeEntrypoints": 0,
        "deletionCandidates": 0
      },
      "findings": {
        "brokenImports": [],
        "orphanFiles": [
          {
            "file": "/tmp/kratos/src/package.ts",
            "kind": "bogus",
            "reason": "bad",
            "confidence": 0.5
          }
        ],
        "deadExports": [],
        "unusedImports": [],
        "routeEntrypoints": [],
        "deletionCandidates": []
      },
      "graph": {
        "modules": []
      }
    }"#;

    let import_error =
        parse_report_json(invalid_import_kind).expect_err("invalid import kind should fail");
    let orphan_error =
        parse_report_json(invalid_orphan_kind).expect_err("invalid orphan kind should fail");

    assert!(
        import_error
            .to_string()
            .contains("findings.brokenImports[0].kind"),
        "expected broken import kind error, got: {import_error}"
    );
    assert!(
        orphan_error
            .to_string()
            .contains("findings.orphanFiles[0].kind"),
        "expected orphan kind error, got: {orphan_error}"
    );
}

#[test]
fn legacy_report_parses_into_renderable_v2_report() {
    let legacy_report = r#"{
      "version": 1,
      "generatedAt": "2026-04-19T00:00:00Z",
      "root": "/tmp/kratos",
      "summary": {
        "filesScanned": 1,
        "entrypoints": 1,
        "brokenImports": 0,
        "orphanFiles": 0,
        "deadExports": 0,
        "unusedImports": 0,
        "routeEntrypoints": 0,
        "deletionCandidates": 0
      },
      "findings": {
        "brokenImports": [],
        "orphanFiles": [],
        "deadExports": [],
        "unusedImports": [],
        "routeEntrypoints": [],
        "deletionCandidates": []
      },
      "modules": [
        {
          "file": "/tmp/kratos/src/main.ts",
          "relativePath": "src/main.ts",
          "entrypointKind": "app-entry",
          "importedByCount": 0,
          "importCount": 0,
          "exportCount": 0
        }
      ]
    }"#;

    let parsed = parse_report_json(legacy_report).expect("legacy report should parse");
    let report_path = Path::new("/tmp/kratos/.kratos/latest-report.json");

    assert_eq!(parsed.version, 2);
    validate_report_version(&parsed).expect("legacy report should be canonicalized to v2");
    serialize_report_pretty(&parsed).expect("legacy report should serialize");
    format_summary_report(&parsed, report_path).expect("legacy report should format as summary");
    format_markdown_report(&parsed, report_path).expect("legacy report should format as markdown");
}

#[test]
fn report_v2_requires_project_root_field() {
    let invalid_root_shape = r#"{
      "schemaVersion": 2,
      "generatedAt": "2026-04-19T00:00:00Z",
      "root": "/tmp/kratos",
      "summary": {
        "filesScanned": 0,
        "entrypoints": 0,
        "brokenImports": 0,
        "orphanFiles": 0,
        "deadExports": 0,
        "unusedImports": 0,
        "routeEntrypoints": 0,
        "deletionCandidates": 0
      },
      "findings": {
        "brokenImports": [],
        "orphanFiles": [],
        "deadExports": [],
        "unusedImports": [],
        "routeEntrypoints": [],
        "deletionCandidates": []
      },
      "graph": {
        "modules": []
      }
    }"#;

    let error = parse_report_json(invalid_root_shape)
        .expect_err("v2 reports without project.root should fail");
    assert!(
        error.to_string().contains("project"),
        "expected project root error, got: {error}"
    );
}

#[test]
fn report_v2_preserves_non_null_config_path() {
    let repo_root = repo_root();
    let demo_root = repo_root.join("fixtures/demo-app");
    let mut report = analyze_project(&demo_root).expect("demo app should analyze");
    let config_path = repo_root.join("kratos.config.json");
    report.config_path = Some(config_path.clone());

    let serialized = serialize_report_pretty(&report).expect("report should serialize");
    let serialized_value: Value =
        serde_json::from_str(&serialized).expect("serialized report should be valid JSON");
    let parsed = parse_report_json(&serialized).expect("serialized report should parse");

    assert_eq!(
        serialized_value["project"]["configPath"],
        Value::from(config_path.to_string_lossy().to_string())
    );
    assert_eq!(parsed.config_path, Some(config_path));
}

#[test]
fn analyze_project_records_config_path_when_default_config_exists() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("kratos-report-config-path-{unique}"));
    std::fs::create_dir_all(root.join("src")).expect("temp project should be created");
    std::fs::write(
        root.join("kratos.config.json"),
        "{\n  \"entry\": [\"src/main.ts\"]\n}\n",
    )
    .expect("config should be written");
    std::fs::write(root.join("src/main.ts"), "export const main = true;\n")
        .expect("source should be written");

    let report = analyze_project(&root).expect("project should analyze");

    assert_eq!(report.config_path, Some(root.join("kratos.config.json")));

    std::fs::remove_dir_all(&root).expect("temp project should be removed");
}

fn normalize_text(
    input: &str,
    root: &Path,
    report_path: &Path,
    generated_at: Option<&str>,
) -> String {
    let report_path = report_path.to_string_lossy().to_string();
    let root = root.to_string_lossy().to_string();
    let mut normalized = input.replace(&report_path, "<REPORT>");
    normalized = normalized.replace(&root, "<ROOT>");

    if let Some(generated_at) = generated_at {
        normalized = normalized.replace(generated_at, "<GENERATED_AT>");
    }

    normalized
}

fn normalize_json(mut value: Value, root: &Path, generated_at: &str) -> Value {
    let root = root.to_string_lossy().to_string();
    normalize_json_value(&mut value, &root, generated_at);
    value
}

fn normalize_json_value(value: &mut Value, root: &str, generated_at: &str) {
    match value {
        Value::String(string) => {
            if string == root {
                *string = "<ROOT>".to_string();
            } else if string == generated_at {
                *string = "<GENERATED_AT>".to_string();
            } else if string.starts_with(root) {
                *string = string.replacen(root, "<ROOT>", 1);
            }
        }
        Value::Array(values) => {
            for item in values {
                normalize_json_value(item, root, generated_at);
            }
        }
        Value::Object(map) => {
            for value in map.values_mut() {
                normalize_json_value(value, root, generated_at);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
}

fn strip_orphan_confidence(value: &Value) -> Value {
    let mut stripped = value.clone();

    if let Value::Array(items) = &mut stripped {
        for item in items {
            if let Value::Object(object) = item {
                object.remove("confidence");
            }
        }
    }

    stripped
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root should resolve")
}
