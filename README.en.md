# Kratos

[![CI](https://github.com/JeremyDev87/kratos/actions/workflows/ci.yml/badge.svg)](https://github.com/JeremyDev87/kratos/actions/workflows/ci.yml)

[한국어](README.md) | English | [中文](README.zh-CN.md) | [Español](README.es.md) | [日本語](README.ja.md)

Destroy dead code ruthlessly.

[License](LICENSE) · [Contributing](CONTRIBUTING.md) · [Code of Conduct](CODE_OF_CONDUCT.md) · [Security](SECURITY.md) · [Sponsor](https://github.com/sponsors/JeremyDev87)

Kratos is a CLI tool that finds dead code hidden inside your project, including unused files, broken imports, and orphan modules, then suggests safe removal candidates. As legacy grows, codebases get heavier and maintenance costs rise. Kratos focuses on exposing unnecessary leftovers and helping your codebase feel lean again.

## Core Capabilities

- Detect unused files
- Detect dead exports
- Detect broken imports
- Detect orphan modules and orphan components
- Suggest safe deletion candidates
- Generate codebase slimming reports

## Quick Start

```bash
npm install
npx kratos scan
npx kratos report
npx kratos clean
```

For local development, you can also run:

```bash
node ./src/cli.js scan
node ./src/cli.js report
node ./src/cli.js clean
```

## Commands

### `kratos scan [root]`

Scans a project and writes the analysis result to `.kratos/latest-report.json`.

Options:

- `--output <path>`: set a custom report output path
- `--json`: print the full JSON result instead of the console summary

### `kratos report [report-path-or-root]`

Reads the latest saved report and prints it in a human-friendly format.

Options:

- `--format summary`: default summary output
- `--format json`: raw JSON output
- `--format md`: Markdown report output

### `kratos clean [report-path-or-root]`

Shows deletion candidates or removes them.

Options:

- `--apply`: perform actual deletion

The default behavior is dry-run. Without `--apply`, no files are deleted.

## What The Current MVP Detects

- JS / JSX / TS / TSX / MJS / CJS file graph
- relative import / require / dynamic import
- `tsconfig.json` / `jsconfig.json` `baseUrl` and `paths`
- Next.js `app/` / `pages/` entrypoint heuristics
- package.json `main`, `module`, `bin`, and `exports` entrypoints
- orphan file / orphan component candidates
- dead export candidates
- unused import candidates
- broken internal imports

## Report Example

```bash
$ node ./src/cli.js scan ./fixtures/demo-app
Kratos scan complete.

Root: /.../fixtures/demo-app
Files scanned: 5
Entrypoints: 1
Broken imports: 1
Orphan files: 2
Dead exports: 3
Unused imports: 0
Deletion candidates: 2

Saved report: /.../fixtures/demo-app/.kratos/latest-report.json
```

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

- Old React / Next.js projects
- Teams with lots of shipped features and accumulated code
- Teams looking for the right time to refactor

## Releases

Kratos uses semantic version tags such as `v0.1.0` for releases.

```bash
npm version 0.1.0 --no-git-tag-version
git add package*.json
git commit -m "chore: release v0.1.0"
git tag v0.1.0
git push origin HEAD
git push origin v0.1.0
```

When a tag is pushed, the [release workflow](.github/workflows/release.yml) will:

- run `npm run verify`
- build the npm release tarball
- publish stable releases to npm `latest` and prereleases to npm `next`
- create a GitHub Release and attach the tarball

The recommended setup is npm Trusted Publishing (OIDC). If that is not configured yet, the workflow can fall back to a repository `NPM_TOKEN` secret.

## Open Source

Kratos is open source under the MIT license.

- Use GitHub Issues for bug reports and feature requests.
- Do not report security issues publicly; follow the process in [SECURITY.md](SECURITY.md).
- Read [CONTRIBUTING.md](CONTRIBUTING.md) and [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) before contributing.
- If you want to support the project, you can sponsor it through [GitHub Sponsors](https://github.com/sponsors/JeremyDev87).

## Note

This version is a heuristic MVP, not an AST-based analyzer. It is optimized for quickly scanning large projects, and you should always review the report before deleting anything.
