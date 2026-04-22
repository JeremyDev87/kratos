use std::cmp::Reverse;
use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

use crate::error::KratosResult;
use crate::jsonc::{parse_loose_json, JsonValue};
use crate::model::{PathAlias, ProjectConfig};
use crate::suppressions::{parse_suppression_rules, SuppressionSource};

const DEFAULT_CONFIG_FILENAME: &str = "kratos.config.json";
const PACKAGE_ENTRY_EXTENSIONS: &[&str] = &[
    ".js", ".jsx", ".ts", ".tsx", ".mjs", ".cjs", ".mts", ".cts", ".d.ts", ".d.mts", ".d.cts",
];
const PACKAGE_TYPE_ENTRY_EXTENSIONS: &[&str] = &[
    ".d.ts", ".d.mts", ".d.cts", ".ts", ".tsx", ".mts", ".cts", ".js", ".jsx", ".mjs", ".cjs",
];
const DEFAULT_IGNORED_DIRS: &[&str] = &[
    ".git",
    ".next",
    ".nuxt",
    ".output",
    ".parcel-cache",
    ".svelte-kit",
    ".turbo",
    ".vercel",
    ".yarn",
    "build",
    "coverage",
    "dist",
    "examples",
    "fixtures",
    "node_modules",
    "out",
    "storybook-static",
    "test",
    "tests",
    "__fixtures__",
    "__tests__",
];

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RawConfigDocument {
    pub package_json: Option<JsonValue>,
    pub tsconfig_json: Option<JsonValue>,
    pub kratos_json: Option<JsonValue>,
}

pub fn load_project_config(root: impl Into<PathBuf>) -> KratosResult<ProjectConfig> {
    let root = normalize_project_root(root.into());
    let package_json = read_loose_json_file(&resolve_config_path(&root, "package.json"))?
        .unwrap_or_else(empty_object);
    let tsconfig_json = match read_loose_json_file(&resolve_config_path(&root, "tsconfig.json"))? {
        Some(value) => value,
        None => read_loose_json_file(&resolve_config_path(&root, "jsconfig.json"))?
            .unwrap_or_else(empty_object),
    };
    let user_config = read_loose_json_file(&resolve_config_path(&root, DEFAULT_CONFIG_FILENAME))?
        .unwrap_or_else(empty_object);

    let compiler_options = tsconfig_json.get("compilerOptions");
    let base_url = compiler_options
        .and_then(|value| value.get("baseUrl"))
        .and_then(JsonValue::as_str)
        .map(|value| resolve_path(&root, value));
    let config_path = resolve_config_path(&root, DEFAULT_CONFIG_FILENAME);

    Ok(ProjectConfig {
        root: root.clone(),
        config_path: config_path.exists().then_some(config_path),
        base_url: base_url.clone(),
        roots: normalize_roots(&root, user_config.get("roots"))?,
        ignored_directories: normalize_ignored_directories(user_config.get("ignore")),
        ignore_patterns: extract_required_string_array(
            user_config.get("ignorePatterns"),
            "ignorePatterns",
        )?,
        explicit_entries: normalize_entry_paths(&root, user_config.get("entry"))?,
        package_entries: collect_package_entry_files(&root, &package_json),
        path_aliases: normalize_path_aliases(
            &root,
            compiler_options.and_then(|value| value.get("paths")),
            base_url.as_deref(),
        )?,
        external_packages: collect_external_packages(&package_json),
        suppressions: parse_suppression_rules(
            &root,
            user_config.get("suppressions"),
            SuppressionSource::Config,
        ),
    })
}

pub fn load_clean_min_confidence(root: impl Into<PathBuf>) -> KratosResult<f32> {
    let root = normalize_project_root(root.into());
    let user_config = read_loose_json_file(&resolve_config_path(&root, DEFAULT_CONFIG_FILENAME))?
        .unwrap_or_else(empty_object);

    read_clean_min_confidence(&user_config)
}

pub fn resolve_config_path(root: &Path, file_name: &str) -> PathBuf {
    root.join(file_name)
}

pub fn apply_path_aliases(
    config: &mut ProjectConfig,
    mut aliases: Vec<PathAlias>,
) -> KratosResult<()> {
    // Stable ordering keeps same-length aliases in insertion order, matching JS behavior.
    aliases.sort_by_key(|alias| Reverse(alias.alias.len()));
    config.path_aliases = aliases;
    Ok(())
}

fn read_loose_json_file(file_path: &Path) -> KratosResult<Option<JsonValue>> {
    if !file_path.exists() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(file_path)?;
    Ok(Some(parse_loose_json(&content)?))
}

fn read_clean_min_confidence(user_config: &JsonValue) -> KratosResult<f32> {
    let Some(thresholds) = user_config.get("thresholds") else {
        return Ok(0.0);
    };
    let Some(thresholds) = thresholds.as_object() else {
        return Err(config_error(
            "thresholds must be an object when specifying thresholds.cleanMinConfidence",
        ));
    };

    let Some(value) = thresholds.get("cleanMinConfidence") else {
        return Err(config_error(
            "thresholds.cleanMinConfidence is required when thresholds is present",
        ));
    };

    let Some(value) = (match value {
        JsonValue::Number(raw) => raw.parse::<f64>().ok(),
        _ => None,
    }) else {
        return Err(config_error(
            "thresholds.cleanMinConfidence must be a number between 0.0 and 1.0",
        ));
    };

    validate_clean_min_confidence(value, "thresholds.cleanMinConfidence")
}

fn normalize_roots(root: &Path, roots: Option<&JsonValue>) -> KratosResult<Vec<PathBuf>> {
    let values = extract_required_string_array(roots, "roots")?;

    if values.is_empty() {
        return Ok(vec![root.to_path_buf()]);
    }

    Ok(values
        .into_iter()
        .map(|value| resolve_path(root, &value))
        .collect())
}

fn normalize_ignored_directories(ignore: Option<&JsonValue>) -> Vec<String> {
    let mut values = DEFAULT_IGNORED_DIRS
        .iter()
        .map(|entry| (*entry).to_string())
        .collect::<Vec<_>>();

    for value in extract_string_array(ignore) {
        push_unique_string(&mut values, value);
    }

    values
}

fn normalize_entry_paths(root: &Path, entries: Option<&JsonValue>) -> KratosResult<Vec<PathBuf>> {
    let mut normalized = Vec::new();

    for entry in extract_required_string_array(entries, "entry")? {
        push_unique_path(&mut normalized, resolve_path(root, &entry));
    }

    Ok(normalized)
}

fn normalize_path_aliases(
    root: &Path,
    raw_paths: Option<&JsonValue>,
    base_url: Option<&Path>,
) -> KratosResult<Vec<PathAlias>> {
    let Some(paths) = raw_paths.and_then(JsonValue::as_object) else {
        return Ok(Vec::new());
    };

    let resolution_base = base_url.unwrap_or(root);
    let mut aliases = Vec::new();

    for (alias, targets) in paths.iter() {
        let Some(targets) = targets.as_array() else {
            continue;
        };

        for target in targets {
            let Some(target) = target.as_str() else {
                return Err(config_error(&format!(
                    "compilerOptions.paths['{alias}'] must contain only string targets"
                )));
            };

            aliases.push(PathAlias {
                alias: alias.clone(),
                target: resolve_path(resolution_base, &target.replace('*', "")),
                target_pattern: target
                    .contains('*')
                    .then(|| resolve_alias_target_pattern(resolution_base, target)),
            });
        }
    }

    // Stable ordering keeps same-length aliases in insertion order, matching JS behavior.
    aliases.sort_by_key(|alias| Reverse(alias.alias.len()));
    Ok(aliases)
}

fn collect_package_entry_files(root: &Path, package_json: &JsonValue) -> Vec<PathBuf> {
    let mut entries = Vec::new();

    add_entry_value(&mut entries, root, package_json.get("main"), false);
    add_entry_value(&mut entries, root, package_json.get("module"), false);
    add_entry_value(&mut entries, root, package_json.get("types"), true);

    if let Some(bin) = package_json.get("bin") {
        if bin.as_str().is_some() {
            add_entry_value(&mut entries, root, Some(bin), false);
        } else if let Some(values) = bin.as_object() {
            for value in values.values() {
                add_entry_value(&mut entries, root, Some(value), false);
            }
        }
    }

    if let Some(exports) = package_json.get("exports") {
        collect_exports(&mut entries, root, exports, false);
    }

    entries
}

fn collect_external_packages(package_json: &JsonValue) -> BTreeSet<String> {
    let mut packages = BTreeSet::new();

    collect_dependency_names(&mut packages, package_json.get("dependencies"));
    collect_dependency_names(&mut packages, package_json.get("devDependencies"));
    collect_dependency_names(&mut packages, package_json.get("peerDependencies"));
    collect_dependency_names(&mut packages, package_json.get("optionalDependencies"));

    packages
}

fn collect_dependency_names(packages: &mut BTreeSet<String>, value: Option<&JsonValue>) {
    let Some(entries) = value.and_then(JsonValue::as_object) else {
        return;
    };

    for (name, _) in entries.iter() {
        packages.insert(name.clone());
    }
}

fn collect_exports(
    entries: &mut Vec<PathBuf>,
    root: &Path,
    value: &JsonValue,
    prefer_declaration_files: bool,
) {
    match value {
        JsonValue::String(path) => {
            if path.is_empty() {
                return;
            }

            let resolved = resolve_package_entry_path(root, path, prefer_declaration_files)
                .unwrap_or_else(|| resolve_path(root, path));
            push_unique_path(entries, resolved)
        }
        JsonValue::Array(values) => {
            for nested in values {
                collect_exports(entries, root, nested, prefer_declaration_files);
            }
        }
        JsonValue::Object(values) => {
            for (key, nested) in values.iter() {
                collect_exports(
                    entries,
                    root,
                    nested,
                    prefer_declaration_files || key == "types",
                );
            }
        }
        JsonValue::Null | JsonValue::Bool(_) | JsonValue::Number(_) => {}
    }
}

fn add_entry_value(
    entries: &mut Vec<PathBuf>,
    root: &Path,
    value: Option<&JsonValue>,
    prefer_declaration_files: bool,
) {
    let Some(value) = value.and_then(JsonValue::as_str) else {
        return;
    };

    let resolved = resolve_package_entry_path(root, value, prefer_declaration_files)
        .unwrap_or_else(|| resolve_path(root, value));
    push_unique_path(entries, resolved);
}

fn resolve_package_entry_path(
    root: &Path,
    value: &str,
    prefer_declaration_files: bool,
) -> Option<PathBuf> {
    let resolved = resolve_path(root, value);
    let extensions = package_entry_extensions(prefer_declaration_files);

    if resolved.exists() {
        if resolved.is_dir() {
            return resolve_package_entry_directory(&resolved, extensions);
        }

        return Some(resolved);
    }

    if resolved.extension().is_none() {
        for extension in extensions {
            let candidate = PathBuf::from(format!("{}{}", resolved.to_string_lossy(), extension));
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    None
}

fn resolve_package_entry_directory(directory: &Path, extensions: &[&str]) -> Option<PathBuf> {
    for extension in extensions {
        let candidate = directory.join(format!("index{extension}"));
        if candidate.is_file() {
            return Some(candidate);
        }
    }

    None
}

fn package_entry_extensions(prefer_declaration_files: bool) -> &'static [&'static str] {
    if prefer_declaration_files {
        PACKAGE_TYPE_ENTRY_EXTENSIONS
    } else {
        PACKAGE_ENTRY_EXTENSIONS
    }
}

fn extract_string_array(value: Option<&JsonValue>) -> Vec<String> {
    let Some(values) = value.and_then(JsonValue::as_array) else {
        return Vec::new();
    };

    values
        .iter()
        .filter_map(JsonValue::as_str)
        .map(str::to_string)
        .collect()
}

fn extract_required_string_array(
    value: Option<&JsonValue>,
    field_name: &str,
) -> KratosResult<Vec<String>> {
    let Some(values) = value else {
        return Ok(Vec::new());
    };

    let Some(values) = values.as_array() else {
        return Ok(Vec::new());
    };

    let mut output = Vec::new();

    for entry in values {
        let Some(entry) = entry.as_str() else {
            return Err(config_error(&format!(
                "{field_name} must contain only string values"
            )));
        };

        output.push(entry.to_string());
    }

    Ok(output)
}

fn empty_object() -> JsonValue {
    JsonValue::Object(Default::default())
}

fn normalize_project_root(root: PathBuf) -> PathBuf {
    let absolute = if root.is_absolute() {
        root
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(root)
    };

    normalize_path(absolute)
}

pub(crate) fn resolve_path(root: &Path, value: &str) -> PathBuf {
    let path = Path::new(value);

    if path.is_absolute() {
        normalize_path(path.to_path_buf())
    } else {
        normalize_path(root.join(path))
    }
}

fn push_unique_path(values: &mut Vec<PathBuf>, candidate: PathBuf) {
    if !values.iter().any(|entry| entry == &candidate) {
        values.push(candidate);
    }
}

fn push_unique_string(values: &mut Vec<String>, candidate: String) {
    if !values.iter().any(|entry| entry == &candidate) {
        values.push(candidate);
    }
}

fn resolve_alias_target_pattern(root: &Path, value: &str) -> String {
    const WILDCARD_TOKEN: &str = "__KRATOS_WILDCARD__";

    let tokenized = value.replace('*', WILDCARD_TOKEN);
    let resolved = resolve_path(root, &tokenized);
    resolved.to_string_lossy().replace(WILDCARD_TOKEN, "*")
}

fn config_error(message: &str) -> crate::error::KratosError {
    crate::error::KratosError::Config(message.to_string())
}

fn validate_clean_min_confidence(value: f64, field_name: &str) -> KratosResult<f32> {
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        return Err(config_error(&format!(
            "{field_name} must be between 0.0 and 1.0"
        )));
    }

    Ok(value as f32)
}

fn normalize_path(path: PathBuf) -> PathBuf {
    let mut normalized = PathBuf::new();

    for component in path.components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(component.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => match normalized.components().next_back() {
                Some(Component::Normal(_)) => {
                    normalized.pop();
                }
                Some(Component::ParentDir) | None => {
                    if !path.is_absolute() {
                        normalized.push(component.as_os_str());
                    }
                }
                Some(Component::Prefix(_)) | Some(Component::RootDir) | Some(Component::CurDir) => {
                }
            },
            Component::Normal(part) => normalized.push(part),
        }
    }

    if normalized.as_os_str().is_empty() && !path.is_absolute() {
        PathBuf::from(".")
    } else {
        normalized
    }
}

#[cfg(test)]
mod tests {
    use super::normalize_path;
    use std::path::PathBuf;

    #[test]
    fn normalize_path_preserves_leading_parent_segments() {
        assert_eq!(
            normalize_path(PathBuf::from("../../src")),
            PathBuf::from("../../src")
        );
    }
}
