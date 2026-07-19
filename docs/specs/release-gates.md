# Release gates

Status: active release contract

This document names the durable release evidence and commands that the release
workflow and `xtask` validate. A local green command is evidence only for the
exact source commit recorded by its artifact; it is not publication proof.

## Required command families

- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `cargo check --examples`
- `cargo run -p xtask -- doctor --full`
- `cargo publish --dry-run`

Browser, GPU, rendered-output, and performance lanes add their own typed proof
artifacts. Required lanes fail closed when their renderer or hardware work is
unavailable; optional diagnostic lanes must say that they are non-release
evidence.

## Required release artifacts

The release bundle includes provenance-bearing `m5-benchmarks.json` and
`m5-public-api-freeze.json` records. Their producer command, source commit,
toolchain/profile, timestamp, and content digest must be validated before the
artifacts can satisfy release readiness. Staging copies validated evidence; it
does not synthesize a passing result.

Their explicit production prerequisite is
`SCENA_RELEASE_COMMIT=$COMMIT SCENA_RELEASE_PROFILE=test-unoptimized cargo test --test m5_release`.
CI and the release workflow run that command after the broad workspace test so
the final uploaded M5 artifacts are not accidental leftovers from another test
invocation. Each artifact records `producing_command`, `toolchain`, `profile`,
`commit_sha`, `timestamp_unix_seconds`, non-empty `source_checksums`, and a
positive `sample_count`. Its `payload_sha256` is SHA-256 over the normalized,
compact JSON object after removing only `payload_sha256`; staging recomputes and
compares it.
The current `sample_count: 1` means one deterministic gate observation, not a
performance distribution or statistical benchmark claim.

`doctor --full` validates the durable source and documentation contract but
does not require or generate ignored `target/gate-artifacts` files. Artifact
production is the explicit prerequisite above, and release staging is the
separate fail-closed consumer.

Independent review reports and maintainer sign-off follow
`docs/specs/release-reviews.md`. GitHub workflow status, a Git tag, a GitHub
release object, and registry publication are separate downstream facts.

## Workflow dependency policy

Third-party GitHub Actions are pinned to lowercase 40-hex commit IDs. Each
`uses:` line retains the resolved release version in an adjacent comment so a
reviewer can understand the intended upgrade. `doctor --full` scans every YAML
file under `.github/workflows/` and rejects mutable references or immutable
references without a reviewable version comment. Local `./` actions and
`docker://` references are outside that third-party action rule.

`.github/dependabot.yml` opens reviewed weekly `github-actions` update pull
requests; updates must refresh both the commit and version comment. These pins
are preventive supply-chain hardening. They are not evidence or an allegation
that any previously referenced action tag was compromised.
