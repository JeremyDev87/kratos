use std::path::{Component, Path, PathBuf};

use crate::error::KratosResult;
use crate::jsonc::{parse_loose_json, JsonValue};
use crate::model::{ImportResolution, ImportResolutionKind, PathAlias, ProjectConfig};

const SOURCE_EXTENSIONS: &[&str] = &[".js", ".jsx", ".ts", ".tsx", ".mjs", ".cjs", ".mts", ".cts"];

pub fn unresolved_import(source: impl Into<String>) -> ImportResolution {
    ImportResolution {
        kind: ImportResolutionKind::MissingInternal,
        source: source.into(),
        path: None,
    }
}

pub fn resolve_import_target(
    request: &str,
    importer_path: &Path,
    config: &ProjectConfig,
) -> KratosResult<ImportResolution> {
    let project_root = normalize_config_path(&config.root);
    let importer_path = normalize_config_path(importer_path);

    if request.starts_with("node:") {
        return Ok(external_import(request, None));
    }

    if request.starts_with('.') {
        let base = importer_path
            .parent()
            .unwrap_or(project_root.as_path())
            .join(request);
        return resolve_internal_path(&base, request);
    }

    if request.starts_with('/') {
        let relative = request.trim_start_matches('/');
        return resolve_internal_path(&project_root.join(relative), request);
    }

    for alias in &config.path_aliases {
        if !matches_alias(&alias.alias, request) {
            continue;
        }

        let candidate = resolve_aliased_import(request, alias);
        return resolve_internal_path(&candidate, request);
    }

    if let Some(base_url) = config
        .base_url
        .as_ref()
        .map(|value| normalize_config_path(value))
    {
        let resolution = resolve_internal_path(&base_url.join(request), request)?;

        if resolution.kind != ImportResolutionKind::MissingInternal {
            return Ok(resolution);
        }

        if is_builtin_module(request) {
            return Ok(external_import(request, None));
        }

        if !is_external_package(request, importer_path.as_path(), config) {
            return Ok(unresolved_import(request));
        }
    }

    if is_builtin_module(request) {
        return Ok(external_import(request, None));
    }

    Ok(external_import(request, None))
}

fn resolve_aliased_import(request: &str, alias: &PathAlias) -> PathBuf {
    let Some(capture) = match_alias_capture(&alias.alias, request) else {
        return alias.target.clone();
    };

    if let Some(pattern) = &alias.target_pattern {
        let substituted = pattern.replacen('*', capture, 1);
        return normalize_path(PathBuf::from(substituted));
    }

    if capture.is_empty() {
        alias.target.clone()
    } else {
        normalize_path(alias.target.join(capture))
    }
}

fn matches_alias(alias: &str, request: &str) -> bool {
    match_alias_capture(alias, request).is_some()
}

fn match_alias_capture<'a>(alias: &str, request: &'a str) -> Option<&'a str> {
    let Some((prefix, suffix)) = alias.split_once('*') else {
        return (request == alias).then_some("");
    };

    let remainder = request.strip_prefix(prefix)?;
    let capture = remainder.strip_suffix(suffix)?;
    Some(capture)
}

fn resolve_internal_path(base_path: &Path, request: &str) -> KratosResult<ImportResolution> {
    let normalized_base_path = normalize_path(base_path.to_path_buf());

    let Some(path) = resolve_file(&normalized_base_path)? else {
        return Ok(unresolved_import(request));
    };

    let kind = if is_source_path(&path) {
        ImportResolutionKind::Source
    } else if path.is_file() {
        ImportResolutionKind::Asset
    } else {
        ImportResolutionKind::External
    };

    Ok(ImportResolution {
        kind,
        source: request.to_string(),
        path: Some(path),
    })
}

fn resolve_file(base_path: &Path) -> KratosResult<Option<PathBuf>> {
    if base_path.is_file() {
        return Ok(Some(base_path.to_path_buf()));
    }

    if base_path.extension().is_none() {
        for extension in SOURCE_EXTENSIONS {
            let candidate = append_extension(base_path, extension);

            if candidate.is_file() {
                return Ok(Some(candidate));
            }
        }
    }

    if base_path.is_dir() {
        for extension in SOURCE_EXTENSIONS {
            let candidate = base_path.join(format!("index{extension}"));

            if candidate.is_file() {
                return Ok(Some(candidate));
            }
        }
    }

    Ok(None)
}

fn append_extension(base_path: &Path, extension: &str) -> PathBuf {
    PathBuf::from(format!("{}{}", base_path.to_string_lossy(), extension))
}

fn normalize_config_path(path: &Path) -> PathBuf {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    };

    normalize_path(absolute)
}

fn external_import(source: &str, path: Option<PathBuf>) -> ImportResolution {
    ImportResolution {
        kind: ImportResolutionKind::External,
        source: source.to_string(),
        path,
    }
}

fn is_external_package(request: &str, importer_path: &Path, config: &ProjectConfig) -> bool {
    if is_declared_external_package(request, config) {
        return true;
    }

    let Some(package_name) = requested_package_name(request) else {
        return false;
    };

    importer_declares_external_package(
        importer_path,
        &normalize_config_path(&config.root),
        &package_name,
    )
}

fn is_declared_external_package(request: &str, config: &ProjectConfig) -> bool {
    let Some(package_name) = requested_package_name(request) else {
        return false;
    };

    config.external_packages.contains(package_name.as_str())
}

fn importer_declares_external_package(
    importer_path: &Path,
    project_root: &Path,
    package_name: &str,
) -> bool {
    let mut current = importer_path
        .parent()
        .unwrap_or(importer_path)
        .to_path_buf();

    loop {
        if !current.starts_with(project_root) {
            return false;
        }

        if current != project_root
            && package_json_declares_dependency(&current.join("package.json"), package_name)
        {
            return true;
        }

        if current == project_root {
            return false;
        }

        if !current.pop() {
            return false;
        }
    }
}

fn package_json_declares_dependency(package_json_path: &Path, package_name: &str) -> bool {
    let Ok(content) = std::fs::read_to_string(package_json_path) else {
        return false;
    };
    let Ok(json) = parse_loose_json(&content) else {
        return false;
    };

    dependency_map_contains(json.get("dependencies"), package_name)
        || dependency_map_contains(json.get("devDependencies"), package_name)
        || dependency_map_contains(json.get("peerDependencies"), package_name)
        || dependency_map_contains(json.get("optionalDependencies"), package_name)
}

fn dependency_map_contains(value: Option<&JsonValue>, package_name: &str) -> bool {
    value
        .and_then(JsonValue::as_object)
        .map(|packages| packages.get(package_name).is_some())
        .unwrap_or(false)
}

fn requested_package_name(request: &str) -> Option<String> {
    if request.is_empty()
        || request.starts_with('.')
        || request.starts_with('/')
        || request.starts_with("node:")
    {
        return None;
    }

    if request.starts_with('@') {
        let mut parts = request.split('/');
        let scope = parts.next()?;
        let package = parts.next()?;

        if package.is_empty() {
            return None;
        }

        return Some(format!("{scope}/{package}"));
    }

    request
        .split('/')
        .next()
        .filter(|segment| !segment.is_empty())
        .map(str::to_string)
}

fn is_builtin_module(request: &str) -> bool {
    matches!(
        requested_package_name(request).as_deref(),
        Some(
            "_http_agent"
                | "_http_client"
                | "_http_common"
                | "_http_incoming"
                | "_http_outgoing"
                | "_http_server"
                | "_stream_duplex"
                | "_stream_passthrough"
                | "_stream_readable"
                | "_stream_transform"
                | "_stream_wrap"
                | "_stream_writable"
                | "_tls_common"
                | "_tls_wrap"
                | "assert"
                | "async_hooks"
                | "buffer"
                | "child_process"
                | "cluster"
                | "console"
                | "constants"
                | "crypto"
                | "dgram"
                | "diagnostics_channel"
                | "dns"
                | "domain"
                | "events"
                | "fs"
                | "http"
                | "http2"
                | "https"
                | "inspector"
                | "module"
                | "net"
                | "os"
                | "path"
                | "perf_hooks"
                | "process"
                | "punycode"
                | "querystring"
                | "readline"
                | "repl"
                | "stream"
                | "string_decoder"
                | "sys"
                | "timers"
                | "tls"
                | "trace_events"
                | "tty"
                | "url"
                | "util"
                | "v8"
                | "vm"
                | "wasi"
                | "worker_threads"
                | "zlib"
        )
    )
}

fn is_source_path(path: &Path) -> bool {
    let Some(extension) = path.extension().and_then(|value| value.to_str()) else {
        return false;
    };

    SOURCE_EXTENSIONS
        .iter()
        .any(|candidate| candidate.trim_start_matches('.') == extension)
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
