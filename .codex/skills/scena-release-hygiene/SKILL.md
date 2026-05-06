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
7. Do not publish or tag unless the user asks.

## Versioning Defaults

- `0.0.x`: foundation, scaffolding, docs, and internal tooling before real renderer API.
- `0.x.0`: backward-compatible public renderer capability after implementation starts.
- `1.0.0`: only after the acceptance index and release gates are complete.

Breaking public API changes are allowed before `1.0.0`, but they must update examples,
docs, and migration notes when users can reasonably have adopted the previous API.

## Required Local Gates

Run before release-ready handoff:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo run -p xtask -- doctor --full
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features
```

For publish-readiness:

```bash
cargo publish --dry-run
```

An unrun required gate is not a pass. Record the exact blocker when a gate cannot run.
