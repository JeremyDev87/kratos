use std::path::{Path, PathBuf};

use kratos_core::analyze::analyze_project;
use kratos_core::model::{EntrypointKind, OrphanKind};

#[test]
fn analyze_demo_app_matches_expected_graph_and_findings() {
    let demo_root = repo_root().join("fixtures/demo-app");
    let report = analyze_project(&demo_root).expect("demo app should analyze");

    assert_eq!(report.version, 2);
    assert!(report.generated_at.is_some());
    assert_eq!(report.root, demo_root);
    assert_eq!(report.summary.files_scanned, 5);
    assert_eq!(report.summary.entrypoints, 1);
    assert_eq!(report.summary.broken_imports, 1);
    assert_eq!(report.summary.orphan_files, 2);
    assert_eq!(report.summary.dead_exports, 3);
    assert_eq!(report.summary.unused_imports, 0);
    assert_eq!(report.summary.route_entrypoints, 1);
    assert_eq!(report.summary.deletion_candidates, 2);

    assert_eq!(report.findings.broken_imports.len(), 1);
    assert_eq!(
        report.findings.broken_imports[0].file,
        demo_root.join("src/lib/broken.ts")
    );
    assert_eq!(report.findings.broken_imports[0].source, "./missing-helper");

    let orphan_kinds = report
        .findings
        .orphan_files
        .iter()
        .map(|item| (item.file.clone(), item.kind.clone(), item.reason.clone()))
        .collect::<Vec<_>>();
    assert_eq!(
        orphan_kinds,
        vec![
            (
                demo_root.join("src/components/DeadWidget.tsx"),
                OrphanKind::Component,
                "Component-like module has no inbound references.".to_string(),
            ),
            (
                demo_root.join("src/lib/broken.ts"),
                OrphanKind::Module,
                "Module has no inbound references and is not treated as an entrypoint.".to_string(),
            ),
        ]
    );

    let dead_exports = report
        .findings
        .dead_exports
        .iter()
        .map(|item| (item.file.clone(), item.export_name.clone()))
        .collect::<Vec<_>>();
    assert_eq!(
        dead_exports,
        vec![
            (
                demo_root.join("src/components/DeadWidget.tsx"),
                "DeadWidget".to_string(),
            ),
            (
                demo_root.join("src/lib/broken.ts"),
                "brokenFeature".to_string(),
            ),
            (demo_root.join("src/lib/math.ts"), "multiply".to_string(),),
        ]
    );

    assert_eq!(report.findings.route_entrypoints.len(), 1);
    assert_eq!(
        report.findings.route_entrypoints[0].file,
        demo_root.join("pages/home.tsx")
    );
    assert_eq!(
        report.findings.route_entrypoints[0].kind,
        EntrypointKind::NextPagesRoute
    );

    let modules = report
        .modules
        .iter()
        .map(|module| {
            (
                module.relative_path.clone(),
                module.entrypoint_kind.clone(),
                module.imported_by.len(),
                module.resolved_imports.len(),
                module.exports.len(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        modules,
        vec![
            (
                "pages/home.tsx".to_string(),
                Some(EntrypointKind::NextPagesRoute),
                0,
                2,
                1,
            ),
            ("src/components/DeadWidget.tsx".to_string(), None, 0, 0, 1,),
            ("src/components/LiveCard.tsx".to_string(), None, 1, 0, 1,),
            ("src/lib/broken.ts".to_string(), None, 0, 0, 1),
            ("src/lib/math.ts".to_string(), None, 1, 0, 2),
        ]
    );
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root should resolve")
}
