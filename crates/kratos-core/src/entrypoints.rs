use std::path::Path;

use crate::error::KratosResult;
use crate::model::{EntrypointKind, ProjectConfig};

const NEXT_APP_ROUTE_STEMS: &[&str] = &[
    "page",
    "route",
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
    "robots",
    "sitemap",
    "icon",
    "apple-icon",
    "opengraph-image",
    "twitter-image",
    "manifest",
];

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
    let segments = relative_path.split('/').collect::<Vec<_>>();

    has_supported_segment(&segments, "app")
        && segments.len() >= 2
        && matches_file_stem(
            segments.last().copied().unwrap_or_default(),
            NEXT_APP_ROUTE_STEMS,
        )
}

#[cfg(test)]
mod tests {
    use super::detect_entrypoint_kind;
    use crate::model::{EntrypointKind, ProjectConfig};
    use std::path::PathBuf;

    #[test]
    fn next_app_route_supports_root_src_and_nested_package_paths() {
        let root = PathBuf::from("/tmp/kratos-entrypoints");
        let config = ProjectConfig::new(root.clone());

        let root_level = detect_entrypoint_kind(&root.join("app/page.tsx"), &config)
            .expect("entrypoint detection should succeed");
        let src_root_level = detect_entrypoint_kind(&root.join("src/app/page.tsx"), &config)
            .expect("entrypoint detection should succeed");
        let nested = detect_entrypoint_kind(&root.join("app/home/page.tsx"), &config)
            .expect("entrypoint detection should succeed");
        let template = detect_entrypoint_kind(&root.join("app/template.tsx"), &config)
            .expect("entrypoint detection should succeed");
        let global_error = detect_entrypoint_kind(&root.join("src/app/global-error.tsx"), &config)
            .expect("entrypoint detection should succeed");
        let global_not_found =
            detect_entrypoint_kind(&root.join("src/app/global-not-found.tsx"), &config)
                .expect("entrypoint detection should succeed");
        let forbidden = detect_entrypoint_kind(&root.join("src/app/forbidden.tsx"), &config)
            .expect("entrypoint detection should succeed");
        let unauthorized = detect_entrypoint_kind(&root.join("src/app/unauthorized.tsx"), &config)
            .expect("entrypoint detection should succeed");
        let parallel_default =
            detect_entrypoint_kind(&root.join("app/@modal/default.tsx"), &config)
                .expect("entrypoint detection should succeed");
        let robots = detect_entrypoint_kind(&root.join("app/robots.ts"), &config)
            .expect("entrypoint detection should succeed");
        let opengraph = detect_entrypoint_kind(&root.join("app/opengraph-image.tsx"), &config)
            .expect("entrypoint detection should succeed");
        let package_root = detect_entrypoint_kind(&root.join("packages/web/app/page.tsx"), &config)
            .expect("entrypoint detection should succeed");
        let nested_package = detect_entrypoint_kind(
            &root.join("packages/web/src/app/settings/page.tsx"),
            &config,
        )
        .expect("entrypoint detection should succeed");
        let nested_test = detect_entrypoint_kind(&root.join("app/home/page.test.tsx"), &config)
            .expect("entrypoint detection should succeed");
        let nested_spec =
            detect_entrypoint_kind(&root.join("app/home/not-found.spec.tsx"), &config)
                .expect("entrypoint detection should succeed");

        assert_eq!(root_level, Some(EntrypointKind::NextAppRoute));
        assert_eq!(src_root_level, Some(EntrypointKind::NextAppRoute));
        assert_eq!(nested, Some(EntrypointKind::NextAppRoute));
        assert_eq!(template, Some(EntrypointKind::NextAppRoute));
        assert_eq!(global_error, Some(EntrypointKind::NextAppRoute));
        assert_eq!(global_not_found, Some(EntrypointKind::NextAppRoute));
        assert_eq!(forbidden, Some(EntrypointKind::NextAppRoute));
        assert_eq!(unauthorized, Some(EntrypointKind::NextAppRoute));
        assert_eq!(parallel_default, Some(EntrypointKind::NextAppRoute));
        assert_eq!(robots, Some(EntrypointKind::NextAppRoute));
        assert_eq!(opengraph, Some(EntrypointKind::NextAppRoute));
        assert_eq!(package_root, Some(EntrypointKind::NextAppRoute));
        assert_eq!(nested_package, Some(EntrypointKind::NextAppRoute));
        assert_eq!(nested_test, None);
        assert_eq!(nested_spec, None);
    }

    #[test]
    fn next_pages_route_supports_optional_src_prefix_and_nested_packages() {
        let root = PathBuf::from("/tmp/kratos-entrypoints");
        let config = ProjectConfig::new(root.clone());

        let root_pages = detect_entrypoint_kind(&root.join("pages/index.tsx"), &config)
            .expect("entrypoint detection should succeed");
        let src_pages = detect_entrypoint_kind(&root.join("src/pages/index.tsx"), &config)
            .expect("entrypoint detection should succeed");
        let package_root =
            detect_entrypoint_kind(&root.join("packages/web/pages/index.tsx"), &config)
                .expect("entrypoint detection should succeed");
        let nested_package =
            detect_entrypoint_kind(&root.join("packages/web/src/pages/index.tsx"), &config)
                .expect("entrypoint detection should succeed");
        let dotted_page = detect_entrypoint_kind(&root.join("pages/home.page.tsx"), &config)
            .expect("entrypoint detection should succeed");
        let dotted_api = detect_entrypoint_kind(&root.join("src/pages/api.v1.ts"), &config)
            .expect("entrypoint detection should succeed");

        assert_eq!(root_pages, Some(EntrypointKind::NextPagesRoute));
        assert_eq!(src_pages, Some(EntrypointKind::NextPagesRoute));
        assert_eq!(package_root, Some(EntrypointKind::NextPagesRoute));
        assert_eq!(nested_package, Some(EntrypointKind::NextPagesRoute));
        assert_eq!(dotted_page, Some(EntrypointKind::NextPagesRoute));
        assert_eq!(dotted_api, Some(EntrypointKind::NextPagesRoute));
    }

    #[test]
    fn app_entry_supports_nested_package_src_paths() {
        let root = PathBuf::from("/tmp/kratos-entrypoints");
        let config = ProjectConfig::new(root.clone());

        let root_entry = detect_entrypoint_kind(&root.join("src/main.ts"), &config)
            .expect("entrypoint detection should succeed");
        let nested_entry = detect_entrypoint_kind(&root.join("packages/cli/src/main.ts"), &config)
            .expect("entrypoint detection should succeed");
        let nested_non_src = detect_entrypoint_kind(&root.join("packages/cli/main.ts"), &config)
            .expect("entrypoint detection should succeed");

        assert_eq!(root_entry, Some(EntrypointKind::AppEntry));
        assert_eq!(nested_entry, Some(EntrypointKind::AppEntry));
        assert_eq!(nested_non_src, None);
    }

    #[test]
    fn tooling_entry_requires_single_extension_suffix() {
        let root = PathBuf::from("/tmp/kratos-entrypoints");
        let config = ProjectConfig::new(root.clone());

        let real_config = detect_entrypoint_kind(&root.join("next.config.js"), &config)
            .expect("entrypoint detection should succeed");
        let eslint_config = detect_entrypoint_kind(&root.join("eslint.config.mjs"), &config)
            .expect("entrypoint detection should succeed");
        let test_config = detect_entrypoint_kind(&root.join("next.config.test.ts"), &config)
            .expect("entrypoint detection should succeed");
        let backup_config = detect_entrypoint_kind(&root.join("vite.config.backup.js"), &config)
            .expect("entrypoint detection should succeed");

        assert_eq!(real_config, Some(EntrypointKind::ToolingEntry));
        assert_eq!(eslint_config, Some(EntrypointKind::ToolingEntry));
        assert_eq!(test_config, None);
        assert_eq!(backup_config, None);
    }
}

fn is_next_pages_route(relative_path: &str) -> bool {
    let segments = relative_path.split('/').collect::<Vec<_>>();
    has_supported_segment(&segments, "pages")
        && segments.len() >= 2
        && has_extension(segments.last().copied().unwrap_or_default())
}

fn is_app_entry(relative_path: &str) -> bool {
    let segments = relative_path.split('/').collect::<Vec<_>>();
    let Some(file_name) = segments.last().copied() else {
        return false;
    };

    if !matches_file_stem(file_name, &["main", "index", "bootstrap", "cli"]) {
        return false;
    }

    segments.len() == 1 || segments[segments.len() - 2] == "src"
}

fn is_tooling_entry(relative_path: &str) -> bool {
    let file_name = relative_path.rsplit('/').next().unwrap_or(relative_path);
    [
        "next",
        "eslint",
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
    .any(|tool| {
        file_name
            .strip_prefix(&format!("{tool}.config."))
            .is_some_and(|extension| !extension.is_empty() && !extension.contains('.'))
    })
}

fn has_supported_segment(segments: &[&str], name: &str) -> bool {
    segments.iter().enumerate().any(|(index, segment)| {
        *segment == name
            && (index == 0
                || segments[index - 1] == "src"
                || is_package_root_segment(segments, index))
    })
}

fn is_package_root_segment(segments: &[&str], index: usize) -> bool {
    index >= 2 && matches!(segments[index - 2], "packages" | "apps")
}

fn matches_file_stem(file_name: &str, stems: &[&str]) -> bool {
    let Some((stem, _extension)) = split_single_extension(file_name) else {
        return false;
    };

    stems.iter().any(|candidate| *candidate == stem)
}

fn has_extension(file_name: &str) -> bool {
    file_name
        .rsplit_once('.')
        .is_some_and(|(stem, extension)| !stem.is_empty() && !extension.is_empty())
}

fn split_single_extension(file_name: &str) -> Option<(&str, &str)> {
    let (stem, extension) = file_name.rsplit_once('.')?;

    if extension.is_empty() || extension.contains('.') || stem.contains('.') {
        return None;
    }

    Some((stem, extension))
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
