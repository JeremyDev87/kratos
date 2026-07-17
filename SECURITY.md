# Security Policy

## Supported Versions

Kratos security support starts with the v1.0 release line. Security fixes are
prepared on the default development branch, `master`, and released through the
supported release lines below.

The currently published `0.3.7` line is pre-v1 and is not a supported security
release line. Until v1.0 ships, report an issue against `master` with the
affected commit and do not infer a backport promise from the npm version.

| Version or branch | Supported | Policy |
| --- | --- | --- |
| `master` | Yes | Default development branch for unreleased security fixes. |
| `1.x` after v1.0 ships | Yes | Current supported release line; use its newest stable release. |
| Pre-v1 `0.x` releases | No | No security backport or support promise before v1.0. |
| Older major releases | No | Major lines older than the current supported major are unsupported unless maintainers announce an exception. |

Definitions:

- Latest supported release means the newest stable v1+ GitHub release or tag
  published from `master`.
- Current major means the active stable major release line that contains the
  latest supported release. After v1.0 ships, this is the `1.x` line.
- Pre-v1 releases and major release lines older than the current major are
  unsupported. Upgrade to the latest supported release before requesting a
  security fix.

## Reporting a Vulnerability

Please do not report security vulnerabilities in public GitHub issues,
discussions, or pull requests.

If GitHub Private Vulnerability Reporting is enabled for this repository, use
that channel first. It is the preferred way to share vulnerability details with
the maintainers.

If Private Vulnerability Reporting is not available, open a minimal public
issue that does **not** include exploit details and request a private contact
channel from the maintainers. Keep proof-of-concept code, payloads, and
reproduction steps private until a secure channel is established.

When reporting a vulnerability, please include:

- The affected version, commit, or branch
- The attack scenario and expected impact
- Reproduction steps
- Any proof-of-concept artifacts, if safe to share privately
- Suggested remediation, if you have one

We will acknowledge reports as quickly as possible, investigate, and work
toward a fix before public disclosure when feasible.
