# Kratos

[![CI](https://github.com/JeremyDev87/kratos/actions/workflows/ci.yml/badge.svg)](https://github.com/JeremyDev87/kratos/actions/workflows/ci.yml)

[한국어](README.md) | English | [中文](README.zh-CN.md) | [Español](README.es.md) | [日本語](README.ja.md)

Destroy dead code ruthlessly.

[License](LICENSE) · [Contributing](CONTRIBUTING.md) · [Code of Conduct](CODE_OF_CONDUCT.md) · [Security](SECURITY.md) · [Sponsor](https://github.com/sponsors/JeremyDev87)

Kratos is a CLI tool for JavaScript and TypeScript projects. It finds unused files, broken imports, unused exports, and orphaned modules, then writes the results to a report. The current implementation combines a Rust core/CLI with an npm launcher, and the npm package `@jeremyfellaz/kratos` loads an optional platform-specific native add-on.

Kratos is an analysis tool for a safe cleanup workflow, not an automatic deletion bot. `clean` is dry-run by default, and files are only removed after you review the report and explicitly pass `--apply`.

## Core Capabilities

- Detect unused files and orphaned component/module candidates
- Detect broken internal imports
- Detect unused export and import candidates
- Apply Next.js `app/` / `pages/` route entrypoint heuristics
- Resolve `tsconfig.json` / `jsconfig.json` `baseUrl` and `paths` aliases
- Resolve `package.json` `main`, `module`, `types`, `bin`, and `exports` entrypoints
- Protect local script entrypoints executed by `package.json` scripts and GitHub Actions workflow/composite action `run` commands
- Protect manual verification scripts shaped like `scripts/verify-*`, `scripts/*smoke*`, and `scripts/*validation*`
- Exclude `*.test.*`, `*.spec.*`, and `src/__tests__/**` test files inside the scanned source tree from deletion candidates while preserving their import edges
- Conservatively handle Next.js framework-consumed exports, recognized dynamic import wrappers, and pure re-export barrels
- Print saved reports as summary, JSON, or Markdown
- Compare finding changes between two reports
- Preview safe deletion candidates with a confidence threshold

## Quick Start

For package users, the default entrypoint is `npx`.

```bash
npx @jeremyfellaz/kratos scan ./my-app
npx @jeremyfellaz/kratos report ./my-app
npx @jeremyfellaz/kratos report ./my-app --format md
npx @jeremyfellaz/kratos clean ./my-app --min-confidence 0.9
```

Only add `--apply` after reviewing the report and deciding to delete the listed targets.

```bash
npx @jeremyfellaz/kratos clean ./my-app --apply --min-confidence 0.9
```

You can also compare reports from two points in time.

```bash
npx @jeremyfellaz/kratos scan ./my-app --output .kratos/before.json
# clean up code or switch branches
npx @jeremyfellaz/kratos scan ./my-app --output .kratos/after.json
npx @jeremyfellaz/kratos diff ./my-app/.kratos/before.json ./my-app/.kratos/after.json
```

When `scan --output` receives a relative path, it is resolved from the scanned root. The default saved report is `<root>/.kratos/latest-report.json`.

## Commands

### `kratos scan [root] [--output path] [--json]`

Analyzes a project and writes a report JSON file.

- Omit `root` to scan the current working directory.
- `--output path` sets the report output path.
- `--json` prints the full report JSON to stdout instead of the console summary.
- The default output path is `<root>/.kratos/latest-report.json`.

### `kratos report [report-path-or-root] [--format summary|json|md]`

Prints a saved report in a human-readable format or as raw JSON.

- `summary` is the default console summary.
- `json` pretty-prints the saved report JSON.
- `md` prints a Markdown report that is easy to share.
- If the input is a project root, Kratos resolves `.kratos/latest-report.json` automatically.

### `kratos diff [before-report-path-or-root] [after-report-path-or-root] [--format summary|json|md]`

Compares finding changes between two reports.

- The default format is `summary`.
- `json` prints introduced/resolved/persisted findings in a machine-readable shape.
- `md` prints a Markdown diff suitable for reviews or issues.
- Each input can be either a report file path or a project root.

### `kratos clean [report-path-or-root] [--apply] [--min-confidence value]`

Previews deletion candidates or deletes them.

- Dry-run is the default behavior.
- Files are deleted only when `--apply` is present.
- `--min-confidence value` is a confidence threshold from `0.0` to `1.0`.
- If `--min-confidence` is omitted, Kratos reads `thresholds.cleanMinConfidence` from `kratos.config.json`; when no setting exists, it uses `0.0`.

## Output Examples

Scanning `fixtures/demo-app` produces a summary like this.

```text
Kratos scan complete.

Impact: 6 actionable findings: 1 broken import, 2 cleanup candidates, 3 dead exports.
Best next move: Fix broken imports before deleting files.

Root: <root>
Files scanned: 5
Entrypoints: 1
Broken imports: 1
Orphan files: 2
Dead exports: 3
Unused imports: 0
Route entrypoints: 1
Deletion candidates: 2

Saved report: <root>/.kratos/latest-report.json

Next steps:
- Preview cleanup: kratos clean <root>/.kratos/latest-report.json
- Shareable markdown: kratos report <root>/.kratos/latest-report.json --format md

Broken imports:
- src/lib/broken.ts -> ./missing-helper

Top cleanup candidates:
- src/components/DeadWidget.tsx (confidence 0.92, Component-like module has no inbound references.)
- src/lib/broken.ts (confidence 0.88, Module has no inbound references and is not treated as an entrypoint.)

Orphan files:
- src/components/DeadWidget.tsx
- src/lib/broken.ts

Dead exports:
- src/components/DeadWidget.tsx#DeadWidget
- src/lib/broken.ts#brokenFeature
- src/lib/math.ts#multiply
```

`clean --min-confidence 0.9` separates deletion targets from candidates skipped by the threshold.

```text
Kratos clean dry run.

Deletion targets: 1
- <root>/src/components/DeadWidget.tsx (confidence 0.92, Component-like module has no inbound references.)

Threshold-skipped targets: 1
- <root>/src/lib/broken.ts (confidence 0.88, Module has no inbound references and is not treated as an entrypoint.)

Re-run with --apply to delete these files.
```

Comparing identical reports shows no introduced or resolved findings, only persisted counts.

```text
Kratos diff complete.

Before: <before-report>
After: <after-report>

Broken imports: introduced 0, resolved 0, persisted 1
Orphan files: introduced 0, resolved 0, persisted 2
Dead exports: introduced 0, resolved 0, persisted 3
Unused imports: introduced 0, resolved 0, persisted 0
Route entrypoints: introduced 0, resolved 0, persisted 1
Deletion candidates: introduced 0, resolved 0, persisted 2

Totals: introduced 0, resolved 0, persisted 9
```

## Report Schema

`scan` currently writes `schemaVersion: 2` reports.

```json
{
  "schemaVersion": 2,
  "summary": {
    "filesScanned": 5,
    "entrypoints": 1,
    "brokenImports": 1,
    "orphanFiles": 2,
    "deadExports": 3,
    "unusedImports": 0,
    "routeEntrypoints": 1,
    "deletionCandidates": 2
  }
}
```

`findings` contains `brokenImports`, `orphanFiles`, `deadExports`, `unusedImports`, `routeEntrypoints`, and `deletionCandidates`. `graph.modules` records analyzed module paths, entrypoint status, and import/export counts.

## Configuration

You can place `kratos.config.json` in the project root. JSONC-style comments and trailing commas are accepted.

```json
{
  "ignore": ["storybook-static", "generated"],
  "ignorePatterns": ["src/generated/**", "!src/generated/keep.ts"],
  "keepPatterns": ["scripts/manual-*.mjs"],
  "entry": ["src/bootstrap.ts"],
  "roots": ["src", "app", "pages"],
  "thresholds": {
    "cleanMinConfidence": 0.85
  },
  "suppressions": [
    {
      "kind": "deadExport",
      "file": "src/components/LazyCard.tsx",
      "export": "default",
      "reason": "Loaded dynamically by route metadata."
    },
    {
      "kind": "brokenImport",
      "file": "src/legacy/shim.ts",
      "source": "./generated-shim",
      "reason": "Generated at deploy time."
    }
  ]
}
```

- `ignore`: directory names added to the default ignore list.
- `ignorePatterns`: `.gitignore`-style path patterns. Use `!` negation for exceptions.
- After the default ignored directories, Kratos also reads the project root `.gitignore` automatically, then applies `ignorePatterns` for exceptions or overrides.
- `keepPatterns`: `.gitignore`-style path patterns that keep matching files out of orphan/deletion candidates without excluding them from scan.
- `entry`: root-relative files forced to be entrypoints.
- `roots`: root-relative directories that limit scan scope.
- `thresholds.cleanMinConfidence`: default confidence threshold for `clean`.
- `suppressions`: findings that should be intentionally ignored. `kind` must be one of `brokenImport`, `orphanFile`, `deadExport`, `unusedImport`, or `deletionCandidate`.

If `.kratos/suppressions.json` exists, Kratos reads it with the same suppression format. `file` values must be relative to the project root.

## Local Development

Requirements:

- Node.js 18+
- npm 9+
- Rust stable toolchain

Install:

```bash
npm install
```

Recommended verification:

```bash
cargo test --workspace
npm run verify
npm run smoke
```

In a repository checkout, published native add-on packages may not exist yet, so these commands are safer than `npx @jeremyfellaz/kratos ...`.

```bash
npm run scan -- ./fixtures/demo-app
npm run report -- ./fixtures/demo-app/.kratos/latest-report.json
npm run clean -- ./fixtures/demo-app/.kratos/latest-report.json
cargo run -p kratos-cli -- diff ./fixtures/demo-app ./fixtures/demo-app
```

## Distribution

- The root npm package is `@jeremyfellaz/kratos`.
- The CLI binary name is `kratos`.
- Platform add-on packages target macOS arm64/x64, Linux x64/arm64, and Windows x64.
- Root package `optionalDependencies` point to platform add-on packages at the same release version.
- A raw checkout may not have a native add-on, but the released package launcher loads the add-on for the current platform.

## Release Flow

Releases are driven by semver tags such as `vX.Y.Z` or `vX.Y.Z-prerelease.N`.

- The `Manual Release Bump` workflow prepares a version-only PR that aligns `package.json` and platform `optionalDependencies`.
- Before tagging, the same commit should pass `cargo test --workspace`, `npm run verify`, and native packaging CI.
- The `Release Publish` workflow checks out the exact tag ref, verifies the Node package, runs Rust workspace tests, and builds platform native artifacts.
- Platform add-on npm packages are packed, smoke-tested, and published first; the root package is published last, then the GitHub Release is created or updated.
- The `Release Published Follow-up` workflow audits whether the published release has a matching successful publish run and release assets. It does not rerun publishing.

Actions such as pushing release tags, publishing to npm, or publishing a GitHub Release should only happen after maintainer confirmation.

## Open Source

Kratos is open source under the MIT license.

- Use GitHub Issues for bug reports and feature requests.
- Do not report security issues publicly; follow the process in [SECURITY.md](SECURITY.md).
- Read [CONTRIBUTING.md](CONTRIBUTING.md) and [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) before contributing.
- If you want to support the project, you can sponsor it through [GitHub Sponsors](https://github.com/sponsors/JeremyDev87).

## Note

Kratos combines conservative static analysis with heuristics. The current implementation protects known Next.js route exports, package/workflow/action script entrypoints, manual verification scripts, tooling config files, test files, recognized `React.lazy`/`next/dynamic` wrappers, and pure re-export barrels. Custom runtime entrypoints, generated files, and project-specific loaders may still sit outside static analysis, so declare them with `entry`, `keepPatterns`, or suppressions when needed and review the remaining deletion candidates with the report and diff before running `--apply`.
