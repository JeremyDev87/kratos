use std::path::{Component, Path, PathBuf};

use crate::error::KratosResult;
use crate::jsonc::{parse_loose_json, JsonValue};
use crate::model::{PathAlias, ProjectConfig};

const DEFAULT_CONFIG_FILENAME: &str = "kratos.config.json";
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

    Ok(ProjectConfig {
        root: root.clone(),
        base_url: base_url.clone(),
        roots: normalize_roots(&root, user_config.get("roots"))?,
        ignored_directories: normalize_ignored_directories(user_config.get("ignore")),
        explicit_entries: normalize_entry_paths(&root, user_config.get("entry"))?,
        package_entries: collect_package_entry_files(&root, &package_json),
        path_aliases: normalize_path_aliases(
            &root,
            compiler_options.and_then(|value| value.get("paths")),
            base_url.as_deref(),
        )?,
    })
}

pub fn resolve_config_path(root: &Path, file_name: &str) -> PathBuf {
    root.join(file_name)
}

pub fn apply_path_aliases(
    config: &mut ProjectConfig,
    mut aliases: Vec<PathAlias>,
) -> KratosResult<()> {
    aliases.sort_by(|left, right| right.alias.len().cmp(&left.alias.len()));
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
    Ok(extract_required_string_array(entries, "entry")?
        .into_iter()
        .map(|entry| resolve_path(root, &entry))
        .collect())
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
                target: resolve_path(resolution_base, strip_wildcard(target)),
            });
        }
    }

    aliases.sort_by(|left, right| {
        right.alias.len().cmp(&left.alias.len())
    });
    Ok(aliases)
}

fn collect_package_entry_files(root: &Path, package_json: &JsonValue) -> Vec<PathBuf> {
    let mut entries = Vec::new();

    add_entry_value(&mut entries, root, package_json.get("main"));
    add_entry_value(&mut entries, root, package_json.get("module"));
    add_entry_value(&mut entries, root, package_json.get("types"));

    if let Some(bin) = package_json.get("bin") {
        if let Some(value) = bin.as_str() {
            push_unique_path(&mut entries, resolve_path(root, value));
        } else if let Some(values) = bin.as_object() {
            for value in values.values() {
                add_entry_value(&mut entries, root, Some(value));
            }
        }
    }

    if let Some(exports) = package_json.get("exports") {
        collect_exports(&mut entries, root, exports);
    }

    entries
}

fn collect_exports(entries: &mut Vec<PathBuf>, root: &Path, value: &JsonValue) {
    match value {
        JsonValue::String(path) => push_unique_path(entries, resolve_path(root, path)),
        JsonValue::Array(values) => {
            for nested in values {
                collect_exports(entries, root, nested);
            }
        }
        JsonValue::Object(values) => {
            for nested in values.values() {
                collect_exports(entries, root, nested);
            }
        }
        JsonValue::Null | JsonValue::Bool(_) | JsonValue::Number(_) => {}
    }
}

fn add_entry_value(entries: &mut Vec<PathBuf>, root: &Path, value: Option<&JsonValue>) {
    let Some(value) = value.and_then(JsonValue::as_str) else {
        return;
    };

    push_unique_path(entries, resolve_path(root, value));
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

fn resolve_path(root: &Path, value: &str) -> PathBuf {
    let path = Path::new(value);

    if path.is_absolute() {
        normalize_path(path.to_path_buf())
    } else {
        normalize_path(root.join(path))
    }
}

fn strip_wildcard(value: &str) -> &str {
    value.strip_suffix('*').unwrap_or(value)
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

fn config_error(message: &str) -> crate::error::KratosError {
    crate::error::KratosError::Config(message.to_string())
}

fn normalize_path(path: PathBuf) -> PathBuf {
    let mut normalized = PathBuf::new();

    for component in path.components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(component.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                if !normalized.pop() && !path.is_absolute() {
                    normalized.push(component.as_os_str());
                }
            }
            Component::Normal(part) => normalized.push(part),
        }
    }

    if normalized.as_os_str().is_empty() && !path.is_absolute() {
        PathBuf::from(".")
    } else {
        normalized
    }
}
