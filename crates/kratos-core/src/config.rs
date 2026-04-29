use std::cmp::Reverse;
use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

use crate::error::KratosResult;
use crate::jsonc::{parse_loose_json, JsonValue};
use crate::model::{PathAlias, ProjectConfig};
use crate::suppressions::{parse_suppression_rules, SuppressionSource};

const DEFAULT_CONFIG_FILENAME: &str = "kratos.config.json";
const GITIGNORE_FILENAME: &str = ".gitignore";
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

    let package_entries = collect_project_entry_files(&root, &package_json)?;

    Ok(ProjectConfig {
        root: root.clone(),
        config_path: config_path.exists().then_some(config_path),
        base_url: base_url.clone(),
        roots: normalize_roots(&root, user_config.get("roots"))?,
        ignored_directories: normalize_ignored_directories(user_config.get("ignore")),
        ignore_patterns: normalize_ignore_patterns(
            &root,
            extract_required_string_array(user_config.get("ignorePatterns"), "ignorePatterns")?,
        )?,
        explicit_entries: normalize_entry_paths(&root, user_config.get("entry"))?,
        package_entries,
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

fn normalize_ignore_patterns(root: &Path, user_patterns: Vec<String>) -> KratosResult<Vec<String>> {
    let mut patterns = read_gitignore_patterns(&resolve_config_path(root, GITIGNORE_FILENAME))?;
    patterns.extend(user_patterns);
    Ok(patterns)
}

fn read_gitignore_patterns(file_path: &Path) -> KratosResult<Vec<String>> {
    if !file_path.exists() {
        return Ok(Vec::new());
    }

    let content = std::fs::read_to_string(file_path)?;
    Ok(content
        .lines()
        .filter_map(normalize_gitignore_line)
        .collect())
}

fn normalize_gitignore_line(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }

    Some(trimmed.to_string())
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

fn collect_project_entry_files(
    root: &Path,
    package_json: &JsonValue,
) -> KratosResult<Vec<PathBuf>> {
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

    collect_package_script_entry_files(&mut entries, root, package_json, 0);
    collect_workflow_run_entry_files(&mut entries, root, package_json)?;

    Ok(entries)
}

fn collect_package_script_entry_files(
    entries: &mut Vec<PathBuf>,
    root: &Path,
    package_json: &JsonValue,
    depth: usize,
) {
    let Some(scripts) = package_json.get("scripts").and_then(JsonValue::as_object) else {
        return;
    };

    for script in scripts.values().filter_map(JsonValue::as_str) {
        collect_command_entry_files(entries, root, root, package_json, script, depth);
    }
}

fn collect_workflow_run_entry_files(
    entries: &mut Vec<PathBuf>,
    root: &Path,
    package_json: &JsonValue,
) -> KratosResult<()> {
    collect_yamlish_run_entries(
        entries,
        root,
        package_json,
        &root.join(".github/workflows"),
        is_workflow_file,
    )?;
    collect_yamlish_run_entries(
        entries,
        root,
        package_json,
        &root.join(".github/actions"),
        is_action_file,
    )
}

fn collect_yamlish_run_entries(
    entries: &mut Vec<PathBuf>,
    root: &Path,
    package_json: &JsonValue,
    directory: &Path,
    include_file: fn(&Path) -> bool,
) -> KratosResult<()> {
    if !directory.is_dir() {
        return Ok(());
    }

    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let path = entry.path();

        if file_type.is_dir() {
            collect_yamlish_run_entries(entries, root, package_json, &path, include_file)?;
            continue;
        }

        if file_type.is_file() && include_file(&path) {
            let content = std::fs::read_to_string(&path)?;
            for command in extract_yamlish_run_commands(&content) {
                let command_root = command
                    .working_directory
                    .as_deref()
                    .map(|directory| resolve_path(root, directory))
                    .unwrap_or_else(|| root.to_path_buf());
                collect_command_entry_files(
                    entries,
                    root,
                    &command_root,
                    package_json,
                    &command.command,
                    0,
                );
            }
        }
    }

    Ok(())
}

fn is_workflow_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|value| value.to_str()),
        Some("yml" | "yaml")
    )
}

fn is_action_file(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|value| value.to_str()),
        Some("action.yml" | "action.yaml")
    )
}

struct YamlishRunCommand {
    command: String,
    working_directory: Option<String>,
}

fn extract_yamlish_run_commands(content: &str) -> Vec<YamlishRunCommand> {
    let lines = content.lines().collect::<Vec<_>>();
    let mut commands = Vec::new();
    let mut index = 0;

    while index < lines.len() {
        let line = lines[index];
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();
        let run_index = index;
        let Some(raw) = trimmed
            .strip_prefix("run:")
            .or_else(|| trimmed.strip_prefix("- run:"))
        else {
            index += 1;
            continue;
        };

        let raw = raw.trim_start();
        if raw.starts_with('|') || raw.starts_with('>') {
            let mut block = String::new();
            index += 1;
            while index < lines.len() {
                let next = lines[index];
                if !next.trim().is_empty() && leading_spaces(next) <= indent {
                    break;
                }
                block.push_str(next.trim());
                block.push('\n');
                index += 1;
            }
            commands.push(YamlishRunCommand {
                command: block,
                working_directory: find_yamlish_working_directory(&lines, run_index, indent),
            });
            continue;
        }

        commands.push(YamlishRunCommand {
            command: unquote_yaml_scalar(raw).to_string(),
            working_directory: find_yamlish_working_directory(&lines, run_index, indent),
        });
        index += 1;
    }

    commands
}

fn find_yamlish_working_directory(
    lines: &[&str],
    run_index: usize,
    run_indent: usize,
) -> Option<String> {
    let mut start = run_index;
    while start > 0 {
        let previous = lines[start - 1];
        let previous_indent = leading_spaces(previous);
        let previous_trimmed = previous.trim_start();
        if previous_indent < run_indent || previous_trimmed.starts_with("- ") {
            if previous_trimmed.starts_with("- ") {
                start -= 1;
            }
            break;
        }
        start -= 1;
    }

    let mut end = run_index + 1;
    while end < lines.len() {
        let next = lines[end];
        let next_indent = leading_spaces(next);
        let next_trimmed = next.trim_start();
        if next_indent < run_indent || (next_indent == run_indent && next_trimmed.starts_with("- "))
        {
            break;
        }
        end += 1;
    }

    lines[start..end]
        .iter()
        .filter_map(|line| {
            let trimmed = line.trim_start();
            trimmed
                .strip_prefix("working-directory:")
                .or_else(|| trimmed.strip_prefix("- working-directory:"))
                .map(str::trim)
        })
        .next()
        .map(unquote_yaml_scalar)
        .map(str::to_string)
}

fn leading_spaces(value: &str) -> usize {
    value.len() - value.trim_start().len()
}

fn unquote_yaml_scalar(value: &str) -> &str {
    value
        .strip_prefix('"')
        .and_then(|inner| inner.strip_suffix('"'))
        .or_else(|| {
            value
                .strip_prefix('\'')
                .and_then(|inner| inner.strip_suffix('\''))
        })
        .unwrap_or(value)
}

fn collect_command_entry_files(
    entries: &mut Vec<PathBuf>,
    project_root: &Path,
    command_root: &Path,
    package_json: &JsonValue,
    command: &str,
    depth: usize,
) {
    if depth > 4 {
        return;
    }

    let tokens = shellish_tokens(command);
    for (index, token) in tokens.iter().enumerate() {
        if is_script_runner(token) {
            if let Some(script_name) = package_script_name_after_runner(&tokens, index) {
                if let Some(script) = package_script(package_json, script_name) {
                    collect_command_entry_files(
                        entries,
                        project_root,
                        project_root,
                        package_json,
                        script,
                        depth + 1,
                    );
                }
            }
            continue;
        }

        if is_script_interpreter(token) {
            collect_interpreter_entry_paths(entries, command_root, &tokens, index);
            continue;
        }

        if looks_like_local_script_path(token) {
            add_script_entry_path(entries, command_root, token);
        }
    }
}

fn shellish_tokens(command: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    let mut escaped = false;

    for character in command.chars() {
        if escaped {
            current.push(character);
            escaped = false;
            continue;
        }

        if character == '\\' {
            escaped = true;
            continue;
        }

        if let Some(active_quote) = quote {
            if character == active_quote {
                quote = None;
            } else {
                current.push(character);
            }
            continue;
        }

        match character {
            '\'' | '"' => quote = Some(character),
            '\n' | '\r' | '\t' | ' ' | '&' | '|' | ';' | '(' | ')' => {
                push_token(&mut tokens, &mut current);
            }
            '#' if current.is_empty() => {
                push_token(&mut tokens, &mut current);
            }
            _ => current.push(character),
        }
    }

    push_token(&mut tokens, &mut current);
    tokens
}

fn push_token(tokens: &mut Vec<String>, current: &mut String) {
    if !current.is_empty() {
        tokens.push(std::mem::take(current));
    }
}

fn is_script_runner(token: &str) -> bool {
    matches!(
        command_name(token),
        "npm" | "pnpm" | "yarn" | "bun" | "corepack"
    )
}

fn package_script_name_after_runner<'a>(tokens: &'a [String], index: usize) -> Option<&'a str> {
    let runner = command_name(tokens.get(index)?);
    let mut cursor = index + 1;

    if runner == "corepack" {
        cursor += 1;
    }

    if matches!(runner, "npm" | "pnpm" | "bun" | "corepack") {
        if matches!(
            tokens.get(cursor).map(|value| value.as_str()),
            Some("run" | "run-script")
        ) {
            cursor += 1;
        } else {
            return None;
        }
    } else if matches!(tokens.get(cursor).map(|value| value.as_str()), Some("run")) {
        cursor += 1;
    }

    while tokens
        .get(cursor)
        .is_some_and(|token| token.starts_with('-'))
    {
        cursor += 1;
    }

    tokens.get(cursor).map(String::as_str)
}

fn package_script<'a>(package_json: &'a JsonValue, name: &str) -> Option<&'a str> {
    package_json
        .get("scripts")
        .and_then(JsonValue::as_object)?
        .get(name)?
        .as_str()
}

fn is_script_interpreter(token: &str) -> bool {
    matches!(
        command_name(token),
        "node" | "tsx" | "ts-node" | "ts-node-esm" | "bash" | "sh" | "zsh"
    )
}

fn collect_interpreter_entry_paths(
    entries: &mut Vec<PathBuf>,
    command_root: &Path,
    tokens: &[String],
    index: usize,
) {
    let mut cursor = index + 1;

    while let Some(token) = tokens.get(cursor) {
        if is_eval_like_option(token) {
            return;
        }

        if option_takes_value(token) {
            if let Some(value) = inline_option_value(token) {
                if looks_like_interpreter_script_path(value) {
                    add_script_entry_path(entries, command_root, value);
                }
                cursor += 1;
            } else {
                if let Some(value) = tokens.get(cursor + 1) {
                    if looks_like_interpreter_script_path(value) {
                        add_script_entry_path(entries, command_root, value);
                    }
                }
                cursor += 2;
            }
            continue;
        }

        if token.starts_with('-') || token.contains('=') && !looks_like_local_script_path(token) {
            cursor += 1;
            continue;
        }

        if looks_like_interpreter_script_path(token) {
            add_script_entry_path(entries, command_root, token);
        }
        return;
    }
}

fn is_eval_like_option(token: &str) -> bool {
    matches!(
        token,
        "-e" | "--eval" | "-p" | "--print" | "-c" | "--command"
    )
}

fn option_takes_value(token: &str) -> bool {
    let name = token.split_once('=').map(|(name, _)| name).unwrap_or(token);
    matches!(
        name,
        "-r" | "--require" | "--import" | "--loader" | "--env-file" | "--project" | "--tsconfig"
    )
}

fn inline_option_value(token: &str) -> Option<&str> {
    if let Some((_, value)) = token.split_once('=') {
        return Some(value);
    }

    if token.starts_with("-r") && token.len() > 2 {
        return Some(&token[2..]);
    }

    None
}

fn add_script_entry_path(entries: &mut Vec<PathBuf>, root: &Path, value: &str) {
    let value = value.trim_matches(|character| matches!(character, '"' | '\'' | '`'));
    let resolved =
        resolve_package_entry_path(root, value, false).unwrap_or_else(|| resolve_path(root, value));
    push_unique_path(entries, resolved);
}

fn looks_like_local_script_path(token: &str) -> bool {
    if token.starts_with('-')
        || token.starts_with('$')
        || token.starts_with("http://")
        || token.starts_with("https://")
        || token.contains('=')
    {
        return false;
    }

    let path = Path::new(token);
    let Some(extension) = path.extension().and_then(|value| value.to_str()) else {
        return false;
    };

    let has_local_shape = token.starts_with('.')
        || token.starts_with('/')
        || token.contains('/')
        || token.contains('\\');
    has_local_shape
        && matches!(
            extension,
            "js" | "jsx" | "ts" | "tsx" | "mjs" | "cjs" | "mts" | "cts" | "sh"
        )
}

fn looks_like_interpreter_script_path(token: &str) -> bool {
    if looks_like_local_script_path(token) {
        return true;
    }

    if token.starts_with('-')
        || token.starts_with('$')
        || token.starts_with("http://")
        || token.starts_with("https://")
        || token.contains('=')
    {
        return false;
    }

    matches!(
        Path::new(token)
            .extension()
            .and_then(|value| value.to_str()),
        Some("js" | "jsx" | "ts" | "tsx" | "mjs" | "cjs" | "mts" | "cts" | "sh")
    )
}

fn command_name(token: &str) -> &str {
    token.rsplit(['/', '\\']).next().unwrap_or(token)
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
