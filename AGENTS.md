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

## Validation

Heavy Rust work runs on the Hetzner CPU builder by default:

- SSH alias: `scena-builder`
- Remote repo path: `/home/johannes/projects/scena`
- Keep the remote checkout matched to the work being validated. If local changes are not
  committed and pushed, verify the remote tree is clean, then mirror the local working tree
  to the remote repo with `rsync`, excluding `.git` and `target`.
- Do not put private SSH key material or cloud credentials in this repo.

Use this command shape for remote gates:

```bash
ssh scena-builder 'cd "$HOME/projects/scena" && <command>'
```

For any code change, run:

```bash
ssh scena-builder 'cd "$HOME/projects/scena" && cargo fmt --check'
ssh scena-builder 'cd "$HOME/projects/scena" && cargo clippy --all-targets -- -D warnings'
ssh scena-builder 'cd "$HOME/projects/scena" && cargo test'
ssh scena-builder 'cd "$HOME/projects/scena" && cargo run -p xtask -- doctor --full'
```

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
