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
fn analyze_project_excludes_pure_reexport_barrels_from_deletion_candidates() {
    let project = TestProject::new("pure-reexport-barrel");
    project.write(
        "src/widgets/ui/index.ts",
        r#"
// public widget barrel
export { default as CycleTimeChart } from "./CycleTimeChart";
export * from "./chartTypes";
"#,
    );
    project.write(
        "src/widgets/ui/CycleTimeChart.tsx",
        "export default function CycleTimeChart() { return null; }\n",
    );
    project.write(
        "src/widgets/ui/chartTypes.ts",
        "export type ChartId = string;\n",
    );
    project.write(
        "src/widgets/ui/compact.ts",
        r#"export{ChartMeta}from"./chartMeta";"#,
    );
    project.write(
        "src/widgets/ui/chartMeta.ts",
        "export const ChartMeta = true;\n",
    );
    project.write(
        "src/widgets/ui/clientIndex.ts",
        r#""use client";
export { default as ClientWidget } from "./ClientWidget";
"#,
    );
    project.write(
        "src/widgets/ui/ClientWidget.tsx",
        "export default function ClientWidget() { return null; }\n",
    );
    project.write(
        "src/widgets/ui/impure.ts",
        r#"export { default as ImpureWidget } from "./ImpureWidget";
registerIcons();
"#,
    );
    project.write(
        "src/widgets/ui/ImpureWidget.tsx",
        "export default function ImpureWidget() { return null; }\n",
    );
    project.write("src/widgets/logic.ts", "export const unusedLogic = true;\n");

    let report = analyze_project(project.root()).expect("project should analyze");
    let barrel = project.root().join("src/widgets/ui/index.ts");
    let compact_barrel = project.root().join("src/widgets/ui/compact.ts");
    let client_barrel = project.root().join("src/widgets/ui/clientIndex.ts");
    let impure_barrel = project.root().join("src/widgets/ui/impure.ts");
    let logic = project.root().join("src/widgets/logic.ts");

    for pure_barrel in [barrel, compact_barrel, client_barrel] {
        assert!(!report
            .findings
            .orphan_files
            .iter()
            .any(|finding| finding.file == pure_barrel));
        assert!(!report
            .findings
            .deletion_candidates
            .iter()
            .any(|finding| finding.file == pure_barrel));
        assert!(!report
            .findings
            .dead_exports
            .iter()
            .any(|finding| finding.file == pure_barrel));
    }
    assert!(report
        .findings
        .deletion_candidates
        .iter()
        .any(|finding| finding.file == impure_barrel));
    assert!(report
        .findings
        .deletion_candidates
        .iter()
        .any(|finding| finding.file == logic));
}

#[test]
fn analyze_project_skips_next_app_framework_exports_for_route_conventions() {
    let project = TestProject::new("next-app-framework-exports");
    project.write(
        "src/app/page.tsx",
        r#"export const metadata = { title: "Home" };
export const viewport = { themeColor: "black" };
export const dynamic = "force-static";
export async function generateStaticParams() { return []; }
export default function Page() { return null; }
export const helperPage = 1;
"#,
    );
    project.write(
        "src/app/layout.tsx",
        r#"export function generateMetadata() { return { title: "Layout" }; }
export function generateViewport() { return { themeColor: "white" }; }
export const revalidate = 60;
export default function Layout({ children }: { children: unknown }) { return children; }
export const helperLayout = 1;
"#,
    );
    project.write(
        "src/app/error.tsx",
        r#"export default function ErrorPage() { return null; }
export const helperError = 1;
"#,
    );
    project.write(
        "src/app/global-not-found.tsx",
        r#"export const metadata = { title: "Not found" };
export default function GlobalNotFound() { return <html><body>Not found</body></html>; }
export const helperGlobalNotFound = 1;
"#,
    );
    project.write(
        "src/app/forbidden.tsx",
        r#"export default function Forbidden() { return null; }
export const helperForbidden = 1;
"#,
    );
    project.write(
        "src/app/unauthorized.tsx",
        r#"export default function Unauthorized() { return null; }
export const helperUnauthorized = 1;
"#,
    );
    project.write(
        "src/app/loading.tsx",
        r#"export default function Loading() { return null; }
export const helperLoading = 1;
"#,
    );
    project.write(
        "src/app/@modal/default.tsx",
        r#"export default function ModalDefault() { return null; }
export const helperDefault = 1;
"#,
    );
    project.write(
        "src/app/api/route.ts",
        r#"export const runtime = "nodejs";
export const maxDuration = 5;
export async function generateStaticParams() { return []; }
export async function GET() { return Response.json({ ok: true }); }
export async function POST() { return Response.json({ ok: true }); }
export const helperRoute = 1;
"#,
    );

    let report = analyze_project(project.root()).expect("project should analyze");
    let mut dead_exports = report
        .findings
        .dead_exports
        .iter()
        .map(|item| {
            (
                item.file
                    .strip_prefix(project.root())
                    .unwrap_or(&item.file)
                    .to_path_buf(),
                item.export_name.clone(),
            )
        })
        .collect::<Vec<_>>();
    dead_exports.sort();

    assert_eq!(report.summary.entrypoints, 9);
    assert_eq!(report.summary.route_entrypoints, 9);
    assert_eq!(report.summary.dead_exports, 9);
    assert_eq!(
        dead_exports,
        vec![
            (
                PathBuf::from("src/app/@modal/default.tsx"),
                "helperDefault".to_string(),
            ),
            (
                PathBuf::from("src/app/api/route.ts"),
                "helperRoute".to_string(),
            ),
            (
                PathBuf::from("src/app/error.tsx"),
                "helperError".to_string(),
            ),
            (
                PathBuf::from("src/app/forbidden.tsx"),
                "helperForbidden".to_string(),
            ),
            (
                PathBuf::from("src/app/global-not-found.tsx"),
                "helperGlobalNotFound".to_string(),
            ),
            (
                PathBuf::from("src/app/layout.tsx"),
                "helperLayout".to_string(),
            ),
            (
                PathBuf::from("src/app/loading.tsx"),
                "helperLoading".to_string(),
            ),
            (PathBuf::from("src/app/page.tsx"), "helperPage".to_string(),),
            (
                PathBuf::from("src/app/unauthorized.tsx"),
                "helperUnauthorized".to_string(),
            ),
        ]
    );
}

#[test]
fn analyze_project_skips_next_metadata_helpers_for_metadata_files() {
    let project = TestProject::new("metadata-helpers");
    project.write(
        "app/opengraph-image.tsx",
        r#"export function generateImageMetadata() {
  return [{ id: "1", contentType: "image/png", size: { width: 1200, height: 630 } }];
}
export const alt = "About Acme";
export const size = { width: 1200, height: 630 };
export const contentType = "image/png";
export const runtime = "edge";
export default function Image() { return null; }
export const helperImage = 1;
"#,
    );
    project.write(
        "app/icon.tsx",
        r#"export const size = { width: 32, height: 32 };
export const contentType = "image/png";
export const preferredRegion = "global";
export default function Icon() { return null; }
export const helperIcon = 1;
"#,
    );

    let report = analyze_project(project.root()).expect("project should analyze");

    let mut dead_exports = report
        .findings
        .dead_exports
        .iter()
        .map(|item| {
            (
                item.file
                    .strip_prefix(project.root())
                    .unwrap_or(&item.file)
                    .to_path_buf(),
                item.export_name.clone(),
            )
        })
        .collect::<Vec<_>>();
    dead_exports.sort();

    assert_eq!(report.summary.entrypoints, 2);
    assert_eq!(report.summary.route_entrypoints, 2);
    assert_eq!(report.summary.dead_exports, 2);
    assert_eq!(
        dead_exports,
        vec![
            (PathBuf::from("app/icon.tsx"), "helperIcon".to_string()),
            (
                PathBuf::from("app/opengraph-image.tsx"),
                "helperImage".to_string(),
            ),
        ]
    );
}

#[test]
fn analyze_project_protects_orbit_dashboard_regression_surface() {
    let project = TestProject::new("orbit-dashboard-regression");
    project.write(
        "package.json",
        r#"{
  "scripts": {
    "generate:roster": "node scripts/generate-roster-json.mjs",
    "sync:pr-daily": "npm run sync:pr-reviews",
    "sync:pr-reviews": "node scripts/github-pr-reviews-daily-insert.mjs"
  },
  "dependencies": {
    "next": "latest",
    "react": "latest"
  }
}
"#,
    );
    project.write(
        ".github/workflows/pr-weekly-snapshot.yml",
        r#"
name: PR weekly snapshot
jobs:
  snapshot:
    steps:
      - run: node scripts/github-pr-weekly-snapshot.mjs
"#,
    );
    project.write(
        ".github/actions/ai-readiness-scan/action.yml",
        r#"
runs:
  using: composite
  steps:
    - run: node "$ACTION_PATH/../../../scripts/ai-readiness-submit.mjs"
    - run: node ${{github.action_path}}/../../../scripts/ai-readiness-submit-expression.mjs
    - working-directory: scripts
      run: node "$ACTION_PATH/../../../scripts/ai-readiness-submit-working-dir.mjs"
"#,
    );
    project.write("eslint.config.mjs", "export default [];\n");
    project.write(
        "scripts/generate-roster-json.mjs",
        "console.log('generate roster');\n",
    );
    project.write(
        "scripts/github-pr-reviews-daily-insert.mjs",
        "console.log('daily reviews');\n",
    );
    project.write(
        "scripts/github-pr-weekly-snapshot.mjs",
        "console.log('weekly snapshot');\n",
    );
    project.write(
        "scripts/ai-readiness-submit.mjs",
        "export function submitPayload() { return true; }\n",
    );
    project.write(
        "scripts/ai-readiness-submit-expression.mjs",
        "export function submitExpressionPayload() { return true; }\n",
    );
    project.write(
        "scripts/ai-readiness-submit-working-dir.mjs",
        "export function submitWorkingDirPayload() { return true; }\n",
    );
    project.write(
        "scripts/verify-beta-smoke.mjs",
        "export function verifyBetaSmoke() { return true; }\n",
    );
    project.write(
        "scripts/verify-integrated-insights-smoke.mjs",
        "export function verifyIntegratedInsightsSmoke() { return true; }\n",
    );
    project.write(
        "src/__tests__/scripts/verify-beta-smoke.test.ts",
        r#"import { verifyBetaSmoke } from "../../../scripts/verify-beta-smoke.mjs";

verifyBetaSmoke();
"#,
    );
    project.write(
        "src/shared/lib/validators.ts",
        "export function validateEngineer() { return true; }\n",
    );
    project.write(
        "src/__tests__/lib/validators.test.ts",
        r#"import { validateEngineer } from "../../shared/lib/validators";

validateEngineer();
"#,
    );
    project.write(
        "src/app/admin/data/_tabs.test.ts",
        "export const adminTabsTest = true;\n",
    );
    project.write(
        "src/app/admin/page.tsx",
        r#"import dynamic from "next/dynamic";

function loadDashboard(loader: () => Promise<unknown>) {
  return dynamic(loader);
}

const AdminPanel = loadDashboard(() => import("../../features/admin/AdminPanel"));

export const metadata = { title: "Admin" };
export default function AdminPage() { return <AdminPanel />; }
export const localHelper = true;
"#,
    );
    project.write(
        "src/features/admin/AdminPanel.tsx",
        r#"export default function AdminPanel() { return null; }
export const panelConfig = {};
"#,
    );
    project.write(
        "src/Widgets/CycleTime/ui/index.ts",
        "export { default as CycleTimeChart } from \"./CycleTimeChart\";\n",
    );
    project.write(
        "src/Widgets/CycleTime/ui/CycleTimeChart.tsx",
        "export default function CycleTimeChart() { return null; }\n",
    );
    project.write(
        "src/Widgets/CycleTime/model/stale.ts",
        "export const staleWidgetModel = true;\n",
    );

    let report = analyze_project(project.root()).expect("project should analyze");

    let deletion_candidates = relative_finding_files(
        project.root(),
        report
            .findings
            .deletion_candidates
            .iter()
            .map(|finding| &finding.file),
    );
    let orphan_files = relative_finding_files(
        project.root(),
        report
            .findings
            .orphan_files
            .iter()
            .map(|finding| &finding.file),
    );
    let dead_exports = report
        .findings
        .dead_exports
        .iter()
        .map(|finding| {
            (
                to_relative_path(project.root(), &finding.file),
                finding.export_name.clone(),
            )
        })
        .collect::<Vec<_>>();

    for protected in [
        "eslint.config.mjs",
        "scripts/generate-roster-json.mjs",
        "scripts/github-pr-reviews-daily-insert.mjs",
        "scripts/github-pr-weekly-snapshot.mjs",
        "scripts/ai-readiness-submit.mjs",
        "scripts/ai-readiness-submit-expression.mjs",
        "scripts/ai-readiness-submit-working-dir.mjs",
        "scripts/verify-beta-smoke.mjs",
        "scripts/verify-integrated-insights-smoke.mjs",
        "src/__tests__/scripts/verify-beta-smoke.test.ts",
        "src/shared/lib/validators.ts",
        "src/__tests__/lib/validators.test.ts",
        "src/app/admin/data/_tabs.test.ts",
        "src/app/admin/page.tsx",
        "src/features/admin/AdminPanel.tsx",
        "src/Widgets/CycleTime/ui/index.ts",
    ] {
        assert!(
            !deletion_candidates.contains(&protected.to_string()),
            "{protected} should not be a deletion candidate"
        );
        assert!(
            !orphan_files.contains(&protected.to_string()),
            "{protected} should not be reported as an orphan file"
        );
    }

    assert!(!dead_exports.contains(&(
        "src/features/admin/AdminPanel.tsx".to_string(),
        "default".to_string(),
    )));
    assert!(!dead_exports.contains(&(
        "src/features/admin/AdminPanel.tsx".to_string(),
        "panelConfig".to_string(),
    )));
    assert!(!dead_exports.contains(&(
        "scripts/ai-readiness-submit.mjs".to_string(),
        "submitPayload".to_string(),
    )));
    assert!(!dead_exports.contains(&(
        "scripts/ai-readiness-submit-expression.mjs".to_string(),
        "submitExpressionPayload".to_string(),
    )));
    assert!(!dead_exports.contains(&(
        "scripts/ai-readiness-submit-working-dir.mjs".to_string(),
        "submitWorkingDirPayload".to_string(),
    )));
    assert!(!dead_exports.contains(&(
        "scripts/verify-beta-smoke.mjs".to_string(),
        "verifyBetaSmoke".to_string(),
    )));
    assert!(!dead_exports.contains(&(
        "scripts/verify-integrated-insights-smoke.mjs".to_string(),
        "verifyIntegratedInsightsSmoke".to_string(),
    )));
    assert!(!dead_exports.contains(&(
        "src/shared/lib/validators.ts".to_string(),
        "validateEngineer".to_string(),
    )));
    assert!(!dead_exports
        .iter()
        .any(|(file, _)| file == "src/Widgets/CycleTime/ui/index.ts"));
    assert!(dead_exports.contains(&(
        "src/app/admin/page.tsx".to_string(),
        "localHelper".to_string(),
    )));
    assert!(deletion_candidates.contains(&"src/Widgets/CycleTime/model/stale.ts".to_string()));

    let entrypoint_kinds = report
        .modules
        .iter()
        .filter_map(|module| {
            module
                .entrypoint_kind
                .clone()
                .map(|kind| (module.relative_path.clone(), kind))
        })
        .collect::<Vec<_>>();

    for (path, kind) in [
        (
            "scripts/generate-roster-json.mjs",
            EntrypointKind::PackageEntry,
        ),
        (
            "scripts/github-pr-reviews-daily-insert.mjs",
            EntrypointKind::PackageEntry,
        ),
        (
            "scripts/github-pr-weekly-snapshot.mjs",
            EntrypointKind::PackageEntry,
        ),
        (
            "scripts/ai-readiness-submit.mjs",
            EntrypointKind::PackageEntry,
        ),
        (
            "scripts/ai-readiness-submit-expression.mjs",
            EntrypointKind::PackageEntry,
        ),
        (
            "scripts/ai-readiness-submit-working-dir.mjs",
            EntrypointKind::PackageEntry,
        ),
        ("eslint.config.mjs", EntrypointKind::ToolingEntry),
        (
            "src/__tests__/scripts/verify-beta-smoke.test.ts",
            EntrypointKind::ToolingEntry,
        ),
        (
            "src/__tests__/lib/validators.test.ts",
            EntrypointKind::ToolingEntry,
        ),
        (
            "src/app/admin/data/_tabs.test.ts",
            EntrypointKind::ToolingEntry,
        ),
        (
            "scripts/verify-integrated-insights-smoke.mjs",
            EntrypointKind::ToolingEntry,
        ),
        ("src/app/admin/page.tsx", EntrypointKind::NextAppRoute),
    ] {
        assert!(
            entrypoint_kinds.contains(&(path.to_string(), kind.clone())),
            "{path} should be classified as {kind:?}"
        );
    }
}

#[test]
fn analyze_project_protects_user_entries_and_keep_patterns_from_deletion_candidates() {
    let project = TestProject::new("user-entry-keep-patterns");
    project.write(
        "kratos.config.json",
        r#"{
  "entry": ["src/bootstrap.ts"],
  "keepPatterns": ["scripts/manual-*.mjs", "src/generated/**", "!src/generated/drop.ts"]
}
"#,
    );
    project.write("src/bootstrap.ts", "export const bootstrap = true;\n");
    project.write(
        "scripts/manual-release.mjs",
        "export const manualRelease = true;\n",
    );
    project.write(
        "src/generated/keep.ts",
        "export const generatedKeep = true;\n",
    );
    project.write(
        "src/generated/drop.ts",
        "export const generatedDrop = true;\n",
    );
    project.write("scripts/drop.mjs", "export const dropScript = true;\n");

    let report = analyze_project(project.root()).expect("project should analyze");
    let deletion_candidates = relative_finding_files(
        project.root(),
        report
            .findings
            .deletion_candidates
            .iter()
            .map(|finding| &finding.file),
    );
    let orphan_files = relative_finding_files(
        project.root(),
        report
            .findings
            .orphan_files
            .iter()
            .map(|finding| &finding.file),
    );

    for protected in [
        "src/bootstrap.ts",
        "scripts/manual-release.mjs",
        "src/generated/keep.ts",
    ] {
        assert!(
            !deletion_candidates.contains(&protected.to_string()),
            "{protected} should not be a deletion candidate"
        );
        assert!(
            !orphan_files.contains(&protected.to_string()),
            "{protected} should not be reported as an orphan file"
        );
    }

    assert!(deletion_candidates.contains(&"src/generated/drop.ts".to_string()));
    assert!(deletion_candidates.contains(&"scripts/drop.mjs".to_string()));

    let user_entry = report
        .modules
        .iter()
        .find(|module| module.relative_path == "src/bootstrap.ts")
        .expect("user entry module should be scanned");
    assert_eq!(user_entry.entrypoint_kind, Some(EntrypointKind::UserEntry));
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

fn relative_finding_files<'a>(
    root: &Path,
    files: impl Iterator<Item = &'a PathBuf>,
) -> Vec<String> {
    files.map(|file| to_relative_path(root, file)).collect()
}

fn to_relative_path(root: &Path, file: &Path) -> String {
    file.strip_prefix(root)
        .unwrap_or(file)
        .to_string_lossy()
        .replace('\\', "/")
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
