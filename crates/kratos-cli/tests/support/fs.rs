use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root should exist")
        .to_path_buf()
}

pub fn copy_demo_app(label: &str) -> PathBuf {
    let destination = temp_dir(label).join("demo-app");
    copy_directory(&repo_root().join("fixtures/demo-app"), &destination);
    destination
}

pub fn copy_directory(source: &Path, destination: &Path) {
    std::fs::create_dir_all(destination).expect("destination directory should exist");

    for entry in std::fs::read_dir(source).expect("source directory should be readable") {
        let entry = entry.expect("directory entry should load");
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        let file_type = entry.file_type().expect("file type should load");

        if file_type.is_dir() {
            copy_directory(&source_path, &destination_path);
        } else if file_type.is_file() {
            std::fs::copy(&source_path, &destination_path).expect("file should copy");
        }
    }
}

pub fn temp_dir(label: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be valid")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("kratos-cli-{label}-{unique}"));
    std::fs::create_dir_all(&path).expect("temp dir should be created");
    path
}
