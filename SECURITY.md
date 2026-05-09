# Security Policy

## Supported Versions

Kratos security support starts with the v1.0 release line. Security fixes are
prepared on the default development branch, `master`, and released through the
supported release lines below.

| Version or branch | Supported | Policy |
| --- | --- | --- |
| `master` | Yes | Default development branch for unreleased security fixes. |
| Latest release | Yes | Newest published stable release. |
| Current major | Yes | Supported release line sharing the latest stable major version, such as `1.x` for v1.0. |
| Older releases | No | Earlier major versions and pre-v1.0 releases are unsupported unless maintainers announce an exception. |

Definitions:

- Latest release means the newest stable GitHub release or tag published from
  `master`.
- Current major means the active stable major release line that contains the
  latest release. For v1.0, this is the `1.x` line.
- Older releases means pre-v1.0 releases and any major release line older than
  the current major. Upgrade to the latest supported release before requesting
  a security fix.

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
