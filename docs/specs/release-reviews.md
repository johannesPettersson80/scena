# Release Review Evidence

Status: required for release readiness.

This specification defines independently authored review evidence consumed by
`cargo run -p xtask -- stage-release-artifacts`. Staging validates and copies
this evidence. It must never author a review, clear a finding, sign a review, or
approve a release.

All review evidence targets one exact 40-hex Git commit. `local-checkout`, a
blank value, a branch name, and a foreign commit are invalid release
provenance.

## 1. Per-role reports

Every role in the xtask `REQUIRED_REVIEW_ROLES` policy must provide exactly one
report at:

```text
reviews/<role>/<reviewed_commit>.md
```

The report starts with this frontmatter:

```yaml
---
role: scena-rfc-reviewer
reviewed_commit: 0123456789abcdef0123456789abcdef01234567
session_id: independent-review-session
date: 2026-07-16
reviewer_identity: github:reviewer-login
reviewer_provenance: https://github.com/scena-rs/scena/actions/runs/123
blocker_status: clear
findings_count: 0
---
```

`reviewer_identity` is a verifiable `github:<login>` identity. Automation,
GitHub Actions, release bots, and synthetic release actors are not reviewers.
`reviewer_provenance` is an HTTPS URL that identifies the independent review
run or durable review record.

One identity may satisfy only one required review role in a release bundle.
The maintainer signing the bundle must also be distinct from every required
reviewer. A report can enter a releasable bundle only with
`blocker_status: clear`.

Every `### Finding` block in a report contains `Severity:`, `Status:`,
`Evidence:`, and `Notes:` fields. `findings_count` equals the number of those
blocks. The complete normalized finding history is recorded in the findings
register rather than inferred from prose.

## 2. Findings register

`reviews/findings.json` uses schema `scena.release.findings.v1` and records the
same exact `reviewed_commit` as every report:

```json
{
  "schema": "scena.release.findings.v1",
  "reviewed_commit": "0123456789abcdef0123456789abcdef01234567",
  "generated_at": "2026-07-16T00:00:00Z",
  "findings": []
}
```

Each finding records:

- `id`, `role`, `summary`, `severity`, `status`, `evidence`, `notes`, and
  `deferral_target`;
- a non-empty `history` array recording status transitions and their times;
- a role from the required review-role policy.

A `blocker` or `critical` finding must have terminal status `fixed`, `closed`,
or `resolved`. `open`, `accepted`, and `deferred` are not terminal blocker
states. A deferred non-blocking finding requires a non-null durable
`deferral_target`.

The register is complete: every report finding appears in it, and no register
entry claims a role report that does not exist.

## 3. Maintainer sign-off

`reviews/maintainer-signoff.toml` is independently authored after the exact
report bytes and findings register are final:

```toml
[maintainer]
name = "Independent Maintainer"
identity = "github:maintainer-login"
signed_commit = "0123456789abcdef0123456789abcdef01234567"

[reviews]
all_clear = true
findings_register = "reviews/findings.json"
findings_sha256 = "<64 lowercase hex characters>"
required_roles = ["scena-rfc-reviewer"]
scena_rfc_reviewer_sha256 = "<64 lowercase hex characters>"

[approval]
decision = "approve"
approved_at = "2026-07-16T00:00:00Z"
```

There is one `<role_with_hyphens_replaced_by_underscores>_sha256` field for
every required role. Each hash covers the complete report bytes. The findings
hash covers the complete `findings.json` bytes. Any edit after sign-off makes
the bundle invalid.

The maintainer identity follows the same human GitHub identity rule as
reviewers and is distinct from all of them. `decision = "approve"` is valid
only with `all_clear = true`, exact commit agreement, all required role hashes,
the exact findings-register hash, and no open blocker.

## 4. Staging behavior

Release staging:

1. requires one unambiguous input for every role report, the findings
   register, and maintainer sign-off;
2. validates identities, provenance, distinct-role policy, commits, finding
   states, and hashes before accepting the bundle;
3. copies all review inputs byte-for-byte;
4. emits separate `staging-metadata.json` with schema
   `scena.release.staging.v1`, `staged_at`, staging checkout, staging tool, and
   staging tool version;
5. never rewrites source generation or review provenance.

Missing evidence, automation-authored approval, duplicate reviewer identity,
commit mismatch, open blocker, stale hash, and absent sign-off all fail closed.

## 5. Workflow delivery

Review evidence is produced outside the release workflow after reviewers have
examined the exact tagged commit. The bundle is a gzip-compressed tar archive
whose only top-level directory is `reviews/`. It contains every role report,
`reviews/findings.json`, and `reviews/maintainer-signoff.toml` in the layout
above.

Manual release dispatch supplies both `review_bundle_url` and
`review_bundle_sha256`. The URL must use HTTPS. The workflow downloads the
archive, verifies the exact lowercase SHA-256, rejects traversal, links,
special files, duplicates, and payloads outside `reviews/`, then installs it
under the downloaded lane artifacts. Normal staging performs the full
identity, commit, finding, and sign-off validation afterward. Tag-triggered
publication with no supplied review bundle fails closed; it cannot synthesize
or reuse an approval.

Pull-request and main-branch CI cannot possess a final review of its own head
commit without creating a circular self-review. Its evidence-integrity job
therefore validates every lane/source artifact through visual-proof creation
and succeeds only when staging reaches the stable
`RELEASE-REVIEWS-MISSING` boundary. It does not run final release readiness or
claim that the commit is approved. Final release readiness exists only in the
release workflow after the external bundle is installed.
