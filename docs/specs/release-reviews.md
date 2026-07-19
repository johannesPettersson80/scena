# Optional Release Review Evidence

Status: optional supplementary evidence. Human review artifacts are not a
release-readiness or publication prerequisite.

Scena releases are blocked by reproducible technical evidence: exact source
provenance, required platform and browser lanes, rendered-output contracts,
performance and API evidence, package checks, and the release-readiness gate.
The release workflow must not require a reviewer count, role bundle,
maintainer-signoff file, or externally hosted approval archive. It must never
manufacture an approval.

Normal repository review happens through GitHub collaboration and may be
recorded in pull requests, issues, or release discussion. Repository size and
available maintainers determine that governance; automation does not invent
additional people to satisfy a numeric policy.

## Retired automated bundle policy

The former six role reports plus a seventh maintainer sign-off were an
unsatisfiable automation policy, not technical release evidence. The bundle
validator, archive installer, staging integration, workflow inputs, and bundle
fixtures have been removed. Historical `reviews/<role>/<commit>.md`,
`reviews/findings.json`, and `reviews/maintainer-signoff.toml` files may remain
as records, but no release tool consumes or requires them.

## Release behavior

- `stage-release-artifacts` consumes only source-provenance-bearing technical
  lane output and never scans, copies, creates, or requires `reviews/`.
- `release-readiness` validates only the staged technical bundle.
- Branch CI stages and validates the complete technical bundle successfully;
  `RELEASE-REVIEWS-MISSING` is not an accepted success boundary.
- Tag and manual release workflows have no review-bundle URL or checksum input.
- `xtask doctor --full` rejects workflow or staging changes that reintroduce an
  external review bundle as a machine publication prerequisite.

This policy does not weaken code review or technical release evidence. It
removes an unsatisfiable external-personnel gate from automation and keeps
release truth tied to artifacts the repository can actually produce and
verify.
