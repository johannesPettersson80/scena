---
name: scena-release-hygiene
description: Use when preparing scena user-visible changes for release, changing crate metadata, versioning, changelog/release notes, public API stability, cargo publish dry runs, semver checks, or v1.0 release evidence.
---

# Scena Release Hygiene

## Scope

Use this skill for user-visible API, renderer behavior, docs/tutorial, crate metadata,
release gate, and publish-readiness work.

Pure internal refactors can skip release-note work unless they change public behavior,
developer commands, diagnostics, or documented contracts.

## Workflow

1. Identify whether the change is release-notable.
2. Keep `Cargo.toml` metadata accurate for the current maturity level.
3. Once `CHANGELOG.md` exists, add user-facing changes under `## [Unreleased]`.
4. Keep README, RFC, specs, examples, and milestone checklists aligned with shipped
   behavior.
5. For public API changes, update or add examples and API-diff evidence once the M5 baseline
   exists.
6. For rendering, browser, visual, glTF, or platform changes, require the proof named in
   `docs/specs/release-gates.md`; unit tests alone are not release evidence.
7. Before starting a release matrix, prove every configured publication prerequisite is
   satisfiable by the repository's actual lanes, secrets, and governance. Technical evidence
   and provenance may block; unavailable reviewer counts and external approval bundles must
   not be machine publication prerequisites or be synthesized by automation.
8. Do not publish or tag unless the user asks.

## Versioning Defaults

- `0.0.x`: foundation, scaffolding, docs, and internal tooling before real renderer API.
- `0.x.0`: backward-compatible public renderer capability after implementation starts.
- `1.0.0`: only after the acceptance index and release gates are complete.

Breaking public API changes are allowed before `1.0.0`, but they must update examples,
docs, and migration notes when users can reasonably have adopted the previous API.

## Required Remote Gates

Run on `scena-builder` before release-ready handoff. These are release-checkpoint gates, not
the default inner loop for every small fix. During implementation, use the
`scena-remote-builder` validation ladder: focused proof first, scoped gates second, full
release gates once before the release-ready claim.

```bash
ssh scena-builder 'cd "$HOME/.cache/codex-worktrees/scena-<task-slug>" && env CARGO_TARGET_DIR="$HOME/.cache/codex-targets/scena-<task-slug>" cargo fmt --check'
ssh scena-builder 'cd "$HOME/.cache/codex-worktrees/scena-<task-slug>" && env CARGO_TARGET_DIR="$HOME/.cache/codex-targets/scena-<task-slug>" cargo clippy --all-targets -- -D warnings'
ssh scena-builder 'cd "$HOME/.cache/codex-worktrees/scena-<task-slug>" && env CARGO_TARGET_DIR="$HOME/.cache/codex-targets/scena-<task-slug>" cargo test'
ssh scena-builder 'cd "$HOME/.cache/codex-worktrees/scena-<task-slug>" && env CARGO_TARGET_DIR="$HOME/.cache/codex-targets/scena-<task-slug>" cargo run -p xtask -- doctor --full'
ssh scena-builder 'cd "$HOME/.cache/codex-worktrees/scena-<task-slug>" && env CARGO_TARGET_DIR="$HOME/.cache/codex-targets/scena-<task-slug>" RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features'
```

For publish-readiness:

```bash
ssh scena-builder 'cd "$HOME/.cache/codex-worktrees/scena-<task-slug>" && env CARGO_TARGET_DIR="$HOME/.cache/codex-targets/scena-<task-slug>" cargo publish --dry-run --locked'
```

An unrun required gate is not a pass. Record the exact blocker when a gate cannot run.
Use `scena-remote-builder` to sync local uncommitted work to the builder before running
these gates.

If the work is not release-ready yet, do not imply these gates are required after every
increment. Report the focused/scoped proof actually run and the reason release gates are
being deferred until the checkpoint.

## Frozen Candidate Rehearsal

Before tagging, freeze one candidate SHA and keep a concise ledger for that SHA. The ledger
must name every required command, artifact, result, and covered source surface. Do not
reconstruct release commands from memory: read the current CI and Release workflows plus the
xtask release-lane contracts, then execute the exact checked-in wrappers for every locally
reproducible lane.

For an isolated builder snapshot without `.git`, set `SCENA_RELEASE_COMMIT` to the frozen
local SHA for every provenance-sensitive command. Before tagging, the same frozen snapshot
must have:

- focused regression proof and scoped gates green;
- the full native/WASM/browser checkpoint green once;
- each locally reproducible `scripts/release_lane_command.sh` command recorded;
- release-lane artifacts accepted by `stage-release-artifacts` and `release-readiness`;
- `cargo publish --dry-run --locked` green; and
- no source, harness, workflow, lockfile, or release-contract edit after the rehearsal.

Any such edit invalidates the affected ledger entries. Return to the focused proof, rerun
only affected scoped lanes, refreeze once, and perform one final full checkpoint. Do not tag
an incompletely replayed candidate and use GitHub as the first complete test environment.

Before patching a failed release run, execute
`scripts/collect_ci_failure_evidence.sh <run-id>` and classify every failed job. Batch all
known corrections into one release candidate and run one deciding full matrix. Two failed
remedies with the same signature trip the investigation circuit breaker; no third push is
allowed without a smaller discriminating proof.

Shared GitHub-hosted machines are not controlled performance hardware. Their M9 wall-clock
measurements use `SCENA_M9_TIMING_POLICY=report-only-hosted`; sample validity and allocation
budgets still block. Strict 5% timing evidence must come from a stable controlled lane and
must never be replaced by a widened hosted-runner baseline.

For a backlog or checklist that contains many fixes, release hygiene is satisfied by one
full release-gate run at the final integration checkpoint, plus focused/scoped evidence for
each logical fix. Do not re-run publish/doc/browser gates after each small patch unless that
patch specifically changes the release artifact surface.
