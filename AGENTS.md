# AGENTS

Repo-specific instructions for agents working on `scena`.

## Mission

`scena` is a Rust-native Three.js replacement for scene-graph, glTF, model-viewer,
industrial visualization, CAD/viewer, and digital-twin style applications. It is not a
simulation engine, not a robotics engine, not physics, not PLC/domain logic, and not a game
engine.

The canonical charter is `docs/RFC-rust-3d-renderer.md`. Start architectural, API, scope, or
milestone work from that RFC.

## Required Skills

- Use `scena-rfc-governance` when editing the RFC, changing v1.0/v1.x scope, changing
  milestones, or reviewing whether a feature belongs in the renderer.
- Use `scena-renderer-architecture` when implementing or refactoring scene/assets/render
  module ownership, typed handles, prepare/render lifecycle, resource lifetime, or public
  API.
- Use `scena-renderer-quality` when adding tests, visual proof, browser/WASM checks,
  headless screenshots, leak tests, allocation tests, or capability gates.
- Use `scena-gltf-assets` when touching glTF/GLB loading, extensions, animation, skinning,
  morph targets, anchors, units, or hot reload.
- Use `scena-doctor` when adding, changing, or reviewing doctor checks, validation gates,
  silent-failure prevention, checklist enforcement, or source-derived architecture rules.
- Use `scena-release-hygiene` when preparing user-visible changes for release, changing
  crate metadata, versioning, changelog/release notes, publish readiness, or v1.0 release
  evidence.
- Use `scena-git-github` when working with branches, commits, tags, GitHub issues, pull
  requests, workflow runs, releases, or local-vs-remote state proof.
- Use `scena-remote-builder` when compiling, running cargo tests, running doctor, or
  preparing remote proof from the Hetzner CPU build machine.

## Skill Trigger Guidance

- Scope, milestone, and "should this belong in scena?" questions start with
  `scena-rfc-governance`.
- Implementation work starts with `scena-renderer-architecture`, then adds the narrower
  skill for the touched area: `scena-gltf-assets` for import/animation assets,
  `scena-renderer-quality` for tests/visual/browser/performance proof, and `scena-doctor`
  for enforceable drift checks.
- Any review finding, silent fallback, or repeated mistake must be checked for doctor
  coverage. If it can be detected from source, docs, manifests, or gate artifacts, extend
  `xtask doctor` before considering the finding closed.
- User-visible API, renderer behavior, docs/tutorial, crate metadata, publish readiness,
  or v1.0 release evidence also uses `scena-release-hygiene`.
- Compile, test, clippy, doc, publish dry-run, and doctor proof must use
  `scena-remote-builder` so the heavy Rust work runs on the remote machine.
- Branch, commit, tag, issue, PR, workflow, release, crash-recovery, or local-vs-remote
  proof work also uses `scena-git-github`.
- When multiple skills apply, use all relevant skills in this order: RFC scope, renderer
  architecture, area-specific implementation, quality proof, doctor enforcement, release
  hygiene, remote builder proof, then Git/GitHub follow-through.

## Architecture Rules

- Keep renderer logic out of `platform`; platform modules are adapters only.
- Keep asset fetching/parsing/cache ownership in `assets`; `Renderer` must not fetch assets.
- Keep scene graph state in `scene`; `Renderer` consumes prepared scene/resource state.
- Do not add simulation, robotics, PLC, process, or physics concepts to `scena`.
- Prefer typed handles and structured errors over stringly contracts or silent fallbacks.
- Do not hide async fetches, shader compilation, or GPU upload inside `render()`.
- Follow SOLID/KISS: every public feature has one owner module, no catch-all manager/engine
  types, no global singleton state, and no abstraction added only for hypothetical future
  flexibility.

## Unit Test First Rule

- For production implementation code, add or update the narrowest unit or integration test
  that captures the expected contract before changing the implementation.
- Run the focused test and confirm it fails for the expected reason on the remote builder
  before patching production code.
- Implement the smallest code change that makes the focused test pass, then run the broader
  required gates.
- If a change cannot be meaningfully unit-tested first, record why in the checklist or final
  handoff and add the closest deterministic proof before implementation.
- Do not mark a checklist implementation item complete without naming the test-first proof
  or the documented exception.

## Test Flow Discipline

Do not turn every edit into a full release-gate loop. Use an evidence ladder and stop
expanding when the current proof is sufficient for the risk:

1. Reproduce or pin the exact failure first. Prefer one focused test, one CLI proof, one
   browser probe, or one rendered-output comparison that would fail on the old behavior.
2. Patch the smallest surface that can make that proof pass.
3. Rerun the same focused proof. If it still fails, keep iterating there; do not broaden to
   unrelated suites.
4. Add scoped gates only for the touched surface: formatting after Rust edits, the affected
   integration test after CLI/schema edits, `doctor --full` after doctor/checklist/schema
   pins, and browser proof only for browser-visible changes.
5. Run the full release chain once at a natural checkpoint: before a release handoff, before
   a requested commit that claims broad readiness, after cross-backend renderer changes, or
   when the user explicitly asks for it. Do not run the same full chain after every small
   patch in a multi-step investigation.

For checklist or multi-slice work, validate and report one logical unit at a time. The
default per-unit rhythm is focused proof -> scoped gate(s) -> commit-or-handoff for that
unit. Treat boilerplate "run all gates" text in a checklist as the checkpoint requirement
for the completed batch unless the current user request explicitly says to run the full
chain for each unit. Do not keep running broader suites to compensate for an unclear root
cause; reduce the repro until it measures the defect.

When reporting validation, say which tier was run and why broader gates were or were not
needed. A focused failing proof is more useful than hours of unrelated green tests.
If a broad gate already passed for the current diff and no file in that gate's risk surface
changed afterward, do not rerun it just to generate a newer timestamp; report the existing
evidence and the unchanged surface.

Use a short validation ledger in handoffs:

- `focused`: exact reproducer/proof and result.
- `scoped`: only the gate(s) added because of touched files.
- `full`: release-level gates, only when warranted, with the reason.
- `skipped`: broader gates intentionally not run, with the risk reason.

## Investigation Circuit Breakers

Before changing code for a failed test, browser proof, benchmark, workflow, or release,
classify the failure as exactly one of: product defect, test-harness defect, environment
failure, policy failure, or provenance failure. Preserve the exact failing command, source
commit, complete job log, and relevant artifact before proposing a remedy. A theory is not
a root cause until it predicts a focused observation that distinguishes it from the other
classes.

- After two failed remediation attempts with the same failure signature, the investigation
  circuit breaker trips. Freeze production and harness changes; do not make a third patch
  until a smaller reproducer or captured diagnostic falsifies at least one competing cause.
- After 30 minutes without a proven cause, stop and write a checkpoint with elapsed time,
  exact signature, attempted remedies, current classification, missing evidence, and the
  next single discriminating probe. Do not compensate with a broader suite.
- Collect every failed job from one GitHub run before editing. Use
  `scripts/collect_ci_failure_evidence.sh <run-id>` and batch all known fixes into one
  release-candidate push. Run one deciding full matrix; do not push serial guesses or rerun
  a passing matrix merely for a newer timestamp.
- On user-operated hardware, provide one checksum-verified bundle and one runner command.
  Windows GPU release proof must use `scripts/run_windows_complete_hardware_proof.ps1` and
  its automatically uploaded archive. Do not relay individual patch overlays or ask the
  user to copy diagnostic commands back and forth. A second user-assisted run requires
  explicit user approval after a written root-cause checkpoint.
- Strict wall-clock regression thresholds belong only on controlled or dedicated hardware.
  GitHub-hosted runners use an explicit report-only timing policy while sample validity and
  deterministic allocation budgets remain blocking. GitHub-hosted M9 commands must set
  `SCENA_M9_TIMING_POLICY=report-only-hosted`. Never silently widen a baseline to make a
  shared-runner failure pass.

The validation ledger must also record elapsed investigation time, remediation-attempt
count, release-candidate push count, full-matrix run count, and user-required action count.
If any count exceeds the limits above, stop and report the process failure before doing
more work.

## Validation

Heavy Rust work runs on the Hetzner CPU builder by default:

- SSH alias: `scena-builder`
- Shared remote repo observation path: `/home/johannes/projects/scena`. It is not
  provisioned on the maintained builder as of 2026-07-17 and must never be assumed to exist.
- Default validation path: `$HOME/.cache/codex-worktrees/scena-<task-slug>`, paired with
  `CARGO_TARGET_DIR=$HOME/.cache/codex-targets/scena-<task-slug>`.
- Do not run local `cargo build`, `cargo check`, `cargo test`, `cargo clippy`, `cargo doc`,
  wasm builds, npm browser proof, or long-running render probes unless the user explicitly
  permits it. Local inspection commands such as `rg`, `sed`, `git diff`, and `git status`
  are fine.
- Before every sync and cargo gate, run the checked-in preflight from the local canonical
  checkout. It reports `shared_checkout_status=missing` when the old documented shared path
  is absent and always chooses `validation_mode=isolated`, `validation_path=...`, and
  `cargo_target_dir=...` without mutating remote state:

```bash
ssh scena-builder 'bash -s -- <task-slug>' < scripts/scena_remote_builder_preflight.sh
```

- Mirror the exact local tree to the returned isolated `validation_path` with
  `rsync --delete --exclude .git --exclude target`. Then manually copy root `AGENTS.md` and
  the complete `.codex/skills/**` tree again and verify both with `sha256sum` before edits or
  gates. A full-tree rsync does not substitute for this explicit bootstrap.
- Report `validation_path`, `CARGO_TARGET_DIR`, shared-checkout status, canonical source,
  branch, HEAD, and agent-file hashes. Clean only that task-scoped snapshot/cache when done.

- Prefer a task-scoped target cache, for example
  `CARGO_TARGET_DIR=$HOME/.cache/codex-targets/scena-<task-slug>`. If the preflight shows
  low space or a previous run failed with `No space left on device`, `Disk full`, or
  `Disk quota exceeded`, remove only that task-scoped cache and rerun the preflight. Do not
  delete unrelated caches, checkouts, or user files unless the user explicitly approves it.
- If `/tmp` is the constrained filesystem, set a task-local `TMPDIR` under the validation
  checkout or target cache before rerunning.
- Do not put private SSH key material or cloud credentials in this repo.

Use this command shape for remote gates:

```bash
ssh scena-builder 'cd "$HOME/.cache/codex-worktrees/scena-<task-slug>" && env CARGO_TARGET_DIR="$HOME/.cache/codex-targets/scena-<task-slug>" <command>'
```

Use a tiered validation flow. Do not default to the full release gate chain for every
small edit:

1. Focused proof first: run only the test, CLI command, doctor rule, browser proof, or
   rendered-output check that directly exercises the changed contract. For test-only or
   doctor-pin-only changes, this focused proof is the main signal.
2. Scoped gates next: run the narrowest compile/lint/check gate that can catch regressions
   in the touched surface. Examples: `cargo fmt --check` after Rust edits, one integration
   test file after CLI/recipe changes, `doctor --full` after doctor/checklist/schema pins,
   or a browser lane only after browser/WASM-visible changes.
3. Full release gates only when warranted: run the full cargo/clippy/test/doc/browser/publish
   chain for production renderer behavior, public API/schema changes, release-ready work,
   cross-backend rendering changes, or when the user explicitly asks for release-level proof.

For a sequence of related renderer fixes, run the full chain at the integration checkpoint
after the focused proofs and scoped gates have passed. If no file in a broad gate's risk
surface changed since the last passing run, reuse that evidence and say so instead of
spending another cycle.

For a normal production code change, the default remote gate set is:

```bash
ssh scena-builder 'cd "$HOME/.cache/codex-worktrees/scena-<task-slug>" && env CARGO_TARGET_DIR="$HOME/.cache/codex-targets/scena-<task-slug>" cargo fmt --check'
ssh scena-builder 'cd "$HOME/.cache/codex-worktrees/scena-<task-slug>" && env CARGO_TARGET_DIR="$HOME/.cache/codex-targets/scena-<task-slug>" cargo clippy --all-targets -- -D warnings'
ssh scena-builder 'cd "$HOME/.cache/codex-worktrees/scena-<task-slug>" && env CARGO_TARGET_DIR="$HOME/.cache/codex-targets/scena-<task-slug>" cargo test'
ssh scena-builder 'cd "$HOME/.cache/codex-worktrees/scena-<task-slug>" && env CARGO_TARGET_DIR="$HOME/.cache/codex-targets/scena-<task-slug>" cargo run -p xtask -- doctor --full'
```

Use that normal set as the default only after the focused proof is green. During active
debugging, run the focused proof first and repeat only that proof until the defect is
understood.

For a narrow proof-only change, do not run the whole suite just to spend time. Run the
focused proof, `cargo fmt --check` if Rust formatting could change, and `doctor --full` only
if doctor/checklist/schema evidence changed. State clearly which broader gates were not run
and why.

For browser, WebGPU/WebGL2, visual, or 3D rendering changes, add rendered-output proof.
Prefer Playwright or a deterministic headless harness. Do not declare a visual fix from
unit tests alone. The Hetzner builder is a CPU build/test machine; use a real GPU machine
for GPU-specific proof when hardware acceleration matters.

When a bug or review finding exposes a silent-failure family, add or extend a doctor rule
when the pattern can be checked from source, docs, manifests, or gate artifacts.

## Git And Release Hygiene

- Do not commit, tag, push, merge, close issues, or delete branches unless explicitly asked.
- Treat local checkout state, remote branch state, GitHub workflow state, and published
  release state as separate evidence.
- If no GitHub remote exists yet, report local git evidence and state that GitHub proof is
  unavailable.
- For release-ready work, keep crate metadata, docs/specs/examples, release gates, and
  public API evidence aligned before handoff.

## Subagents

Claude Code subagents live in `.claude/agents/`. Routing guidance lives in
`docs/agents/subagents.md`.
