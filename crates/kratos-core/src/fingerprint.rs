use std::fs::{File, Metadata};
use std::io::{Error, ErrorKind, Read};
use std::path::Path;

use sha2::{Digest, Sha256};

pub(crate) const CONTENT_FINGERPRINT_ALGORITHM: &str = "sha256";

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FileSnapshot {
    pub fingerprint: String,
    pub identity: String,
    pub parent_identity: String,
}

pub(crate) fn read_source_and_snapshot(
    path: &Path,
) -> std::io::Result<(String, Option<FileSnapshot>)> {
    let parent = path.parent();
    let before = std::fs::symlink_metadata(path).ok();
    let before_parent = parent.and_then(|value| std::fs::metadata(value).ok());
    let mut file = File::open(path)?;
    let opened_metadata = file.metadata()?;
    let mut source = String::new();
    file.read_to_string(&mut source)?;
    let after = std::fs::symlink_metadata(path).ok();
    let after_parent = parent.and_then(|value| std::fs::metadata(value).ok());

    let opened_identity = regular_file_identity(&opened_metadata);
    let stable_path = before
        .as_ref()
        .zip(after.as_ref())
        .and_then(|(before, after)| {
            let before_identity = regular_file_identity(before)?;
            let after_identity = regular_file_identity(after)?;
            (before_identity == opened_identity.as_deref()? && after_identity == before_identity)
                .then_some(before_identity)
        });
    let stable_parent = before_parent
        .as_ref()
        .zip(after_parent.as_ref())
        .and_then(|(before, after)| {
            let before_identity = directory_identity(before)?;
            let after_identity = directory_identity(after)?;
            (before_identity == after_identity).then_some(before_identity)
        });

    let snapshot = stable_path
        .zip(stable_parent)
        .map(|(identity, parent_identity)| FileSnapshot {
            fingerprint: fingerprint_bytes(source.as_bytes()),
            identity,
            parent_identity,
        });

    Ok((source, snapshot))
}

pub(crate) fn inspect_regular_file(path: &Path) -> std::io::Result<FileSnapshot> {
    let Some(parent) = path.parent() else {
        return Err(Error::new(
            ErrorKind::Unsupported,
            "content fingerprints require a parent directory",
        ));
    };
    let path_metadata = std::fs::symlink_metadata(path)?;
    if !path_metadata.file_type().is_file() {
        return Err(Error::new(
            ErrorKind::Unsupported,
            "content fingerprints require a regular file",
        ));
    }
    let parent_metadata = std::fs::metadata(parent)?;
    let Some(parent_identity) = directory_identity(&parent_metadata) else {
        return Err(Error::new(
            ErrorKind::Unsupported,
            "stable parent directory identity is unavailable",
        ));
    };

    let mut file = File::open(path)?;
    let opened_metadata = file.metadata()?;
    let Some(identity) = regular_file_identity(&opened_metadata) else {
        return Err(Error::new(
            ErrorKind::Unsupported,
            "stable file identity is unavailable",
        ));
    };
    if regular_file_identity(&path_metadata).as_deref() != Some(identity.as_str()) {
        return Err(Error::other("file identity changed while opening"));
    }

    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    let final_path_metadata = std::fs::symlink_metadata(path)?;
    let final_parent_metadata = std::fs::metadata(parent)?;
    if regular_file_identity(&final_path_metadata).as_deref() != Some(identity.as_str()) {
        return Err(Error::other("file identity changed while fingerprinting"));
    }
    if directory_identity(&final_parent_metadata).as_deref() != Some(parent_identity.as_str()) {
        return Err(Error::other(
            "parent directory identity changed while fingerprinting",
        ));
    }

    Ok(FileSnapshot {
        fingerprint: format!("{:x}", hasher.finalize()),
        identity,
        parent_identity,
    })
}

pub(crate) fn current_parent_identity(path: &Path) -> Option<String> {
    directory_identity(&std::fs::metadata(path).ok()?)
}

fn regular_file_identity(metadata: &Metadata) -> Option<String> {
    if !metadata.file_type().is_file() {
        return None;
    }

    platform_file_identity(metadata)
}

fn directory_identity(metadata: &Metadata) -> Option<String> {
    if !metadata.file_type().is_dir() {
        return None;
    }

    platform_object_identity(metadata)
}

fn fingerprint_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(unix)]
fn platform_file_identity(metadata: &Metadata) -> Option<String> {
    use std::os::unix::fs::MetadataExt;

    Some(format!(
        "unix:{}:{}:{}:{}:{}",
        metadata.dev(),
        metadata.ino(),
        metadata.mtime(),
        metadata.mtime_nsec(),
        metadata.len()
    ))
}

#[cfg(unix)]
fn platform_object_identity(metadata: &Metadata) -> Option<String> {
    use std::os::unix::fs::MetadataExt;

    Some(format!("unix:{}:{}", metadata.dev(), metadata.ino()))
}

#[cfg(windows)]
fn platform_file_identity(_metadata: &Metadata) -> Option<String> {
    // Stable Rust does not expose a Windows file identity suitable for the
    // destructive clean contract. Returning `None` keeps report generation
    // useful while making clean/apply fail closed on this platform.
    None
}

#[cfg(windows)]
fn platform_object_identity(_metadata: &Metadata) -> Option<String> {
    None
}

#[cfg(not(any(unix, windows)))]
fn platform_file_identity(_metadata: &Metadata) -> Option<String> {
    None
}

#[cfg(not(any(unix, windows)))]
fn platform_object_identity(_metadata: &Metadata) -> Option<String> {
    None
}
