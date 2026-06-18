# Security Policy

## Supported versions

browserlane is pre-1.0; security fixes land on the latest `0.1.x` release.
Please reproduce against the latest release before reporting.

## Reporting a vulnerability

**Please do not open a public issue for security problems.**

Report privately through GitHub's
**[private vulnerability reporting](https://github.com/browserlane/browserlane/security/advisories/new)**
— the *Security → Report a vulnerability* button on this repository. You'll get
an acknowledgement, and we'll coordinate the fix and disclosure with you.

## Verifying downloads

browserlane ships signed binaries — macOS (Developer ID + Apple notarization)
and Windows (Azure Artifact Signing). Always verify a download against the
published `SHA256SUMS` on the [release](https://github.com/browserlane/browserlane/releases/latest);
the one-line installer does this automatically.
