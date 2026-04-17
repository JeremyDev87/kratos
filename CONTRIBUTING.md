# Contributing to Kratos

Thanks for helping make Kratos sharper.

Kratos aims to find dead code safely and conservatively. That means changes
should prefer correctness over aggressiveness, and we should bias toward fewer
false positives when the analysis is uncertain.

## Before You Start

- Search existing issues and pull requests before opening a new one
- For larger ideas, open an issue first so we can align on direction
- Keep changes focused; one pull request should solve one problem well

## Development Setup

### Requirements

- Node.js 18+
- npm 9+

### Install

```bash
npm install
```

### Useful Commands

```bash
npm test
npm run smoke
node ./src/cli.js scan
node ./src/cli.js report
node ./src/cli.js clean
```

## What We Review For

- Correctness of the analysis
- Safety of deletion behavior
- Low false-positive risk for scan results
- Clear CLI behavior and documentation
- Regression coverage for parser and resolver edge cases

## Pull Request Guidelines

1. Fork the repo and create a focused branch.
2. Add or update tests for behavior changes.
3. Update documentation when commands, config, or output changes.
4. Keep commits and PR descriptions specific.
5. Explain trade-offs when a heuristic is intentionally conservative.

## Testing Expectations

At minimum, please run:

```bash
npm test
npm run smoke
```

If your change affects CLI behavior, include the command you ran and the
observed result in the PR description.

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
