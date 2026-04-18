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
