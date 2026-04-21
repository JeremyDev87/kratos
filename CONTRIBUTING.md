# Contributing to Kratos

Thanks for helping make Kratos sharper.

Kratos aims to find dead code safely and conservatively. That means changes
should prefer correctness over aggressiveness, and we should bias toward fewer
false positives when the analysis is uncertain. The current runtime shape is a
Rust core/CLI distributed through an npm launcher plus per-platform native
addon packages.

## Before You Start

- Search existing issues and pull requests before opening a new one
- For larger ideas, open an issue first so we can align on direction
- Keep changes focused; one pull request should solve one problem well

## Development Setup

### Requirements

- Node.js 18+
- npm 9+
- Rust stable toolchain

### Install

```bash
npm install
```

### Recommended Validation

```bash
cargo test --workspace
npm run verify
npm run smoke
npm run scan -- ./fixtures/demo-app
npm run report -- ./fixtures/demo-app/.kratos/latest-report.json
npm run clean -- ./fixtures/demo-app/.kratos/latest-report.json
```

If you want to invoke the Rust CLI directly from the repo checkout, you can
also use:

```bash
cargo run -p kratos-cli -- scan ./fixtures/demo-app
cargo run -p kratos-cli -- report ./fixtures/demo-app/.kratos/latest-report.json --format md
cargo run -p kratos-cli -- clean ./fixtures/demo-app/.kratos/latest-report.json
```

Do not assume `npx @jeremyfellaz/kratos ...` will work from a raw repository checkout. The
packaged launcher expects published native addon packages, so local development
should use the npm scripts above or the Rust CLI directly.

## What We Review For

- Correctness of the analysis
- Safety of deletion behavior
- Low false-positive risk for scan results
- Clear CLI behavior, packaging, and release ergonomics
- Documentation parity across `README.md`, `README.en.md`, `README.es.md`,
  `README.ja.md`, and `README.zh-CN.md` when user-facing behavior changes
- Regression coverage for parser and resolver edge cases

## Pull Request Guidelines

1. Fork the repo and create a focused branch.
2. Add or update tests for behavior changes.
3. Update documentation when commands, config, output, packaging, or release
   flow changes.
4. Keep all README translations aligned when you change shared user-facing
   behavior.
5. Keep commits and PR descriptions specific.
6. Explain trade-offs when a heuristic is intentionally conservative.

## Testing Expectations

At minimum, please run:

```bash
cargo test --workspace
npm run verify
```

Also run `npm run smoke` when your change touches packaging, the launcher, or
release automation.

If your change affects CLI behavior or documentation, include the command you
ran and the observed result in the PR description. For command examples, prefer
`./fixtures/demo-app` so reviewers can reproduce the same path locally.

## Reporting Bugs

When filing a bug, include:

- Kratos version or commit
- Node.js version
- OS and shell
- The command you ran
- Expected behavior
- Actual behavior
- A minimal reproduction when possible

## Security

Please do not open public issues for security vulnerabilities. Follow the
process in [SECURITY.md](SECURITY.md).

## Code of Conduct

By participating in this project, you agree to abide by
[CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).
