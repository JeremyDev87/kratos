use std::path::Path;

use kratos_core::model::{ExportKind, ImportKind, ImportSpecifierKind};
use kratos_core::parser::parse_module_source;

#[test]
fn parser_extracts_namespace_reexports() {
    let parsed = parse_module_source(Path::new("index.ts"), "export * as ns from './lib.ts';\n")
        .expect("parser should succeed");

    assert_eq!(parsed.imports.len(), 1);
    assert_eq!(parsed.imports[0].kind, ImportKind::ReexportNamespace);
    assert_eq!(parsed.imports[0].source, "./lib.ts");
    assert_eq!(
        parsed.imports[0].specifiers[0].kind,
        ImportSpecifierKind::Namespace
    );
    assert_eq!(parsed.imports[0].specifiers[0].local.as_deref(), Some("ns"));

    assert_eq!(
        parsed.exports,
        vec![kratos_core::model::ExportRecord {
            name: "ns".to_string(),
            kind: ExportKind::ReexportNamespace,
        }]
    );
}

#[test]
fn parser_falls_back_to_empty_module_on_syntax_errors() {
    let parsed = parse_module_source(
        Path::new("index.ts"),
        "import { foo } from './dep'\nexport default function (\n",
    )
    .expect("parser should degrade gracefully");

    assert!(parsed.imports.is_empty());
    assert!(parsed.exports.is_empty());
    assert!(parsed.unused_imports.is_empty());
}

#[test]
fn parser_uses_module_fallback_for_unknown_extensions() {
    let parsed = parse_module_source(
        Path::new("index.unknown"),
        "import foo from './dep';\nexport default foo;\n",
    )
    .expect("parser should succeed");

    assert_eq!(parsed.imports.len(), 1);
    assert_eq!(parsed.imports[0].kind, ImportKind::Static);
    assert_eq!(parsed.imports[0].source, "./dep");
    assert!(parsed
        .exports
        .iter()
        .any(|entry| entry.kind == ExportKind::Default && entry.name == "default"));
}

#[test]
fn parser_collects_typescript_specific_exports() {
    let parsed = parse_module_source(
        Path::new("index.ts"),
        r#"
export interface Foo {}
export type Bar = string;
export enum Baz { A }
export = value;
export as namespace GlobalFoo;
"#,
    )
    .expect("parser should succeed");

    assert!(parsed
        .exports
        .iter()
        .any(|entry| entry.kind == ExportKind::Named && entry.name == "Foo"));
    assert!(parsed
        .exports
        .iter()
        .any(|entry| entry.kind == ExportKind::Named && entry.name == "Bar"));
    assert!(parsed
        .exports
        .iter()
        .any(|entry| entry.kind == ExportKind::Named && entry.name == "Baz"));
    assert!(parsed
        .exports
        .iter()
        .any(|entry| entry.kind == ExportKind::Default && entry.name == "default"));
    assert!(parsed
        .exports
        .iter()
        .any(|entry| { entry.kind == ExportKind::ReexportNamespace && entry.name == "GlobalFoo" }));
}

#[test]
fn parser_collects_typescript_import_equals_requires() {
    let parsed = parse_module_source(
        Path::new("index.ts"),
        "import foo = require('./foo');\nexport default foo;\n",
    )
    .expect("parser should succeed");

    assert_eq!(parsed.imports.len(), 1);
    assert_eq!(parsed.imports[0].kind, ImportKind::Require);
    assert_eq!(parsed.imports[0].source, "./foo");
    assert_eq!(
        parsed.imports[0].specifiers[0].kind,
        ImportSpecifierKind::Unknown
    );
    assert_eq!(
        parsed.imports[0].specifiers[0].local.as_deref(),
        Some("foo")
    );
}

#[test]
fn parser_detects_template_interpolation_usage() {
    let parsed = parse_module_source(
        Path::new("index.ts"),
        "import { sum } from './math';\nconst message = `value: ${sum(1, 2)}`;\n",
    )
    .expect("parser should succeed");

    assert!(parsed.unused_imports.is_empty());
}

#[test]
fn parser_marks_shadowed_imports_as_unused() {
    let parsed = parse_module_source(
        Path::new("index.ts"),
        r#"
import { foo, bar } from "./lib";

function wrap(foo: number) {
  return foo + 1;
}

console.log(bar);
"#,
    )
    .expect("parser should succeed");

    assert_eq!(parsed.unused_imports.len(), 1);
    assert_eq!(parsed.unused_imports[0].local, "foo");
    assert_eq!(parsed.unused_imports[0].imported, "foo");
}

#[test]
fn parser_treats_typescript_type_positions_as_usage() {
    let parsed = parse_module_source(
        Path::new("index.ts"),
        r#"
import { Foo } from "./types";

const value: Foo = {} as Foo;
class Service implements Foo {}
const checked = value satisfies Foo;
type Copy = typeof Foo;
"#,
    )
    .expect("parser should succeed");

    assert!(parsed.unused_imports.is_empty());
}

#[test]
fn parser_extracts_destructured_require_bindings() {
    let parsed = parse_module_source(
        Path::new("index.js"),
        "const { helper } = require('./helper');\nmodule.exports = { run: () => helper() };\n",
    )
    .expect("parser should succeed");

    assert_eq!(parsed.imports.len(), 1);
    assert_eq!(parsed.imports[0].kind, ImportKind::Require);
    assert_eq!(parsed.imports[0].source, "./helper");
    assert_eq!(
        parsed.imports[0].specifiers[0].kind,
        ImportSpecifierKind::Named
    );
    assert_eq!(
        parsed.imports[0].specifiers[0].imported.as_deref(),
        Some("helper")
    );
    assert_eq!(
        parsed.imports[0].specifiers[0].local.as_deref(),
        Some("helper")
    );
}

#[test]
fn parser_treats_plain_require_bindings_conservatively() {
    let parsed = parse_module_source(
        Path::new("index.js"),
        "const mod = require('./mod');\nmodule.exports = mod;\n",
    )
    .expect("parser should succeed");

    assert_eq!(parsed.imports.len(), 1);
    assert_eq!(parsed.imports[0].kind, ImportKind::Require);
    assert_eq!(parsed.imports[0].source, "./mod");
    assert_eq!(
        parsed.imports[0].specifiers[0].kind,
        ImportSpecifierKind::Unknown
    );
    assert_eq!(parsed.imports[0].specifiers[0].imported, None);
    assert_eq!(
        parsed.imports[0].specifiers[0].local.as_deref(),
        Some("mod")
    );
    assert!(parsed.unused_imports.is_empty());
}

#[test]
fn parser_collects_side_effect_and_delegating_commonjs_requires() {
    let side_effect = parse_module_source(Path::new("index.js"), "require('./setup');\n")
        .expect("parser should succeed");
    let delegated = parse_module_source(
        Path::new("index.js"),
        "module.exports = require('./setup');\n",
    )
    .expect("parser should succeed");

    assert_eq!(side_effect.imports.len(), 1);
    assert_eq!(side_effect.imports[0].kind, ImportKind::Require);
    assert_eq!(side_effect.imports[0].source, "./setup");
    assert!(side_effect.imports[0].specifiers.is_empty());

    assert_eq!(delegated.imports.len(), 1);
    assert_eq!(delegated.imports[0].kind, ImportKind::Require);
    assert_eq!(delegated.imports[0].source, "./setup");
    assert!(delegated.imports[0].specifiers.is_empty());
}

#[test]
fn parser_collects_member_access_commonjs_requires() {
    let parsed = parse_module_source(
        Path::new("index.js"),
        "const helper = require('./dep').helper;\nmodule.exports = helper;\n",
    )
    .expect("parser should succeed");

    assert_eq!(parsed.imports.len(), 1);
    assert_eq!(parsed.imports[0].kind, ImportKind::Require);
    assert_eq!(parsed.imports[0].source, "./dep");
}

#[test]
fn parser_uses_conservative_fallback_for_nested_require_destructuring() {
    let parsed = parse_module_source(
        Path::new("index.js"),
        "const { helper: { nested } } = require('./helper');\nmodule.exports = { run: () => nested() };\n",
    )
    .expect("parser should succeed");

    assert_eq!(parsed.imports.len(), 1);
    assert_eq!(parsed.imports[0].kind, ImportKind::Unknown);
    assert_eq!(parsed.imports[0].source, "./helper");
}

#[test]
fn parser_handles_ternary_defaults_inside_require_destructuring() {
    let parsed = parse_module_source(
        Path::new("index.js"),
        "const { a = cond ? fallback() : other() } = require('./helper');\nmodule.exports = { run: () => a };\n",
    )
    .expect("parser should succeed");

    assert_eq!(parsed.imports.len(), 1);
    assert_eq!(parsed.imports[0].kind, ImportKind::Require);
    assert_eq!(
        parsed.imports[0].specifiers[0].imported.as_deref(),
        Some("a")
    );
    assert_eq!(parsed.imports[0].specifiers[0].local.as_deref(), Some("a"));
    assert!(parsed.unused_imports.is_empty());
}

#[test]
fn parser_collects_static_dynamic_and_commonjs_exports() {
    let parsed = parse_module_source(
        Path::new("index.tsx"),
        r#"
import thing, { value as renamed } from "./lib";
import "./setup";
export { renamed };
export const answer = 42;
export default function Main() { return <div>{thing}</div>; }
const dynamic = import("./chunk");
exports.named = answer;
module.exports = Main;
"#,
    )
    .expect("parser should succeed");

    assert!(parsed
        .imports
        .iter()
        .any(|entry| entry.kind == ImportKind::Static && entry.source == "./lib"));
    assert!(parsed
        .imports
        .iter()
        .any(|entry| entry.kind == ImportKind::SideEffect && entry.source == "./setup"));
    assert!(parsed
        .imports
        .iter()
        .any(|entry| entry.kind == ImportKind::Dynamic && entry.source == "./chunk"));
    assert!(parsed
        .exports
        .iter()
        .any(|entry| entry.kind == ExportKind::Named && entry.name == "answer"));
    assert!(parsed
        .exports
        .iter()
        .any(|entry| entry.kind == ExportKind::Named && entry.name == "renamed"));
    assert!(parsed
        .exports
        .iter()
        .any(|entry| entry.kind == ExportKind::Default && entry.name == "default"));
    assert_eq!(
        parsed
            .exports
            .iter()
            .filter(|entry| entry.kind == ExportKind::Default && entry.name == "default")
            .count(),
        1
    );
}

#[test]
fn parser_marks_lazy_default_usage_for_react_lazy() {
    let parsed = parse_module_source(
        Path::new("index.tsx"),
        r#"
import React from "react";

const LazyChunk = React.lazy(() => import("./Chunk"));
"#,
    )
    .expect("parser should succeed");

    dbg!(&parsed.imports);

    let dynamic_import = parsed
        .imports
        .iter()
        .find(|entry| entry.source == "./Chunk")
        .expect("dynamic import should be recorded");

    assert_eq!(dynamic_import.kind, ImportKind::Dynamic);
    assert_eq!(dynamic_import.specifiers.len(), 1);
    assert_eq!(
        dynamic_import.specifiers[0].kind,
        ImportSpecifierKind::Default
    );
    assert_eq!(
        dynamic_import.specifiers[0].imported.as_deref(),
        Some("default")
    );
    assert_eq!(dynamic_import.specifiers[0].local, None);
}

#[test]
fn parser_marks_lazy_default_usage_for_named_react_lazy_import() {
    let parsed = parse_module_source(
        Path::new("index.tsx"),
        r#"
import { lazy } from "react";

const LazyChunk = lazy(() => import("./Chunk"));
"#,
    )
    .expect("parser should succeed");

    let dynamic_import = parsed
        .imports
        .iter()
        .find(|entry| entry.source == "./Chunk")
        .expect("dynamic import should be recorded");

    assert_eq!(dynamic_import.kind, ImportKind::Dynamic);
    assert_eq!(dynamic_import.specifiers.len(), 1);
    assert_eq!(
        dynamic_import.specifiers[0].kind,
        ImportSpecifierKind::Default
    );
    assert_eq!(
        dynamic_import.specifiers[0].imported.as_deref(),
        Some("default")
    );
    assert_eq!(dynamic_import.specifiers[0].local, None);
}

#[test]
fn parser_marks_lazy_default_usage_for_commonjs_react_wrapper() {
    let parsed = parse_module_source(
        Path::new("index.js"),
        r#"
const React = require("react");

const LazyChunk = React.lazy(() => import("./Chunk"));
"#,
    )
    .expect("parser should succeed");

    let dynamic_import = parsed
        .imports
        .iter()
        .find(|entry| entry.source == "./Chunk")
        .expect("dynamic import should be recorded");

    assert_eq!(dynamic_import.kind, ImportKind::Dynamic);
    assert_eq!(dynamic_import.specifiers.len(), 1);
    assert_eq!(
        dynamic_import.specifiers[0].kind,
        ImportSpecifierKind::Default
    );
    assert_eq!(
        dynamic_import.specifiers[0].imported.as_deref(),
        Some("default")
    );
    assert_eq!(dynamic_import.specifiers[0].local, None);
}

#[test]
fn parser_marks_lazy_default_usage_for_nested_block_var_commonjs_react_wrapper() {
    let parsed = parse_module_source(
        Path::new("index.js"),
        r#"
function build() {
  if (enabled) {
    var React = require("react");
    return React.lazy(() => import("./Chunk"));
  }
}
"#,
    )
    .expect("parser should succeed");

    let dynamic_import = parsed
        .imports
        .iter()
        .find(|entry| entry.source == "./Chunk")
        .expect("dynamic import should be recorded");

    assert_eq!(dynamic_import.kind, ImportKind::Dynamic);
    assert_eq!(dynamic_import.specifiers.len(), 1);
    assert_eq!(
        dynamic_import.specifiers[0].kind,
        ImportSpecifierKind::Default
    );
    assert_eq!(
        dynamic_import.specifiers[0].imported.as_deref(),
        Some("default")
    );
    assert_eq!(dynamic_import.specifiers[0].local, None);
}

#[test]
fn parser_marks_lazy_default_usage_for_ts_import_equals_react_wrapper() {
    let parsed = parse_module_source(
        Path::new("index.tsx"),
        r#"
import React = require("react");

const LazyChunk = React.lazy(() => import("./Chunk"));
"#,
    )
    .expect("parser should succeed");

    let dynamic_import = parsed
        .imports
        .iter()
        .find(|entry| entry.source == "./Chunk")
        .expect("dynamic import should be recorded");

    assert_eq!(dynamic_import.kind, ImportKind::Dynamic);
    assert_eq!(dynamic_import.specifiers.len(), 1);
    assert_eq!(
        dynamic_import.specifiers[0].kind,
        ImportSpecifierKind::Default
    );
    assert_eq!(
        dynamic_import.specifiers[0].imported.as_deref(),
        Some("default")
    );
    assert_eq!(dynamic_import.specifiers[0].local, None);
}

#[test]
fn parser_marks_lazy_default_usage_for_react_lazy_alias() {
    let parsed = parse_module_source(
        Path::new("index.tsx"),
        r#"
import React from "react";

const load = React.lazy;
const LazyChunk = load(() => import("./Chunk"));
"#,
    )
    .expect("parser should succeed");

    let dynamic_import = parsed
        .imports
        .iter()
        .find(|entry| entry.source == "./Chunk")
        .expect("dynamic import should be recorded");

    assert_eq!(dynamic_import.kind, ImportKind::Dynamic);
    assert_eq!(dynamic_import.specifiers.len(), 1);
    assert_eq!(
        dynamic_import.specifiers[0].kind,
        ImportSpecifierKind::Default
    );
    assert_eq!(
        dynamic_import.specifiers[0].imported.as_deref(),
        Some("default")
    );
    assert_eq!(dynamic_import.specifiers[0].local, None);
}

#[test]
fn parser_marks_lazy_default_usage_for_react_lazy_loader_identifier() {
    let parsed = parse_module_source(
        Path::new("index.tsx"),
        r#"
import React from "react";

const loader = () => import("./Chunk");
const LazyChunk = React.lazy(loader);
"#,
    )
    .expect("parser should succeed");

    let dynamic_import = parsed
        .imports
        .iter()
        .find(|entry| entry.source == "./Chunk")
        .expect("dynamic import should be recorded");

    assert_eq!(dynamic_import.kind, ImportKind::Dynamic);
    assert_eq!(dynamic_import.specifiers.len(), 1);
    assert_eq!(
        dynamic_import.specifiers[0].kind,
        ImportSpecifierKind::Default
    );
    assert_eq!(
        dynamic_import.specifiers[0].imported.as_deref(),
        Some("default")
    );
    assert_eq!(dynamic_import.specifiers[0].local, None);
}

#[test]
fn parser_marks_lazy_default_usage_for_hoisted_function_loader_identifier() {
    let parsed = parse_module_source(
        Path::new("index.tsx"),
        r#"
import React from "react";

const LazyChunk = React.lazy(loader);

function loader() {
  return import("./Chunk");
}
"#,
    )
    .expect("parser should succeed");

    let dynamic_import = parsed
        .imports
        .iter()
        .find(|entry| entry.source == "./Chunk")
        .expect("dynamic import should be recorded");

    assert_eq!(dynamic_import.kind, ImportKind::Dynamic);
    assert_eq!(dynamic_import.specifiers.len(), 1);
    assert_eq!(
        dynamic_import.specifiers[0].kind,
        ImportSpecifierKind::Default
    );
    assert_eq!(
        dynamic_import.specifiers[0].imported.as_deref(),
        Some("default")
    );
    assert_eq!(dynamic_import.specifiers[0].local, None);
}

#[test]
fn parser_marks_lazy_default_usage_for_react_lazy_loader_alias_identifier() {
    let parsed = parse_module_source(
        Path::new("index.tsx"),
        r#"
import React from "react";

const loader = () => import("./Chunk");
const alias = loader;
const LazyChunk = React.lazy(alias);
"#,
    )
    .expect("parser should succeed");

    let dynamic_import = parsed
        .imports
        .iter()
        .find(|entry| entry.source == "./Chunk")
        .expect("dynamic import should be recorded");

    assert_eq!(dynamic_import.kind, ImportKind::Dynamic);
    assert_eq!(dynamic_import.specifiers.len(), 1);
    assert_eq!(
        dynamic_import.specifiers[0].kind,
        ImportSpecifierKind::Default
    );
    assert_eq!(
        dynamic_import.specifiers[0].imported.as_deref(),
        Some("default")
    );
    assert_eq!(dynamic_import.specifiers[0].local, None);
}

#[test]
fn parser_marks_lazy_default_usage_for_nested_loader_alias_after_non_loader_shadow() {
    let parsed = parse_module_source(
        Path::new("index.tsx"),
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

const LazyChunk = build();
"#,
    )
    .expect("parser should succeed");

    let dynamic_import = parsed
        .imports
        .iter()
        .find(|entry| entry.source == "./InnerChunk")
        .expect("inner dynamic import should be recorded");

    assert_eq!(dynamic_import.kind, ImportKind::Dynamic);
    assert_eq!(dynamic_import.specifiers.len(), 1);
    assert_eq!(
        dynamic_import.specifiers[0].kind,
        ImportSpecifierKind::Default
    );
    assert_eq!(
        dynamic_import.specifiers[0].imported.as_deref(),
        Some("default")
    );
    assert_eq!(dynamic_import.specifiers[0].local, None);
}

#[test]
fn parser_marks_named_usage_for_next_dynamic_then_selection() {
    let parsed = parse_module_source(
        Path::new("index.tsx"),
        r#"
import dynamic from "next/dynamic";

const LazyChunk = dynamic(() =>
  import("./Chunk").then((module) => ({ default: module.Named }))
);
"#,
    )
    .expect("parser should succeed");

    let dynamic_import = parsed
        .imports
        .iter()
        .find(|entry| entry.source == "./Chunk")
        .expect("dynamic import should be recorded");

    assert_eq!(dynamic_import.kind, ImportKind::Dynamic);
    assert_eq!(dynamic_import.specifiers.len(), 1);
    assert_eq!(
        dynamic_import.specifiers[0].kind,
        ImportSpecifierKind::Named
    );
    assert_eq!(
        dynamic_import.specifiers[0].imported.as_deref(),
        Some("Named")
    );
    assert_eq!(dynamic_import.specifiers[0].local, None);
}

#[test]
fn parser_marks_named_usage_for_then_block_alias_selection() {
    let parsed = parse_module_source(
        Path::new("index.tsx"),
        r#"
import dynamic from "next/dynamic";

const LazyChunk = dynamic(() =>
  import("./Chunk").then((module) => {
    const selected = module.Named;
    return { default: selected };
  })
);
"#,
    )
    .expect("parser should succeed");

    let dynamic_import = parsed
        .imports
        .iter()
        .find(|entry| entry.source == "./Chunk")
        .expect("dynamic import should be recorded");

    assert_eq!(dynamic_import.kind, ImportKind::Dynamic);
    assert_eq!(dynamic_import.specifiers.len(), 1);
    assert_eq!(
        dynamic_import.specifiers[0].kind,
        ImportSpecifierKind::Named
    );
    assert_eq!(
        dynamic_import.specifiers[0].imported.as_deref(),
        Some("Named")
    );
    assert_eq!(dynamic_import.specifiers[0].local, None);
}

#[test]
fn parser_marks_named_usage_for_then_named_callback_identifier() {
    let parsed = parse_module_source(
        Path::new("index.tsx"),
        r#"
import dynamic from "next/dynamic";

const select = (module) => ({ default: module.Named });
const LazyChunk = dynamic(() =>
  import("./Chunk").then(select)
);
"#,
    )
    .expect("parser should succeed");

    let dynamic_import = parsed
        .imports
        .iter()
        .find(|entry| entry.source == "./Chunk")
        .expect("dynamic import should be recorded");

    assert_eq!(dynamic_import.kind, ImportKind::Dynamic);
    assert_eq!(dynamic_import.specifiers.len(), 1);
    assert_eq!(
        dynamic_import.specifiers[0].kind,
        ImportSpecifierKind::Named
    );
    assert_eq!(
        dynamic_import.specifiers[0].imported.as_deref(),
        Some("Named")
    );
    assert_eq!(dynamic_import.specifiers[0].local, None);
}

#[test]
fn parser_does_not_treat_hoisted_var_then_callback_alias_as_named_usage() {
    let parsed = parse_module_source(
        Path::new("index.tsx"),
        r#"
import dynamic from "next/dynamic";

const LazyChunk = dynamic(() =>
  import("./Chunk").then(select)
);

var select = (module) => ({ default: module.Named });
"#,
    )
    .expect("parser should succeed");

    let dynamic_import = parsed
        .imports
        .iter()
        .find(|entry| entry.source == "./Chunk")
        .expect("dynamic import should be recorded");

    assert_eq!(dynamic_import.kind, ImportKind::Dynamic);
    assert!(dynamic_import.specifiers.is_empty());
}

#[test]
fn parser_marks_named_usage_for_hoisted_then_function_callback() {
    let parsed = parse_module_source(
        Path::new("index.tsx"),
        r#"
import dynamic from "next/dynamic";

const LazyChunk = dynamic(() =>
  import("./Chunk").then(select)
);

function select(module) {
  return { default: module.Named };
}
"#,
    )
    .expect("parser should succeed");

    let dynamic_import = parsed
        .imports
        .iter()
        .find(|entry| entry.source == "./Chunk")
        .expect("dynamic import should be recorded");

    assert_eq!(dynamic_import.kind, ImportKind::Dynamic);
    assert_eq!(dynamic_import.specifiers.len(), 1);
    assert_eq!(
        dynamic_import.specifiers[0].kind,
        ImportSpecifierKind::Named
    );
    assert_eq!(
        dynamic_import.specifiers[0].imported.as_deref(),
        Some("Named")
    );
    assert_eq!(dynamic_import.specifiers[0].local, None);
}

#[test]
fn parser_marks_named_usage_for_then_nested_block_followed_by_outer_return() {
    let parsed = parse_module_source(
        Path::new("index.tsx"),
        r#"
import dynamic from "next/dynamic";

const LazyChunk = dynamic(() =>
  import("./Chunk").then((module) => {
    {
      const selected = module.Named;
    }

    return { default: module.Named };
  })
);
"#,
    )
    .expect("parser should succeed");

    let dynamic_import = parsed
        .imports
        .iter()
        .find(|entry| entry.source == "./Chunk")
        .expect("dynamic import should be recorded");

    assert_eq!(dynamic_import.kind, ImportKind::Dynamic);
    assert_eq!(dynamic_import.specifiers.len(), 1);
    assert_eq!(
        dynamic_import.specifiers[0].kind,
        ImportSpecifierKind::Named
    );
    assert_eq!(
        dynamic_import.specifiers[0].imported.as_deref(),
        Some("Named")
    );
    assert_eq!(dynamic_import.specifiers[0].local, None);
}

#[test]
fn parser_marks_named_usage_for_then_alias_selection_with_unrelated_local() {
    let parsed = parse_module_source(
        Path::new("index.tsx"),
        r#"
import dynamic from "next/dynamic";

const LazyChunk = dynamic(() =>
  import("./Chunk").then((module) => {
    const selected = module.Named;
    const noop = 1;
    return { default: selected };
  })
);
"#,
    )
    .expect("parser should succeed");

    let dynamic_import = parsed
        .imports
        .iter()
        .find(|entry| entry.source == "./Chunk")
        .expect("dynamic import should be recorded");

    assert_eq!(dynamic_import.kind, ImportKind::Dynamic);
    assert_eq!(dynamic_import.specifiers.len(), 1);
    assert_eq!(
        dynamic_import.specifiers[0].kind,
        ImportSpecifierKind::Named
    );
    assert_eq!(
        dynamic_import.specifiers[0].imported.as_deref(),
        Some("Named")
    );
    assert_eq!(dynamic_import.specifiers[0].local, None);
}

#[test]
fn parser_marks_named_usage_for_then_destructured_local_alias() {
    let parsed = parse_module_source(
        Path::new("index.tsx"),
        r#"
import dynamic from "next/dynamic";

const LazyChunk = dynamic(() =>
  import("./Chunk").then((module) => {
    const { Named: selected } = module;
    return { default: selected };
  })
);
"#,
    )
    .expect("parser should succeed");

    let dynamic_import = parsed
        .imports
        .iter()
        .find(|entry| entry.source == "./Chunk")
        .expect("dynamic import should be recorded");

    assert_eq!(dynamic_import.kind, ImportKind::Dynamic);
    assert_eq!(dynamic_import.specifiers.len(), 1);
    assert_eq!(
        dynamic_import.specifiers[0].kind,
        ImportSpecifierKind::Named
    );
    assert_eq!(
        dynamic_import.specifiers[0].imported.as_deref(),
        Some("Named")
    );
    assert_eq!(dynamic_import.specifiers[0].local, None);
}

#[test]
fn parser_marks_named_usage_for_then_local_function_alias_call() {
    let parsed = parse_module_source(
        Path::new("index.tsx"),
        r#"
import dynamic from "next/dynamic";

const LazyChunk = dynamic(() =>
  import("./Chunk").then((module) => {
    function select() {
      return module.Named;
    }

    return { default: select() };
  })
);
"#,
    )
    .expect("parser should succeed");

    let dynamic_import = parsed
        .imports
        .iter()
        .find(|entry| entry.source == "./Chunk")
        .expect("dynamic import should be recorded");

    assert_eq!(dynamic_import.kind, ImportKind::Dynamic);
    assert_eq!(dynamic_import.specifiers.len(), 1);
    assert_eq!(
        dynamic_import.specifiers[0].kind,
        ImportSpecifierKind::Named
    );
    assert_eq!(
        dynamic_import.specifiers[0].imported.as_deref(),
        Some("Named")
    );
    assert_eq!(dynamic_import.specifiers[0].local, None);
}

#[test]
fn parser_marks_named_usage_for_then_nested_block_alias_selection() {
    let parsed = parse_module_source(
        Path::new("index.tsx"),
        r#"
import dynamic from "next/dynamic";

const LazyChunk = dynamic(() =>
  import("./Chunk").then((module) => {
    {
      const selected = module.Named;
      return { default: selected };
    }
  })
);
"#,
    )
    .expect("parser should succeed");

    let dynamic_import = parsed
        .imports
        .iter()
        .find(|entry| entry.source == "./Chunk")
        .expect("dynamic import should be recorded");

    assert_eq!(dynamic_import.kind, ImportKind::Dynamic);
    assert_eq!(dynamic_import.specifiers.len(), 1);
    assert_eq!(
        dynamic_import.specifiers[0].kind,
        ImportSpecifierKind::Named
    );
    assert_eq!(
        dynamic_import.specifiers[0].imported.as_deref(),
        Some("Named")
    );
    assert_eq!(dynamic_import.specifiers[0].local, None);
}

#[test]
fn parser_marks_named_usage_for_next_dynamic_loader_alias_identifier() {
    let parsed = parse_module_source(
        Path::new("index.tsx"),
        r#"
import dynamic from "next/dynamic";

const loader = () =>
  import("./Chunk").then(({ Named }) => ({ default: Named }));
const alias = loader;
const LazyChunk = dynamic(alias);
"#,
    )
    .expect("parser should succeed");

    let dynamic_import = parsed
        .imports
        .iter()
        .find(|entry| entry.source == "./Chunk")
        .expect("dynamic import should be recorded");

    assert_eq!(dynamic_import.kind, ImportKind::Dynamic);
    assert_eq!(dynamic_import.specifiers.len(), 1);
    assert_eq!(
        dynamic_import.specifiers[0].kind,
        ImportSpecifierKind::Named
    );
    assert_eq!(
        dynamic_import.specifiers[0].imported.as_deref(),
        Some("Named")
    );
    assert_eq!(dynamic_import.specifiers[0].local, None);
}

#[test]
fn parser_marks_named_usage_for_next_dynamic_alias() {
    let parsed = parse_module_source(
        Path::new("index.tsx"),
        r#"
import dynamic from "next/dynamic";

const load = dynamic;
const LazyChunk = load(() =>
  import("./Chunk").then(({ Named }) => ({ default: Named }))
);
"#,
    )
    .expect("parser should succeed");

    let dynamic_import = parsed
        .imports
        .iter()
        .find(|entry| entry.source == "./Chunk")
        .expect("dynamic import should be recorded");

    assert_eq!(dynamic_import.kind, ImportKind::Dynamic);
    assert_eq!(dynamic_import.specifiers.len(), 1);
    assert_eq!(
        dynamic_import.specifiers[0].kind,
        ImportSpecifierKind::Named
    );
    assert_eq!(
        dynamic_import.specifiers[0].imported.as_deref(),
        Some("Named")
    );
    assert_eq!(dynamic_import.specifiers[0].local, None);
}

#[test]
fn parser_marks_named_usage_for_commonjs_next_dynamic_wrapper() {
    let parsed = parse_module_source(
        Path::new("index.js"),
        r#"
const dynamic = require("next/dynamic");

const LazyChunk = dynamic(() =>
  import("./Chunk").then(({ Named }) => ({ default: Named }))
);
"#,
    )
    .expect("parser should succeed");

    let dynamic_import = parsed
        .imports
        .iter()
        .find(|entry| entry.source == "./Chunk")
        .expect("dynamic import should be recorded");

    assert_eq!(dynamic_import.kind, ImportKind::Dynamic);
    assert_eq!(dynamic_import.specifiers.len(), 1);
    assert_eq!(
        dynamic_import.specifiers[0].kind,
        ImportSpecifierKind::Named
    );
    assert_eq!(
        dynamic_import.specifiers[0].imported.as_deref(),
        Some("Named")
    );
    assert_eq!(dynamic_import.specifiers[0].local, None);
}

#[test]
fn parser_does_not_treat_direct_commonjs_next_dynamic_call_as_wrapper() {
    let parsed = parse_module_source(
        Path::new("index.js"),
        r#"
const LazyChunk = require("next/dynamic")(() => import("./Chunk"));
"#,
    )
    .expect("parser should succeed");

    let dynamic_import = parsed
        .imports
        .iter()
        .find(|entry| entry.source == "./Chunk")
        .expect("dynamic import should be recorded");

    assert_eq!(dynamic_import.kind, ImportKind::Dynamic);
    assert!(dynamic_import.specifiers.is_empty());
}

#[test]
fn parser_marks_named_usage_for_ts_import_equals_next_dynamic_wrapper() {
    let parsed = parse_module_source(
        Path::new("index.tsx"),
        r#"
import dynamic = require("next/dynamic");

const LazyChunk = dynamic(() =>
  import("./Chunk").then(({ Named }) => ({ default: Named }))
);
"#,
    )
    .expect("parser should succeed");

    let dynamic_import = parsed
        .imports
        .iter()
        .find(|entry| entry.source == "./Chunk")
        .expect("dynamic import should be recorded");

    assert_eq!(dynamic_import.kind, ImportKind::Dynamic);
    assert_eq!(dynamic_import.specifiers.len(), 1);
    assert_eq!(
        dynamic_import.specifiers[0].kind,
        ImportSpecifierKind::Named
    );
    assert_eq!(
        dynamic_import.specifiers[0].imported.as_deref(),
        Some("Named")
    );
    assert_eq!(dynamic_import.specifiers[0].local, None);
}

#[test]
fn parser_marks_named_usage_for_nested_commonjs_next_dynamic_wrapper() {
    let parsed = parse_module_source(
        Path::new("index.js"),
        r#"
function build() {
  const dynamic = require("next/dynamic");
  return dynamic(() =>
    import("./Chunk").then(({ Named }) => ({ default: Named }))
  );
}
"#,
    )
    .expect("parser should succeed");

    let dynamic_import = parsed
        .imports
        .iter()
        .find(|entry| entry.source == "./Chunk")
        .expect("dynamic import should be recorded");

    assert_eq!(dynamic_import.kind, ImportKind::Dynamic);
    assert_eq!(dynamic_import.specifiers.len(), 1);
    assert_eq!(
        dynamic_import.specifiers[0].kind,
        ImportSpecifierKind::Named
    );
    assert_eq!(
        dynamic_import.specifiers[0].imported.as_deref(),
        Some("Named")
    );
    assert_eq!(dynamic_import.specifiers[0].local, None);
}

#[test]
fn parser_marks_named_usage_for_direct_commonjs_next_dynamic_default_wrapper() {
    let parsed = parse_module_source(
        Path::new("index.js"),
        r#"
const LazyChunk = require("next/dynamic").default(() =>
  import("./Chunk").then(({ Named }) => ({ default: Named }))
);
"#,
    )
    .expect("parser should succeed");

    let dynamic_import = parsed
        .imports
        .iter()
        .find(|entry| entry.source == "./Chunk")
        .expect("dynamic import should be recorded");

    assert_eq!(dynamic_import.kind, ImportKind::Dynamic);
    assert_eq!(dynamic_import.specifiers.len(), 1);
    assert_eq!(
        dynamic_import.specifiers[0].kind,
        ImportSpecifierKind::Named
    );
    assert_eq!(
        dynamic_import.specifiers[0].imported.as_deref(),
        Some("Named")
    );
    assert_eq!(dynamic_import.specifiers[0].local, None);
}

#[test]
fn parser_marks_named_usage_for_destructured_then_selection() {
    let parsed = parse_module_source(
        Path::new("index.tsx"),
        r#"
import dynamic from "next/dynamic";

const LazyChunk = dynamic(() =>
  import("./Chunk").then(({ Named }) => ({ default: Named }))
);
"#,
    )
    .expect("parser should succeed");

    let dynamic_import = parsed
        .imports
        .iter()
        .find(|entry| entry.source == "./Chunk")
        .expect("dynamic import should be recorded");

    assert_eq!(dynamic_import.kind, ImportKind::Dynamic);
    assert_eq!(dynamic_import.specifiers.len(), 1);
    assert_eq!(
        dynamic_import.specifiers[0].kind,
        ImportSpecifierKind::Named
    );
    assert_eq!(
        dynamic_import.specifiers[0].imported.as_deref(),
        Some("Named")
    );
    assert_eq!(dynamic_import.specifiers[0].local, None);
}

#[test]
fn parser_marks_named_usage_for_then_selection_with_reject_handler() {
    let parsed = parse_module_source(
        Path::new("index.tsx"),
        r#"
import dynamic from "next/dynamic";

const LazyChunk = dynamic(() =>
  import("./Chunk").then(
    ({ Named }) => ({ default: Named }),
    () => fallback
  )
);
"#,
    )
    .expect("parser should succeed");

    let dynamic_import = parsed
        .imports
        .iter()
        .find(|entry| entry.source == "./Chunk")
        .expect("dynamic import should be recorded");

    assert_eq!(dynamic_import.kind, ImportKind::Dynamic);
    assert_eq!(dynamic_import.specifiers.len(), 1);
    assert_eq!(
        dynamic_import.specifiers[0].kind,
        ImportSpecifierKind::Named
    );
    assert_eq!(
        dynamic_import.specifiers[0].imported.as_deref(),
        Some("Named")
    );
    assert_eq!(dynamic_import.specifiers[0].local, None);
}

#[test]
fn parser_does_not_treat_shadowed_wrapper_names_as_dynamic_usage() {
    let parsed = parse_module_source(
        Path::new("index.tsx"),
        r#"
import { lazy } from "react";
import dynamic from "next/dynamic";

function build(lazy: unknown, dynamic: unknown) {
  const A = lazy(() => import("./LazyChunk"));
  const B = dynamic(() => import("./DynamicChunk"));
  return { A, B };
}
"#,
    )
    .expect("parser should succeed");

    let lazy_import = parsed
        .imports
        .iter()
        .find(|entry| entry.source == "./LazyChunk")
        .expect("lazy import should be recorded");
    assert_eq!(lazy_import.kind, ImportKind::Dynamic);
    assert!(lazy_import.specifiers.is_empty());

    let dynamic_import = parsed
        .imports
        .iter()
        .find(|entry| entry.source == "./DynamicChunk")
        .expect("dynamic import should be recorded");
    assert_eq!(dynamic_import.kind, ImportKind::Dynamic);
    assert!(dynamic_import.specifiers.is_empty());
}

#[test]
fn parser_does_not_treat_hoisted_wrapper_names_as_dynamic_usage() {
    let parsed = parse_module_source(
        Path::new("index.tsx"),
        r#"
import React from "react";

function build() {
  const LazyChunk = React.lazy(() => import("./Chunk"));
  var React = { lazy(loader: unknown) { return loader; } };
  return LazyChunk;
}
"#,
    )
    .expect("parser should succeed");

    let dynamic_import = parsed
        .imports
        .iter()
        .find(|entry| entry.source == "./Chunk")
        .expect("dynamic import should be recorded");

    assert_eq!(dynamic_import.kind, ImportKind::Dynamic);
    assert!(dynamic_import.specifiers.is_empty());
}

#[test]
fn parser_does_not_treat_forward_referenced_commonjs_react_wrapper_as_dynamic_usage() {
    let parsed = parse_module_source(
        Path::new("index.js"),
        r#"
React.lazy(() => import("./Chunk"));
const React = require("react");
"#,
    )
    .expect("parser should succeed");

    let dynamic_import = parsed
        .imports
        .iter()
        .find(|entry| entry.source == "./Chunk")
        .expect("dynamic import should be recorded");

    assert_eq!(dynamic_import.kind, ImportKind::Dynamic);
    assert!(dynamic_import.specifiers.is_empty());
}

#[test]
fn parser_does_not_treat_forward_referenced_commonjs_next_dynamic_wrapper_as_dynamic_usage() {
    let parsed = parse_module_source(
        Path::new("index.js"),
        r#"
dynamic(() => import("./Chunk"));
const dynamic = require("next/dynamic");
"#,
    )
    .expect("parser should succeed");

    let dynamic_import = parsed
        .imports
        .iter()
        .find(|entry| entry.source == "./Chunk")
        .expect("dynamic import should be recorded");

    assert_eq!(dynamic_import.kind, ImportKind::Dynamic);
    assert!(dynamic_import.specifiers.is_empty());
}

#[test]
fn parser_does_not_treat_next_dynamic_namespace_import_as_wrapper() {
    let parsed = parse_module_source(
        Path::new("index.tsx"),
        r#"
import * as dynamic from "next/dynamic";

const LazyChunk = dynamic(() => import("./Chunk"));
"#,
    )
    .expect("parser should succeed");

    let dynamic_import = parsed
        .imports
        .iter()
        .find(|entry| entry.source == "./Chunk")
        .expect("dynamic import should be recorded");

    assert_eq!(dynamic_import.kind, ImportKind::Dynamic);
    assert!(dynamic_import.specifiers.is_empty());
}

#[test]
fn parser_does_not_treat_top_level_shadow_of_nested_wrapper_alias_as_dynamic_usage() {
    let parsed = parse_module_source(
        Path::new("index.js"),
        r#"
function seed() {
  const dynamic = require("next/dynamic");
  return dynamic;
}

const dynamic = (loader) => loader;
const LazyChunk = dynamic(() => import("./Chunk"));
"#,
    )
    .expect("parser should succeed");

    let dynamic_import = parsed
        .imports
        .iter()
        .find(|entry| entry.source == "./Chunk")
        .expect("dynamic import should be recorded");

    assert_eq!(dynamic_import.kind, ImportKind::Dynamic);
    assert!(dynamic_import.specifiers.is_empty());
}

#[test]
fn parser_does_not_treat_shadowed_react_lazy_alias_as_dynamic_usage() {
    let parsed = parse_module_source(
        Path::new("index.tsx"),
        r#"
import React from "react";

const load = React.lazy;

function build() {
  const load = (loader: unknown) => loader;
  return load(() => import("./Chunk"));
}
"#,
    )
    .expect("parser should succeed");

    let dynamic_import = parsed
        .imports
        .iter()
        .find(|entry| entry.source == "./Chunk")
        .expect("dynamic import should be recorded");

    assert_eq!(dynamic_import.kind, ImportKind::Dynamic);
    assert!(dynamic_import.specifiers.is_empty());
}

#[test]
fn parser_does_not_treat_hoisted_react_lazy_alias_as_dynamic_usage() {
    let parsed = parse_module_source(
        Path::new("index.tsx"),
        r#"
import React from "react";

const load = React.lazy;

function build() {
  const LazyChunk = load(() => import("./Chunk"));
  var load = (loader: unknown) => loader;
  return LazyChunk;
}
"#,
    )
    .expect("parser should succeed");

    let dynamic_import = parsed
        .imports
        .iter()
        .find(|entry| entry.source == "./Chunk")
        .expect("dynamic import should be recorded");

    assert_eq!(dynamic_import.kind, ImportKind::Dynamic);
    assert!(dynamic_import.specifiers.is_empty());
}

#[test]
fn parser_does_not_treat_shadowed_require_react_alias_as_dynamic_usage() {
    let parsed = parse_module_source(
        Path::new("index.tsx"),
        r#"
function build(require: unknown) {
  const React = require("react");
  const load = React.lazy;
  return load(() => import("./Chunk"));
}
"#,
    )
    .expect("parser should succeed");

    let dynamic_import = parsed
        .imports
        .iter()
        .find(|entry| entry.source == "./Chunk")
        .expect("dynamic import should be recorded");

    assert_eq!(dynamic_import.kind, ImportKind::Dynamic);
    assert!(dynamic_import.specifiers.is_empty());
}

#[test]
fn parser_does_not_treat_shadowed_require_next_dynamic_default_as_wrapper() {
    let parsed = parse_module_source(
        Path::new("index.js"),
        r#"
function build(require) {
  return require("next/dynamic").default(() => import("./Chunk"));
}
"#,
    )
    .expect("parser should succeed");

    let dynamic_import = parsed
        .imports
        .iter()
        .find(|entry| entry.source == "./Chunk")
        .expect("dynamic import should be recorded");

    assert_eq!(dynamic_import.kind, ImportKind::Dynamic);
    assert!(dynamic_import.specifiers.is_empty());
}

#[test]
fn parser_falls_back_for_complex_dynamic_callbacks() {
    let parsed = parse_module_source(
        Path::new("index.tsx"),
        r#"
import React from "react";

const LazyChunk = React.lazy(() =>
  condition ? import("./Chunk") : fallback("./Chunk")
);
"#,
    )
    .expect("parser should succeed");

    let dynamic_import = parsed
        .imports
        .iter()
        .find(|entry| entry.source == "./Chunk")
        .expect("dynamic import should be recorded");

    assert_eq!(dynamic_import.kind, ImportKind::Dynamic);
    assert!(dynamic_import.specifiers.is_empty());
}

#[test]
fn parser_keeps_named_default_declarations_as_default_only() {
    let parsed = parse_module_source(
        Path::new("index.tsx"),
        "export default function Main() { return null; }\n",
    )
    .expect("parser should succeed");

    assert_eq!(
        parsed.exports,
        vec![kratos_core::model::ExportRecord {
            name: "default".to_string(),
            kind: ExportKind::Default,
        }]
    );
}

#[test]
fn parser_ignores_compound_commonjs_assignment_mutations() {
    let parsed = parse_module_source(
        Path::new("index.js"),
        "exports.count += 1;\nmodule.exports ||= factory();\n",
    )
    .expect("parser should succeed");

    assert!(parsed.exports.is_empty());
}

#[test]
fn parser_collects_module_exports_named_members() {
    let parsed = parse_module_source(
        Path::new("index.js"),
        "module.exports.run = run;\nmodule.exports.value = 1;\n",
    )
    .expect("parser should succeed");

    assert!(parsed
        .exports
        .iter()
        .any(|entry| entry.kind == ExportKind::Named && entry.name == "run"));
    assert!(parsed
        .exports
        .iter()
        .any(|entry| entry.kind == ExportKind::Named && entry.name == "value"));
}

#[test]
fn parser_collects_computed_commonjs_named_members() {
    let parsed = parse_module_source(
        Path::new("index.js"),
        "exports['named'] = 1;\nmodule.exports['other'] = 2;\n",
    )
    .expect("parser should succeed");

    assert!(parsed
        .exports
        .iter()
        .any(|entry| entry.kind == ExportKind::Named && entry.name == "named"));
    assert!(parsed
        .exports
        .iter()
        .any(|entry| entry.kind == ExportKind::Named && entry.name == "other"));
}

#[test]
fn parser_collects_module_exports_object_literal_members() {
    let parsed = parse_module_source(
        Path::new("index.js"),
        "module.exports = { run, value: 1, execute() {}, ['skip']: 2, ...rest };\n",
    )
    .expect("parser should succeed");

    assert!(parsed
        .exports
        .iter()
        .any(|entry| entry.kind == ExportKind::Default && entry.name == "default"));
    assert!(parsed
        .exports
        .iter()
        .any(|entry| entry.kind == ExportKind::Named && entry.name == "run"));
    assert!(parsed
        .exports
        .iter()
        .any(|entry| entry.kind == ExportKind::Named && entry.name == "value"));
    assert!(parsed
        .exports
        .iter()
        .any(|entry| entry.kind == ExportKind::Named && entry.name == "execute"));
    assert!(!parsed
        .exports
        .iter()
        .any(|entry| entry.kind == ExportKind::Named && entry.name == "skip"));
}

#[test]
fn parser_treats_reexports_consistently_for_unused_imports() {
    let same_name = parse_module_source(Path::new("index.ts"), "export { foo } from './lib';\n")
        .expect("parser should succeed");
    let renamed = parse_module_source(
        Path::new("index.ts"),
        "export { foo as bar } from './lib';\n",
    )
    .expect("parser should succeed");

    assert!(same_name.unused_imports.is_empty());
    assert!(renamed.unused_imports.is_empty());
}

#[test]
fn parser_collects_destructured_export_declarations() {
    let object_pattern =
        parse_module_source(Path::new("index.ts"), "export const { foo } = source;\n")
            .expect("parser should succeed");
    let array_pattern =
        parse_module_source(Path::new("index.ts"), "export const [foo] = source;\n")
            .expect("parser should succeed");

    assert!(object_pattern
        .exports
        .iter()
        .any(|entry| entry.kind == ExportKind::Named && entry.name == "foo"));
    assert!(array_pattern
        .exports
        .iter()
        .any(|entry| entry.kind == ExportKind::Named && entry.name == "foo"));
}

#[test]
fn parser_does_not_treat_intrinsic_jsx_tags_as_import_usage() {
    let parsed = parse_module_source(
        Path::new("index.tsx"),
        r#"
import { div } from "./dep";

export default function App() {
  return <div />;
}
"#,
    )
    .expect("parser should succeed");

    assert_eq!(parsed.unused_imports.len(), 1);
    assert_eq!(parsed.unused_imports[0].local, "div");
    assert_eq!(parsed.unused_imports[0].imported, "div");
}
