use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use kratos_core::clean_preview::{
    build_clean_preview, BINARY_PREVIEW_MARKER, MISSING_PREVIEW_MARKER, UNREADABLE_PREVIEW_MARKER,
};
use kratos_core::model::{DeletionCandidateFinding, ReportV2};

#[test]
fn build_clean_preview_orders_items_by_confidence_then_relative_path() {
    let temp_root = temp_dir("clean-preview-order");
    let report = report_with_candidates(
        &temp_root,
        &[
            (
                "src/beta.ts",
                0.80,
                Some("export const beta = true;\n".as_bytes()),
            ),
            (
                "src/alpha.ts",
                0.80,
                Some("export const alpha = true;\n".as_bytes()),
            ),
            (
                "src/high.ts",
                0.95,
                Some("export const high = true;\n".as_bytes()),
            ),
        ],
    );

    let preview = build_clean_preview(&report, 0.5).expect("preview should build");

    assert_eq!(
        preview
            .items
            .iter()
            .map(|item| item.relative_path.as_str())
            .collect::<Vec<_>>(),
        vec!["src/high.ts", "src/alpha.ts", "src/beta.ts"]
    );
    assert_eq!(
        preview.deletion_target_paths,
        vec![
            temp_root.join("src/high.ts"),
            temp_root.join("src/alpha.ts"),
            temp_root.join("src/beta.ts"),
        ]
    );
}

#[test]
fn build_clean_preview_excludes_below_threshold_targets_and_keeps_counts() {
    let temp_root = temp_dir("clean-preview-threshold");
    let report = report_with_candidates(
        &temp_root,
        &[
            (
                "src/keep.ts",
                0.95,
                Some("export const keep = true;\n".as_bytes()),
            ),
            (
                "src/skip.ts",
                0.20,
                Some("export const skip = true;\n".as_bytes()),
            ),
        ],
    );

    let preview = build_clean_preview(&report, 0.9).expect("preview should build");

    assert_eq!(preview.items.len(), 1);
    assert_eq!(preview.items[0].relative_path, "src/keep.ts");
    assert_eq!(preview.threshold_skipped_targets.len(), 1);
    assert_eq!(
        preview.threshold_skipped_targets[0].file,
        temp_root.join("src/skip.ts")
    );
}

#[test]
fn build_clean_preview_limits_excerpt_and_marks_missing_files() {
    let temp_root = temp_dir("clean-preview-excerpt");
    let long_text = (1..=25)
        .map(|index| format!("line {index}"))
        .collect::<Vec<_>>()
        .join("\n");
    let report = report_with_candidates(
        &temp_root,
        &[
            ("src/long.ts", 0.90, Some(long_text.as_bytes())),
            ("src/missing.ts", 0.85, None),
        ],
    );

    let preview = build_clean_preview(&report, 0.8).expect("preview should build");
    let long_item = preview
        .items
        .iter()
        .find(|item| item.relative_path == "src/long.ts")
        .expect("long item should exist");
    let missing_item = preview
        .items
        .iter()
        .find(|item| item.relative_path == "src/missing.ts")
        .expect("missing item should exist");

    assert!(long_item.exists);
    assert!(long_item.preview_excerpt.contains("line 1"));
    assert!(long_item.preview_excerpt.contains("line 20"));
    assert!(!long_item.preview_excerpt.contains("line 21"));

    assert!(!missing_item.exists);
    assert_eq!(missing_item.preview_excerpt, MISSING_PREVIEW_MARKER);
}

#[test]
fn build_clean_preview_ignores_binary_tail_after_excerpt_limit() {
    let temp_root = temp_dir("clean-preview-tail-limit");
    let mut bytes = (1..=20)
        .map(|index| format!("line {index}\n"))
        .collect::<String>()
        .into_bytes();
    bytes.extend_from_slice(&[0, 159, 146, 150]);

    let report = report_with_candidates(&temp_root, &[("src/tail.ts", 0.9, Some(&bytes))]);

    let preview = build_clean_preview(&report, 0.5).expect("preview should build");

    assert!(preview.items[0].exists);
    assert_eq!(
        preview.items[0].preview_excerpt,
        (1..=20)
            .map(|index| format!("line {index}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn build_clean_preview_truncates_large_single_line_files() {
    let temp_root = temp_dir("clean-preview-long-line");
    let long_line = "a".repeat(64 * 1024);
    let report = report_with_candidates(
        &temp_root,
        &[("src/long-line.ts", 0.9, Some(long_line.as_bytes()))],
    );

    let preview = build_clean_preview(&report, 0.5).expect("preview should build");

    assert!(preview.items[0].exists);
    assert!(preview.items[0].preview_excerpt.starts_with('a'));
    assert!(preview.items[0].preview_excerpt.len() < long_line.len());
    assert!(preview.items[0].preview_excerpt.len() <= 16 * 1024);
}

#[test]
fn build_clean_preview_truncates_multibyte_text_without_marking_binary() {
    let temp_root = temp_dir("clean-preview-multibyte-line");
    let long_line = "가".repeat(6_000);
    let report = report_with_candidates(
        &temp_root,
        &[("src/multibyte.txt", 0.9, Some(long_line.as_bytes()))],
    );

    let preview = build_clean_preview(&report, 0.5).expect("preview should build");

    assert!(preview.items[0].exists);
    assert_ne!(preview.items[0].preview_excerpt, BINARY_PREVIEW_MARKER);
    assert!(preview.items[0].preview_excerpt.starts_with('가'));
    assert!(preview.items[0].preview_excerpt.len() < long_line.len());
    assert!(preview.items[0].preview_excerpt.chars().all(|ch| ch == '가'));
}

#[test]
fn build_clean_preview_uses_binary_and_unreadable_markers() {
    let temp_root = temp_dir("clean-preview-binary");
    let report = report_with_candidates(
        &temp_root,
        &[("src/binary.bin", 0.9, Some(&[0, 159, 146, 150]))],
    );

    let preview = build_clean_preview(&report, 0.5).expect("preview should build");
    assert_eq!(preview.items[0].preview_excerpt, BINARY_PREVIEW_MARKER);

    #[cfg(unix)]
    {
        let unreadable_dir = temp_root.join("src/private");
        let unreadable_path = unreadable_dir.join("blocked.ts");
        std::fs::create_dir_all(&unreadable_dir).expect("private dir should exist");
        std::fs::write(&unreadable_path, "export const blocked = true;\n")
            .expect("unreadable file should write");
        let permissions = std::os::unix::fs::PermissionsExt::from_mode(0o000);
        std::fs::set_permissions(&unreadable_dir, permissions).expect("permissions should update");

        let unreadable_report =
            report_with_candidates(&temp_root, &[("src/private/blocked.ts", 0.91, None)]);
        let unreadable_preview =
            build_clean_preview(&unreadable_report, 0.5).expect("preview should build");

        assert_eq!(
            unreadable_preview.items[0].preview_excerpt,
            UNREADABLE_PREVIEW_MARKER
        );

        let restore = std::os::unix::fs::PermissionsExt::from_mode(0o755);
        std::fs::set_permissions(&unreadable_dir, restore).expect("permissions should restore");
    }
}

#[cfg(unix)]
#[test]
fn build_clean_preview_keeps_unknown_existence_false_for_unreadable_paths() {
    let temp_root = temp_dir("clean-preview-unreadable-missing");
    let hidden_dir = temp_root.join("src/hidden");

    std::fs::create_dir_all(&hidden_dir).expect("hidden dir should exist");
    let permissions = std::os::unix::fs::PermissionsExt::from_mode(0o000);
    std::fs::set_permissions(&hidden_dir, permissions).expect("permissions should update");

    let report = report_with_candidates(&temp_root, &[("src/hidden/missing.ts", 0.91, None)]);
    let preview = build_clean_preview(&report, 0.5).expect("preview should build");

    assert_eq!(preview.items[0].preview_excerpt, UNREADABLE_PREVIEW_MARKER);
    assert!(!preview.items[0].exists);

    let restore = std::os::unix::fs::PermissionsExt::from_mode(0o755);
    std::fs::set_permissions(&hidden_dir, restore).expect("permissions should restore");
}

#[test]
fn build_clean_preview_skips_symlink_escaped_targets() {
    let temp_root = temp_dir("clean-preview-symlink-escape");
    let report_root = temp_root.join("app");
    let outside_root = temp_root.join("outside");
    let outside_file = outside_root.join("secret.ts");
    let link_path = report_root.join("link");

    std::fs::create_dir_all(&report_root).expect("report root should exist");
    std::fs::create_dir_all(&outside_root).expect("outside root should exist");
    std::fs::write(&outside_file, "export const secret = true;\n")
        .expect("outside file should write");
    symlink_dir(&outside_root, &link_path);

    let mut report = ReportV2::new(report_root.clone());
    report
        .findings
        .deletion_candidates
        .push(DeletionCandidateFinding {
            file: link_path.join("secret.ts"),
            reason: "escaped candidate".to_string(),
            confidence: 0.95,
            safe: true,
        });

    let preview = build_clean_preview(&report, 0.5).expect("preview should build");

    assert!(preview.items.is_empty());
    assert!(preview.deletion_target_paths.is_empty());
    assert_eq!(preview.unavailable_targets.len(), 1);
    assert_eq!(preview.unavailable_targets[0].file, link_path.join("secret.ts"));
}

#[test]
fn build_clean_preview_does_not_follow_live_symlink_targets() {
    let temp_root = temp_dir("clean-preview-symlink-file");
    let report_root = temp_root.join("app");
    let outside_root = temp_root.join("outside");
    let outside_file = outside_root.join("target.ts");
    let link_path = report_root.join("linked.ts");

    std::fs::create_dir_all(&report_root).expect("report root should exist");
    std::fs::create_dir_all(&outside_root).expect("outside root should exist");
    std::fs::write(&outside_file, "export const secret = true;\n")
        .expect("outside file should write");
    symlink_file(&outside_file, &link_path);

    let mut report = ReportV2::new(report_root.clone());
    report
        .findings
        .deletion_candidates
        .push(DeletionCandidateFinding {
            file: link_path.clone(),
            reason: "linked candidate".to_string(),
            confidence: 0.95,
            safe: true,
        });

    let preview = build_clean_preview(&report, 0.5).expect("preview should build");

    assert_eq!(preview.items.len(), 1);
    assert_eq!(preview.items[0].file, link_path);
    assert!(preview.items[0].exists);
    assert_eq!(preview.items[0].preview_excerpt, UNREADABLE_PREVIEW_MARKER);
}

#[test]
fn build_clean_preview_marks_dangling_symlink_targets_as_unavailable() {
    let temp_root = temp_dir("clean-preview-dangling-symlink");
    let report_root = temp_root.join("app");
    let missing_target = temp_root.join("outside/missing.ts");
    let link_path = report_root.join("dangling.ts");

    std::fs::create_dir_all(&report_root).expect("report root should exist");
    symlink_file(&missing_target, &link_path);

    let mut report = ReportV2::new(report_root.clone());
    report
        .findings
        .deletion_candidates
        .push(DeletionCandidateFinding {
            file: link_path.clone(),
            reason: "dangling linked candidate".to_string(),
            confidence: 0.95,
            safe: true,
        });

    let preview = build_clean_preview(&report, 0.5).expect("preview should build");

    assert_eq!(preview.items.len(), 1);
    assert_eq!(preview.items[0].file, link_path);
    assert!(preview.items[0].exists);
    assert_eq!(preview.items[0].preview_excerpt, UNREADABLE_PREVIEW_MARKER);
}

fn report_with_candidates(root: &Path, candidates: &[(&str, f32, Option<&[u8]>)]) -> ReportV2 {
    let mut report = ReportV2::new(root.to_path_buf());
    std::fs::create_dir_all(root).expect("report root should exist");

    report.findings.deletion_candidates = candidates
        .iter()
        .map(|(relative, confidence, bytes)| {
            let path = root.join(relative);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).expect("candidate parent should exist");
            }
            if let Some(bytes) = bytes {
                std::fs::write(&path, bytes).expect("candidate file should write");
            }

            DeletionCandidateFinding {
                file: path,
                reason: format!("{relative} candidate"),
                confidence: *confidence,
                safe: true,
            }
        })
        .collect();

    report
}

fn temp_dir(label: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be valid")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("kratos-core-{label}-{unique}"));
    std::fs::create_dir_all(&path).expect("temp dir should be created");
    path
}

#[cfg(unix)]
fn symlink_dir(target: &Path, link: &Path) {
    std::os::unix::fs::symlink(target, link).expect("directory symlink should be created");
}

#[cfg(unix)]
fn symlink_file(target: &Path, link: &Path) {
    std::os::unix::fs::symlink(target, link).expect("file symlink should be created");
}

#[cfg(windows)]
fn symlink_dir(target: &Path, link: &Path) {
    std::os::windows::fs::symlink_dir(target, link).expect("directory symlink should be created");
}

#[cfg(windows)]
fn symlink_file(target: &Path, link: &Path) {
    std::os::windows::fs::symlink_file(target, link).expect("file symlink should be created");
}
