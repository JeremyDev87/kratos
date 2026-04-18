use std::path::Path;

use crate::error::KratosResult;
use crate::model::{EntrypointKind, ProjectConfig};

pub fn detect_entrypoint_kind(
    file_path: &Path,
    config: &ProjectConfig,
) -> KratosResult<Option<EntrypointKind>> {
    if config
        .explicit_entries
        .iter()
        .any(|entry| entry == file_path)
    {
        return Ok(Some(EntrypointKind::UserEntry));
    }

    if config
        .package_entries
        .iter()
        .any(|entry| entry == file_path)
    {
        return Ok(Some(EntrypointKind::PackageEntry));
    }

    let relative_path = to_project_path(file_path, &config.root);

    if is_next_app_route(&relative_path) {
        return Ok(Some(EntrypointKind::NextAppRoute));
    }

    if is_next_pages_route(&relative_path) {
        return Ok(Some(EntrypointKind::NextPagesRoute));
    }

    if is_app_entry(&relative_path) {
        return Ok(Some(EntrypointKind::AppEntry));
    }

    if is_tooling_entry(&relative_path) {
        return Ok(Some(EntrypointKind::ToolingEntry));
    }

    if is_framework_entry(&relative_path) {
        return Ok(Some(EntrypointKind::FrameworkEntry));
    }

    Ok(None)
}

fn to_project_path(file_path: &Path, root: &Path) -> String {
    file_path
        .strip_prefix(root)
        .unwrap_or(file_path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn is_next_app_route(relative_path: &str) -> bool {
    if !relative_path.starts_with("app/") {
        return false;
    }

    let nested = &relative_path["app/".len()..];
    if !nested.contains('/') {
        return false;
    }

    let file_name = relative_path
        .rsplit('/')
        .next()
        .unwrap_or(relative_path)
        .split('.')
        .next()
        .unwrap_or_default();

    matches!(
        file_name,
        "page" | "route" | "layout" | "loading" | "error" | "not-found"
    )
}

#[cfg(test)]
mod tests {
    use super::detect_entrypoint_kind;
    use crate::model::{EntrypointKind, ProjectConfig};
    use std::path::PathBuf;

    #[test]
    fn next_app_route_requires_nested_segment_for_js_parity() {
        let root = PathBuf::from("/tmp/kratos-entrypoints");
        let config = ProjectConfig::new(root.clone());

        let root_level = detect_entrypoint_kind(&root.join("app/page.tsx"), &config)
            .expect("entrypoint detection should succeed");
        let nested = detect_entrypoint_kind(&root.join("app/home/page.tsx"), &config)
            .expect("entrypoint detection should succeed");

        assert_eq!(root_level, None);
        assert_eq!(nested, Some(EntrypointKind::NextAppRoute));
    }
}

fn is_next_pages_route(relative_path: &str) -> bool {
    relative_path.starts_with("pages/") && relative_path.rsplit('/').next().is_some()
}

fn is_app_entry(relative_path: &str) -> bool {
    matches!(
        relative_path,
        "main.js"
            | "main.jsx"
            | "main.ts"
            | "main.tsx"
            | "main.mjs"
            | "main.cjs"
            | "main.mts"
            | "main.cts"
            | "index.js"
            | "index.jsx"
            | "index.ts"
            | "index.tsx"
            | "index.mjs"
            | "index.cjs"
            | "index.mts"
            | "index.cts"
            | "bootstrap.js"
            | "bootstrap.jsx"
            | "bootstrap.ts"
            | "bootstrap.tsx"
            | "bootstrap.mjs"
            | "bootstrap.cjs"
            | "bootstrap.mts"
            | "bootstrap.cts"
            | "cli.js"
            | "cli.jsx"
            | "cli.ts"
            | "cli.tsx"
            | "cli.mjs"
            | "cli.cjs"
            | "cli.mts"
            | "cli.cts"
            | "src/main.js"
            | "src/main.jsx"
            | "src/main.ts"
            | "src/main.tsx"
            | "src/main.mjs"
            | "src/main.cjs"
            | "src/main.mts"
            | "src/main.cts"
            | "src/index.js"
            | "src/index.jsx"
            | "src/index.ts"
            | "src/index.tsx"
            | "src/index.mjs"
            | "src/index.cjs"
            | "src/index.mts"
            | "src/index.cts"
            | "src/bootstrap.js"
            | "src/bootstrap.jsx"
            | "src/bootstrap.ts"
            | "src/bootstrap.tsx"
            | "src/bootstrap.mjs"
            | "src/bootstrap.cjs"
            | "src/bootstrap.mts"
            | "src/bootstrap.cts"
            | "src/cli.js"
            | "src/cli.jsx"
            | "src/cli.ts"
            | "src/cli.tsx"
            | "src/cli.mjs"
            | "src/cli.cjs"
            | "src/cli.mts"
            | "src/cli.cts"
    )
}

fn is_tooling_entry(relative_path: &str) -> bool {
    let file_name = relative_path.rsplit('/').next().unwrap_or(relative_path);
    [
        "next",
        "vite",
        "webpack",
        "rollup",
        "vitest",
        "jest",
        "postcss",
        "tailwind",
        "babel",
        "playwright",
        "cypress",
    ]
    .iter()
    .any(|tool| file_name.starts_with(&format!("{tool}.config.")))
}

fn is_framework_entry(relative_path: &str) -> bool {
    matches!(
        relative_path,
        "middleware.js"
            | "middleware.jsx"
            | "middleware.ts"
            | "middleware.tsx"
            | "middleware.mjs"
            | "middleware.cjs"
            | "middleware.mts"
            | "middleware.cts"
            | "instrumentation.js"
            | "instrumentation.jsx"
            | "instrumentation.ts"
            | "instrumentation.tsx"
            | "instrumentation.mjs"
            | "instrumentation.cjs"
            | "instrumentation.mts"
            | "instrumentation.cts"
    )
}
