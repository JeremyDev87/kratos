use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use kratos_core::analyze::analyze_project;
use kratos_core::model::{ImportKind, ImportSpecifierKind};

#[test]
fn analyze_project_keeps_react_lazy_default_export_live() {
    let project = TestProject::new("react-lazy-default");
    project.write(
        "src/consumer.tsx",
        r#"
import React from "react";

const consumer = React.lazy(() => import("./Chunk"));
"#,
    );
    project.write(
        "src/Chunk.tsx",
        r#"
export default function Chunk() {
  return null;
}
"#,
    );

    let report = analyze_project(project.root()).expect("project should analyze");

    assert!(report.findings.dead_exports.is_empty());

    let chunk = report
        .modules
        .iter()
        .find(|module| module.relative_path == "src/Chunk.tsx")
        .expect("chunk module should exist");

    assert_eq!(chunk.imported_by_count, 1);
    assert_eq!(chunk.importers.len(), 1);
    assert_eq!(chunk.importers[0].kind, ImportKind::Dynamic);
    assert_eq!(
        chunk.importers[0].specifiers[0].kind,
        ImportSpecifierKind::Default
    );
    assert_eq!(
        chunk.importers[0].specifiers[0].imported.as_deref(),
        Some("default")
    );
}

#[test]
fn analyze_project_keeps_named_react_lazy_default_export_live() {
    let project = TestProject::new("react-lazy-named-import");
    project.write(
        "src/consumer.tsx",
        r#"
import { lazy } from "react";

const consumer = lazy(() => import("./Chunk"));
"#,
    );
    project.write(
        "src/Chunk.tsx",
        r#"
export default function Chunk() {
  return null;
}
"#,
    );

    let report = analyze_project(project.root()).expect("project should analyze");

    assert!(report.findings.dead_exports.is_empty());

    let chunk = report
        .modules
        .iter()
        .find(|module| module.relative_path == "src/Chunk.tsx")
        .expect("chunk module should exist");

    assert_eq!(chunk.imported_by_count, 1);
    assert_eq!(chunk.importers.len(), 1);
    assert_eq!(chunk.importers[0].kind, ImportKind::Dynamic);
    assert_eq!(
        chunk.importers[0].specifiers[0].kind,
        ImportSpecifierKind::Default
    );
    assert_eq!(
        chunk.importers[0].specifiers[0].imported.as_deref(),
        Some("default")
    );
}

#[test]
fn analyze_project_keeps_commonjs_react_lazy_default_export_live() {
    let project = TestProject::new("react-lazy-commonjs");
    project.write(
        "src/consumer.js",
        r#"
const React = require("react");

const consumer = React.lazy(() => import("./Chunk"));
"#,
    );
    project.write(
        "src/Chunk.js",
        r#"
export default function Chunk() {
  return null;
}
"#,
    );

    let report = analyze_project(project.root()).expect("project should analyze");

    assert!(report.findings.dead_exports.is_empty());

    let chunk = report
        .modules
        .iter()
        .find(|module| module.relative_path == "src/Chunk.js")
        .expect("chunk module should exist");

    assert_eq!(chunk.imported_by_count, 1);
    assert_eq!(chunk.importers.len(), 1);
    assert_eq!(chunk.importers[0].kind, ImportKind::Dynamic);
    assert_eq!(
        chunk.importers[0].specifiers[0].kind,
        ImportSpecifierKind::Default
    );
    assert_eq!(
        chunk.importers[0].specifiers[0].imported.as_deref(),
        Some("default")
    );
}

#[test]
fn analyze_project_keeps_nested_block_var_commonjs_react_lazy_default_export_live() {
    let project = TestProject::new("react-lazy-commonjs-nested-var");
    project.write(
        "src/consumer.js",
        r#"
function build() {
  if (enabled) {
    var React = require("react");
    return React.lazy(() => import("./Chunk"));
  }
}

const consumer = build();
"#,
    );
    project.write(
        "src/Chunk.js",
        r#"
export default function Chunk() {
  return null;
}
"#,
    );

    let report = analyze_project(project.root()).expect("project should analyze");

    assert!(report.findings.dead_exports.is_empty());

    let chunk = report
        .modules
        .iter()
        .find(|module| module.relative_path == "src/Chunk.js")
        .expect("chunk module should exist");

    assert_eq!(chunk.imported_by_count, 1);
    assert_eq!(chunk.importers.len(), 1);
    assert_eq!(chunk.importers[0].kind, ImportKind::Dynamic);
    assert_eq!(
        chunk.importers[0].specifiers[0].kind,
        ImportSpecifierKind::Default
    );
    assert_eq!(
        chunk.importers[0].specifiers[0].imported.as_deref(),
        Some("default")
    );
}

#[test]
fn analyze_project_keeps_ts_import_equals_react_lazy_default_export_live() {
    let project = TestProject::new("react-lazy-ts-import-equals");
    project.write(
        "src/consumer.tsx",
        r#"
import React = require("react");

const consumer = React.lazy(() => import("./Chunk"));
"#,
    );
    project.write(
        "src/Chunk.tsx",
        r#"
export default function Chunk() {
  return null;
}
"#,
    );

    let report = analyze_project(project.root()).expect("project should analyze");

    assert!(report.findings.dead_exports.is_empty());

    let chunk = report
        .modules
        .iter()
        .find(|module| module.relative_path == "src/Chunk.tsx")
        .expect("chunk module should exist");

    assert_eq!(chunk.imported_by_count, 1);
    assert_eq!(chunk.importers.len(), 1);
    assert_eq!(chunk.importers[0].kind, ImportKind::Dynamic);
    assert_eq!(
        chunk.importers[0].specifiers[0].kind,
        ImportSpecifierKind::Default
    );
    assert_eq!(
        chunk.importers[0].specifiers[0].imported.as_deref(),
        Some("default")
    );
}

#[test]
fn analyze_project_keeps_react_lazy_alias_default_export_live() {
    let project = TestProject::new("react-lazy-alias");
    project.write(
        "src/consumer.tsx",
        r#"
import React from "react";

const load = React.lazy;
const consumer = load(() => import("./Chunk"));
"#,
    );
    project.write(
        "src/Chunk.tsx",
        r#"
export default function Chunk() {
  return null;
}
"#,
    );

    let report = analyze_project(project.root()).expect("project should analyze");

    assert!(report.findings.dead_exports.is_empty());

    let chunk = report
        .modules
        .iter()
        .find(|module| module.relative_path == "src/Chunk.tsx")
        .expect("chunk module should exist");

    assert_eq!(chunk.imported_by_count, 1);
    assert_eq!(chunk.importers.len(), 1);
    assert_eq!(chunk.importers[0].kind, ImportKind::Dynamic);
    assert_eq!(
        chunk.importers[0].specifiers[0].kind,
        ImportSpecifierKind::Default
    );
    assert_eq!(
        chunk.importers[0].specifiers[0].imported.as_deref(),
        Some("default")
    );
}

#[test]
fn analyze_project_keeps_react_lazy_loader_identifier_default_export_live() {
    let project = TestProject::new("react-lazy-loader-identifier");
    project.write(
        "src/consumer.tsx",
        r#"
import React from "react";

const loader = () => import("./Chunk");
const consumer = React.lazy(loader);
"#,
    );
    project.write(
        "src/Chunk.tsx",
        r#"
export default function Chunk() {
  return null;
}
"#,
    );

    let report = analyze_project(project.root()).expect("project should analyze");

    assert!(report.findings.dead_exports.is_empty());

    let chunk = report
        .modules
        .iter()
        .find(|module| module.relative_path == "src/Chunk.tsx")
        .expect("chunk module should exist");

    assert_eq!(chunk.imported_by_count, 1);
    assert_eq!(chunk.importers.len(), 1);
    assert_eq!(chunk.importers[0].kind, ImportKind::Dynamic);
    assert_eq!(
        chunk.importers[0].specifiers[0].kind,
        ImportSpecifierKind::Default
    );
    assert_eq!(
        chunk.importers[0].specifiers[0].imported.as_deref(),
        Some("default")
    );
}

#[test]
fn analyze_project_keeps_hoisted_function_loader_default_export_live() {
    let project = TestProject::new("react-lazy-hoisted-function-loader");
    project.write(
        "src/consumer.tsx",
        r#"
import React from "react";

const consumer = React.lazy(loader);

function loader() {
  return import("./Chunk");
}
"#,
    );
    project.write(
        "src/Chunk.tsx",
        r#"
export default function Chunk() {
  return null;
}
"#,
    );

    let report = analyze_project(project.root()).expect("project should analyze");

    assert!(report.findings.dead_exports.is_empty());

    let chunk = report
        .modules
        .iter()
        .find(|module| module.relative_path == "src/Chunk.tsx")
        .expect("chunk module should exist");

    assert_eq!(chunk.imported_by_count, 1);
    assert_eq!(chunk.importers.len(), 1);
    assert_eq!(chunk.importers[0].kind, ImportKind::Dynamic);
    assert_eq!(
        chunk.importers[0].specifiers[0].kind,
        ImportSpecifierKind::Default
    );
    assert_eq!(
        chunk.importers[0].specifiers[0].imported.as_deref(),
        Some("default")
    );
}

#[test]
fn analyze_project_keeps_react_lazy_loader_alias_identifier_default_export_live() {
    let project = TestProject::new("react-lazy-loader-alias-identifier");
    project.write(
        "src/consumer.tsx",
        r#"
import React from "react";

const loader = () => import("./Chunk");
const alias = loader;
const consumer = React.lazy(alias);
"#,
    );
    project.write(
        "src/Chunk.tsx",
        r#"
export default function Chunk() {
  return null;
}
"#,
    );

    let report = analyze_project(project.root()).expect("project should analyze");

    assert!(report.findings.dead_exports.is_empty());

    let chunk = report
        .modules
        .iter()
        .find(|module| module.relative_path == "src/Chunk.tsx")
        .expect("chunk module should exist");

    assert_eq!(chunk.imported_by_count, 1);
    assert_eq!(chunk.importers.len(), 1);
    assert_eq!(chunk.importers[0].kind, ImportKind::Dynamic);
    assert_eq!(
        chunk.importers[0].specifiers[0].kind,
        ImportSpecifierKind::Default
    );
    assert_eq!(
        chunk.importers[0].specifiers[0].imported.as_deref(),
        Some("default")
    );
}

#[test]
fn analyze_project_keeps_nested_loader_alias_after_non_loader_shadow_default_export_live() {
    let project = TestProject::new("react-lazy-loader-nested-shadow");
    project.write(
        "src/consumer.tsx",
        r#"
import React from "react";

const load = () => import("./OuterChunk");

function build() {
  const load = 0;

  {
    const load = () => import("./InnerChunk");
    return React.lazy(load);
  }
}

const consumer = build();
"#,
    );
    project.write(
        "src/OuterChunk.tsx",
        r#"
export default function OuterChunk() {
  return null;
}
"#,
    );
    project.write(
        "src/InnerChunk.tsx",
        r#"
export default function InnerChunk() {
  return null;
}
"#,
    );

    let report = analyze_project(project.root()).expect("project should analyze");

    assert!(
        report
            .findings
            .dead_exports
            .iter()
            .all(|finding| finding.file != project.root().join("src/InnerChunk.tsx"))
    );

    let chunk = report
        .modules
        .iter()
        .find(|module| module.relative_path == "src/InnerChunk.tsx")
        .expect("inner chunk module should exist");

    assert_eq!(chunk.imported_by_count, 1);
    assert_eq!(chunk.importers.len(), 1);
    assert_eq!(chunk.importers[0].kind, ImportKind::Dynamic);
    assert_eq!(
        chunk.importers[0].specifiers[0].kind,
        ImportSpecifierKind::Default
    );
    assert_eq!(
        chunk.importers[0].specifiers[0].imported.as_deref(),
        Some("default")
    );
}

#[test]
fn analyze_project_keeps_next_dynamic_named_export_live() {
    let project = TestProject::new("next-dynamic-named");
    project.write(
        "src/consumer.tsx",
        r#"
import dynamic from "next/dynamic";

const consumer = dynamic(() =>
  import("./Chunk").then((module) => ({ default: module.Named }))
);
"#,
    );
    project.write(
        "src/Chunk.tsx",
        r#"
export default function Chunk() {
  return null;
}

export const Named = true;
"#,
    );

    let report = analyze_project(project.root()).expect("project should analyze");

    assert_eq!(report.findings.dead_exports.len(), 1);
    assert_eq!(
        report.findings.dead_exports[0].file,
        project.root().join("src/Chunk.tsx")
    );
    assert_eq!(report.findings.dead_exports[0].export_name, "default");

    let chunk = report
        .modules
        .iter()
        .find(|module| module.relative_path == "src/Chunk.tsx")
        .expect("chunk module should exist");

    assert_eq!(chunk.imported_by_count, 1);
    assert_eq!(chunk.importers.len(), 1);
    assert_eq!(chunk.importers[0].kind, ImportKind::Dynamic);
    assert_eq!(
        chunk.importers[0].specifiers[0].kind,
        ImportSpecifierKind::Named
    );
    assert_eq!(
        chunk.importers[0].specifiers[0].imported.as_deref(),
        Some("Named")
    );
}

#[test]
fn analyze_project_keeps_next_dynamic_loader_alias_identifier_named_export_live() {
    let project = TestProject::new("next-dynamic-loader-alias-identifier");
    project.write(
        "src/consumer.tsx",
        r#"
import dynamic from "next/dynamic";

const loader = () =>
  import("./Chunk").then(({ Named }) => ({ default: Named }));
const alias = loader;
const consumer = dynamic(alias);
"#,
    );
    project.write(
        "src/Chunk.tsx",
        r#"
export default function Chunk() {
  return null;
}

export const Named = true;
"#,
    );

    let report = analyze_project(project.root()).expect("project should analyze");

    assert_eq!(report.findings.dead_exports.len(), 1);
    assert_eq!(
        report.findings.dead_exports[0].file,
        project.root().join("src/Chunk.tsx")
    );
    assert_eq!(report.findings.dead_exports[0].export_name, "default");

    let chunk = report
        .modules
        .iter()
        .find(|module| module.relative_path == "src/Chunk.tsx")
        .expect("chunk module should exist");

    assert_eq!(chunk.imported_by_count, 1);
    assert_eq!(chunk.importers.len(), 1);
    assert_eq!(chunk.importers[0].kind, ImportKind::Dynamic);
    assert_eq!(
        chunk.importers[0].specifiers[0].kind,
        ImportSpecifierKind::Named
    );
    assert_eq!(
        chunk.importers[0].specifiers[0].imported.as_deref(),
        Some("Named")
    );
}

#[test]
fn analyze_project_keeps_then_block_alias_named_export_live() {
    let project = TestProject::new("next-dynamic-then-block-alias");
    project.write(
        "src/consumer.tsx",
        r#"
import dynamic from "next/dynamic";

const consumer = dynamic(() =>
  import("./Chunk").then((module) => {
    const selected = module.Named;
    return { default: selected };
  })
);
"#,
    );
    project.write(
        "src/Chunk.tsx",
        r#"
export default function Chunk() {
  return null;
}

export const Named = true;
"#,
    );

    let report = analyze_project(project.root()).expect("project should analyze");

    assert_eq!(report.findings.dead_exports.len(), 1);
    assert_eq!(
        report.findings.dead_exports[0].file,
        project.root().join("src/Chunk.tsx")
    );
    assert_eq!(report.findings.dead_exports[0].export_name, "default");

    let chunk = report
        .modules
        .iter()
        .find(|module| module.relative_path == "src/Chunk.tsx")
        .expect("chunk module should exist");

    assert_eq!(chunk.imported_by_count, 1);
    assert_eq!(chunk.importers.len(), 1);
    assert_eq!(chunk.importers[0].kind, ImportKind::Dynamic);
    assert_eq!(
        chunk.importers[0].specifiers[0].kind,
        ImportSpecifierKind::Named
    );
    assert_eq!(
        chunk.importers[0].specifiers[0].imported.as_deref(),
        Some("Named")
    );
}

#[test]
fn analyze_project_keeps_then_named_callback_identifier_export_live() {
    let project = TestProject::new("next-dynamic-then-named-callback");
    project.write(
        "src/consumer.tsx",
        r#"
import dynamic from "next/dynamic";

const select = (module) => ({ default: module.Named });
const consumer = dynamic(() =>
  import("./Chunk").then(select)
);
"#,
    );
    project.write(
        "src/Chunk.tsx",
        r#"
export default function Chunk() {
  return null;
}

export const Named = true;
"#,
    );

    let report = analyze_project(project.root()).expect("project should analyze");

    assert_eq!(report.findings.dead_exports.len(), 1);
    assert_eq!(
        report.findings.dead_exports[0].file,
        project.root().join("src/Chunk.tsx")
    );
    assert_eq!(report.findings.dead_exports[0].export_name, "default");

    let chunk = report
        .modules
        .iter()
        .find(|module| module.relative_path == "src/Chunk.tsx")
        .expect("chunk module should exist");

    assert_eq!(chunk.imported_by_count, 1);
    assert_eq!(chunk.importers.len(), 1);
    assert_eq!(chunk.importers[0].kind, ImportKind::Dynamic);
    assert_eq!(
        chunk.importers[0].specifiers[0].kind,
        ImportSpecifierKind::Named
    );
    assert_eq!(
        chunk.importers[0].specifiers[0].imported.as_deref(),
        Some("Named")
    );
}

#[test]
fn analyze_project_falls_back_for_hoisted_var_then_callback_alias() {
    let project = TestProject::new("next-dynamic-hoisted-var-then-callback");
    project.write(
        "src/consumer.tsx",
        r#"
import dynamic from "next/dynamic";

const consumer = dynamic(() =>
  import("./Chunk").then(select)
);

var select = (module) => ({ default: module.Named });
"#,
    );
    project.write(
        "src/Chunk.tsx",
        r#"
export default function Chunk() {
  return null;
}

export const Named = true;
"#,
    );

    let report = analyze_project(project.root()).expect("project should analyze");

    assert_eq!(report.findings.dead_exports.len(), 2);

    let chunk = report
        .modules
        .iter()
        .find(|module| module.relative_path == "src/Chunk.tsx")
        .expect("chunk module should exist");

    assert_eq!(chunk.imported_by_count, 1);
    assert_eq!(chunk.importers.len(), 1);
    assert_eq!(chunk.importers[0].kind, ImportKind::Dynamic);
    assert!(chunk.importers[0].specifiers.is_empty());
}

#[test]
fn analyze_project_keeps_hoisted_then_function_callback_export_live() {
    let project = TestProject::new("next-dynamic-hoisted-then-function-callback");
    project.write(
        "src/consumer.tsx",
        r#"
import dynamic from "next/dynamic";

const consumer = dynamic(() =>
  import("./Chunk").then(select)
);

function select(module) {
  return { default: module.Named };
}
"#,
    );
    project.write(
        "src/Chunk.tsx",
        r#"
export default function Chunk() {
  return null;
}

export const Named = true;
"#,
    );

    let report = analyze_project(project.root()).expect("project should analyze");

    assert_eq!(report.findings.dead_exports.len(), 1);
    assert_eq!(
        report.findings.dead_exports[0].file,
        project.root().join("src/Chunk.tsx")
    );
    assert_eq!(report.findings.dead_exports[0].export_name, "default");

    let chunk = report
        .modules
        .iter()
        .find(|module| module.relative_path == "src/Chunk.tsx")
        .expect("chunk module should exist");

    assert_eq!(chunk.imported_by_count, 1);
    assert_eq!(chunk.importers.len(), 1);
    assert_eq!(chunk.importers[0].kind, ImportKind::Dynamic);
    assert_eq!(
        chunk.importers[0].specifiers[0].kind,
        ImportSpecifierKind::Named
    );
    assert_eq!(
        chunk.importers[0].specifiers[0].imported.as_deref(),
        Some("Named")
    );
}

#[test]
fn analyze_project_keeps_then_nested_block_followed_by_outer_return_live() {
    let project = TestProject::new("next-dynamic-then-nested-block-outer-return");
    project.write(
        "src/consumer.tsx",
        r#"
import dynamic from "next/dynamic";

const consumer = dynamic(() =>
  import("./Chunk").then((module) => {
    {
      const selected = module.Named;
    }

    return { default: module.Named };
  })
);
"#,
    );
    project.write(
        "src/Chunk.tsx",
        r#"
export default function Chunk() {
  return null;
}

export const Named = true;
"#,
    );

    let report = analyze_project(project.root()).expect("project should analyze");

    assert_eq!(report.findings.dead_exports.len(), 1);
    assert_eq!(
        report.findings.dead_exports[0].file,
        project.root().join("src/Chunk.tsx")
    );
    assert_eq!(report.findings.dead_exports[0].export_name, "default");

    let chunk = report
        .modules
        .iter()
        .find(|module| module.relative_path == "src/Chunk.tsx")
        .expect("chunk module should exist");

    assert_eq!(chunk.imported_by_count, 1);
    assert_eq!(chunk.importers.len(), 1);
    assert_eq!(chunk.importers[0].kind, ImportKind::Dynamic);
    assert_eq!(
        chunk.importers[0].specifiers[0].kind,
        ImportSpecifierKind::Named
    );
    assert_eq!(
        chunk.importers[0].specifiers[0].imported.as_deref(),
        Some("Named")
    );
}

#[test]
fn analyze_project_keeps_then_alias_selection_with_unrelated_local_live() {
    let project = TestProject::new("next-dynamic-then-alias-with-unrelated-local");
    project.write(
        "src/consumer.tsx",
        r#"
import dynamic from "next/dynamic";

const consumer = dynamic(() =>
  import("./Chunk").then((module) => {
    const selected = module.Named;
    const noop = 1;
    return { default: selected };
  })
);
"#,
    );
    project.write(
        "src/Chunk.tsx",
        r#"
export default function Chunk() {
  return null;
}

export const Named = true;
"#,
    );

    let report = analyze_project(project.root()).expect("project should analyze");

    assert_eq!(report.findings.dead_exports.len(), 1);
    assert_eq!(
        report.findings.dead_exports[0].file,
        project.root().join("src/Chunk.tsx")
    );
    assert_eq!(report.findings.dead_exports[0].export_name, "default");

    let chunk = report
        .modules
        .iter()
        .find(|module| module.relative_path == "src/Chunk.tsx")
        .expect("chunk module should exist");

    assert_eq!(chunk.imported_by_count, 1);
    assert_eq!(chunk.importers.len(), 1);
    assert_eq!(chunk.importers[0].kind, ImportKind::Dynamic);
    assert_eq!(
        chunk.importers[0].specifiers[0].kind,
        ImportSpecifierKind::Named
    );
    assert_eq!(
        chunk.importers[0].specifiers[0].imported.as_deref(),
        Some("Named")
    );
}

#[test]
fn analyze_project_keeps_then_destructured_local_alias_live() {
    let project = TestProject::new("next-dynamic-then-destructured-local-alias");
    project.write(
        "src/consumer.tsx",
        r#"
import dynamic from "next/dynamic";

const consumer = dynamic(() =>
  import("./Chunk").then((module) => {
    const { Named: selected } = module;
    return { default: selected };
  })
);
"#,
    );
    project.write(
        "src/Chunk.tsx",
        r#"
export default function Chunk() {
  return null;
}

export const Named = true;
"#,
    );

    let report = analyze_project(project.root()).expect("project should analyze");

    assert_eq!(report.findings.dead_exports.len(), 1);
    assert_eq!(
        report.findings.dead_exports[0].file,
        project.root().join("src/Chunk.tsx")
    );
    assert_eq!(report.findings.dead_exports[0].export_name, "default");

    let chunk = report
        .modules
        .iter()
        .find(|module| module.relative_path == "src/Chunk.tsx")
        .expect("chunk module should exist");

    assert_eq!(chunk.imported_by_count, 1);
    assert_eq!(chunk.importers.len(), 1);
    assert_eq!(chunk.importers[0].kind, ImportKind::Dynamic);
    assert_eq!(
        chunk.importers[0].specifiers[0].kind,
        ImportSpecifierKind::Named
    );
    assert_eq!(
        chunk.importers[0].specifiers[0].imported.as_deref(),
        Some("Named")
    );
}

#[test]
fn analyze_project_keeps_then_local_function_alias_call_live() {
    let project = TestProject::new("next-dynamic-then-local-function-alias");
    project.write(
        "src/consumer.tsx",
        r#"
import dynamic from "next/dynamic";

const consumer = dynamic(() =>
  import("./Chunk").then((module) => {
    function select() {
      return module.Named;
    }

    return { default: select() };
  })
);
"#,
    );
    project.write(
        "src/Chunk.tsx",
        r#"
export default function Chunk() {
  return null;
}

export const Named = true;
"#,
    );

    let report = analyze_project(project.root()).expect("project should analyze");

    assert_eq!(report.findings.dead_exports.len(), 1);
    assert_eq!(
        report.findings.dead_exports[0].file,
        project.root().join("src/Chunk.tsx")
    );
    assert_eq!(report.findings.dead_exports[0].export_name, "default");

    let chunk = report
        .modules
        .iter()
        .find(|module| module.relative_path == "src/Chunk.tsx")
        .expect("chunk module should exist");

    assert_eq!(chunk.imported_by_count, 1);
    assert_eq!(chunk.importers.len(), 1);
    assert_eq!(chunk.importers[0].kind, ImportKind::Dynamic);
    assert_eq!(
        chunk.importers[0].specifiers[0].kind,
        ImportSpecifierKind::Named
    );
    assert_eq!(
        chunk.importers[0].specifiers[0].imported.as_deref(),
        Some("Named")
    );
}

#[test]
fn analyze_project_keeps_then_nested_block_alias_named_export_live() {
    let project = TestProject::new("next-dynamic-then-nested-block-alias");
    project.write(
        "src/consumer.tsx",
        r#"
import dynamic from "next/dynamic";

const consumer = dynamic(() =>
  import("./Chunk").then((module) => {
    {
      const selected = module.Named;
      return { default: selected };
    }
  })
);
"#,
    );
    project.write(
        "src/Chunk.tsx",
        r#"
export default function Chunk() {
  return null;
}

export const Named = true;
"#,
    );

    let report = analyze_project(project.root()).expect("project should analyze");

    assert_eq!(report.findings.dead_exports.len(), 1);
    assert_eq!(
        report.findings.dead_exports[0].file,
        project.root().join("src/Chunk.tsx")
    );
    assert_eq!(report.findings.dead_exports[0].export_name, "default");

    let chunk = report
        .modules
        .iter()
        .find(|module| module.relative_path == "src/Chunk.tsx")
        .expect("chunk module should exist");

    assert_eq!(chunk.imported_by_count, 1);
    assert_eq!(chunk.importers.len(), 1);
    assert_eq!(chunk.importers[0].kind, ImportKind::Dynamic);
    assert_eq!(
        chunk.importers[0].specifiers[0].kind,
        ImportSpecifierKind::Named
    );
    assert_eq!(
        chunk.importers[0].specifiers[0].imported.as_deref(),
        Some("Named")
    );
}

#[test]
fn analyze_project_keeps_next_dynamic_alias_named_export_live() {
    let project = TestProject::new("next-dynamic-alias");
    project.write(
        "src/consumer.tsx",
        r#"
import dynamic from "next/dynamic";

const load = dynamic;
const consumer = load(() =>
  import("./Chunk").then(({ Named }) => ({ default: Named }))
);
"#,
    );
    project.write(
        "src/Chunk.tsx",
        r#"
export default function Chunk() {
  return null;
}

export const Named = true;
"#,
    );

    let report = analyze_project(project.root()).expect("project should analyze");

    assert_eq!(report.findings.dead_exports.len(), 1);
    assert_eq!(
        report.findings.dead_exports[0].file,
        project.root().join("src/Chunk.tsx")
    );
    assert_eq!(report.findings.dead_exports[0].export_name, "default");

    let chunk = report
        .modules
        .iter()
        .find(|module| module.relative_path == "src/Chunk.tsx")
        .expect("chunk module should exist");

    assert_eq!(chunk.imported_by_count, 1);
    assert_eq!(chunk.importers.len(), 1);
    assert_eq!(chunk.importers[0].kind, ImportKind::Dynamic);
    assert_eq!(
        chunk.importers[0].specifiers[0].kind,
        ImportSpecifierKind::Named
    );
    assert_eq!(
        chunk.importers[0].specifiers[0].imported.as_deref(),
        Some("Named")
    );
}

#[test]
fn analyze_project_keeps_commonjs_next_dynamic_named_export_live() {
    let project = TestProject::new("next-dynamic-commonjs");
    project.write(
        "src/consumer.js",
        r#"
const dynamic = require("next/dynamic");

const consumer = dynamic(() =>
  import("./Chunk").then(({ Named }) => ({ default: Named }))
);
"#,
    );
    project.write(
        "src/Chunk.js",
        r#"
export default function Chunk() {
  return null;
}

export const Named = true;
"#,
    );

    let report = analyze_project(project.root()).expect("project should analyze");

    assert_eq!(report.findings.dead_exports.len(), 1);
    assert_eq!(
        report.findings.dead_exports[0].file,
        project.root().join("src/Chunk.js")
    );
    assert_eq!(report.findings.dead_exports[0].export_name, "default");

    let chunk = report
        .modules
        .iter()
        .find(|module| module.relative_path == "src/Chunk.js")
        .expect("chunk module should exist");

    assert_eq!(chunk.imported_by_count, 1);
    assert_eq!(chunk.importers.len(), 1);
    assert_eq!(chunk.importers[0].kind, ImportKind::Dynamic);
    assert_eq!(
        chunk.importers[0].specifiers[0].kind,
        ImportSpecifierKind::Named
    );
    assert_eq!(
        chunk.importers[0].specifiers[0].imported.as_deref(),
        Some("Named")
    );
}

#[test]
fn analyze_project_does_not_treat_direct_commonjs_next_dynamic_call_as_live_usage() {
    let project = TestProject::new("next-dynamic-commonjs-direct-call");
    project.write(
        "src/consumer.js",
        r#"
const consumer = require("next/dynamic")(() => import("./Chunk"));
"#,
    );
    project.write(
        "src/Chunk.js",
        r#"
export default function Chunk() {
  return null;
}
"#,
    );

    let report = analyze_project(project.root()).expect("project should analyze");

    assert_eq!(report.findings.dead_exports.len(), 1);
    assert_eq!(
        report.findings.dead_exports[0].file,
        project.root().join("src/Chunk.js")
    );
    assert_eq!(report.findings.dead_exports[0].export_name, "default");

    let chunk = report
        .modules
        .iter()
        .find(|module| module.relative_path == "src/Chunk.js")
        .expect("chunk module should exist");

    assert_eq!(chunk.imported_by_count, 1);
    assert_eq!(chunk.importers.len(), 1);
    assert_eq!(chunk.importers[0].kind, ImportKind::Dynamic);
    assert!(chunk.importers[0].specifiers.is_empty());
}

#[test]
fn analyze_project_keeps_ts_import_equals_next_dynamic_named_export_live() {
    let project = TestProject::new("next-dynamic-ts-import-equals");
    project.write(
        "src/consumer.tsx",
        r#"
import dynamic = require("next/dynamic");

const consumer = dynamic(() =>
  import("./Chunk").then(({ Named }) => ({ default: Named }))
);
"#,
    );
    project.write(
        "src/Chunk.tsx",
        r#"
export default function Chunk() {
  return null;
}

export const Named = true;
"#,
    );

    let report = analyze_project(project.root()).expect("project should analyze");

    assert_eq!(report.findings.dead_exports.len(), 1);
    assert_eq!(
        report.findings.dead_exports[0].file,
        project.root().join("src/Chunk.tsx")
    );
    assert_eq!(report.findings.dead_exports[0].export_name, "default");

    let chunk = report
        .modules
        .iter()
        .find(|module| module.relative_path == "src/Chunk.tsx")
        .expect("chunk module should exist");

    assert_eq!(chunk.imported_by_count, 1);
    assert_eq!(chunk.importers.len(), 1);
    assert_eq!(chunk.importers[0].kind, ImportKind::Dynamic);
    assert_eq!(
        chunk.importers[0].specifiers[0].kind,
        ImportSpecifierKind::Named
    );
    assert_eq!(
        chunk.importers[0].specifiers[0].imported.as_deref(),
        Some("Named")
    );
}

#[test]
fn analyze_project_keeps_nested_commonjs_next_dynamic_named_export_live() {
    let project = TestProject::new("next-dynamic-commonjs-nested");
    project.write(
        "src/consumer.js",
        r#"
function build() {
  const dynamic = require("next/dynamic");
  return dynamic(() =>
    import("./Chunk").then(({ Named }) => ({ default: Named }))
  );
}

const consumer = build();
"#,
    );
    project.write(
        "src/Chunk.js",
        r#"
export default function Chunk() {
  return null;
}

export const Named = true;
"#,
    );

    let report = analyze_project(project.root()).expect("project should analyze");

    assert_eq!(report.findings.dead_exports.len(), 1);
    assert_eq!(
        report.findings.dead_exports[0].file,
        project.root().join("src/Chunk.js")
    );
    assert_eq!(report.findings.dead_exports[0].export_name, "default");

    let chunk = report
        .modules
        .iter()
        .find(|module| module.relative_path == "src/Chunk.js")
        .expect("chunk module should exist");

    assert_eq!(chunk.imported_by_count, 1);
    assert_eq!(chunk.importers.len(), 1);
    assert_eq!(chunk.importers[0].kind, ImportKind::Dynamic);
    assert_eq!(
        chunk.importers[0].specifiers[0].kind,
        ImportSpecifierKind::Named
    );
    assert_eq!(
        chunk.importers[0].specifiers[0].imported.as_deref(),
        Some("Named")
    );
}

#[test]
fn analyze_project_keeps_direct_commonjs_next_dynamic_default_named_export_live() {
    let project = TestProject::new("next-dynamic-commonjs-default-call");
    project.write(
        "src/consumer.js",
        r#"
const consumer = require("next/dynamic").default(() =>
  import("./Chunk").then(({ Named }) => ({ default: Named }))
);
"#,
    );
    project.write(
        "src/Chunk.js",
        r#"
export default function Chunk() {
  return null;
}

export const Named = true;
"#,
    );

    let report = analyze_project(project.root()).expect("project should analyze");

    assert_eq!(report.findings.dead_exports.len(), 1);
    assert_eq!(
        report.findings.dead_exports[0].file,
        project.root().join("src/Chunk.js")
    );
    assert_eq!(report.findings.dead_exports[0].export_name, "default");

    let chunk = report
        .modules
        .iter()
        .find(|module| module.relative_path == "src/Chunk.js")
        .expect("chunk module should exist");

    assert_eq!(chunk.imported_by_count, 1);
    assert_eq!(chunk.importers.len(), 1);
    assert_eq!(chunk.importers[0].kind, ImportKind::Dynamic);
    assert_eq!(
        chunk.importers[0].specifiers[0].kind,
        ImportSpecifierKind::Named
    );
    assert_eq!(
        chunk.importers[0].specifiers[0].imported.as_deref(),
        Some("Named")
    );
}

#[test]
fn analyze_project_keeps_destructured_then_named_export_live() {
    let project = TestProject::new("next-dynamic-destructured-then");
    project.write(
        "src/consumer.tsx",
        r#"
import dynamic from "next/dynamic";

const consumer = dynamic(() =>
  import("./Chunk").then(({ Named }) => ({ default: Named }))
);
"#,
    );
    project.write(
        "src/Chunk.tsx",
        r#"
export default function Chunk() {
  return null;
}

export const Named = true;
"#,
    );

    let report = analyze_project(project.root()).expect("project should analyze");

    assert_eq!(report.findings.dead_exports.len(), 1);
    assert_eq!(
        report.findings.dead_exports[0].file,
        project.root().join("src/Chunk.tsx")
    );
    assert_eq!(report.findings.dead_exports[0].export_name, "default");

    let chunk = report
        .modules
        .iter()
        .find(|module| module.relative_path == "src/Chunk.tsx")
        .expect("chunk module should exist");

    assert_eq!(chunk.imported_by_count, 1);
    assert_eq!(chunk.importers.len(), 1);
    assert_eq!(chunk.importers[0].kind, ImportKind::Dynamic);
    assert_eq!(
        chunk.importers[0].specifiers[0].kind,
        ImportSpecifierKind::Named
    );
    assert_eq!(
        chunk.importers[0].specifiers[0].imported.as_deref(),
        Some("Named")
    );
}

#[test]
fn analyze_project_keeps_then_named_export_live_with_reject_handler() {
    let project = TestProject::new("next-dynamic-then-reject");
    project.write(
        "src/consumer.tsx",
        r#"
import dynamic from "next/dynamic";

const consumer = dynamic(() =>
  import("./Chunk").then(
    ({ Named }) => ({ default: Named }),
    () => fallback
  )
);
"#,
    );
    project.write(
        "src/Chunk.tsx",
        r#"
export default function Chunk() {
  return null;
}

export const Named = true;
"#,
    );

    let report = analyze_project(project.root()).expect("project should analyze");

    assert_eq!(report.findings.dead_exports.len(), 1);
    assert_eq!(
        report.findings.dead_exports[0].file,
        project.root().join("src/Chunk.tsx")
    );
    assert_eq!(report.findings.dead_exports[0].export_name, "default");

    let chunk = report
        .modules
        .iter()
        .find(|module| module.relative_path == "src/Chunk.tsx")
        .expect("chunk module should exist");

    assert_eq!(chunk.imported_by_count, 1);
    assert_eq!(chunk.importers.len(), 1);
    assert_eq!(chunk.importers[0].kind, ImportKind::Dynamic);
    assert_eq!(
        chunk.importers[0].specifiers[0].kind,
        ImportSpecifierKind::Named
    );
    assert_eq!(
        chunk.importers[0].specifiers[0].imported.as_deref(),
        Some("Named")
    );
}

#[test]
fn analyze_project_falls_back_when_wrapper_name_is_shadowed() {
    let project = TestProject::new("dynamic-shadowed-wrapper");
    project.write(
        "src/consumer.tsx",
        r#"
import { lazy } from "react";

function build(lazy: unknown) {
  return lazy(() => import("./Chunk"));
}

const consumer = build((loader: unknown) => loader);
"#,
    );
    project.write(
        "src/Chunk.tsx",
        r#"
export default function Chunk() {
  return null;
}
"#,
    );

    let report = analyze_project(project.root()).expect("project should analyze");

    assert_eq!(report.findings.dead_exports.len(), 1);
    assert_eq!(
        report.findings.dead_exports[0].file,
        project.root().join("src/Chunk.tsx")
    );
    assert_eq!(report.findings.dead_exports[0].export_name, "default");

    let chunk = report
        .modules
        .iter()
        .find(|module| module.relative_path == "src/Chunk.tsx")
        .expect("chunk module should exist");

    assert_eq!(chunk.imported_by_count, 1);
    assert_eq!(chunk.importers.len(), 1);
    assert_eq!(chunk.importers[0].kind, ImportKind::Dynamic);
    assert!(chunk.importers[0].specifiers.is_empty());
}

#[test]
fn analyze_project_falls_back_when_wrapper_name_is_hoisted_shadowed() {
    let project = TestProject::new("dynamic-hoisted-shadow");
    project.write(
        "src/consumer.tsx",
        r#"
import React from "react";

function build() {
  const consumer = React.lazy(() => import("./Chunk"));
  var React = { lazy(loader: unknown) { return loader; } };
  return consumer;
}

const app = build();
"#,
    );
    project.write(
        "src/Chunk.tsx",
        r#"
export default function Chunk() {
  return null;
}
"#,
    );

    let report = analyze_project(project.root()).expect("project should analyze");

    assert_eq!(report.findings.dead_exports.len(), 1);
    assert_eq!(
        report.findings.dead_exports[0].file,
        project.root().join("src/Chunk.tsx")
    );
    assert_eq!(report.findings.dead_exports[0].export_name, "default");

    let chunk = report
        .modules
        .iter()
        .find(|module| module.relative_path == "src/Chunk.tsx")
        .expect("chunk module should exist");

    assert_eq!(chunk.imported_by_count, 1);
    assert_eq!(chunk.importers.len(), 1);
    assert_eq!(chunk.importers[0].kind, ImportKind::Dynamic);
    assert!(chunk.importers[0].specifiers.is_empty());
}

#[test]
fn analyze_project_falls_back_for_forward_referenced_commonjs_react_wrapper() {
    let project = TestProject::new("react-lazy-commonjs-forward-reference");
    project.write(
        "src/consumer.js",
        r#"
React.lazy(() => import("./Chunk"));
const React = require("react");
"#,
    );
    project.write(
        "src/Chunk.js",
        r#"
export default function Chunk() {
  return null;
}
"#,
    );

    let report = analyze_project(project.root()).expect("project should analyze");

    assert_eq!(report.findings.dead_exports.len(), 1);
    assert_eq!(
        report.findings.dead_exports[0].file,
        project.root().join("src/Chunk.js")
    );
    assert_eq!(report.findings.dead_exports[0].export_name, "default");

    let chunk = report
        .modules
        .iter()
        .find(|module| module.relative_path == "src/Chunk.js")
        .expect("chunk module should exist");

    assert_eq!(chunk.imported_by_count, 1);
    assert_eq!(chunk.importers.len(), 1);
    assert_eq!(chunk.importers[0].kind, ImportKind::Dynamic);
    assert!(chunk.importers[0].specifiers.is_empty());
}

#[test]
fn analyze_project_falls_back_for_forward_referenced_commonjs_next_dynamic_wrapper() {
    let project = TestProject::new("next-dynamic-commonjs-forward-reference");
    project.write(
        "src/consumer.js",
        r#"
dynamic(() => import("./Chunk"));
const dynamic = require("next/dynamic");
"#,
    );
    project.write(
        "src/Chunk.js",
        r#"
export default function Chunk() {
  return null;
}
"#,
    );

    let report = analyze_project(project.root()).expect("project should analyze");

    assert_eq!(report.findings.dead_exports.len(), 1);
    assert_eq!(
        report.findings.dead_exports[0].file,
        project.root().join("src/Chunk.js")
    );
    assert_eq!(report.findings.dead_exports[0].export_name, "default");

    let chunk = report
        .modules
        .iter()
        .find(|module| module.relative_path == "src/Chunk.js")
        .expect("chunk module should exist");

    assert_eq!(chunk.imported_by_count, 1);
    assert_eq!(chunk.importers.len(), 1);
    assert_eq!(chunk.importers[0].kind, ImportKind::Dynamic);
    assert!(chunk.importers[0].specifiers.is_empty());
}

#[test]
fn analyze_project_does_not_treat_next_dynamic_namespace_import_as_live_usage() {
    let project = TestProject::new("next-dynamic-namespace-import");
    project.write(
        "src/consumer.tsx",
        r#"
import * as dynamic from "next/dynamic";

const consumer = dynamic(() => import("./Chunk"));
"#,
    );
    project.write(
        "src/Chunk.tsx",
        r#"
export default function Chunk() {
  return null;
}
"#,
    );

    let report = analyze_project(project.root()).expect("project should analyze");

    assert_eq!(report.findings.dead_exports.len(), 1);
    assert_eq!(
        report.findings.dead_exports[0].file,
        project.root().join("src/Chunk.tsx")
    );
    assert_eq!(report.findings.dead_exports[0].export_name, "default");

    let chunk = report
        .modules
        .iter()
        .find(|module| module.relative_path == "src/Chunk.tsx")
        .expect("chunk module should exist");

    assert_eq!(chunk.imported_by_count, 1);
    assert_eq!(chunk.importers.len(), 1);
    assert_eq!(chunk.importers[0].kind, ImportKind::Dynamic);
    assert!(chunk.importers[0].specifiers.is_empty());
}

#[test]
fn analyze_project_does_not_treat_top_level_shadow_of_nested_wrapper_alias_as_live_usage() {
    let project = TestProject::new("next-dynamic-top-level-shadow");
    project.write(
        "src/consumer.js",
        r#"
function seed() {
  const dynamic = require("next/dynamic");
  return dynamic;
}

const dynamic = (loader) => loader;
const consumer = dynamic(() => import("./Chunk"));
"#,
    );
    project.write(
        "src/Chunk.js",
        r#"
export default function Chunk() {
  return null;
}
"#,
    );

    let report = analyze_project(project.root()).expect("project should analyze");

    assert_eq!(report.findings.dead_exports.len(), 1);
    assert_eq!(
        report.findings.dead_exports[0].file,
        project.root().join("src/Chunk.js")
    );
    assert_eq!(report.findings.dead_exports[0].export_name, "default");

    let chunk = report
        .modules
        .iter()
        .find(|module| module.relative_path == "src/Chunk.js")
        .expect("chunk module should exist");

    assert_eq!(chunk.imported_by_count, 1);
    assert_eq!(chunk.importers.len(), 1);
    assert_eq!(chunk.importers[0].kind, ImportKind::Dynamic);
    assert!(chunk.importers[0].specifiers.is_empty());
}

#[test]
fn analyze_project_does_not_treat_shadowed_react_lazy_alias_as_live_usage() {
    let project = TestProject::new("react-lazy-alias-shadowed");
    project.write(
        "src/consumer.tsx",
        r#"
import React from "react";

const load = React.lazy;

function build() {
  const load = (loader: unknown) => loader;
  return load(() => import("./Chunk"));
}

const consumer = build();
"#,
    );
    project.write(
        "src/Chunk.tsx",
        r#"
export default function Chunk() {
  return null;
}
"#,
    );

    let report = analyze_project(project.root()).expect("project should analyze");

    assert_eq!(report.findings.dead_exports.len(), 1);
    assert_eq!(
        report.findings.dead_exports[0].file,
        project.root().join("src/Chunk.tsx")
    );
    assert_eq!(report.findings.dead_exports[0].export_name, "default");

    let chunk = report
        .modules
        .iter()
        .find(|module| module.relative_path == "src/Chunk.tsx")
        .expect("chunk module should exist");

    assert_eq!(chunk.imported_by_count, 1);
    assert_eq!(chunk.importers.len(), 1);
    assert_eq!(chunk.importers[0].kind, ImportKind::Dynamic);
    assert!(chunk.importers[0].specifiers.is_empty());
}

#[test]
fn analyze_project_does_not_treat_hoisted_react_lazy_alias_as_live_usage() {
    let project = TestProject::new("react-lazy-alias-hoisted");
    project.write(
        "src/consumer.tsx",
        r#"
import React from "react";

const load = React.lazy;

function build() {
  const consumer = load(() => import("./Chunk"));
  var load = (loader: unknown) => loader;
  return consumer;
}

const app = build();
"#,
    );
    project.write(
        "src/Chunk.tsx",
        r#"
export default function Chunk() {
  return null;
}
"#,
    );

    let report = analyze_project(project.root()).expect("project should analyze");

    assert_eq!(report.findings.dead_exports.len(), 1);
    assert_eq!(
        report.findings.dead_exports[0].file,
        project.root().join("src/Chunk.tsx")
    );
    assert_eq!(report.findings.dead_exports[0].export_name, "default");

    let chunk = report
        .modules
        .iter()
        .find(|module| module.relative_path == "src/Chunk.tsx")
        .expect("chunk module should exist");

    assert_eq!(chunk.imported_by_count, 1);
    assert_eq!(chunk.importers.len(), 1);
    assert_eq!(chunk.importers[0].kind, ImportKind::Dynamic);
    assert!(chunk.importers[0].specifiers.is_empty());
}

#[test]
fn analyze_project_does_not_treat_shadowed_require_react_alias_as_live_usage() {
    let project = TestProject::new("react-lazy-shadowed-require");
    project.write(
        "src/consumer.tsx",
        r#"
function build(require: unknown) {
  const React = require("react");
  const load = React.lazy;
  return load(() => import("./Chunk"));
}

const consumer = build((specifier: string) => specifier);
"#,
    );
    project.write(
        "src/Chunk.tsx",
        r#"
export default function Chunk() {
  return null;
}
"#,
    );

    let report = analyze_project(project.root()).expect("project should analyze");

    assert_eq!(report.findings.dead_exports.len(), 1);
    assert_eq!(
        report.findings.dead_exports[0].file,
        project.root().join("src/Chunk.tsx")
    );
    assert_eq!(report.findings.dead_exports[0].export_name, "default");

    let chunk = report
        .modules
        .iter()
        .find(|module| module.relative_path == "src/Chunk.tsx")
        .expect("chunk module should exist");

    assert_eq!(chunk.imported_by_count, 1);
    assert_eq!(chunk.importers.len(), 1);
    assert_eq!(chunk.importers[0].kind, ImportKind::Dynamic);
    assert!(chunk.importers[0].specifiers.is_empty());
}

#[test]
fn analyze_project_does_not_treat_shadowed_require_next_dynamic_default_as_live_usage() {
    let project = TestProject::new("next-dynamic-shadowed-require-default");
    project.write(
        "src/consumer.js",
        r#"
function build(require) {
  return require("next/dynamic").default(() => import("./Chunk"));
}

const consumer = build((specifier) => specifier);
"#,
    );
    project.write(
        "src/Chunk.js",
        r#"
export default function Chunk() {
  return null;
}
"#,
    );

    let report = analyze_project(project.root()).expect("project should analyze");

    assert_eq!(report.findings.dead_exports.len(), 1);
    assert_eq!(
        report.findings.dead_exports[0].file,
        project.root().join("src/Chunk.js")
    );
    assert_eq!(report.findings.dead_exports[0].export_name, "default");

    let chunk = report
        .modules
        .iter()
        .find(|module| module.relative_path == "src/Chunk.js")
        .expect("chunk module should exist");

    assert_eq!(chunk.imported_by_count, 1);
    assert_eq!(chunk.importers.len(), 1);
    assert_eq!(chunk.importers[0].kind, ImportKind::Dynamic);
    assert!(chunk.importers[0].specifiers.is_empty());
}

#[test]
fn analyze_project_falls_back_for_complex_dynamic_callbacks() {
    let project = TestProject::new("dynamic-fallback");
    project.write(
        "src/consumer.tsx",
        r#"
import React from "react";

const consumer = React.lazy(() =>
  condition ? import("./Chunk") : fallback("./Chunk")
);
"#,
    );
    project.write(
        "src/Chunk.tsx",
        r#"
export default function Chunk() {
  return null;
}
"#,
    );

    let report = analyze_project(project.root()).expect("project should analyze");

    assert_eq!(report.findings.dead_exports.len(), 1);
    assert_eq!(
        report.findings.dead_exports[0].file,
        project.root().join("src/Chunk.tsx")
    );
    assert_eq!(report.findings.dead_exports[0].export_name, "default");

    let chunk = report
        .modules
        .iter()
        .find(|module| module.relative_path == "src/Chunk.tsx")
        .expect("chunk module should exist");

    assert_eq!(chunk.imported_by_count, 1);
    assert_eq!(chunk.importers.len(), 1);
    assert_eq!(chunk.importers[0].kind, ImportKind::Dynamic);
    assert!(chunk.importers[0].specifiers.is_empty());
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
        let root = std::env::temp_dir().join(format!("kratos-dynamic-{prefix}-{unique}"));
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
