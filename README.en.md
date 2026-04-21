# Kratos

[![CI](https://github.com/JeremyDev87/kratos/actions/workflows/ci.yml/badge.svg)](https://github.com/JeremyDev87/kratos/actions/workflows/ci.yml)

[한국어](README.md) | English | [中文](README.zh-CN.md) | [Español](README.es.md) | [日本語](README.ja.md)

Destroy dead code ruthlessly.

[License](LICENSE) · [Contributing](CONTRIBUTING.md) · [Code of Conduct](CODE_OF_CONDUCT.md) · [Security](SECURITY.md) · [Sponsor](https://github.com/sponsors/JeremyDev87)

Kratos is a CLI tool that finds dead code hidden inside your project, including unused files, broken imports, and orphan modules, then suggests safe removal candidates. The current distribution combines a Rust core/CLI with an npm launcher and focuses on conservative analysis and safe cleanup suggestions.

## Core Capabilities

- Detect unused files
- Detect dead exports
- Detect broken imports
- Detect orphan modules and orphan components
- Suggest safe deletion candidates
- Generate codebase slimming reports

## Quick Start

For package users, the default entrypoint is `npx`.

```bash
npx @jeremyfellaz/kratos scan ./your-project
npx @jeremyfellaz/kratos report ./your-project/.kratos/latest-report.json
npx @jeremyfellaz/kratos clean ./your-project/.kratos/latest-report.json
```

- `scan` writes `.kratos/latest-report.json` by default.
- `clean` is dry-run by default and only deletes files when you add `--apply`.

## Local Development

Work from a repository checkout with the Rust CLI and npm scripts.

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

To exercise the CLI from the repo itself, use:

```bash
npm run scan -- ./fixtures/demo-app
npm run report -- ./fixtures/demo-app/.kratos/latest-report.json
npm run clean -- ./fixtures/demo-app/.kratos/latest-report.json
```

In a checkout, published native addon packages may not be present yet, so prefer the commands above or `cargo run -p kratos-cli -- ...` instead of `npx @jeremyfellaz/kratos ...`.

## Commands

### `kratos scan [root]`

Scans a project and writes the latest report.

- default output path: `<root>/.kratos/latest-report.json`
- `--output <path>`: set a custom report output path
- `--json`: print the full report JSON to stdout instead of the console summary

### `kratos report [report-path-or-root]`

Prints a saved report in summary, JSON, or Markdown form.

- `--format summary`: default summary output
- `--format json`: raw JSON output
- `--format md`: Markdown report output
- when given a project root instead of a report path, Kratos resolves the latest report automatically

### `kratos clean [report-path-or-root]`

Shows deletion candidates or deletes them.

- dry-run by default
- `--apply`: perform the actual deletion
- when given a project root instead of a report path, Kratos resolves the latest report automatically

## Report Schema

Current Rust scan output writes `schemaVersion: 2`.

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

- the default saved location is `.kratos/latest-report.json`
- `report` and `clean` accept either a saved report path or a project root
- Markdown output includes broken imports, orphan files, dead exports, route entrypoints, and deletion candidates

## Current Coverage

- Rust analyzer with Oxc-based JS/TS import and export parsing
- relative import / require / dynamic import
- `tsconfig.json` / `jsconfig.json` `baseUrl` and `paths`
- Next.js `app/` / `pages/` route entrypoint heuristics
- `package.json` `main`, `module`, `bin`, and `exports` entrypoints
- orphan file / orphan component candidates
- dead export candidates
- unused import candidates
- broken internal imports

## Configuration

You can optionally add `kratos.config.json`.

```json
{
  "ignore": ["storybook-static", "generated"],
  "entry": ["src/bootstrap.ts"],
  "roots": ["src", "app", "pages"]
}
```

- `ignore`: additional directory names to skip
- `entry`: file paths that should be forced as entrypoints
- `roots`: folders to limit the scan scope

## Recommended For

- older React / Next.js projects
- teams with many shipped features and accumulated code
- teams looking for the right time to refactor

## Release Flow

Kratos release automation is driven by semantic version tags such as `v0.2.0-alpha.1` and `v0.2.0`.

Alpha readiness:

- keep the root package version at `0.2.0-alpha.1`
- before tagging, run `cargo test --workspace`, `npm run verify`, `npm run smoke`, and fixture-based `scan/report/clean` smoke checks
- create and push the alpha tag only after maintainer confirmation

The [release workflow](.github/workflows/release.yml) runs on a pushed tag or manual dispatch with an existing tag and then:

- resolves release metadata and maps prereleases to npm dist-tag `next`
- verifies the Node package and Rust workspace separately
- builds native artifacts for macOS arm64/x64, Linux x64/arm64, and Windows x64
- packs and smoke-tests per-platform addon npm packages before publishing them
- publishes the root `kratos` package and creates a GitHub Release last

Stable promotion:

- after alpha verification, bump `package.json` from `0.2.0-alpha.1` to `0.2.0` in a version-only release-prep commit
- create the stable tag `v0.2.0` as a separate step, which publishes to npm `latest`

The recommended publishing setup is npm Trusted Publishing (OIDC). A repository `NPM_TOKEN` fallback can still be used when needed.

## Open Source

Kratos is open source under the MIT license.

- Use GitHub Issues for bug reports and feature requests.
- Do not report security issues publicly; follow the process in [SECURITY.md](SECURITY.md).
- Read [CONTRIBUTING.md](CONTRIBUTING.md) and [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) before contributing.
- If you want to support the project, you can sponsor it through [GitHub Sponsors](https://github.com/sponsors/JeremyDev87).

## Note

The current alpha uses a Rust core and Oxc-based parsing, but entrypoint detection and safe deletion candidates still include conservative heuristics. Always review the report before running `--apply`.
