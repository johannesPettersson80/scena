---
name: scena-remote-builder
description: Use when compiling scena, running cargo fmt/clippy/test/doc/doctor/publish dry-run, synchronizing local work to the Hetzner CPU builder, or reporting remote build/test proof.
---

# Scena Remote Builder

## Builder Contract

Use the Hetzner CPU builder for heavy Rust compilation and test gates.

- SSH alias: `scena-builder`
- Shared checkout observation path: `/home/johannes/projects/scena`. The maintained builder
  currently reports this path missing; do not assume or provision it as part of validation.
- Default validation path: `$HOME/.cache/codex-worktrees/scena-<task-slug>`.
- Remote user: `johannes`
- Purpose: cargo compile, fmt, clippy, tests, docs, doctor, publish dry-run, and CPU/headless
  proof.
- Not purpose: real GPU/WebGPU/WebGL2 proof. Use a real GPU machine for GPU-specific visual
  validation.

Do not store private SSH key material, cloud credentials, or provider tokens in the repo.

Do not run local `cargo build`, `cargo check`, `cargo test`, `cargo clippy`, `cargo doc`,
wasm builds, npm browser proof, or long-running render probes unless the user explicitly
permits it. Local inspection commands such as `rg`, `sed`, `git diff`, and `git status` are
allowed.

## Sync Rule

Before every remote sync and cargo gate, run the checked-in preflight from the local
canonical checkout:

```bash
ssh scena-builder 'bash -s -- <task-slug>' < scripts/scena_remote_builder_preflight.sh
```

The preflight performs the mandatory disk report, emits
`shared_checkout_status=missing` when the former shared path is absent, and always emits an
unambiguous `validation_mode=isolated`, `validation_path=...`, and `cargo_target_dir=...`.
It does not create, delete, or overwrite remote state.

Use the emitted task-scoped destination:

```bash
ssh scena-builder 'mkdir -p "$HOME/.cache/codex-worktrees"'
rsync -az --delete --exclude .git --exclude target ./ scena-builder:~/.cache/codex-worktrees/scena-<task-slug>/
ssh scena-builder 'cd "$HOME/.cache/codex-worktrees/scena-<task-slug>" && <focused-or-scoped-command>'
```

After the tree sync, manually copy root `AGENTS.md` and the complete `.codex/skills/**`
directory from the canonical checkout to the destination. Verify both sides with
`sha256sum` before any edit or gate. A normal rsync, clone, or branch operation never
substitutes for this explicit bootstrap. Pair the snapshot with
`CARGO_TARGET_DIR=$HOME/.cache/codex-targets/scena-<task-slug>` and report both paths.
Clean only that isolated copy and target cache when they are no longer needed.

Treat every `rsync --delete` as an invalidation boundary. It can remove `node_modules`,
generated browser packages, lane logs, and other ignored state that existed only on the
builder. Therefore:

- perform the final source sync before dependency installation and lane generation;
- compare source snapshots by checksum, not only size/mtime;
- rerun `npm ci` after that final sync and verify `require("playwright")` plus the pinned
  `wasm-opt` executable before browser/WASM gates; and
- never reuse a lane artifact generated before the latest sync unless its inputs and hash
  are explicitly proven unchanged.

## Mandatory Disk Preflight

The checked-in preflight above is mandatory before every remote sync or cargo gate. It
includes this disk-pressure inspection; the inline equivalent is retained only for initial
recovery if the script cannot be read:

```bash
ssh scena-builder 'df -hT "$HOME" "$HOME/.cache" /tmp && du -sh "$HOME/.cache/codex-targets" 2>/dev/null || true'
```

Use a task-scoped target cache for validation, for example:

```bash
ssh scena-builder 'cd "$HOME/.cache/codex-worktrees/scena-<task-slug>" && env CARGO_TARGET_DIR="$HOME/.cache/codex-targets/scena-<task-slug>" CARGO_PROFILE_TEST_DEBUG=0 cargo test'
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

## Release Candidate Environment

For a full release rehearsal, declare one environment block and reuse it for every command:

```bash
env \
  CARGO_TARGET_DIR="$HOME/.cache/codex-targets/scena-<task-slug>" \
  CARGO_INCREMENTAL=0 \
  CARGO_PROFILE_DEV_DEBUG=0 \
  CARGO_PROFILE_TEST_DEBUG=0 \
  SCENA_RELEASE_COMMIT=<frozen-local-sha> \
  <command>
```

`SCENA_RELEASE_COMMIT` is mandatory when the isolated snapshot has no `.git`; otherwise an
artifact provenance failure is an invocation failure, not a product failure. Record this
environment once in the validation ledger instead of rediscovering it gate by gate.

"Reuse" means pass the block explicitly on every `ssh`/`env` invocation. Environment exported
inside one SSH shell is lost when that command exits. Browser reruns must additionally pass
the lane values from the workflow, for example `SCENA_BROWSER_BACKENDS=webgl2` or
`SCENA_BROWSER_BACKENDS=webgpu SCENA_GPU_EVIDENCE_CLASS=software-conformance`. Before the
command, print the effective release SHA, backend, browser executable, and driver version to
the lane log.

Before browser/WASM release commands, use the Node/npm versions pinned by the current
workflow, run a clean `npm ci`, and verify `wasm-opt --version`. Do not trust reused
`node_modules` after a parser or syntax failure: compare it with a clean install before
changing Rust or JavaScript source. Classify mismatched or corrupt tools as environment
failures.

Also verify browser/driver compatibility before `wasm-pack test`. If using a system browser,
the Chrome/ChromeDriver major versions must match and `wasm-pack` must receive its explicit
`--chromedriver <path>` option; setting `CHROMEDRIVER` is insufficient because wasm-pack may
replace it. Confirm the requested WebGL2/WebGPU backend is available before interpreting a
pixel/readback failure. On an unsupported builder OS, changing browser failure signatures
are environment evidence; GitHub's supported browser lane remains the deciding proof.

Before starting a local HTTP server, inspect the exact port and terminate only a verified
stale Scena-owned listener, or select a fresh task-owned port. Curl the expected file before
the browser probe. A 404 from an old worktree's listener is not a product result.

Before the full chain, compare free space with the previous task-scoped target size. With no
useful history, reserve at least 20 GiB or stop and clean only the current task cache. The
debug-symbol and incremental settings above are preconditions, not post-disk-failure fixes.

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

For checklist, backlog, or multi-slice implementation work, batch validation deliberately:

- Per logical unit: focused proof first, then only scoped gates for files touched by that
  unit.
- Batch checkpoint: run the full release chain once after the related units are integrated,
  or earlier only when the user explicitly requests release-level proof.
- If a prompt includes a full gate list as boilerplate, treat it as the checkpoint bar, not
  permission to rerun every expensive gate after every small patch.
- If a focused proof does not catch the human-visible defect, stop broadening and replace
  the proof with a better measurement. A green broad suite is not useful while the focused
  proof is wrong.

During a multi-step investigation, run the full release chain once at a checkpoint, not after
every patch. If the focused proof is still failing or the root cause is not understood, the
next correct action is another focused measurement or smaller reproducer, not a broader gate.
If a broad gate already passed on the current diff and no file in that gate's risk surface
changed afterward, do not rerun it just to generate another timestamp; report the existing
evidence and the unchanged surface.

The AGENTS investigation circuit breaker also applies on the builder: classify a failure
before changing code, stop after two remedies with the same signature, and checkpoint after
30 minutes. A resource, disk, toolchain, or hosted-runner failure is environment evidence,
not permission to change production behavior or performance baselines.

Keep a short validation ledger in the handoff:

- `focused`: exact reproducer/proof and result
- `scoped`: only the gate(s) added because of touched files
- `full`: run only when release-level proof is warranted, with the reason
- `skipped`: broader gates intentionally not run, with the risk reason

## Gate Commands

Run gates through SSH from the local machine:

```bash
ssh scena-builder 'cd "$HOME/.cache/codex-worktrees/scena-<task-slug>" && env CARGO_TARGET_DIR="$HOME/.cache/codex-targets/scena-<task-slug>" cargo check --all-targets'
ssh scena-builder 'cd "$HOME/.cache/codex-worktrees/scena-<task-slug>" && env CARGO_TARGET_DIR="$HOME/.cache/codex-targets/scena-<task-slug>" cargo build --all-targets'
ssh scena-builder 'cd "$HOME/.cache/codex-worktrees/scena-<task-slug>" && env CARGO_TARGET_DIR="$HOME/.cache/codex-targets/scena-<task-slug>" cargo fmt --check'
ssh scena-builder 'cd "$HOME/.cache/codex-worktrees/scena-<task-slug>" && env CARGO_TARGET_DIR="$HOME/.cache/codex-targets/scena-<task-slug>" cargo clippy --all-targets -- -D warnings'
ssh scena-builder 'cd "$HOME/.cache/codex-worktrees/scena-<task-slug>" && env CARGO_TARGET_DIR="$HOME/.cache/codex-targets/scena-<task-slug>" cargo test'
ssh scena-builder 'cd "$HOME/.cache/codex-worktrees/scena-<task-slug>" && env CARGO_TARGET_DIR="$HOME/.cache/codex-targets/scena-<task-slug>" cargo run -p xtask -- doctor --full'
```

Release-ready handoff, or any task explicitly asking for full release proof, also requires:

```bash
ssh scena-builder 'cd "$HOME/.cache/codex-worktrees/scena-<task-slug>" && env CARGO_TARGET_DIR="$HOME/.cache/codex-targets/scena-<task-slug>" RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features'
ssh scena-builder 'cd "$HOME/.cache/codex-worktrees/scena-<task-slug>" && env CARGO_TARGET_DIR="$HOME/.cache/codex-targets/scena-<task-slug>" cargo publish --dry-run --locked'
```

## Reporting Proof

Report:

- command run
- remote host alias and repo path
- the preflight's shared-checkout status and isolated `validation_path`
- task-scoped `CARGO_TARGET_DIR` when one was used
- pass/fail status and timing when available
- remote git status and HEAD when relevant
- any gate not run and the concrete reason

If a command fails due to environment drift, fix the builder when safe and rerun. If the
failure is in project code, patch the code and rerun the focused failing gate first.
