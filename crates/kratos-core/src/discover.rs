use std::path::{Path, PathBuf};

use crate::error::KratosResult;
use crate::ignore::IgnoreMatcher;
use crate::model::ProjectConfig;

const SOURCE_EXTENSIONS: &[&str] = &[".js", ".jsx", ".ts", ".tsx", ".mjs", ".cjs", ".mts", ".cts"];

pub fn normalize_root(root: &Path) -> PathBuf {
    if root.is_absolute() {
        root.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(root)
    }
}

pub fn collect_source_files(config: &ProjectConfig) -> KratosResult<Vec<PathBuf>> {
    let mut discovered = std::collections::BTreeSet::new();
    let ignore_matcher = IgnoreMatcher::new(&config.ignored_directories, &config.ignore_patterns);

    for root in &config.roots {
        let root = normalize_root(root);

        if !root.is_dir() {
            continue;
        }

        let relative_root = to_project_relative_path(&root, &config.root);
        let traversal_root = (!relative_root.is_empty()).then_some(relative_root.as_str());
        let should_traverse_root = match traversal_root {
            Some(root_override) => {
                ignore_matcher.should_traverse_dir_from_root(&relative_root, Some(root_override))
            }
            None => ignore_matcher.should_traverse_dir(&relative_root),
        };
        if !relative_root.is_empty() && !should_traverse_root {
            continue;
        }

        visit_directory(
            &root,
            &config.root,
            &ignore_matcher,
            traversal_root,
            &mut discovered,
        )?;
    }

    Ok(discovered.into_iter().collect())
}

fn visit_directory(
    current: &Path,
    project_root: &Path,
    ignore_matcher: &IgnoreMatcher,
    traversal_root: Option<&str>,
    discovered: &mut std::collections::BTreeSet<PathBuf>,
) -> KratosResult<()> {
    for entry in std::fs::read_dir(current)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let path = entry.path();
        let relative_path = to_project_relative_path(&path, project_root);

        if file_type.is_dir() {
            let should_traverse = match traversal_root {
                Some(root_override) => {
                    ignore_matcher.should_traverse_dir_from_root(&relative_path, Some(root_override))
                }
                None => ignore_matcher.should_traverse_dir(&relative_path),
            };
            if !should_traverse {
                continue;
            }

            visit_directory(
                &path,
                project_root,
                ignore_matcher,
                traversal_root,
                discovered,
            )?;
            continue;
        }

        if file_type.is_file()
            && is_source_path(&path)
            && !match traversal_root {
                Some(root_override) => {
                    ignore_matcher.is_ignored_from_root(&relative_path, false, Some(root_override))
                }
                None => ignore_matcher.is_ignored(&relative_path, false),
            }
        {
            discovered.insert(path);
        }
    }

    Ok(())
}

fn is_source_path(path: &Path) -> bool {
    let Some(extension) = path.extension().and_then(|value| value.to_str()) else {
        return false;
    };

    SOURCE_EXTENSIONS
        .iter()
        .any(|candidate| candidate.trim_start_matches('.') == extension)
}

fn to_project_relative_path(path: &Path, project_root: &Path) -> String {
    path.strip_prefix(project_root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
        .trim_matches('/')
        .to_string()
}
