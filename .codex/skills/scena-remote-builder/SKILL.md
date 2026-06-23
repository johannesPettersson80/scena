---
name: scena-remote-builder
description: Use when compiling scena, running cargo fmt/clippy/test/doc/doctor/publish dry-run, synchronizing local work to the Hetzner CPU builder, or reporting remote build/test proof.
---

# Scena Remote Builder

## Builder Contract

Use the Hetzner CPU builder for heavy Rust compilation and test gates.

- SSH alias: `scena-builder`
- Remote repo path: `/home/johannes/projects/scena`
- Remote user: `johannes`
- Purpose: cargo compile, fmt, clippy, tests, docs, doctor, publish dry-run, and CPU/headless
  proof.
- Not purpose: real GPU/WebGPU/WebGL2 proof. Use a real GPU machine for GPU-specific visual
  validation.

Do not store private SSH key material, cloud credentials, or provider tokens in the repo.

## Sync Rule

Before a remote gate, make the remote checkout match the exact work being validated.

For a clean branch already pushed to GitHub:

```bash
ssh scena-builder 'cd "$HOME/projects/scena" && git fetch origin && git checkout <branch> && git pull --ff-only'
```

For local uncommitted work:

1. Check local and remote status.
2. If the remote has unrelated dirty changes, stop and report them.
3. Mirror the local working tree to the remote repo, keeping the remote `.git` and build
   cache intact:

```bash
git status --short --branch
ssh scena-builder 'git -C "$HOME/projects/scena" status --short --branch'
rsync -az --delete --exclude .git --exclude target ./ scena-builder:~/projects/scena/
```

After syncing, remote `git status --short --branch` should show the same relevant working
tree changes as the local checkout.

## Mandatory Disk Preflight

Before every remote sync or cargo gate, check builder disk pressure. This is not optional:
late linker failures from full target caches waste long gate runs.

```bash
ssh scena-builder 'df -hT "$HOME" "$HOME/.cache" /tmp && du -sh "$HOME/.cache/codex-targets" "$HOME/projects/scena/target" 2>/dev/null || true'
```

Use a task-scoped target cache for validation, for example:

```bash
ssh scena-builder 'cd "$HOME/projects/scena" && env CARGO_TARGET_DIR="$HOME/.cache/codex-targets/scena-<task-slug>" CARGO_PROFILE_TEST_DEBUG=0 cargo test'
```

If the preflight shows low free space, or a gate fails with `No space left on device`,
`Disk full`, or `Disk quota exceeded`, clean only generated output that belongs to the
current validation task, then rerun the preflight:

```bash
ssh scena-builder 'rm -rf "$HOME/.cache/codex-targets/scena-<task-slug>"'
```

Do not delete unrelated caches, other repositories, checkouts, or user files without
explicit user approval. If `/tmp` is the constrained filesystem, set a task-local `TMPDIR`
inside the validation checkout or task target cache before rerunning.

## Validation Ladder

Do not make every remote run a full release validation by default. Choose the smallest
remote proof that can honestly catch the regression class:

1. **Focused proof**: first run the exact test, CLI command, doctor check, browser check, or
   rendered-output proof that exercises the changed behavior. If this fails, patch and rerun
   this proof before broadening.
2. **Scoped gates**: add only the broad gates relevant to the touched surface:
   - Rust source formatting changed: `cargo fmt --check`.
   - Production Rust behavior changed: focused test plus the relevant `cargo test ...`
     target or package, then broaden if risk crosses modules.
   - Doctor/checklist/schema pins changed: `cargo run -p xtask -- doctor --full`.
   - CLI/recipe behavior changed: the affected integration test file or exact CLI proof.
   - Browser/WASM-visible behavior changed: the browser lane or Playwright proof.
3. **Full release gates**: run the full fmt/clippy/test/doctor/doc/publish/browser chain
   only for release-ready work, public API/schema behavior, cross-backend renderer changes,
   large refactors, or when the user explicitly asks for full release hygiene.

For tiny test-only or doctor-pin-only changes, a focused proof plus the relevant scoped gate
is usually enough. Do not spend hours on unrelated full-suite loops unless there is a real
risk path from the edit to that gate.

## Gate Commands

Run gates through SSH from the local machine:

```bash
ssh scena-builder 'cd "$HOME/projects/scena" && cargo check --all-targets'
ssh scena-builder 'cd "$HOME/projects/scena" && cargo build --all-targets'
ssh scena-builder 'cd "$HOME/projects/scena" && cargo fmt --check'
ssh scena-builder 'cd "$HOME/projects/scena" && cargo clippy --all-targets -- -D warnings'
ssh scena-builder 'cd "$HOME/projects/scena" && cargo test'
ssh scena-builder 'cd "$HOME/projects/scena" && cargo run -p xtask -- doctor --full'
```

Release-ready handoff, or any task explicitly asking for full release proof, also requires:

```bash
ssh scena-builder 'cd "$HOME/projects/scena" && RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features'
ssh scena-builder 'cd "$HOME/projects/scena" && cargo publish --dry-run'
```

## Reporting Proof

Report:

- command run
- remote host alias and repo path
- pass/fail status and timing when available
- remote git status and HEAD when relevant
- any gate not run and the concrete reason

If a command fails due to environment drift, fix the builder when safe and rerun. If the
failure is in project code, patch the code and rerun the focused failing gate first.
