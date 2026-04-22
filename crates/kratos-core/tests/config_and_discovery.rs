use std::path::{Path, PathBuf};

use kratos_core::config::load_clean_min_confidence;
use kratos_core::config::load_project_config;
use kratos_core::discover::collect_source_files;
use kratos_core::jsonc::parse_loose_json;
use kratos_core::jsonc::JsonValue;
use kratos_core::model::ImportResolutionKind;
use kratos_core::resolve::resolve_import_target;
use kratos_core::suppressions::SuppressionKind;

#[test]
fn load_project_config_parses_comments_and_collects_entries() {
    let project = TestProject::new("config-parsing");
    project.write(
        "package.json",
        r#"{
  "main": "./dist/index.js",
  "module": "./dist/index.mjs",
  "types": "./dist/index.d.ts",
  "bin": {
    "kratos": "./bin/kratos.js"
  },
  "exports": {
    ".": {
      "import": "./dist/index.mjs",
      "types": "./dist/index.d.ts"
    },
    "./cli": "./dist/cli.js",
    "./empty": ""
  }
}
"#,
    );
    project.write(
        "tsconfig.json",
        r#"{
  // comment
  "compilerOptions": {
    "baseUrl": "src",
    "paths": {
      "@/*": ["./*"],
      "@deep/*": ["./deep/*"],
    },
  },
}
"#,
    );
    project.write(
        "kratos.config.json",
        r#"{
  "ignore": ["custom-cache",],
  "ignorePatterns": ["src/generated/**", "!src/generated/keep.ts"],
  "entry": ["src/main.ts", "./src/./main.ts"],
  "roots": ["src", "missing",],
}
"#,
    );

    let config = load_project_config(project.root()).expect("config should load");

    assert_eq!(config.base_url, Some(project.root().join("src")));
    assert_eq!(
        config.roots,
        vec![project.root().join("src"), project.root().join("missing")]
    );
    assert!(config
        .ignored_directories
        .iter()
        .any(|entry| entry == ".git"));
    assert!(config
        .ignored_directories
        .iter()
        .any(|entry| entry == "custom-cache"));
    assert_eq!(
        config.ignore_patterns,
        vec![
            "src/generated/**".to_string(),
            "!src/generated/keep.ts".to_string()
        ]
    );
    assert_eq!(
        config.explicit_entries,
        vec![project.root().join("src/main.ts")]
    );
    assert_eq!(config.path_aliases[0].alias, "@deep/*");
    assert_eq!(config.path_aliases[1].alias, "@/*");
    assert!(config
        .package_entries
        .contains(&project.root().join("dist/index.js")));
    assert!(config
        .package_entries
        .contains(&project.root().join("dist/index.mjs")));
    assert!(config
        .package_entries
        .contains(&project.root().join("dist/index.d.ts")));
    assert!(config
        .package_entries
        .contains(&project.root().join("bin/kratos.js")));
    assert!(config
        .package_entries
        .contains(&project.root().join("dist/cli.js")));
    assert!(
        !config
            .package_entries
            .contains(&project.root().to_path_buf()),
        "empty export targets should be ignored"
    );
}

#[test]
fn load_clean_min_confidence_defaults_to_zero_and_reads_thresholds() {
    let project = TestProject::new("clean-confidence-defaults");

    assert_eq!(
        load_clean_min_confidence(project.root()).expect("default threshold should load"),
        0.0
    );

    project.write(
        "kratos.config.json",
        r#"{
  "thresholds": {
    "cleanMinConfidence": 0.85
  }
}
"#,
    );

    assert_eq!(
        load_clean_min_confidence(project.root()).expect("threshold should load"),
        0.85
    );
}

#[test]
fn load_clean_min_confidence_rejects_out_of_range_values() {
    let project = TestProject::new("clean-confidence-invalid");
    project.write(
        "kratos.config.json",
        r#"{
  "thresholds": {
    "cleanMinConfidence": 1.2
  }
}
"#,
    );

    let error = load_clean_min_confidence(project.root()).expect_err("threshold should fail");
    assert!(
        error
            .to_string()
            .contains("thresholds.cleanMinConfidence must be between 0.0 and 1.0"),
        "unexpected error: {error}"
    );
}

#[test]
fn load_clean_min_confidence_rejects_non_object_thresholds() {
    let project = TestProject::new("clean-confidence-invalid-thresholds-shape");
    project.write(
        "kratos.config.json",
        r#"{
  "thresholds": []
}
"#,
    );

    let error = load_clean_min_confidence(project.root()).expect_err("thresholds shape should fail");
    assert!(
        error
            .to_string()
            .contains("thresholds must be an object when specifying thresholds.cleanMinConfidence"),
        "unexpected error: {error}"
    );
}

#[test]
fn load_clean_min_confidence_rejects_missing_key_when_thresholds_is_present() {
    let project = TestProject::new("clean-confidence-missing-threshold-key");
    project.write(
        "kratos.config.json",
        r#"{
  "thresholds": {}
}
"#,
    );

    let error = load_clean_min_confidence(project.root()).expect_err("missing threshold key should fail");
    assert!(
        error
            .to_string()
            .contains("thresholds.cleanMinConfidence is required when thresholds is present"),
        "unexpected error: {error}"
    );
}

#[test]
fn load_project_config_parses_suppressions_and_ignores_invalid_rules() {
    let project = TestProject::new("suppression-config");
    project.write(
        "kratos.config.json",
        r#"{
  "suppressions": [
    {
      "kind": "brokenImport",
      "file": "src/app/main.ts",
      "source": "./missing",
      "reason": "known missing shim"
    },
    {
      "kind": "deadExport",
      "file": "src/components/LazyCard.tsx",
      "export": "default",
      "reason": "loaded dynamically"
    },
    {
      "kind": "deadExport",
      "file": "../escape.ts",
      "reason": "should be ignored"
    },
    {
      "kind": "unusedImport",
      "file": "/tmp/abs.ts",
      "source": "./abs",
      "local": "abs",
      "reason": "should be ignored"
    },
    {
      "kind": "unusedImport",
      "file": "src/app/main.ts",
      "source": 123,
      "local": "main",
      "reason": "should be ignored"
    },
    {
      "kind": "deadExport",
      "file": "src/app/main.ts",
      "reason": ""
    }
  ]
}
"#,
    );

    let config = load_project_config(project.root()).expect("config should load");

    assert_eq!(config.suppressions.len(), 2);
    assert_eq!(config.suppressions[0].kind, SuppressionKind::BrokenImport);
    assert_eq!(
        config.suppressions[0].file,
        project.root().join("src/app/main.ts")
    );
    assert_eq!(config.suppressions[0].source.as_deref(), Some("./missing"));
    assert_eq!(config.suppressions[0].reason, "known missing shim");
    assert_eq!(config.suppressions[1].kind, SuppressionKind::DeadExport);
    assert_eq!(
        config.suppressions[1].file,
        project.root().join("src/components/LazyCard.tsx")
    );
    assert_eq!(config.suppressions[1].export.as_deref(), Some("default"));
    assert_eq!(config.suppressions[1].reason, "loaded dynamically");
}

#[test]
fn collect_source_files_skips_missing_roots_and_ignored_dirs() {
    let project = TestProject::new("source-discovery");
    project.write(
        "kratos.config.json",
        r#"{
  "roots": ["src", "missing"]
}
"#,
    );
    project.write("src/main.ts", "export const main = true;\n");
    project.write("src/nested/util.ts", "export const util = true;\n");
    project.write("src/dist/generated.js", "export const generated = true;\n");
    project.write("src/assets/logo.svg", "<svg />\n");

    let config = load_project_config(project.root()).expect("config should load");
    let discovered = collect_source_files(&config).expect("source discovery should succeed");

    assert_eq!(
        discovered,
        vec![
            project.root().join("src/main.ts"),
            project.root().join("src/nested/util.ts")
        ]
    );
}

#[test]
fn collect_source_files_supports_gitignore_style_negated_patterns() {
    let project = TestProject::new("ignore-patterns");
    project.write(
        "kratos.config.json",
        r#"{
  "ignorePatterns": ["node_modules/**", "!node_modules/@demo/keep.ts"]
}
"#,
    );
    project.write("src/main.ts", "export const main = true;\n");
    project.write("node_modules/@demo/keep.ts", "export const keep = true;\n");
    project.write("node_modules/@demo/drop.ts", "export const drop = true;\n");

    let config = load_project_config(project.root()).expect("config should load");
    let discovered = collect_source_files(&config).expect("source discovery should succeed");

    assert!(discovered.contains(&project.root().join("src/main.ts")));
    assert!(discovered.contains(&project.root().join("node_modules/@demo/keep.ts")));
    assert!(!discovered.contains(&project.root().join("node_modules/@demo/drop.ts")));
}

#[test]
fn collect_source_files_honors_explicit_roots_even_when_root_name_is_ignored_by_default() {
    let project = TestProject::new("explicit-roots-override-default-ignores");
    project.write(
        "kratos.config.json",
        r#"{
  "roots": ["tests"]
}
"#,
    );
    project.write("tests/unit/sample.ts", "export const sample = true;\n");
    project.write("tests/__fixtures__/skip.ts", "export const skip = true;\n");
    project.write("src/main.ts", "export const main = true;\n");

    let config = load_project_config(project.root()).expect("config should load");
    let discovered = collect_source_files(&config).expect("source discovery should succeed");

    assert_eq!(discovered, vec![project.root().join("tests/unit/sample.ts")]);
}

#[test]
fn collect_source_files_still_respects_directory_only_ignore_patterns_inside_explicit_roots() {
    for pattern in ["tests/", "/tests/"] {
        let project_name = format!("explicit-roots-respect-patterns-{}", pattern.len());
        let project = TestProject::new(&project_name);
        project.write(
            "kratos.config.json",
            &format!(
                r#"{{
  "roots": ["tests"],
  "ignorePatterns": ["{pattern}"]
}}
"#
            ),
        );
        project.write("tests/sample.ts", "export const sample = true;\n");

        let config = load_project_config(project.root()).expect("config should load");
        let discovered = collect_source_files(&config).expect("source discovery should succeed");

        assert!(
            discovered.is_empty(),
            "directory-only ignore pattern {pattern} should still suppress the explicit root contents"
        );
    }
}

#[test]
fn collect_source_files_keeps_anchored_negated_double_star_descendants() {
    let project = TestProject::new("anchored-negated-double-star");
    project.write(
        "kratos.config.json",
        r#"{
  "ignorePatterns": ["src/**", "!/src/**/keep.ts"]
}
"#,
    );
    project.write("src/foo/keep.ts", "export const keep = true;\n");
    project.write("src/foo/drop.ts", "export const drop = true;\n");

    let config = load_project_config(project.root()).expect("config should load");
    let discovered = collect_source_files(&config).expect("source discovery should succeed");

    assert_eq!(discovered, vec![project.root().join("src/foo/keep.ts")]);
}

#[test]
fn collect_source_files_does_not_reopen_unrelated_ignored_trees_for_negations() {
    let project = TestProject::new("unrelated-negation-does-not-reopen");
    project.write(
        "kratos.config.json",
        r#"{
  "ignorePatterns": ["node_modules/**", "!src/generated/keep.ts"]
}
"#,
    );
    project.write("src/generated/keep.ts", "export const keep = true;\n");
    project.write("node_modules/pkg/src/generated/keep.ts", "export const nested = true;\n");

    let config = load_project_config(project.root()).expect("config should load");
    let discovered = collect_source_files(&config).expect("source discovery should succeed");

    assert_eq!(discovered, vec![project.root().join("src/generated/keep.ts")]);
}

#[test]
fn collect_source_files_respects_last_matching_rule_for_negated_descendants() {
    let project = TestProject::new("ordered-negation-does-not-reopen");
    project.write(
        "kratos.config.json",
        r#"{
  "ignorePatterns": ["!src/generated/keep.ts", "src/generated/**"]
}
"#,
    );
    project.write("src/generated/keep.ts", "export const keep = true;\n");
    project.write("src/generated/drop.ts", "export const drop = true;\n");

    let config = load_project_config(project.root()).expect("config should load");
    let discovered = collect_source_files(&config).expect("source discovery should succeed");

    assert!(discovered.is_empty());
}

#[test]
fn resolve_import_target_uses_paths_base_url_and_directory_fallbacks() {
    let project = TestProject::new("resolve-targets");
    project.write(
        "tsconfig.json",
        r#"{
  "compilerOptions": {
    "baseUrl": "src",
    "paths": {
      "@/*": ["./*"],
      "@cfg": ["./config/index.ts"],
      "@view/*": ["./views/*/index.ts"],
      "@fixed/*": ["./fixed/index.ts"]
    }
  }
}
"#,
    );
    project.write("src/app/main.ts", "export const main = true;\n");
    project.write("src/app/logo.svg", "<svg />\n");
    project.write("src/lib/math.ts", "export const add = true;\n");
    project.write("src/shared/index.ts", "export const shared = true;\n");
    project.write("src/config/index.ts", "export const config = true;\n");
    project.write("src/views/home/index.ts", "export const view = true;\n");
    project.write("src/fixed/index.ts", "export const fixed = true;\n");
    project.write("lib/root-entry.ts", "export const rootEntry = true;\n");

    let config = load_project_config(project.root()).expect("config should load");
    let importer = project.root().join("src/app/main.ts");

    let alias = resolve_import_target("@/lib/math", &importer, &config).expect("alias resolves");
    assert_eq!(alias.kind, ImportResolutionKind::Source);
    assert_eq!(alias.path, Some(project.root().join("src/lib/math.ts")));

    let exact_alias = resolve_import_target("@cfg", &importer, &config).expect("exact alias");
    assert_eq!(exact_alias.kind, ImportResolutionKind::Source);
    assert_eq!(
        exact_alias.path,
        Some(project.root().join("src/config/index.ts"))
    );

    let base_url = resolve_import_target("shared", &importer, &config).expect("baseUrl resolves");
    assert_eq!(base_url.kind, ImportResolutionKind::Source);
    assert_eq!(
        base_url.path,
        Some(project.root().join("src/shared/index.ts"))
    );

    let middle_wildcard =
        resolve_import_target("@view/home", &importer, &config).expect("middle wildcard");
    assert_eq!(middle_wildcard.kind, ImportResolutionKind::Source);
    assert_eq!(
        middle_wildcard.path,
        Some(project.root().join("src/views/home/index.ts"))
    );

    let fixed_target =
        resolve_import_target("@fixed/anything", &importer, &config).expect("fixed target");
    assert_eq!(fixed_target.kind, ImportResolutionKind::MissingInternal);
    assert_eq!(fixed_target.path, None);

    let root_relative =
        resolve_import_target("/lib/root-entry", &importer, &config).expect("root relative");
    assert_eq!(root_relative.kind, ImportResolutionKind::Source);
    assert_eq!(
        root_relative.path,
        Some(project.root().join("lib/root-entry.ts"))
    );

    let builtin = resolve_import_target("node:path", &importer, &config).expect("builtin");
    assert_eq!(builtin.kind, ImportResolutionKind::External);
    assert_eq!(builtin.path, None);

    let asset = resolve_import_target("./logo.svg", &importer, &config).expect("asset resolves");
    assert_eq!(asset.kind, ImportResolutionKind::Asset);
    assert_eq!(asset.path, Some(project.root().join("src/app/logo.svg")));

    let missing = resolve_import_target("./missing", &importer, &config).expect("missing");
    assert_eq!(missing.kind, ImportResolutionKind::MissingInternal);
    assert_eq!(missing.path, None);

    let missing_base_url =
        resolve_import_target("shared/missing", &importer, &config).expect("missing baseUrl");
    assert_eq!(missing_base_url.kind, ImportResolutionKind::MissingInternal);
    assert_eq!(missing_base_url.path, None);
}

#[test]
fn resolve_import_target_keeps_declared_external_packages_external() {
    let project = TestProject::new("declared-external-packages");
    project.write(
        "package.json",
        r#"{
  "dependencies": {
    "react": "^18.0.0",
    "@scope/pkg": "^1.0.0"
  }
}
"#,
    );
    project.write(
        "tsconfig.json",
        r#"{
  "compilerOptions": {
    "baseUrl": "src"
  }
}
"#,
    );
    project.write("src/app/main.ts", "export const main = true;\n");
    project.write("src/react/index.ts", "export const fakeReact = true;\n");
    project.write(
        "src/@scope/pkg/index.ts",
        "export const fakeScoped = true;\n",
    );

    let config = load_project_config(project.root()).expect("config should load");
    let importer = project.root().join("src/app/main.ts");

    assert!(config.external_packages.contains("react"));
    assert!(config.external_packages.contains("@scope/pkg"));

    let react =
        resolve_import_target("react/jsx-runtime", &importer, &config).expect("react resolves");
    assert_eq!(react.kind, ImportResolutionKind::External);
    assert_eq!(react.path, None);

    let scoped =
        resolve_import_target("@scope/pkg/runtime", &importer, &config).expect("scope resolves");
    assert_eq!(scoped.kind, ImportResolutionKind::External);
    assert_eq!(scoped.path, None);
}

#[test]
fn resolve_import_target_marks_file_backed_base_url_subpaths_as_missing_internal() {
    let project = TestProject::new("file-backed-baseurl");
    project.write(
        "tsconfig.json",
        r#"{
  "compilerOptions": {
    "baseUrl": "src"
  }
}
"#,
    );
    project.write("src/app/main.ts", "export const main = true;\n");
    project.write("src/helpers.ts", "export const FLAG = true;\n");

    let config = load_project_config(project.root()).expect("config should load");
    let importer = project.root().join("src/app/main.ts");

    let missing =
        resolve_import_target("helpers/foo", &importer, &config).expect("missing subpath");
    assert_eq!(missing.kind, ImportResolutionKind::MissingInternal);
    assert_eq!(missing.path, None);
}

#[test]
fn resolve_import_target_marks_directory_backed_base_url_subpaths_as_missing_internal() {
    let project = TestProject::new("directory-backed-baseurl");
    project.write(
        "tsconfig.json",
        r#"{
  "compilerOptions": {
    "baseUrl": "src"
  }
}
"#,
    );
    project.write("src/app/main.ts", "export const main = true;\n");
    project.write("src/shared/placeholder.txt", "shadow directory\n");

    let config = load_project_config(project.root()).expect("config should load");
    let importer = project.root().join("src/app/main.ts");

    let missing =
        resolve_import_target("shared/missing", &importer, &config).expect("missing subpath");
    assert_eq!(missing.kind, ImportResolutionKind::MissingInternal);
    assert_eq!(missing.path, None);
}

#[test]
fn resolve_import_target_marks_base_url_misses_without_internal_signal_as_missing_internal() {
    let project = TestProject::new("baseurl-miss-without-signal");
    project.write(
        "tsconfig.json",
        r#"{
  "compilerOptions": {
    "baseUrl": "src"
  }
}
"#,
    );
    project.write("src/app/main.ts", "export const main = true;\n");

    let config = load_project_config(project.root()).expect("config should load");
    let importer = project.root().join("src/app/main.ts");

    let missing = resolve_import_target("utils/missing", &importer, &config).expect("missing");
    assert_eq!(missing.kind, ImportResolutionKind::MissingInternal);
    assert_eq!(missing.path, None);
}

#[test]
fn resolve_import_target_keeps_node_builtins_external_with_base_url() {
    let project = TestProject::new("builtin-baseurl");
    project.write(
        "tsconfig.json",
        r#"{
  "compilerOptions": {
    "baseUrl": "src"
  }
}
"#,
    );
    project.write("src/app/main.ts", "export const main = true;\n");

    let config = load_project_config(project.root()).expect("config should load");
    let importer = project.root().join("src/app/main.ts");

    let fs = resolve_import_target("fs", &importer, &config).expect("fs should stay external");
    assert_eq!(fs.kind, ImportResolutionKind::External);
    assert_eq!(fs.path, None);

    let path =
        resolve_import_target("path/posix", &importer, &config).expect("path should stay external");
    assert_eq!(path.kind, ImportResolutionKind::External);
    assert_eq!(path.path, None);
}

#[test]
fn resolve_import_target_allows_base_url_to_shadow_node_builtins() {
    let project = TestProject::new("shadow-builtin");
    project.write(
        "tsconfig.json",
        r#"{
  "compilerOptions": {
    "baseUrl": "src"
  }
}
"#,
    );
    project.write("src/app/main.ts", "export const main = true;\n");
    project.write("src/fs.ts", "export const shadowed = true;\n");

    let config = load_project_config(project.root()).expect("config should load");
    let importer = project.root().join("src/app/main.ts");

    let resolution = resolve_import_target("fs", &importer, &config).expect("fs should resolve");
    assert_eq!(resolution.kind, ImportResolutionKind::Source);
    assert_eq!(resolution.path, Some(project.root().join("src/fs.ts")));
}

#[test]
fn resolve_import_target_allows_base_url_files_to_shadow_declared_packages() {
    let project = TestProject::new("shadowed-dependency");
    project.write(
        "package.json",
        r#"{
  "dependencies": {
    "react": "^18.0.0"
  }
}
"#,
    );
    project.write(
        "tsconfig.json",
        r#"{
  "compilerOptions": {
    "baseUrl": "src"
  }
}
"#,
    );
    project.write("src/app/main.ts", "export const main = true;\n");
    project.write("src/react.ts", "export const shadowed = true;\n");

    let config = load_project_config(project.root()).expect("config should load");
    let importer = project.root().join("src/app/main.ts");

    let resolution = resolve_import_target("react", &importer, &config).expect("react resolves");
    assert_eq!(resolution.kind, ImportResolutionKind::Source);
    assert_eq!(resolution.path, Some(project.root().join("src/react.ts")));
}

#[test]
fn resolve_import_target_uses_nested_package_dependencies_for_base_url_misses() {
    let project = TestProject::new("nested-package-dependencies");
    project.write(
        "tsconfig.json",
        r#"{
  "compilerOptions": {
    "baseUrl": "src"
  }
}
"#,
    );
    project.write(
        "packages/ui/package.json",
        r#"{
  "dependencies": {
    "react": "^18.0.0",
    "@scope/pkg": "^1.0.0"
  }
}
"#,
    );
    project.write("packages/ui/src/app/main.ts", "export const main = true;\n");

    let config = load_project_config(project.root()).expect("config should load");
    let importer = project.root().join("packages/ui/src/app/main.ts");

    let react =
        resolve_import_target("react/jsx-runtime", &importer, &config).expect("react resolves");
    assert_eq!(react.kind, ImportResolutionKind::External);
    assert_eq!(react.path, None);

    let scoped =
        resolve_import_target("@scope/pkg/runtime", &importer, &config).expect("scope resolves");
    assert_eq!(scoped.kind, ImportResolutionKind::External);
    assert_eq!(scoped.path, None);
}

#[test]
fn load_project_config_resolves_extensionless_and_directory_package_entries() {
    let project = TestProject::new("package-entry-resolution");
    project.write(
        "package.json",
        r#"{
  "main": "./src/index",
  "bin": "./src/cli",
  "exports": {
    ".": "./src/routes"
  }
}
"#,
    );
    project.write("src/index.ts", "export const main = true;\n");
    project.write("src/cli.ts", "export const cli = true;\n");
    project.write("src/routes/index.ts", "export const route = true;\n");

    let config = load_project_config(project.root()).expect("config should load");

    assert!(config
        .package_entries
        .contains(&project.root().join("src/index.ts")));
    assert!(config
        .package_entries
        .contains(&project.root().join("src/cli.ts")));
    assert!(config
        .package_entries
        .contains(&project.root().join("src/routes/index.ts")));
}

#[test]
fn load_project_config_prefers_declaration_files_for_types_entries() {
    let project = TestProject::new("types-entry-resolution");
    project.write(
        "package.json",
        r#"{
  "types": "./dist/index",
  "exports": {
    ".": {
      "types": "./dist/types"
    }
  }
}
"#,
    );
    project.write("dist/index.ts", "export const runtime = true;\n");
    project.write("dist/index.d.ts", "export declare const runtime: true;\n");
    project.write("dist/types.ts", "export const typesRuntime = true;\n");
    project.write(
        "dist/types.d.ts",
        "export declare const typesRuntime: true;\n",
    );

    let config = load_project_config(project.root()).expect("config should load");

    assert!(config
        .package_entries
        .contains(&project.root().join("dist/index.d.ts")));
    assert!(config
        .package_entries
        .contains(&project.root().join("dist/types.d.ts")));
    assert!(
        !config
            .package_entries
            .contains(&project.root().join("dist/index.ts")),
        "types entry should not prefer runtime source files over declarations"
    );
    assert!(
        !config
            .package_entries
            .contains(&project.root().join("dist/types.ts")),
        "exports.types should not prefer runtime source files over declarations"
    );
}

#[test]
fn resolve_import_target_preserves_same_length_alias_insertion_order() {
    let project = TestProject::new("alias-order");
    project.write(
        "tsconfig.json",
        r#"{
  "compilerOptions": {
    "baseUrl": "src",
    "paths": {
      "@foo*": ["./preferred/*"],
      "@*bar": ["./fallback/*"]
    }
  }
}
"#,
    );
    project.write("src/main.ts", "export const main = true;\n");
    project.write("src/preferred/bar.ts", "export const preferred = true;\n");
    project.write("src/fallback/foo.ts", "export const fallback = true;\n");

    let config = load_project_config(project.root()).expect("config should load");
    let aliases = config
        .path_aliases
        .iter()
        .map(|alias| alias.alias.as_str())
        .collect::<Vec<_>>();
    assert_eq!(aliases, vec!["@foo*", "@*bar"]);

    let importer = project.root().join("src/main.ts");
    let resolution =
        resolve_import_target("@foobar", &importer, &config).expect("alias should resolve");
    assert_eq!(resolution.kind, ImportResolutionKind::Source);
    assert_eq!(
        resolution.path,
        Some(project.root().join("src/preferred/bar.ts"))
    );
}

#[test]
fn resolve_import_target_honors_wildcard_alias_suffix() {
    let project = TestProject::new("alias-suffix");
    project.write(
        "tsconfig.json",
        r#"{
  "compilerOptions": {
    "baseUrl": "src",
    "paths": {
      "@foo*baz": ["./preferred/*"],
      "@foo*bar": ["./fallback/*"]
    }
  }
}
"#,
    );
    project.write("src/main.ts", "export const main = true;\n");
    project.write("src/preferred/qux.ts", "export const preferred = true;\n");
    project.write("src/fallback/qux.ts", "export const fallback = true;\n");

    let config = load_project_config(project.root()).expect("config should load");
    let importer = project.root().join("src/main.ts");

    let preferred =
        resolve_import_target("@fooquxbaz", &importer, &config).expect("suffix alias resolves");
    assert_eq!(preferred.kind, ImportResolutionKind::Source);
    assert_eq!(
        preferred.path,
        Some(project.root().join("src/preferred/qux.ts"))
    );

    let fallback =
        resolve_import_target("@fooquxbar", &importer, &config).expect("suffix alias resolves");
    assert_eq!(fallback.kind, ImportResolutionKind::Source);
    assert_eq!(
        fallback.path,
        Some(project.root().join("src/fallback/qux.ts"))
    );

    let missing =
        resolve_import_target("@fooquxnope", &importer, &config).expect("suffix mismatch");
    assert_eq!(missing.kind, ImportResolutionKind::MissingInternal);
    assert_eq!(missing.path, None);
}

#[test]
fn load_project_config_normalizes_relative_root_for_discovery_and_resolution() {
    let (project, relative_root) = TestProject::new_relative_to_current_dir("relative-root");
    project.write(
        "tsconfig.json",
        r#"{
  "compilerOptions": {
    "baseUrl": "src"
  }
}
"#,
    );
    project.write(
        "kratos.config.json",
        r#"{
  "roots": ["src"]
}
"#,
    );
    project.write("src/app/main.ts", "export const main = true;\n");
    project.write("src/lib/math.ts", "export const math = true;\n");
    project.write("app/root-entry.ts", "export const rootEntry = true;\n");

    let config = load_project_config(relative_root).expect("relative root should load");
    assert_eq!(config.root, project.root());
    assert_eq!(config.base_url, Some(project.root().join("src")));
    assert_eq!(config.roots, vec![project.root().join("src")]);

    let discovered = collect_source_files(&config).expect("source discovery should succeed");
    assert_eq!(
        discovered,
        vec![
            project.root().join("src/app/main.ts"),
            project.root().join("src/lib/math.ts")
        ]
    );

    let importer = project.root().join("src/app/main.ts");
    let base_url = resolve_import_target("lib/math", &importer, &config).expect("baseUrl resolves");
    assert_eq!(base_url.kind, ImportResolutionKind::Source);
    assert_eq!(base_url.path, Some(project.root().join("src/lib/math.ts")));

    let root_relative =
        resolve_import_target("/app/root-entry", &importer, &config).expect("root relative");
    assert_eq!(root_relative.kind, ImportResolutionKind::Source);
    assert_eq!(
        root_relative.path,
        Some(project.root().join("app/root-entry.ts"))
    );
}

#[test]
fn load_project_config_normalizes_dot_segments_in_base_url_and_alias_targets() {
    let project = TestProject::new("normalized-config-paths");
    project.write(
        "tsconfig.json",
        r#"{
  "compilerOptions": {
    "baseUrl": "./src/app/..",
    "paths": {
      "@/*": ["./generated/../shared/*"]
    }
  }
}
"#,
    );

    let config = load_project_config(project.root()).expect("config should load");
    assert_eq!(config.base_url, Some(project.root().join("src")));
    assert_eq!(config.path_aliases.len(), 1);
    assert_eq!(
        config.path_aliases[0].target,
        project.root().join("src/shared")
    );
}

#[test]
fn resolve_import_target_normalizes_dot_segments_in_requests() {
    let project = TestProject::new("normalized-requests");
    project.write("src/app/main.ts", "export const main = true;\n");
    project.write("src/app/bar.ts", "export const bar = true;\n");

    let config = load_project_config(project.root()).expect("config should load");
    let importer = project.root().join("src/app/main.ts");

    let resolution =
        resolve_import_target("./foo/../bar", &importer, &config).expect("request resolves");
    assert_eq!(resolution.kind, ImportResolutionKind::Source);
    assert_eq!(resolution.path, Some(project.root().join("src/app/bar.ts")));
}

#[test]
fn resolve_import_target_accepts_relative_importer_paths() {
    let (project, relative_root) = TestProject::new_relative_to_current_dir("relative-importer");
    project.write("src/app/main.ts", "export const main = true;\n");
    project.write("src/app/bar.ts", "export const bar = true;\n");

    let config = load_project_config(relative_root.clone()).expect("config should load");
    let importer = relative_root.join("src/app/main.ts");

    let resolution =
        resolve_import_target("./bar", &importer, &config).expect("relative importer resolves");
    assert_eq!(resolution.kind, ImportResolutionKind::Source);
    assert_eq!(resolution.path, Some(project.root().join("src/app/bar.ts")));
}

#[test]
fn parse_loose_json_rejects_control_characters_and_supports_surrogate_pairs() {
    let parsed =
        parse_loose_json(r#"{ "emoji": "\uD83D\uDE00" }"#).expect("surrogate pair should parse");
    let emoji = parsed
        .get("emoji")
        .and_then(JsonValue::as_str)
        .expect("emoji string should exist");
    assert_eq!(emoji, "😀");

    let invalid = parse_loose_json("{\"label\":\"line\nbreak\"}")
        .expect_err("raw newline inside string should be rejected");
    assert!(
        invalid
            .to_string()
            .contains("Unescaped control character in string"),
        "unexpected error: {invalid}"
    );

    let leading_zero =
        parse_loose_json(r#"{"count":01}"#).expect_err("leading-zero numbers should be rejected");
    assert!(
        leading_zero
            .to_string()
            .contains("Leading zeros are not allowed"),
        "unexpected error: {leading_zero}"
    );

    let lone_surrogate = parse_loose_json(r#"{ "value": "\uD83D" }"#)
        .expect("lone surrogate escapes should stay loadable");
    let lone_value = lone_surrogate
        .get("value")
        .and_then(JsonValue::as_str)
        .expect("value string should exist");
    assert!(
        lone_value == "\\uD83D" || lone_value == "\\ud83d",
        "unexpected lone surrogate representation: {lone_value}"
    );
}

#[test]
fn load_project_config_rejects_non_string_array_items() {
    let project = TestProject::new("invalid-config-items");
    project.write(
        "kratos.config.json",
        r#"{
  "roots": [null],
  "entry": ["src/main.ts"]
}
"#,
    );

    let error = load_project_config(project.root()).expect_err("invalid roots should fail");
    assert!(
        error
            .to_string()
            .contains("roots must contain only string values"),
        "unexpected error: {error}"
    );

    project.write(
        "kratos.config.json",
        r#"{
  "entry": [null]
}
"#,
    );

    let error = load_project_config(project.root()).expect_err("invalid entry should fail");
    assert!(
        error
            .to_string()
            .contains("entry must contain only string values"),
        "unexpected error: {error}"
    );

    project.write(
        "tsconfig.json",
        r#"{
  "compilerOptions": {
    "paths": {
      "@/*": [null]
    }
  }
}
"#,
    );
    project.write("kratos.config.json", "{}\n");

    let error = load_project_config(project.root()).expect_err("invalid paths should fail");
    assert!(
        error
            .to_string()
            .contains("compilerOptions.paths['@/*'] must contain only string targets"),
        "unexpected error: {error}"
    );

    project.write(
        "kratos.config.json",
        r#"{
  "roots": "src"
}
"#,
    );
    project.write("tsconfig.json", "{}\n");

    let config = load_project_config(project.root()).expect("non-array roots should be ignored");
    assert_eq!(config.roots, vec![project.root().to_path_buf()]);

    project.write(
        "tsconfig.json",
        r#"{
  "compilerOptions": {
    "paths": []
  }
}
"#,
    );
    project.write("kratos.config.json", "{}\n");

    let config = load_project_config(project.root()).expect("non-object paths should be ignored");
    assert!(config.path_aliases.is_empty());

    project.write(
        "tsconfig.json",
        r#"{
  "compilerOptions": {
    "paths": {
      "@/*": "./src/*"
    }
  }
}
"#,
    );

    let config =
        load_project_config(project.root()).expect("non-array alias targets should be ignored");
    assert!(config.path_aliases.is_empty());
}

struct TestProject {
    root: PathBuf,
}

impl TestProject {
    fn new(label: &str) -> Self {
        let mut root = std::env::temp_dir();
        root.push(format!(
            "kratos-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time should move forward")
                .as_nanos()
        ));

        std::fs::create_dir_all(&root).expect("temp project should be created");
        Self { root }
    }

    fn new_relative_to_current_dir(label: &str) -> (Self, PathBuf) {
        let current_dir = std::env::current_dir().expect("current dir should be available");
        let unique = format!(
            "kratos-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time should move forward")
                .as_nanos()
        );
        let relative = PathBuf::from(".tmp").join(unique);
        let root = current_dir.join(&relative);

        std::fs::create_dir_all(&root).expect("temp project should be created");
        (Self { root }, relative)
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
