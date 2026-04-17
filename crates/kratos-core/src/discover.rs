use std::path::{Path, PathBuf};

use crate::error::KratosResult;
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

    for root in &config.roots {
        let root = normalize_root(root);

        if !root.is_dir() {
            continue;
        }

        visit_directory(&root, &config.ignored_directories, &mut discovered)?;
    }

    Ok(discovered.into_iter().collect())
}

fn visit_directory(
    current: &Path,
    ignored_directories: &[String],
    discovered: &mut std::collections::BTreeSet<PathBuf>,
) -> KratosResult<()> {
    for entry in std::fs::read_dir(current)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let path = entry.path();

        if file_type.is_dir() {
            let name = entry.file_name();
            let name = name.to_string_lossy();

            if ignored_directories.iter().any(|ignored| ignored == &name) {
                continue;
            }

            visit_directory(&path, ignored_directories, discovered)?;
            continue;
        }

        if file_type.is_file() && is_source_path(&path) {
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
