use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

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

#[test]
fn analyze_project_detects_root_and_src_next_routes() {
    let project = TestProject::new("next-routes");
    project.write(
        "app/page.tsx",
        "export default function RootPage() { return null; }\n",
    );
    project.write(
        "app/template.tsx",
        "export default function RootTemplate({ children }: { children: unknown }) { return children; }\n",
    );
    project.write(
        "app/@modal/default.tsx",
        "export default function ModalDefault() { return null; }\n",
    );
    project.write(
        "src/app/dashboard/page.tsx",
        "export default function DashboardPage() { return null; }\n",
    );
    project.write(
        "src/app/global-error.tsx",
        "export default function GlobalError() { return null; }\n",
    );
    project.write(
        "src/pages/index.tsx",
        "export default function IndexPage() { return null; }\n",
    );

    let report = analyze_project(project.root()).expect("project should analyze");
    let mut route_files = report
        .findings
        .route_entrypoints
        .iter()
        .map(|item| {
            item.file
                .strip_prefix(project.root())
                .unwrap_or(&item.file)
                .to_path_buf()
        })
        .collect::<Vec<_>>();
    route_files.sort();

    assert_eq!(report.summary.entrypoints, 6);
    assert_eq!(report.summary.route_entrypoints, 6);
    assert_eq!(
        route_files,
        vec![
            PathBuf::from("app/@modal/default.tsx"),
            PathBuf::from("app/page.tsx"),
            PathBuf::from("app/template.tsx"),
            PathBuf::from("src/app/dashboard/page.tsx"),
            PathBuf::from("src/app/global-error.tsx"),
            PathBuf::from("src/pages/index.tsx"),
        ]
    );
}

#[test]
fn analyze_project_detects_nested_package_entrypoints() {
    let project = TestProject::new("nested-package-entrypoints");
    project.write(
        "packages/web/app/page.tsx",
        "export default function RootPackagePage() { return null; }\n",
    );
    project.write(
        "packages/web/pages/index.tsx",
        "export default function RootLegacyPage() { return null; }\n",
    );
    project.write(
        "packages/web/src/app/page.tsx",
        "export default function WebPage() { return null; }\n",
    );
    project.write(
        "packages/web/src/pages/index.tsx",
        "export default function LegacyPage() { return null; }\n",
    );
    project.write("packages/cli/src/main.ts", "export const main = true;\n");

    let report = analyze_project(project.root()).expect("project should analyze");
    let mut route_files = report
        .findings
        .route_entrypoints
        .iter()
        .map(|item| {
            item.file
                .strip_prefix(project.root())
                .unwrap_or(&item.file)
                .to_path_buf()
        })
        .collect::<Vec<_>>();
    route_files.sort();

    assert_eq!(report.summary.entrypoints, 5);
    assert_eq!(report.summary.route_entrypoints, 4);
    assert_eq!(
        route_files,
        vec![
            PathBuf::from("packages/web/app/page.tsx"),
            PathBuf::from("packages/web/pages/index.tsx"),
            PathBuf::from("packages/web/src/app/page.tsx"),
            PathBuf::from("packages/web/src/pages/index.tsx"),
        ]
    );
}

#[test]
fn analyze_project_reports_dead_helpers_in_route_entrypoints() {
    let project = TestProject::new("route-dead-helpers");
    project.write(
        "app/page.tsx",
        r#"export default function Page() { return null; }
export const generateMetadata = () => ({ title: "ok" });
export const helper = 1;
"#,
    );

    let report = analyze_project(project.root()).expect("project should analyze");

    assert_eq!(report.summary.entrypoints, 1);
    assert_eq!(report.summary.route_entrypoints, 1);
    assert_eq!(report.summary.dead_exports, 1);
    assert_eq!(report.findings.dead_exports.len(), 1);
    assert_eq!(
        report.findings.dead_exports[0].file,
        project.root().join("app/page.tsx")
    );
    assert_eq!(report.findings.dead_exports[0].export_name, "helper");
}

#[test]
fn analyze_project_skips_next_metadata_helpers_for_metadata_files() {
    let project = TestProject::new("metadata-helpers");
    project.write(
        "app/opengraph-image.tsx",
        r#"export function generateImageMetadata() {
  return [{ id: "1", contentType: "image/png", size: { width: 1200, height: 630 } }];
}
export default function Image() { return null; }
export const helper = 1;
"#,
    );

    let report = analyze_project(project.root()).expect("project should analyze");

    assert_eq!(report.summary.entrypoints, 1);
    assert_eq!(report.summary.route_entrypoints, 1);
    assert_eq!(report.summary.dead_exports, 1);
    assert_eq!(report.findings.dead_exports.len(), 1);
    assert_eq!(
        report.findings.dead_exports[0].file,
        project.root().join("app/opengraph-image.tsx")
    );
    assert_eq!(report.findings.dead_exports[0].export_name, "helper");
}

#[test]
fn analyze_project_reports_missing_base_url_imports_as_broken() {
    let project = TestProject::new("missing-baseurl");
    project.write(
        "tsconfig.json",
        r#"{
  "compilerOptions": {
    "baseUrl": "src"
  }
}
"#,
    );
    project.write(
        "src/main.ts",
        "import { missing } from \"shared/missing\";\nexport const main = missing;\n",
    );
    project.write("src/shared/index.ts", "export const shared = true;\n");

    let report = analyze_project(project.root()).expect("project should analyze");

    assert_eq!(report.summary.broken_imports, 1);
    assert_eq!(report.findings.broken_imports.len(), 1);
    assert_eq!(report.findings.broken_imports[0].source, "shared/missing");
    assert_eq!(
        report.findings.broken_imports[0].file,
        project.root().join("src/main.ts")
    );
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root should resolve")
}

struct TestProject {
    root: PathBuf,
}

impl TestProject {
    fn new(prefix: &str) -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("kratos-analyze-{prefix}-{unique}"));
        std::fs::create_dir_all(&root).expect("temp project should be created");
        Self { root }
    }

    fn root(&self) -> &Path {
        &self.root
    }

    fn write(&self, relative_path: &str, contents: &str) {
        let file_path = self.root.join(relative_path);
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent).expect("parent directory should be created");
        }
        std::fs::write(file_path, contents).expect("file should be written");
    }
}

impl Drop for TestProject {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
