# Full-repo review v1.8.0 remediation checklist

Created: 2026-07-20

Status: **implementation complete on the CPU builder; final physical-GPU,
native-surface, macOS, Windows, clean-tree, and staged release-bundle evidence
remains open and is not counted as passed**

Source baseline: `main@7b4fc9ca77e12fd12a69fab92650e1e46ee10354`, package and
tag version `1.8.0`

Canonical charter: `docs/RFC-rust-3d-renderer.md`

This checklist turns the supplied v1.8.0 full-repository review and the
independent claim-by-claim audit into an execution plan. It covers confirmed
correctness defects, fail-open gates, CLI and agent usability, weaker proof
lanes, measured performance work, documentation drift, and separately governed
feature proposals.

The review's approximate size claim is close for Rust source: the audited tree
contained about 282,728 physical Rust lines. Its schema-count wording is stale:
the current catalog exposes 60 entries, not 45. Performance rankings are
hypotheses until the benchmark rows below establish distributions. Feature
proposals are not bugs and do not block the correctness remediation release.

## 0. Non-negotiable execution contract

### 0.1 Test cadence: focused for every item, full only at the end

Do **not** run the complete workspace, browser, GPU, rustdoc, publish, or
release chain after each small fix. Use this sequence for every logical item:

1. Add the narrowest deterministic test, CLI reproducer, browser probe, rendered
   comparison, or benchmark that fails on the old behavior.
2. Run only that proof on the bootstrapped remote builder or required GPU host
   and record the expected failure.
3. Classify the failure as exactly one of: product defect, test-harness defect,
   environment failure, policy failure, or provenance failure.
4. Make the smallest production change that satisfies the contract.
5. Rerun the same focused proof until it passes.
6. Run only the scoped gates implied by the touched files.
7. Record the item ledger before moving to the next item.

Per-item scoped gates are:

- Rust source changed: `cargo fmt --all --check` at the end of the logical item
  or small related group, not after every edit.
- One CLI/integration surface changed: run only its affected integration-test
  target and feature combination.
- Browser JavaScript or WASM-visible behavior changed: run only the affected
  browser proof on the relevant backend; use real hardware where the claim
  requires hardware.
- Doctor, workflow, checklist, schema, fixture, README, or release contract
  changed: run the affected doctor mutation test when one exists and one
  `cargo run -p xtask -- doctor --full` after the related documentation/gate
  group.
- Performance code changed: run the named focused benchmark distribution on a
  controlled host. GitHub-hosted wall-clock results remain report-only;
  deterministic allocation regressions remain blocking.
- Public API changed: update its compile test and public-api fixture now;
  rustdoc, semver, and publish dry-run wait for the final checkpoint.

The work is divided into four implementation checkpoints. Checkpoints A-C use
focused and scoped evidence only. Checkpoint D is the **single full integration
and release-gate run** for the complete remediation batch.

- [x] Checkpoint A: fail-open/catastrophic correctness items C01-C07 are green
  under their focused proofs and scoped gates. Do not run the full suite.
- [x] Checkpoint B: remaining correctness, import, surface, and API items
  C08-C21 are green under focused proofs and scoped gates. Do not run the full
  suite.
- [ ] Checkpoint C: CLI/agent, proof, performance, and documentation items
  A01-A09, Q01-Q06, P01-P08, and D01-D06 are green under focused proofs and
  scoped gates. Do not run the full suite.
- [ ] Checkpoint D: run the full gate set in section 11 exactly once after all
  in-scope remediation files are stable.

If Checkpoint D exposes a failure, reduce it to a focused reproducer, fix it,
and rerun the failing gate. Rerun the complete Checkpoint D only when a fix
changed a surface covered by another full gate. Reuse still-valid results for
unchanged surfaces and say so in the ledger.

### 0.2 Circuit breakers and evidence preservation

- [x] Preserve the exact command, source commit, complete failing output, input
  fixture hash, backend/adapter, and generated artifact before changing code.
- [x] After two remediation attempts with the same signature, freeze production
  and harness edits until a smaller discriminating proof rules out a competing
  cause.
- [x] After 30 minutes without a proven cause, record elapsed time, attempts,
  classification, missing evidence, and the next single probe.
- [x] Never turn unavailable GPU hardware into a pass. Separate optional smoke
  tests from required hardware evidence.
- [x] Never widen visual or timing thresholds merely to make a shared runner
  green.
- [x] Every item ledger records investigation time, remediation attempts,
  release-candidate pushes, full-matrix runs, and user-required actions.

### 0.3 Checkout and remote-builder bootstrap

Before every remote sync or cargo gate:

- [x] Record canonical source path, destination path, branch, and HEAD.
- [x] Run `scripts/scena_remote_builder_preflight.sh` through `scena-builder`
  for a task-specific slug.
- [x] Mirror the exact local tree to the returned isolated validation path,
  excluding `.git` and `target`.
- [x] Manually copy root `AGENTS.md` and the complete `.codex/skills/**` tree
  from `/home/johannes/projects/scena`; a general rsync does not substitute for
  this step.
- [x] Compare canonical and destination hashes for `AGENTS.md` and the complete
  skills tree before any build or test.
- [x] Read the destination `AGENTS.md` and every required skill.
- [x] Use the task-specific external `CARGO_TARGET_DIR` reported by preflight.
- [ ] Clean only the task-scoped remote snapshot/cache when the batch is done.

### 0.4 Per-item validation ledger template

Copy this under each completed item:

- `focused red`: command, test/probe, expected failure, commit, host/backend.
- `classification`: product, harness, environment, policy, or provenance.
- `implementation`: files and contract changed.
- `focused green`: same proof and result.
- `scoped`: only additional gates required by touched files.
- `full`: `deferred to Checkpoint D` until section 11 is run.
- `skipped`: broader gates intentionally not run and why.
- `counts`: elapsed investigation time; remediation attempts; RC pushes;
  full-matrix runs; user actions.

### 0.5 Preserve proven foundations; do not churn them accidentally

- [x] Keep native/browser readback row-padding alignment covered while changing
  browser transfer and capture paths.
- [x] Keep reversed-Z conventions consistent across preparation, clipping,
  projection, depth tests, semantic depth output, and CPU/GPU comparisons.
- [x] Preserve the asynchronous two-slot readback lifecycle and prove new
  recovery logic does not turn it into a blocking single-slot path.
- [x] Reuse the existing picking BVH and its ownership boundaries; do not add a
  parallel spatial index without measured need.
- [x] Keep q01's reference thresholds and rejected-mutation model as the quality
  baseline while upgrading weaker proofs.
- [x] Extend rather than bypass the environment-flag documentation gate when
  backend-selection variables change.
- [x] Keep doctor's known-bad fixtures executable and fail-closed; comments or
  substring pins are not substitutes for negative tests.
- [x] Record an explicit regression test before refactoring any of these
  surfaces as part of another item.

## 1. Priority and dependency order

1. C01-C07: parallel-render cancellation, portable templates, recipe routing,
   release fail-open behavior, primitive winding, default black success, and
   browser transfer correctness.
2. C08-C14: animated coordinate conversion, texture reload/cache policy,
   device/surface recovery, CPU clipping, glTF semantic coverage, and strict
   authored transform metadata.
3. C15-C21: public API traps, framing, primitive UVs, pointer capture, WASM
   capability degradation, environment-driven execution mode, and CAD
   inspection presentation lighting.
4. A01-A09: one recipe resolution path and a coherent machine-facing CLI.
5. Q01-Q06: replace smoke-only visual/release claims with mutation-tested
   evidence.
6. P01-P08: measure first, then remove proven hot-path waste.
7. D01-D06: finish user-facing docs, README, examples, version pins, changelog,
   and roadmap consolidation for the behavior that actually shipped.
8. Checkpoint D: one complete integration/release run.
9. F01-F08: optional feature projects, each reopened only through its own RFC,
   demand, API, proof, and final integration checkpoint.

Do not let multi-release optional features delay the correctness remediation.

## 2. Critical and fail-closed correctness

### C01 — Force completion of every parallel CPU render band (review B1)

Owner: `src/render/cpu_render.rs` and CPU transparency tests.

- [x] Add a focused regression with order-independent transparency enabled,
  more bands than worker threads, deterministic contention, and sentinel pixels
  in every row band.
- [x] Prove the old Rayon `any` consumer can stop scheduling after the first
  band reporting an OIT pass, leaving at least one band uncleared or undrawn.
- [x] Replace the short-circuit consumer with a full traversal/reduction.
- [x] Preserve the intended meaning of `oit_passes`; do not accidentally count
  worker bands when the statistic is meant to count actual OIT passes.
- [x] Assert every band is cleared, drawn once, and resolved once for opaque,
  transparent, and mixed scenes.
- [x] Add serial/parallel pixel parity under deterministic global-pool
  contention. A nested fixed-size pool is deliberately not used because the
  production worker policy disables nested Rayon parallelism; the test instead
  occupies all but one production-pool worker and compares complete buffers.
- [x] Add a doctor source guard if a side-effectful parallel band map can be
  mechanically detected being consumed by a short-circuit combinator.
- [x] Document no public behavior unless stats semantics change; if they do,
  update rendering/stats docs and the JSON fixture.
- [x] Acceptance: the old implementation fails the focused test; the fixed
  implementation produces identical complete frames across repeated runs.

Validation ledger (2026-07-20):

- `focused red`: on `scena-builder` at source baseline
  `7b4fc9ca77e12fd12a69fab92650e1e46ee10354`,
  `cargo test --lib
  render::cpu_render::tests::cpu_parallel_oit_completes_every_row_band_when_rayon_is_contended
  -- --exact --nocapture` failed at the complete-frame comparison. With all
  but one Rayon worker occupied, the old `.any(...)` processed one OIT band and
  returned while other row bands retained sentinel/incorrect data.
- `classification`: product defect. The failure directly distinguished the
  short-circuit consumer from scheduling, fixture, and environment theories.
- `implementation`: `src/render/cpu_render.rs` now fully consumes row-band
  results with `max()` over their 0/1 OIT result, preserving the renderer-wide
  `oit_passes` statistic. The test covers mixed opaque/transparent drawing,
  buffer clearing, OIT resolve, and complete serial/parallel parity under
  deterministic contention.
- `focused green`: the same exact test passed 1/1 after the reduction change.
  The final mixed-scene form also passed 1/1.
- `doctor red/green`: the exact xtask mutation
  `app::tests_36::c01_doctor_rejects_short_circuit_parallel_band_consumption`
  first failed with no findings, then passed after
  `PF09-DETERMINISTIC-PARALLEL-WORK` required the full-consumption reduction and
  focused test while forbidding the old OIT `.any(...)` pattern.
- `scoped`: `cargo test --lib render::cpu_render::tests:: -- --nocapture`
  passed 4/4; `cargo test -p xtask app::tests_36:: -- --nocapture` passed 5/5;
  remote `cargo fmt --all --check` passed; remote
  `cargo run -p xtask -- doctor --full` reported `mode=Full status=pass`.
  `CHANGELOG.md` records the fix under `[Unreleased]`. OIT stats semantics did
  not change, so no stats schema or rendering-guide migration was needed.
- `full`: deferred to Checkpoint D. No workspace-wide test, Clippy, rustdoc,
  browser/GPU matrix, performance distribution, package, or publish gate was
  run for this focused CPU correction.
- `bootstrap`: canonical source `/home/johannes/projects/scena`, isolated
  destination
  `/home/johannes/.cache/codex-worktrees/scena-full-review-v18-checklist`, and
  target `/home/johannes/.cache/codex-targets/scena-full-review-v18-checklist`.
  Preflight reported `shared_checkout_status=missing`; `AGENTS.md` hash
  `d0ed4759...` and skills-tree hash `d121366e...` matched after every sync.
- `counts`: about 24 minutes investigation; one production remediation attempt;
  zero release-candidate pushes; zero full-matrix runs; zero user-required
  actions.

### C02 — Make every agent template and environment preset portable (review B2)

Owner: `src/bin/scena/examples_agent/`, `src/assets/environment_preset.rs`,
packaging, examples, and getting-started docs.

- [x] Add an end-to-end test that installs/runs the built CLI from a temporary
  directory with no repository-relative assets.
- [x] In that directory, generate every agent template, validate it, build it,
  and render it using the documented commands.
- [x] Pin the old failure: generated recipes validate but rendering returns a
  `policy_violation` for `tests/assets/environment/polyhaven/...hdr`.
- [x] Replace repository-relative runtime dependencies with a licensed,
  package-included builtin environment representation. Verify crate/package
  size and license attribution before choosing `include_bytes!` or an
  equivalent generated/builtin asset path.
- [x] Resolve named presets through `Assets`; do not make `Renderer` fetch or
  own asset bytes.
- [x] Never overwrite an explicitly authored `scene.environment`, including
  `{"kind":"default"}` or an explicit URI.
- [x] Define deterministic precedence: authored environment > requested
  template preset > portable builtin default.
- [x] Ensure missing optional presentation assets degrade with a structured
  diagnostic, while explicitly required assets fail closed.
- [x] Add package-content tests proving every bundled preset byte/license file
  is included in `cargo package`.
- [x] Update README quick-start, `docs/getting-started.md`, `docs/examples.md`,
  `docs/assets.md`, troubleshooting, and agent-builder instructions so every
  command works after `cargo install scena` from an arbitrary CWD.
- [x] Add an `[Unreleased]` changelog entry when the fix lands.
- [x] Acceptance: every template completes generate -> validate -> build ->
  render outside a checkout without undeclared roots or repository files.

Validation ledger (2026-07-20):

- `focused red`: on `scena-builder` at source baseline
  `7b4fc9ca77e12fd12a69fab92650e1e46ee10354`, the exact
  `scena_examples_agent_primitive_flow_runs_from_an_unrelated_working_directory`
  test passed schema validation but failed `recipe build` with
  `policy_violation` at `$.scene.environment.uri` for the repository-relative
  Poly Haven HDR. The explicit-environment test separately showed
  `{"kind":"default"}` being overwritten, and the catalog-wide test failed
  validation at `$.imports[0].uri` for the repository-relative material
  variants fixture.
- `classification`: product defects. The generated recipes depended on the
  checkout CWD, presentation defaults overwrote authored intent, and imported
  template fixtures had the same portability problem as the HDR. A further
  focused probe found an additional product defect missed by the review:
  `optional_environment_skipped` used severity `error`, contradicting its
  “build continues” help and making the build fail.
- `implementation`: environment preset loading now uses an exact embedded
  `scena://bundled/environment/...` source owned by `Assets`; the three small
  scena-authored glTF/GLB template fixtures use an exact embedded asset catalog.
  Recipe policy bypasses local-root resolution only for cataloged builtin scene
  identifiers and still enforces byte/texture limits. Template defaults emit
  `environment.preset:"studio"` with entry-only precedence, and optional URI
  environments now produce a warning while required URIs remain errors.
  Cargo package allowlists, CC0/dual-license attribution, README, getting
  started, examples, assets, troubleshooting, LLM guide/skill, and changelog
  were updated together.
- `focused green`: the primitive arbitrary-CWD flow and explicit-environment
  tests pass individually; all 13 template/starter names complete generate ->
  validate -> build -> render from an unrelated CWD in 46.88 seconds. The
  preset catalog/load/render test file passes 3/3. Recipe-policy proofs reject
  unknown `scena://` identifiers and undersized builtin budgets. The optional
  environment CLI proof now exits zero with
  `optional_environment_skipped`; the existing required-environment proof
  remains fail-closed.
- `package`: `cargo package --list --allow-dirty` contains both license files,
  the neutral fixture, Poly Haven HDR, and all three embedded template source
  assets. The environment package-size test remains below the 2,000,000-byte
  budget. `C02-PORTABLE-AGENT-ASSETS` has a known-bad mutation that removes a
  license allowlist entry and fails closed.
- `scoped`: `cargo test --features scene-host --test
  scena_cli_agent_templates` passed 7/7 in 120.36 seconds;
  `round_c_environment_presets` passed 3/3; focused recipe-policy, FR02, doctor
  mutation, and CLI-isolation tests passed; `cargo fmt --all --check` and
  `xtask doctor --full` passed. A scoped clippy run passed for the library,
  CLI, and three affected integration targets with only the repository's three
  pre-existing scene-host lint classes explicitly allowed.
- `full`: deferred to Checkpoint D. No repository-wide cargo test, browser
  matrix, docs build, publish verification, or release matrix was run for C02.
- `skipped`: the first clippy attempt with unconditional `-D warnings` stopped
  on unchanged `large_enum_variant`, `too_many_arguments`, and
  `result_large_err` debt in scene-host code. Those unrelated refactors were
  not folded into C02; every other warning remained denied in the scoped rerun.
  An initial test command compiled zero tests because `scene-host` was omitted,
  and an initial package-list wrapper lacked remote `rg`; neither was counted
  as product evidence.
- `bootstrap`: canonical source `/home/johannes/projects/scena`, branch `main`,
  HEAD `7b4fc9ca77e12fd12a69fab92650e1e46ee10354`; isolated destination
  `/home/johannes/.cache/codex-worktrees/scena-full-review-v18-checklist` and
  target `/home/johannes/.cache/codex-targets/scena-full-review-v18-checklist`.
  Preflight reported `shared_checkout_status=missing`; `AGENTS.md` hash
  `d0ed4759...` and the updated skills-tree hash `36663a08...` matched after
  every sync.
- `counts`: about 48 minutes investigation; two production remediation
  attempts; two doctor-checker token corrections after a captured smaller
  reproducer; zero release-candidate pushes; zero full-matrix runs; zero
  user-required actions.

### C03 — Route all recipe-consuming commands through one policy-aware build
path (review B11)

Owner: scene recipe/SceneHost build code and CLI input adapters.

- [x] Add a two-import recipe fixture whose second import has a visible,
  machine-assertable contribution.
- [x] Prove legacy `render`, `inspect`, and `doctor`/diagnose paths currently
  consume only the first import or misroute the recipe.
- [x] Route every recipe input through the same recipe parser, validator,
  `RecipeBuildPolicy`, resource resolver, and SceneHost build manifest.
- [x] Keep raw glTF/GLB input on the direct asset path; dispatch by parsed input
  kind, not by trying unrelated parsers and returning their diagnostics.
- [x] Remove duplicate first-import-only scene construction from CLI adapters.
- [x] Assert render, inspect, diagnose, doctor, verify, repair, build, capture,
  and AOV commands agree on imports, policy roots, resources, names, and
  diagnostics.
- [x] Make content loss impossible: a rejected import must produce a nonzero
  command result, not `ok:true` with a partial scene.
- [x] Add a doctor rule pinning the one recipe-build owner and forbidding new
  recipe assembly in CLI adapters.
- [x] Update CLI help, recipe docs, troubleshooting, and migration notes.
- [x] Acceptance: the two-import fixture yields the same manifest and visible
  content through every applicable command.

Validation ledger (2026-07-20):

- `focused red`: on `scena-builder` at source baseline
  `7b4fc9ca77e12fd12a69fab92650e1e46ee10354`,
  `cargo test --features "inspection scene-host" --test scena_cli_recipe
  imports_only_recipe_commands_build_every_import -- --exact --nocapture`
  failed because `recipe build` reported two imports while legacy `inspect`
  omitted `imports` entirely and reported one visible drawable. The independent
  `doctor_recipe_checks_policy_for_every_import` reproducer also failed because
  doctor returned success after checking only import 1 while import 2 violated
  the root policy.
- `classification`: product defect. The parsed recipe was deliberately reduced
  to two imports with no authored directives, distinguishing first-import CLI
  dispatch from parser, asset, renderer, and environment failures.
- `implementation`: `ResolvedSceneInput::is_recipe` now dispatches every parsed
  recipe to the existing SceneHost recipe builder; recipe inputs no longer
  synthesize `asset`/`transform` from `imports[0]`. Render, inspect, diagnose,
  appearance, animation, interaction, doctor, and repair preserve structured
  recipe-build rejection on stdout. Raw glTF/GLB retains the direct viewer or
  asset-doctor path. Appearance variant discovery was also changed from the
  first manifest import to the deduplicated variants of every import.
- `focused green`: the exact two-import test passed 1/1 and now observes two
  imports/two visible drawables through build, inspect, render, diagnose, and
  doctor. `recipe_verifiers_resolve_capabilities_from_the_second_import` passed
  1/1, selecting the second import's `noon` material variant and playing the
  second import's `MoveTriangle` clip. The 12-command
  `recipe_commands_check_policy_for_every_import` matrix passed 1/1: render,
  inspect, diagnose, doctor, repair, all three verify verbs, recipe build,
  recipe render, capture, and AOV all rejected the same
  `$.imports[1].uri` policy violation with nonzero structured output.
- `doctor red/green`: the source mutation in
  `app::tests_36::c03_doctor_rejects_first_import_recipe_command_routing`
  passed 1/1: adding `.imports.first()` to a guarded CLI adapter produces a
  `C03-CANONICAL-RECIPE-COMMAND-ROUTING` finding. The rule also pins parsed-kind
  dispatch, structured build rejection, the SceneHost builder, every relevant
  CLI adapter, and all three focused tests.
- `scoped`: `tests/fr04_cli_schema_matrix.rs` passed 6/6 after help and evidence
  rows were expanded for recipe-build and validation outcomes;
  `scena_cli_missing_assets_emit_json_not_command_errors` passed 1/1 for the
  unchanged raw-asset path; the direct-asset interaction verification passed
  1/1. `cargo check --bin scena --features inspection` passed without
  `scene-host`, and affected-bin Clippy passed with only the three documented
  pre-existing lint classes allowed (`large_enum_variant`,
  `too_many_arguments`, `result_large_err`). README, API/schema docs,
  getting-started, troubleshooting, the Three.js migration guide, app-builder
  skill/reference, roadmap wording, and `[Unreleased]` changelog were updated.
  Remote `cargo fmt --all --check` passed. Remote `cargo run -p xtask -- doctor
  --full` initially identified only the new checker's xtask module-size
  violation; after moving the rule into its owned `command_routing` submodule,
  the exact mutation remained green and full doctor reported
  `mode=Full status=pass`.
- `full`: deferred to Checkpoint D. No workspace-wide test, rustdoc,
  browser/GPU matrix, performance distribution, package, publish, or release
  gate was run for this focused CLI correction.
- `bootstrap`: canonical source `/home/johannes/projects/scena`, isolated
  destination
  `/home/johannes/.cache/codex-worktrees/scena-full-review-v18-checklist`,
  branch `main`, HEAD `7b4fc9ca77e12fd12a69fab92650e1e46ee10354`;
  shared checkout absent; `AGENTS.md` hash
  `d0ed47595f6b6a6ec733651dfde650b7893aa2939d6137dc2f7eeade7528ac7a`
  and skills aggregate
  `36663a08871bc080d27cffcb0174bd6dfd06762b741c640aade8241a7f56b0de`
  matched after every explicit bootstrap.
- `counts`: about 50 minutes investigation; two production remediation
  attempts; three distinct test-fixture corrections, one Clippy correction,
  and one doctor module-split correction; zero release-candidate pushes; zero
  full-matrix runs; zero user-required actions. No failure signature received
  two failed remediation attempts.

### C04 — Make release-readiness fail closed on missing artifact roots and Linux
Vulkan output (review B12)

Owner: `crates/xtask/src/app/release/`, release workflows, release-gate docs.

- [x] Add focused xtask tests with `SCENA_RELEASE_ARTIFACT_ROOT` unset, empty,
  nonexistent, unreadable, and containing an incomplete bundle.
- [x] Prove the old path reports pass with zero evidence validated.
- [x] Require an explicit `--artifact-root` or environment variable for every
  evidence-consuming readiness/staging command; missing configuration is a
  structured policy failure.
- [x] Report the resolved artifact root and validated evidence count in the
  result envelope.
- [x] Add `m9-platform/linux-native-vulkan/rendered-output.json` to the required
  existence list, not merely freshness/metadata validation.
- [x] Audit all platform/backend artifact lists for one canonical owner so
  existence, schema, provenance, and freshness cannot drift separately.
- [x] Add negative mutations for omitted Linux output, wrong lane/backend,
  stale commit, stale digest, empty evidence set, and substituted files.
- [x] Add a doctor rule pinning CLI usage, workflow input, required artifact
  rows, and negative tests.
- [x] Update `docs/specs/release-gates.md`, command usage, release runbook, and
  troubleshooting. Never describe zero validated artifacts as readiness.
- [x] Acceptance: no readiness/staging success is possible without the complete
  explicit artifact root and all required source-bound files.

Validation ledger (2026-07-20):

- `focused red`: on `scena-builder` at source baseline
  `7b4fc9ca77e12fd12a69fab92650e1e46ee10354`,
  `env -u SCENA_RELEASE_ARTIFACT_ROOT cargo run -p xtask --
  release-readiness` exited 0 and printed `scena release readiness: pass`
  despite resolving no artifact root and validating zero evidence.
- `classification`: policy defect. The exact command isolated missing input
  configuration from artifact parsing, workflow availability, and hardware;
  the old early return skipped the evidence consumer entirely.
- `implementation`: `release-readiness` now accepts
  `--artifact-root <staged-root>` with CLI precedence over the environment,
  rejects missing/empty/non-directory/unreadable roots, and prints
  `scena.release_readiness.v1` with the resolved root/source and discovered,
  required, and validated counts. Success requires a positive validated count
  equal to the canonical required count and zero semantic/provenance findings.
  Local and publish-dry-run helpers pass an explicit root. The existing staging
  command remains explicit through its required input/output positionals.
- `inventory`: `REQUIRED_RELEASE_ARTIFACT_SUFFIXES` remains the canonical
  existence owner, every specialized validation list is asserted to be its
  subset, and Linux Vulkan `rendered-output.json` is now in both the existence
  and native-GPU semantic-validation lists. The new schema row is aligned in
  the live catalog and both schema catalog goldens.
- `focused green`: `app::tests_41` passed 5/5 for unset/empty selection,
  missing/incomplete/unreadable roots, positive-count policy, specialized-list
  ownership, the Linux existence row, and the C04 doctor mutation. The live
  unset-environment command exited 1 with `artifact_root: null`,
  `validated_artifact_count: 0`, and `RELEASE-READY-ARTIFACT-ROOT`.
- `negative/scoped`: the 25-test `release_readiness` filter passed, including
  empty evidence, missing files, wrong commit, stale timestamp, invalid native
  proof, failed/forged command records, constant visual output, and incomplete
  capability rows. Canonical staging passed 1/1 while exercising its stale
  digest and substituted-PNG mutations; wrong browser backend rejection passed
  1/1. Schema catalog live/golden serialization and CLI list golden each passed
  1/1. The first scoped run correctly exposed an uncataloged documented schema;
  adding the exact catalog rows resolved that policy defect.
- `doctor/docs`: `C04-FAIL-CLOSED-RELEASE-READINESS` pins CLI usage, both
  workflow inputs, helper scripts, the exact required inventory block, negative
  tests, schema entries, release-gate docs, and troubleshooting. Its mutation
  removes the Linux existence row and is rejected. Release gates now define the
  result/count semantics; troubleshooting names both configuration errors; the
  v1.8.0 note carries an explicit post-release erratum; `[Unreleased]` records
  the correction. `cargo run -p xtask -- doctor --full` reported
  `mode=Full status=pass`.
- `scoped gates`: remote `cargo fmt --all --check` and
  `cargo clippy -p xtask --all-targets -- -D warnings` passed. The initial fmt
  check produced only mechanical formatting diffs, which were applied once.
- `full`: deferred to Checkpoint D. No workspace-wide test, rustdoc,
  browser/GPU matrix, performance distribution, package, publish, or complete
  release evidence run was executed for this release-policy slice.
- `bootstrap`: canonical source `/home/johannes/projects/scena`, isolated
  destination
  `/home/johannes/.cache/codex-worktrees/scena-full-review-v18-checklist`,
  branch `main`, HEAD `7b4fc9ca77e12fd12a69fab92650e1e46ee10354`;
  shared checkout absent; `AGENTS.md` hash
  `d0ed47595f6b6a6ec733651dfde650b7893aa2939d6137dc2f7eeade7528ac7a`
  and skills aggregate
  `ad212b6090c76bfaaf80fdb94eddd0be1703dd55d05d13732031de6944e5b111`
  matched after every explicit bootstrap.
- `counts`: about 25 minutes investigation; one production implementation,
  one schema-policy correction, and one mechanical formatting pass; zero
  release-candidate pushes; zero full-matrix runs; zero user-required actions.
  No failure signature received two failed remediation attempts.

### C05 — Correct cone and wedge outward winding (review B3)

Owner: `src/geometry/primitive_meshes.rs`.

- [x] Add a geometry oracle that computes each nondegenerate face normal from
  vertex positions and compares it with the expected outward radial/planar
  direction; do not trust stored vertex normals as the oracle.
- [x] Pin the current cone lateral and wedge face failures.
- [x] Reverse the affected triangle emission and correct associated vertex
  normals/tangents without changing shape dimensions.
- [x] Check cap winding independently from lateral winding.
- [x] Add a default-back-face-culling render proof showing the near exterior,
  not the far interior.
- [x] Check negative/nonuniform scale behavior is handled by transform parity,
  not baked into primitive topology.
- [x] Update primitive screenshots/docs only if their visible output changes.
- [x] Acceptance: all faces are outward under computed geometry truth and the
  culling render matches its reference.

Validation ledger (2026-07-20):

- `focused red`: the position-only cone oracle failed on lateral triangle 0
  with face `(-0.8844,-0.4020,-0.2370)`, proving inward/downward geometry; the
  wedge oracle failed on triangle 0 with a `+Y` normal on the bottom face. The
  64x64 CPU render also proved the old single-sided cone differed from the
  double-sided depth reference, so default culling exposed the far interior.
- `classification`: product defect. The oracles derive cross products from
  indexed positions and compare against radial direction or the wedge volume
  centroid; they never consult the equally inverted stored normals. The first
  compile failure was a test-harness slice/array mistake and was corrected
  before recording product evidence. A later wedge assertion correction used
  the triangular-prism volume centroid instead of its bounds center.
- `implementation`: cone lateral emission now uses `(p0, tip, p1)` for both
  face normals and indices; the bottom cap remains independently down-facing.
  Every wedge triangle/quad is reversed and its flat normal follows the same
  outward order. Dimensions, vertex/index counts, UV association, and bounds
  are unchanged; generated tangents consume the corrected indices/normals.
- `focused green`: all five primitive-mesh unit tests passed, including 12
  outward cone sides, 12 down-facing cap triangles, all eight wedge triangles,
  stored-normal agreement, deterministic counts, and negative-dimension
  normalization. The CPU near-exterior render passed 1/1 for both cone and
  wedge: default single-sided bytes equal the double-sided depth reference and
  contain more than 100 visible pixels.
- `transform/tangents`: the two existing PF07 focused tests passed for cached
  tangent equivalence under nonuniform world scale and handedness flipping for
  mirrored instances. Primitive constructors continue to use absolute size;
  transform parity remains prepare-owned rather than changing topology.
- `doctor`: `C05-OUTWARD-PRIMITIVE-WINDING` pins the corrected source order,
  forbids both old normal expressions, and requires the position oracle and
  culling proof. Its old-cone-order mutation passed 1/1, and full doctor
  reported `mode=Full status=pass`.
- `docs`: no committed cone/wedge screenshot or primitive-specific prose
  encoded the inverted appearance, so no reference image changed. The
  `[Unreleased]` changelog records the user-visible culling correction.
- `scoped gates`: remote `cargo fmt --all --check`, strict scena Clippy for the
  library plus `placeholder_regression`, and strict xtask all-target Clippy all
  passed. One mechanical fmt pass was applied.
- `full`: deferred to Checkpoint D. No workspace-wide test, rustdoc,
  browser/GPU matrix, performance distribution, package, publish, or full
  release chain was run for this primitive correction.
- `bootstrap`: canonical source `/home/johannes/projects/scena`, isolated
  destination
  `/home/johannes/.cache/codex-worktrees/scena-full-review-v18-checklist`,
  branch `main`, HEAD `7b4fc9ca77e12fd12a69fab92650e1e46ee10354`;
  shared checkout absent; `AGENTS.md` hash
  `d0ed47595f6b6a6ec733651dfde650b7893aa2939d6137dc2f7eeade7528ac7a`
  and skills aggregate
  `ad212b6090c76bfaaf80fdb94eddd0be1703dd55d05d13732031de6944e5b111`
  matched after every explicit bootstrap.
- `counts`: about 20 minutes investigation; one production remediation; two
  distinct test-harness corrections and one mechanical formatting pass; zero
  release-candidate pushes; zero full-matrix runs; zero user-required actions.
  No failure signature received two failed remediation attempts.

### C06 — Stop successful black default renders on high-level happy paths
(review B4)

Owner: viewer builders, CLI defaults, diagnostics/reporting, examples and docs.

- [x] Add a focused PBR glTF viewer/CLI test using only documented defaults.
- [x] Pin the old result: an all-black or invisible frame returns success while
  `MissingLightingOrEnvironment`/`InvisibleScene` remain available only through
  a separate diagnosis call.
- [x] Define which high-level builders and CLI verbs promise a presentable
  default. Keep low-level deterministic `Renderer` construction explicit where
  black/transparent output is a valid contract.
- [x] Add a neutral portable light/environment/background to the high-level
  default path without overriding authored scene choices.
- [x] Fold relevant scene diagnostics into the render outcome as structured
  warnings, including remedy, affected object/setting, and whether fallback was
  applied.
- [x] Define success policy: a technically rendered but provably invisible
  scene must not be silent; verification modes may fail while low-level capture
  may return bytes plus warnings.
- [x] Convert beginner examples that use unlit materials only to avoid missing
  setup into representative PBR examples.
- [x] Update README quick start, getting started, easy-scene guide, rendering,
  examples, errors, troubleshooting "blank output," and API docs.
- [x] Add an `[Unreleased]` changelog entry and release-note entry for the next
  version. Amend v1.8.0 notes only with an explicit erratum, never as if the fix
  shipped in v1.8.0.
- [x] Acceptance: documented PBR happy paths produce a visible neutral render
  or an actionable non-silent result without extra diagnosis calls.

Validation ledger (2026-07-20):

- `focused red`: the new 96x96 PBR `cad_terminal_block.gltf` default-viewer
  proof failed first on the old black renderer background instead of
  `Background::Studio`. The no-light/black-background state still returned a
  successful outcome with draw calls; at 48x48 only 75/2,304 pixels were
  nonblack, while `MissingLightingOrEnvironment` existed only through the
  separate scene diagnosis path. This pins the silent near-black family rather
  than treating successful byte production as visible output.
- `classification`: product defect. High-level builders inherited low-level
  `lighting=None`, `default_environment=false`, and the renderer's black clear
  color even though the scene diagnosis machinery already knew the remedy.
  Low-level black/unlit construction is a valid explicit contract and was not
  changed.
- `implementation`: the headless/interactive glTF viewer common defaults now
  use `Background::Studio` and conditionally add one neutral directional light
  only when there is no explicit viewer lighting, configured environment, or
  authored scene light. Profiles and explicit environment/light choices win;
  `without_default_lighting()` plus an explicit background retains the dark
  diagnostic path. Raw-asset CLI render/inspect/diagnose/verify already choose
  an explicit default light and now inherit the neutral viewer background;
  authored recipe presentation remains explicit.
- `diagnostics and success policy`: `FirstRender::diagnostics()` and both
  reusable viewer diagnostic methods now combine setup, renderer, and live
  scene diagnosis. The applied-fallback warning serializes its remedy,
  `setting="viewer.lighting"`, and `fallback_applied=true`. Explicit opt-out
  returns bytes plus a non-fallback missing-light warning; introspection and
  verification remain free to fail invisible output while low-level capture
  can intentionally return black/transparent bytes.
- `focused green`: the two exact viewer-default/opt-out tests passed; the
  authored-light preservation unit test passed 1/1; the PBR raw CLI
  introspection test passed 1/1 with positive content coverage and tonal
  range; the PBR `glb_model_viewer` rendered-output proof passed 1/1 with more
  than eight distinct RGB values. The complete `first_render_api` target
  passed 11/11.
- `examples and docs`: `glb_model_viewer` and its rendered proof now use the
  representative PBR CAD asset and expose diagnostics. README, getting
  started, easy-scene setup, rendering, examples, errors, troubleshooting, and
  API docs define the high-level/low-level boundary and blank-output remedy.
  `[Unreleased]` records the fix; v1.8.0 notes call it a post-release erratum
  and explicitly say it was not part of v1.8.0.
- `doctor`: `C06-PRESENTABLE-VIEWER-DEFAULTS` pins the conditional fallback,
  authored-choice guard, structured warning, PBR viewer/CLI/example proofs,
  docs, changelog, and erratum. Its disabled-fallback mutation passed 1/1. The
  older M7 rule was corrected from renderer-only diagnostics to the combined
  viewer result. Orbit initialization and diagnostic aggregation moved to
  owner modules after doctor caught `viewer.rs` over its KISS size limit;
  full doctor then reported `mode=Full status=pass`.
- `scoped gates`: remote `cargo fmt --all --check` and strict all-target,
  all-feature Clippy passed. Clippy exposed large temporary SceneHost/CLI enum
  variants after `Diagnostic` grew; those internal variants now box their
  renderer/build payloads. Existing recipe orchestration signatures use
  narrowly documented lint boundaries rather than artificial parameter bags.
  Formatting checks produced only mechanical diffs; two formatting passes were
  applied as the final owner-module split changed import order.
- `test-harness corrections`: an exact-all-black assertion was narrowed after
  the discriminating measurement showed 75 bright object pixels against 2,229
  black pixels; the contract is silent near-black/invisible output, not every
  byte equaling zero. Two filtered commands initially selected zero tests and
  were immediately rerun with their correct feature/filter shape; they are not
  counted as proof.
- `full`: deferred to Checkpoint D. No workspace-wide test, rustdoc,
  browser/GPU matrix, performance distribution, package, publish, or release
  chain was run for this high-level-default slice.
- `bootstrap`: canonical source `/home/johannes/projects/scena`, isolated
  destination
  `/home/johannes/.cache/codex-worktrees/scena-full-review-v18-checklist`,
  branch `main`, HEAD `7b4fc9ca77e12fd12a69fab92650e1e46ee10354`;
  shared checkout absent; `AGENTS.md` hash
  `d0ed47595f6b6a6ec733651dfde650b7893aa2939d6137dc2f7eeade7528ac7a`
  and skills aggregate
  `ad212b6090c76bfaaf80fdb94eddd0be1703dd55d05d13732031de6944e5b111`
  matched after every explicit bootstrap.
- `counts`: about 55 minutes investigation; one production remediation, two
  measured test-harness corrections, two behavior-neutral ownership splits,
  two internal enum boxing fixes, one scoped lint-boundary cleanup, and two
  mechanical formatting passes; zero release-candidate pushes; zero full-matrix
  runs; zero user-required actions. No identical failure signature received
  two failed remediation patches, and no investigation circuit breaker
  tripped.

### C07 — Encode browser readback in the target transfer function, independent
of post-processing (review B5)

Owner: GPU draw common code, browser readback targets, color contract, browser
proofs.

- [x] Add a no-post WebGPU readback fixture with known linear shader values and
  exact expected sRGB bytes/tolerance.
- [x] Prove the old `post_enabled` flag returns linear bytes from
  `Rgba8Unorm` while consumers interpret them as sRGB.
- [x] Derive encoding from the actual render/readback attachment format and
  output contract, not from whether post-processing is enabled.
- [x] Ensure the scene is encoded exactly once for native sRGB targets, browser
  unorm readback, post-enabled paths, and WebGL2 fallbacks.
- [x] Keep lighting, blending, bloom, and other post math linear until the final
  transfer.
- [x] Add post-on/post-off, WebGPU/WebGL2, and native/browser parity coverage.
- [ ] Capture physical hardware evidence for the required browser parity claim;
  software conformance is supplementary.
- [x] Update `docs/specs/color-contract.md`, browser/readback docs, capability
  format reporting, and any affected golden metadata.
- [x] Acceptance: changing post-processing does not change the transfer
  interpretation of otherwise identical output.

Validation ledger (2026-07-20; physical-hardware row remains open):

- `focused red`: the focused unit oracle first observed byte `46` for linear
  `0.18` on a plain `Rgba8Unorm` target instead of expected sRGB byte `118`.
  The old output uniform selected manual encoding from `post_enabled`, so the
  no-post browser readback stored linear bytes under an sRGB consumer contract.
- `classification`: product defect. The target attachment determines whether
  transfer is performed by the shader or the attachment; post enablement is
  independent. A later browser assertion failure was separately classified as
  a harness defect because the post fixture had not forced an FXAA quality
  path. A missing commit in the isolated copy was a provenance failure and was
  remedied by supplying the recorded source HEAD, not by weakening the gate.
- `implementation`: mesh and clear output now derive transfer from the actual
  attachment. Plain RGBA/BGRA unorm targets receive shader sRGB encoding;
  `*Srgb` targets receive linear shader output and attachment encoding.
  Browser readback preserves the surface transfer class while normalizing byte
  order to RGBA. The SDR post scene/ping/pong attachments are now
  `Rgba8UnormSrgb`, so physical storage remains encoded while shader loads,
  bloom, FXAA, SSAO, reflections, depth-of-field, labels, and strokes operate
  in linear RGB. Bloom's authored sRGB threshold is converted to linear before
  GPU comparison.
- `focused green`: the target-format unit oracles passed, including exact
  `0.18 -> 118` and RGBA/BGRA transfer preservation. A strict native wgpu test
  on Lavapipe passed 1/1 and returned `[118,118,118,255]` within two byte values
  with post off and FXAA on. The browser-probe WASM package built successfully.
- `browser conformance`: the focused M6 run passed on both WebGL2 and WebGPU
  under Chromium/SwiftShader. All four post/backend combinations returned
  center `[118,118,118,255]`; no-post hashes were
  `239266a35d359065`, post hashes were `d5e16a7a7fb9ef05`, and CPU/WebGL2
  parity passed. The provenance-bound artifact is
  `target/gate-artifacts/m6-rust-wasm-renderer-probe.json` in the isolated
  validation copy, SHA-256
  `7a91395a6cff507e752bab47bd475fa178b5a9d5a6e786ea75130e6296466e52`,
  commit `7b4fc9ca77e12fd12a69fab92650e1e46ee10354`.
- `hardware`: still open. The builder adapter was ANGLE/SwiftShader CPU, so the
  browser result is supplementary conformance and is not mislabeled as the
  required physical WebGPU/WebGL2 evidence.
- `capabilities and docs`: live renderers report the selected RGBA/BGRA
  unorm/sRGB attachment rather than a hardcoded format. The color contract,
  browser/readback, rendering, capabilities, schema, changelog, and explicit
  v1.8.0 erratum document target-driven transfer and post independence. The
  headless stable fixture remains `Rgba8UnormSrgb`, so no golden byte changed.
- `doctor`: `C07-TARGET-DRIVEN-COLOR-TRANSFER` pins target-driven selection,
  the sRGB post intermediates, readback mapping, native/browser proofs,
  capability reporting, and docs. Its post-driven mutation passed 1/1; full
  doctor reported `mode=Full status=pass`.
- `scoped`: remote `cargo fmt --all --check`, strict all-target/all-feature
  Clippy, strict Lavapipe GPU proof, WASM build, focused WebGPU/WebGL2 browser
  proof, doctor mutation, and full doctor passed. Full repository tests,
  rustdoc, performance, packaging, publish, and release matrices were not run.
- `circuit breaker`: the doctor mutation initially failed twice with the same
  missing-lowercase-`post` signature. Edits were frozen; an exact local/remote
  token probe proved the document used the intended `Post-processing` sentence
  while the generic rule was case-sensitive. The rule now pins that exact
  sentence, and the next focused run passed. No production behavior was
  changed by this harness correction.
- `full`: deferred to Checkpoint D. C07 cannot be marked fully complete until
  the separate physical-hardware artifact is captured.
- `bootstrap`: canonical source `/home/johannes/projects/scena`, isolated
  destination
  `/home/johannes/.cache/codex-worktrees/scena-full-review-v18-checklist`,
  branch `main`, HEAD `7b4fc9ca77e12fd12a69fab92650e1e46ee10354`;
  shared checkout absent; `AGENTS.md` hash
  `d0ed47595f6b6a6ec733651dfde650b7893aa2939d6137dc2f7eeade7528ac7a`
  and skills aggregate
  `ad212b6090c76bfaaf80fdb94eddd0be1703dd55d05d13732031de6944e5b111`
  matched after every explicit bootstrap.
- `counts`: about 80 minutes investigation; one production remediation, one
  visibility-scope compile correction, two fixture corrections, one
  behavior-neutral lint refactor, two failed doctor-pin attempts followed by
  one discriminating probe, zero release-candidate pushes, zero full-matrix
  runs, and zero user-required actions so far. Physical evidence will require
  one future hardware action or an existing admissible lane artifact.

## 3. Rendering, assets, import, and lifecycle correctness

### C08 — Convert Z-up rotation animation channels (review B6)

Owner: `src/scene/import/options.rs` and animation import.

- [x] Add an animated Z-up right-handed fixture with a nontrivial rest
  rotation, at least two quaternion keys, and a known world-axis trajectory.
- [x] Prove the rest pose converts but playback snaps/spins around the source
  basis.
- [x] Basis-conjugate every rotation keyframe consistently with static node
  transforms.
- [x] Convert cubic-spline rotation tangents with mathematically correct
  semantics; reject unsupported encodings rather than silently corrupting them.
- [x] Verify translations, scales, skins, morphs, anchors, and connectors retain
  the intended coordinate/unit contracts.
- [x] Add frame-at-time world-transform assertions and CPU/GPU visual parity.
- [x] Update units/axes documentation and animated import examples.
- [x] Acceptance: rest and animated poses share one converted basis for linear,
  step, and cubic interpolation.

Validation ledger (2026-07-20):

- `focused red`: the new
  `z_up_rotation_animation_uses_the_static_transform_basis` unit proof failed
  against the old import path: the sampled endpoint remained the source Z-axis
  quarter-turn `Quat(0, 0, 0.7071, 0.7071)`, while the statically converted
  transform expected the target Y-axis quarter-turn
  `Quat(0, 0.7071, 0, 0.7071)`; their orientation dot product was `0.50000006`.
- `classification`: product defect. Static rotation conversion and animation
  rebinding were both reached, but the latter cloned quaternion outputs
  unchanged; this distinguished the defect from malformed glTF data, mixer
  sampling, scene hierarchy composition, and renderer state.
- `implementation`: imported animation rebinding now accepts a quaternion
  conversion callback without changing the public authored-animation `rebind`
  API. Rotation keys use the same basis conjugation and final normalization as
  static transforms. glTF cubic output indices identify each
  `[in tangent, value, out tangent]` triplet: values are converted as rotations,
  while derivative tangents are raw basis-conjugated without normalization.
- `fixture/world proof`: committed
  `tests/assets/gltf/z_up_animated_rotation.gltf` carries a 30-degree source-Z
  rest pose, 30-to-90-degree keys, and `LinearZ`, `StepZ`, and `CubicZ` clips.
  Its integration proof passed 1/1 and asserts the converted rest, midpoint,
  endpoint, translation/scale invariants, and the known local-X trajectory in
  world XZ space. Both focused library tests passed 1/1.
- `rendered-output proof`: strict Lavapipe CPU/headless-GPU parity passed 1/1
  at rest and at the linear midpoint. Full-frame RMSE was `0.05968` and
  `0.04271`; mean channel delta was `1.12307` and `0.57523`. The JSON artifact
  SHA-256 is
  `913450a4f483c9984279a9425fbb13159c61e6a9f30d43843c5de2db0dd4066b`;
  four CPU/GPU before/after PNGs were retained under
  `target/gate-artifacts/dynamic-transform-parity/`. This is deterministic GPU
  path conformance on a CPU Vulkan adapter, not physical-hardware evidence.
- `regression surface`: the complete `m3b_gltf_animation` target passed 14/14,
  covering translation, scale, step/cubic sampling, skinning, morph targets,
  replacement rebinding, and lifecycle stability. `c05_import_unit_contracts`
  passed 4/4. Focused Z-up node/connector tests passed 2/2, and the imported
  animated-connector ownership proof passed 1/1.
- `doctor/docs`: `C08-Z-UP-ANIMATION-BASIS` pins the production callback,
  cubic derivative branch, committed fixture, world-transform proof,
  CPU/GPU proof, assets/axis guides, changelog, and v1.8.0 erratum. Its bypass
  mutation passed 1/1 and full doctor reported `mode=Full status=pass`. The
  first mutation run was classified as a test-harness defect because a generic
  lowercase `cubic` needle did not match the guide's exact glTF `CUBICSPLINE`
  spelling; one path-specific contract correction made the same proof green.
- `scoped gates`: remote `cargo fmt --all --check` and strict
  `cargo clippy --all-targets --all-features -- -D warnings` passed.
- `full`: deferred to Checkpoint D. No workspace-wide test, rustdoc,
  browser/hardware matrix, performance distribution, package, publish, or
  release-evidence chain was run for this import-only slice.
- `bootstrap`: canonical source `/home/johannes/projects/scena`, isolated
  destination
  `/home/johannes/.cache/codex-worktrees/scena-full-review-v18-checklist`,
  branch `main`, HEAD `7b4fc9ca77e12fd12a69fab92650e1e46ee10354`;
  shared checkout absent; `AGENTS.md` hash
  `d0ed47595f6b6a6ec733651dfde650b7893aa2939d6137dc2f7eeade7528ac7a`
  and skills aggregate
  `ad212b6090c76bfaaf80fdb94eddd0be1703dd55d05d13732031de6944e5b111`
  matched after every explicit bootstrap.
- `counts`: about 35 minutes investigation; one production remediation and one
  doctor-harness correction; zero release-candidate pushes, zero full-matrix
  runs, and zero user-required actions. No failure signature reached the
  two-remediation circuit breaker.

### C09 — Make external texture hot reload replace dependencies transactionally
(review B7)

Owner: `src/assets/texture.rs`, scene loading cache, reload transaction.

- [x] Add a temp-directory test that loads a scene, changes external texture
  bytes at the same path, reloads, and observes the new decoded pixels.
- [x] Pin the old cache-identity-collision parse error.
- [x] Define cache identity separately from mutable source version/digest.
- [x] During explicit reload, stage new bytes/decoding/dependencies, then atomically
  replace lookup entries only after the full asset succeeds.
- [x] Preserve retained scene handles and source mapping where the public reload
  contract promises identity stability.
- [x] On failure, keep the old complete asset usable and return a structured
  reload report; do not leave mixed old/new dependencies.
- [x] Cover external buffers, embedded textures, shared textures, deleted files,
  and same-byte no-op reload.
- [x] Add doctor ownership enforcement if stale cache mutation can be detected
  outside the asset reload owner.
- [x] Update assets/hot-reload docs, lifecycle docs, examples, and errors.
- [x] Acceptance: editing a texture on disk is the successful primary reload
  case, not a collision.

Validation ledger (2026-07-20):

- `focused red`: the temp-directory
  `reload_scene_replaces_changed_external_texture_at_the_same_path` proof
  loaded a red 1x1 PNG, overwrote the same path with green pixels, then failed
  on the old path with `AssetError::Parse` and
  `texture cache identity collision: incoming source bytes do not match the
  immutable provenance of the already-decoded pixels`.
- `classification`: product defect. The scene and external-image fetches
  succeeded and the replacement PNG decoded independently; the stale
  `texture_lookup` entry alone selected the immutable ordinary-load collision
  path during explicit reload.
- `implementation`: texture cache identity remains path + color space + sampler
  + source format, while decoded provenance remains a source revision. Normal
  loads use `TextureCacheUpdatePolicy::Immutable`; only explicit reload selects
  `ReplaceChangedSource`. The existing glTF parser clones `AssetStorage`,
  parses/decodes every geometry/material/texture dependency in that
  transaction, and swaps it into the locked store only on success. The reload
  policy is restored on both success and error paths.
- `identity/failure contract`: same-path external textures retain their
  `TextureHandle` while the descriptor/provenance revision changes; immutable
  `Arc<TextureDesc>` snapshots remain old revisions. Embedded images continue
  to use content-addressed handles. Reload now fetches referenced buffers and
  supported images strictly, so deleted dependencies fail without falling back
  to a mixed scene. `reload_scene_with_report` returns `AssetLoadReport` on
  success and `AssetReloadError` on failure, including the reload path,
  underlying typed `AssetError`, and `previous_asset_preserved()` evidence;
  compatibility `reload_scene` still returns `AssetError`.
- `focused/scoped`: the `round_d_asset_hot_reload` target passed 6/6 for
  changed/same-byte/malformed/deleted external PNGs, immutable ordinary-load
  provenance, shared consumers, embedded content identity, changed/deleted
  external buffers, last-complete cache rollback, and the watcher/render loop.
  Existing M8 reload/retention/lifetime filters passed 4/4, and the M3A
  retain/replace/reprepare proof passed 1/1.
- `platform`: `cargo check --target wasm32-unknown-unknown --all-features`
  passed after the public error/report and module split. It emitted the same
  six cfg-specific dead-code warnings already present for CPU-only row-band and
  renderer fields; no C09 warning or error was introduced.
- `doctor/docs`: `C09-TRANSACTIONAL-ASSET-RELOAD` pins the explicit policy
  boundary, transactional storage clone, public success/failure report types,
  full dependency fixtures, and user docs. Its immutable-policy mutation
  passed 1/1; full doctor reported `mode=Full status=pass`. Assets, API,
  easy-scene hot reload, errors, changelog, and v1.8.0 errata now distinguish
  ordinary immutable loads, transactional reload, stable external handles,
  content-addressed embedded handles, snapshots, and failure preservation.
- `architecture corrections`: doctor first exposed a stale guide pin and
  production modules just over the 500-line KISS limit. Reload policy and
  texture-format logic moved to `texture_reload.rs` and `texture_format.rs`,
  while `AssetStoreId` moved to `store_id.rs`. The matching M8 doctor checks
  moved to `texture_baseline.rs` before their xtask owner crossed 600 lines.
  No size threshold was widened. The initial C09 mutation also exposed an
  insufficient any-occurrence policy needle; exact two-boundary counting now
  rejects mutation of either fetched or retained-source reload.
- `scoped gates`: remote `cargo fmt --all --check`, strict
  `cargo clippy --all-targets --all-features -- -D warnings`, and full doctor
  passed. One initial clippy run found only an `is_some` + `expect` pattern;
  the behavior-neutral `if let` rewrite made strict lint green.
- `full`: deferred to Checkpoint D. No workspace-wide test, rustdoc,
  browser/hardware render matrix, performance distribution, package, publish,
  or release-evidence chain was run for this asset-reload slice.
- `bootstrap`: canonical source `/home/johannes/projects/scena`, isolated
  destination
  `/home/johannes/.cache/codex-worktrees/scena-full-review-v18-checklist`,
  branch `main`, HEAD `7b4fc9ca77e12fd12a69fab92650e1e46ee10354`;
  shared checkout absent; `AGENTS.md` hash
  `d0ed47595f6b6a6ec733651dfde650b7893aa2939d6137dc2f7eeade7528ac7a`
  and skills aggregate
  `ad212b6090c76bfaaf80fdb94eddd0be1703dd55d05d13732031de6944e5b111`
  matched after every explicit bootstrap.
- `counts`: about 80 minutes investigation; one production remediation, one
  public failure-report completion, one lint-only correction, four distinct
  doctor ownership/size integrations, and one doctor-mutation strengthening;
  zero release-candidate pushes, zero full-matrix runs, and zero user-required
  actions. No identical failure signature reached two remediation attempts.

### C10 — Key scene-load caching by semantic load policy (smaller finding)

Owner: `src/assets/scene_loading.rs`.

- [x] Add lenient-then-strict and strict-then-lenient same-path tests.
- [x] Enumerate every `AssetLoadOptions` field that changes validation,
  resolution, decoding, extensions, policy roots, limits, or diagnostics.
- [x] Include semantic options in the cache key, or prove cached evidence
  satisfies the requested policy before reuse.
- [x] Never let a lenient cached asset bypass a later strict/security policy.
- [x] Avoid duplicate cache entries for fields that provably do not affect the
  loaded result.
- [x] Report cache-hit policy/provenance in asset-load diagnostics.
- [x] Update cache and sandbox documentation.
- [x] Acceptance: cache reuse cannot weaken the active request's policy.

Validation ledger:

- `focused red`: on the isolated Hetzner CPU builder at base commit
  `7b4fc9ca77e12fd12a69fab92650e1e46ee10354`, the new
  `scene_cache_lenient_then_strict_does_not_bypass_texture_policy` regression
  failed because the strict request returned `Ok(AssetLoadReport { cache_hit:
  true, ... ExternalImageMissing ... })` from the path-only lenient entry instead
  of failing on the missing image. This pinned the exact policy-bypass defect
  before production changes.
- `classification`: product/security-policy defect. `scene_lookup` and its
  telemetry map were keyed only by `AssetPath`; the cache lookup ignored all
  three semantic `AssetLoadOptions` fields.
- `implementation`: `SceneCacheKey` now combines path and all semantic options.
  Exact-policy entries are preferred; cross-policy reuse occurs only when
  retained telemetry proves strict texture, strict external-buffer, and total
  fetch-budget requirements. Successful strict evidence may satisfy a lenient
  request without a duplicate entry, while explicit reload replaces every
  policy entry for its path. `AssetLoadReport` and
  `scena.asset_load_report.v1` expose `requested_options` and
  `cache_entry_options`; missing additive v1 fields default compatibly.
  `AssetLoadOptions` ownership moved to `assets/load/options.rs`, and cache
  qualification lives in `assets/scene_cache.rs` to preserve module-size
  boundaries.
- `focused green`: remote all-feature M8 cache-policy filter passed 4/4,
  covering lenient-to-strict missing images, lenient-to-strict missing empty
  buffers, strict-to-lenient compatible evidence, and unlimited-to-bounded
  fetch policy. The asset-load report filter passed 2/2; the complete stable
  contract target passed 59/59, including old additive v1 decoding and all
  nested report fixtures.
- `related scoped proof`: transactional hot reload remained green 6/6, and the
  stale-handle/cache-root target remained green 7/7. This proves the new
  multi-policy cache ownership and reload-wide replacement did not weaken C09
  rollback/handle behavior or descriptor reachability.
- `doctor/docs`: `C10-SEMANTIC-SCENE-CACHE-POLICY` pins the semantic key, every
  current option field, all three evidence checks, active-option lookup,
  report provenance, regressions, docs, changelog, and errata. Its path-only
  lookup mutation passed 1/1, and full doctor reported
  `mode=Full status=pass`. The older M3A architecture rule now requires
  `SceneCacheKey` instead of preserving the unsafe path-only type. Assets, API,
  schema-contract, sandbox, README, changelog, v1.8 errata, and cache-root
  rustdoc describe the new contract.
- `scoped gates`: remote `cargo fmt --all --check` and strict
  `cargo clippy --all-targets --all-features -- -D warnings` passed. Remote
  all-feature `wasm32-unknown-unknown` check passed with the same six known
  target-cfg dead-code warnings recorded under C09; none involve the cache or
  report changes.
- `full`: deferred to Checkpoint D. No workspace-wide test, rustdoc,
  browser/hardware render matrix, performance distribution, package, publish,
  or release-evidence chain was run for this asset-cache slice.
- `bootstrap`: canonical source `/home/johannes/projects/scena`, isolated
  destination
  `/home/johannes/.cache/codex-worktrees/scena-full-review-v18-checklist`,
  branch `main`, HEAD `7b4fc9ca77e12fd12a69fab92650e1e46ee10354`;
  shared checkout absent; `AGENTS.md` hash
  `d0ed47595f6b6a6ec733651dfde650b7893aa2939d6137dc2f7eeade7528ac7a`
  and skills aggregate
  `ad212b6090c76bfaaf80fdb94eddd0be1703dd55d05d13732031de6944e5b111`
  matched after every explicit bootstrap.
- `counts`: about 45 minutes investigation; one production remediation, one
  compile-wiring correction, two distinct doctor-pin/fixture corrections, zero
  release-candidate pushes, zero full-matrix runs, and zero user-required
  actions. No identical failure signature reached two remediation attempts.

### C11 — Implement real device-loss recovery or report rebuild-required
(review B8)

Owner: renderer lifecycle/surface adapters, not scene/assets.

- [x] Add a fake/injected lifecycle test that marks the current Device/Queue
  unusable and rejects subsequent allocation.
- [x] Prove the old `DeviceLost` path clears resources but reuses the dead
  device.
- [x] Define the public recovery contract: internally re-request adapter/device
  and rebuild device-bound state, or return a typed `RebuildRequired` outcome
  for the host to recreate the renderer.
- [x] Keep surface loss/context loss distinct from terminal device loss.
- [x] Recreate pipelines, bind groups, textures, queues, capability reports,
  error callbacks, and surface configuration only under the prepare lifecycle.
- [x] Preserve scene/assets state without pretending GPU resources survived.
- [x] Add repeated-loss, loss-during-prepare, and loss-during-render tests.
- [x] Obtain native and browser hardware evidence where loss injection is
  supported; otherwise label synthetic lifecycle proof honestly.
- [x] Update lifecycle, errors, browser, and recovery examples.
- [x] Acceptance: no allocation or submission is attempted on a known-dead
  device after a reported recovery.

Validation ledger:

- `focused red`: the new injected lifecycle target failed to compile against
  the old public contract because `PrepareError::GpuDeviceRebuildRequired` did
  not exist. Read-only source inspection pinned the pre-fix defect: the
  `DeviceLost` event cleared prepared resources, and `recover_context()` then
  cleared the loss flag without replacing `GpuDeviceState`, so the retained
  Device/Queue would be polled and reused on the next prepare.
- `classification`: product lifecycle defect. A wgpu Device/Queue loss is
  terminal for that object; it is not interchangeable with recoverable surface
  or browser-context loss.
- `implementation`: device loss remains latched on the current renderer.
  `prepare*()` checks it before device polling, allocation, upload, or
  submission and returns typed `GpuDeviceRebuildRequired`; `render*()` remains
  blocked with `GpuDeviceLost`; `recover_context()` cannot clear it. Context
  recovery remains a retained-asset, same-device path. Native attached-surface
  replacement requests fresh GPU state; browser hosts construct a fresh
  renderer and prepare the retained `Scene` and `Assets`.
- `focused green`: remote `tests/c11_device_loss_recovery.rs` passed 3/3,
  covering loss after a prepared render, repeated loss, loss before first
  prepare, blocked render/recovery/prepare paths, and successful fresh-renderer
  preparation with the same retained CPU scene/assets. The existing M4 context
  recovery filter passed 3/3, including retained texture, cubemap, and shadow
  recreation, proving the recoverable context path remains distinct.
- `browser proof`: remote `npm run browser:c11-lifecycle` passed on the rebuilt
  WebGL2/WASM package. Its event trace covers context recovery, surface loss,
  terminal device loss, blocked recovery/prepare/render, and a successful fresh
  renderer. The output is explicitly classified
  `synthetic-headless-browser`; no physical device-loss injection or physical
  GPU claim was made.
- `harness corrections`: the first browser attempt exposed stale
  `target/m6-browser-pkg` reuse; lifecycle-only runs now force a Rust/WASM
  rebuild. The next attempt passed lifecycle assertions but failed release
  provenance stamping in the isolated Git-less copy; lifecycle-only proof now
  exits with a compact non-release result. These were respectively
  test-harness/provenance and environment failures, not product failures.
- `doctor/docs`: `C11-TERMINAL-DEVICE-LOSS` pins typed diagnostics, guard-before-
  poll ordering, terminal recovery ordering, regressions, forced browser-probe
  rebuild, honest synthetic classification, docs, changelog, and v1.8 errata.
  Its poll-before-guard mutation passed 1/1 and full doctor passed. Lifecycle,
  browser, API/error recovery guidance, changelog, and release-note errata now
  distinguish context recovery from renderer/device rebuild.
- `scoped gates`: remote `cargo fmt --check`, strict
  `cargo clippy --all-targets --all-features -- -D warnings`, and the prior
  all-feature `wasm32-unknown-unknown` check passed. The WASM check retained the
  same six known target-cfg dead-code warnings already recorded under C09; none
  involve device-loss behavior.
- `full`: deferred to Checkpoint D. No workspace-wide test, rustdoc,
  physical-GPU matrix, performance distribution, package, publish, or release-
  evidence chain was run for this lifecycle slice.
- `bootstrap`: canonical source `/home/johannes/projects/scena`, isolated
  destination
  `/home/johannes/.cache/codex-worktrees/scena-full-review-v18-checklist`,
  branch `main`, HEAD `7b4fc9ca77e12fd12a69fab92650e1e46ee10354`;
  shared checkout absent; `AGENTS.md` hash
  `d0ed47595f6b6a6ec733651dfde650b7893aa2939d6137dc2f7eeade7528ac7a`
  and skills aggregate
  `ad212b6090c76bfaaf80fdb94eddd0be1703dd55d05d13732031de6944e5b111`
  matched after every explicit bootstrap.
- `counts`: about 35 minutes investigation; one production remediation, two
  distinct browser-harness corrections, one doctor-pin correction, zero
  release-candidate pushes, zero full-matrix runs, and zero user-required
  actions. No identical failure signature reached two remediation attempts.

### C12 — Reconfigure and retry recoverable surface errors once; surface
validation failures (smaller findings)

Owner: native surface draw path.

- [x] Inject `Outdated`, `Lost`, `Timeout`, `Occluded`, `OutOfMemory`, and
  validation-error outcomes independently.
- [x] For `Outdated`, reconfigure and retry the frame exactly once; for surface
  `Lost`, follow wgpu's contract and latch a typed failure until the host
  recreates the surface. Prevent infinite retry loops in both cases.
- [x] Treat timeout/occlusion as a diagnostic frame skip with counters.
- [x] Treat out-of-memory and validation errors as structured hard failures;
  never swallow validation as another transient surface status.
- [x] Recompute size/format/present mode after DPI, resize, or monitor change.
- [ ] Add a windowed resize/monitor lifecycle proof on applicable hardware.
- [x] Update lifecycle/platform/errors docs.
- [x] Acceptance: recoverable surface churn does not freeze silently, and
  programming/validation failures cannot be hidden.

Validation ledger:

- `focused red`: the initial six-outcome policy test failed to compile because
  `SurfaceAcquireStatus`, `SurfaceAcquireAction`, and
  `SurfaceAcquisitionPolicy` did not exist. After checking the pinned wgpu 29
  API, a second test-first correction deliberately failed on the missing
  `FailLost` action: wgpu documents `Outdated` as configure-and-retry but `Lost`
  as requiring `Instance::create_surface`, so the review's recommendation to
  configure a lost surface was not accepted.
- `classification`: product silent-failure defect plus one inaccurate remedy in
  the review. Native and browser draw paths converted every non-success status,
  including validation, to `Ok(...)`; the outer renderer then reported a
  rendered frame. `Outdated` is locally recoverable, while `Lost` crosses the
  host-owned surface recreation boundary.
- `implementation`: attached native, WebGPU, WebGL2, and empty-scene surface
  paths share one acquisition policy. `Outdated` refreshes size, format,
  present mode, alpha mode, and usage and retries once. `Lost` returns and
  latches typed `SurfaceLost` for host recreation. Timeout/occlusion return
  `RenderOutcome::skipped` without a submission and increment dedicated public
  counters. Direct acquisition validation and uncaptured validation/OOM device
  errors return structured hard failures. Suboptimal frames present once and
  then reconfigure; changed format/present mode requires prepare before another
  draw.
- `focused green`: remote `cargo test -p scena surface_frame` passed 6/6,
  covering one-and-only-one outdated retry, lost-surface recreation without a
  fake retry, timeout/occlusion skips, validation/OOM hard failures, suboptimal
  post-present reconfiguration, and independent runtime-fault-channel mapping.
- `browser/WASM`: the all-feature `wasm32-unknown-unknown` check passed with the
  same six known target-cfg dead-code warnings recorded in earlier slices and
  no C12 warnings. The rebuilt WebGL2/WASM lifecycle proof passed with a real
  presented render sequence; it is synthetic headless-browser evidence, not a
  physical window/monitor or device-loss claim.
- `doctor/docs`: `C12-SURFACE-ACQUISITION` pins the bounded policy, typed lost
  boundary, native and browser callers, public counters/reports, runtime error
  callback, regression names, README, lifecycle, platforms, errors, API,
  changelog, and v1.8 errata. Its unbounded-retry mutation passed 1/1 and full
  doctor passed. Older architecture rules now require centralized acquisition
  instead of direct browser calls. New frame diagnostics/status logic was
  split into owner modules rather than weakening the 500-line KISS gate.
- `scoped gates`: remote `cargo fmt --check` and strict
  `cargo clippy --all-targets --all-features -- -D warnings` passed. The first
  WASM attempt found an oversized JSON macro expansion after adding four
  counters; explicit additive object insertion fixed it without increasing the
  crate recursion limit.
- `physical proof`: not run. The Hetzner builder is CPU/software-browser
  infrastructure and cannot provide an honest window move, DPI/monitor churn,
  or physical surface-loss injection. The hardware checklist row remains open
  for the final applicable GPU evidence checkpoint.
- `full`: deferred to Checkpoint D. No workspace-wide test, rustdoc,
  physical-GPU matrix, performance distribution, package, publish, or release-
  evidence chain was run for this surface slice.
- `bootstrap`: canonical source `/home/johannes/projects/scena`, isolated
  destination
  `/home/johannes/.cache/codex-worktrees/scena-full-review-v18-checklist`,
  branch `main`, HEAD `7b4fc9ca77e12fd12a69fab92650e1e46ee10354`;
  shared checkout absent; `AGENTS.md` hash
  `d0ed47595f6b6a6ec733651dfde650b7893aa2939d6137dc2f7eeade7528ac7a`
  and skills aggregate
  `ad212b6090c76bfaaf80fdb94eddd0be1703dd55d05d13732031de6944e5b111`
  matched after every explicit bootstrap.
- `counts`: about 70 minutes investigation; one production implementation with
  one pre-proof contract correction, one compile-wiring correction, one doctor
  fixture correction, one architecture extraction, zero release-candidate
  pushes, zero full-matrix runs, and zero user-required actions. No identical
  failure signature reached two remediation attempts.

### C13 — Clip CPU triangles against near and far planes (review B9)

Owner: CPU rasterizer projection/binning/raster stages.

- [x] Add triangles crossing near, far, and both planes with UV, normal, color,
  tangent, depth, transparency, and picking/semantic attributes.
- [x] Pin the old whole-triangle drop when one vertex is outside depth range.
- [x] Clip in view/clip space with Sutherland-Hodgman or equivalent before
  perspective division and binning.
- [x] Interpolate every downstream attribute perspective-correctly and preserve
  winding/culling.
- [x] Avoid double clipping/projection between binning and band rasterization.
- [x] Test degenerate results, reversed-Z conventions, vertices exactly on a
  plane, and large triangles spanning the camera.
- [x] Add CPU/GPU rendered parity with the camera close to geometry.
- [x] Update headless/CPU limitations docs and remove the limitation once proof
  is green.
- [x] Acceptance: crossing triangles are clipped into visible polygons rather
  than disappearing or producing NaN/depth artifacts.

Validation ledger (2026-07-20):

- `focused red`: the new near-plane rendered proof produced zero foreground
  pixels on the old CPU path because projection rejected one vertex and dropped
  the entire triangle. Classification: product defect. A later large-triangle
  fixture initially missed the viewport and the first exact color expectation
  used CSS palette constants instead of unit-basis linear RGB; both were
  test-harness defects corrected without changing production behavior.
- `implementation`: `cpu_geometry` now clips the complete triangle payload
  against near and far view-depth planes with fixed-capacity
  Sutherland-Hodgman storage before perspective division. It triangulates the
  retained polygon without changing winding, passes exact plane depth into the
  projection boundary, and retains position/color, normal, UV0, tangent,
  tangent handedness, and shadow visibility. Raster interpolation remains
  camera-owned and perspective-correct; post-projection depth remains affine.
- `projection ownership`: row-band construction now projects/clips each
  primitive once into `CpuProjectedPrimitive`; serial, parallel, opaque, OIT,
  transmission, and semantic-AOV consumers reuse that cache instead of
  repeating clipping or projection during band rasterization.
- `focused green`: `c13_cpu_depth_clipping` passed 8/8 for near, far, both
  planes, exact-plane inclusion, empty/degenerate output, a camera-spanning
  triangle, OIT resolution, and retained scene-picking node identity. The
  `render::cpu_` library filter passed 7/7; the exact complete-payload test
  verifies the two generated intersections numerically for linear color and
  every `PrimitiveVertexAttributes` field, and the winding test covers every
  generated triangle. Semantic AOV proof passed 1/1 with stable palette
  identity, finite in-slab depth, and finite normals.
- `picking/semantic boundary`: picking is a scene-owned camera-ray query rather
  than a rasterized vertex attribute, so clipping cannot invent a replacement
  identity. The focused pick test proves the near-crossing primitive retains
  its original node target; the semantic AOV test proves the raster path keeps
  primitive identity together with clipped depth and normals.
- `GPU/reversed-Z`: the env-gated headless-GPU parity proof passed 1/1 on the
  builder's software wgpu/lavapipe adapter and explicitly asserted the GPU
  reversed-Z capability contract. It compares CPU and GPU foreground coverage,
  full-frame RMSE, and mean channel delta for a near-crossing triangle. With
  the required env flag absent it emits `release_evidence:false` metadata
  rather than claiming hardware proof; this scoped result is not physical-GPU
  release evidence.
- `doctor/docs`: `FULL-REVIEW-C13-CPU-DEPTH-CLIPPING` pins both depth-plane
  clips, complete-payload interpolation, the single projection cache, every
  renderer consumer, CPU/GPU and semantic/picking proofs, and the limitation
  removal. Its near-plane-removal mutation passed 1/1 and full doctor reported
  `mode=Full status=pass`. README, headless/rendering docs, `[Unreleased]`, and
  the v1.8.0 post-release erratum now state the corrected contract.
- `scoped gates`: remote `cargo fmt --all --check`, strict all-target/all-feature
  Clippy, and `cargo check -p scena --target wasm32-unknown-unknown
  --all-features` passed. The WASM check retained only two known target-cfg
  dead-code warnings unrelated to C13. No C13 code changed after those broad
  scoped gates except test assertions/imports and an owner-specific type rename;
  focused proofs and full doctor were rerun for those exact surfaces.
- `full`: deferred to Checkpoint D. No workspace-wide test, rustdoc,
  browser/physical-GPU matrix, performance distribution, package, publish, or
  full release chain was run for this CPU rasterizer slice.
- `bootstrap`: canonical source `/home/johannes/projects/scena`, isolated
  destination
  `/home/johannes/.cache/codex-worktrees/scena-full-review-v18-checklist`,
  branch `main`, HEAD `7b4fc9ca77e12fd12a69fab92650e1e46ee10354`;
  shared checkout absent; `AGENTS.md` hash
  `d0ed47595f6b6a6ec733651dfde650b7893aa2939d6137dc2f7eeade7528ac7a`
  and skills aggregate
  `ad212b6090c76bfaaf80fdb94eddd0be1703dd55d05d13732031de6944e5b111`
  matched after every explicit bootstrap.
- `counts`: about 115 minutes investigation; one production implementation;
  two compile-wiring corrections, two Clippy corrections, three doctor-policy
  corrections, two test-fixture corrections, and one zero-test filter
  invocation corrected before recording doctor mutation evidence; zero
  release-candidate pushes; zero full-matrix runs; zero user-required actions.
  No identical failure signature reached two failed remediation attempts.

### C14 — Complete common glTF semantic handling without silent fallback
(review B10 plus missed texture-coordinate case)

Owner: glTF mesh/material/node import and diagnostics.

#### C14a — Missing normals

- [x] Add indexed and nonindexed meshes without `NORMAL`, including hard edges
  and degenerate faces.
- [x] Replace constant `(0,0,1)` fabrication with computed normals matching the
  chosen glTF shading contract.
- [x] Define vertex splitting/flat-vs-smooth behavior and reject irrecoverable
  degenerate data with a precise diagnostic.

#### C14b — UV set selection

- [x] Add base-color, metallic-roughness, normal, emissive, occlusion, and
  extension texture fixtures using `texCoord: 1`.
- [x] Support required `TEXCOORD_n` data through prepared materials and both
  CPU/GPU shaders, or fail closed with the exact unsupported set.
- [x] Validate ordinary `textureInfo.texCoord`; do not limit validation to
  `KHR_texture_transform` overrides.
- [x] Never silently sample UV0 for a texture that requests UV1.

#### C14c — Skin influence sets

- [x] Add JOINTS_0/WEIGHTS_0 plus JOINTS_1/WEIGHTS_1 fixtures with more than
  four valid influences.
- [x] Define and document the renderer limit. If limited, combine sets,
  deterministically select the strongest influences, renormalize, and emit a
  structured degradation report; otherwise support all required influences.
- [x] Assert zero/invalid sums and out-of-range joints fail predictably.

#### C14d — Node morph-weight overrides

- [x] Add shared-mesh nodes with distinct node-level morph weights.
- [x] Apply node overrides before animation and preserve mesh defaults when no
  override exists.
- [x] Verify multi-primitive meshes and animation target cardinality.

#### C14e — Integrated importer proof and docs

- [x] Add official-spec-shaped fixtures and at least one real-world asset for
  each supported semantic.
- [x] Assert the asset-load report records computed/degraded/rejected behavior.
- [x] Add CPU and GPU rendered comparisons where the semantic is visual.
- [x] Update `docs/assets.md`, supported glTF extension/attribute tables,
  diagnostics, and limitations. Do not claim full glTF support while silently
  dropping data.
- [x] Add `[Unreleased]` notes and migration guidance for newly strict inputs.
- [x] Acceptance: every requested normal/UV/influence/morph semantic is either
  rendered correctly or rejected/reported explicitly.

Validation ledger (2026-07-20):

- `focused`: first captured the old missing-normal behavior with indexed and
  nonindexed fixtures: indexed geometry retained shared vertices and every
  normal was fabricated as positive Z; the degenerate fixture also loaded.
  The same focused file now passes all 16 contracts, including flat-face vertex
  splitting, hard edges, precise degenerate rejection, ordinary and extension
  texture-slot UV1 rejection, eight source skin influences reduced to the four
  strongest, invalid/out-of-range joints, node morph overrides, report
  warnings, and a rendered Khronos SimpleSkin asset.
- `focused`: the old importer ignored `JOINTS_1`/`WEIGHTS_1`, selecting joints
  `[0, 1, 2, 3]` where the combined strongest-four contract requires
  `[4, 1, 3, 2]`; the old node path likewise returned the mesh default weight
  `0.1` instead of the authored node override `0.75`. Both exact regressions
  are green after the importer change.
- `focused`: the official Khronos MorphCube animation test and the
  TextureSettings/TextureTransform material test each pass independently.
  SimpleSkin exercises missing normals and skinning in a real asset. UV1 is
  deliberately unsupported and now fails closed with the material slot and
  requested `TEXCOORD_1`; supported UV0 behavior remains covered by the
  Khronos texture samples.
- `focused`: `c14_gltf_semantic_parity` passes under software wgpu/Lavapipe
  with `SCENA_RUN_UNSTABLE_HEADLESS_GPU_RELEASE_TESTS=1`, comparing the same
  Khronos SimpleSkin scene through CPU and GPU renderers. This is backend
  integration evidence, not a claim of physical-hardware GPU proof.
- `scoped`: the complete bundled Khronos sample loader, stable
  `asset_load_report.v1` golden, C14 doctor mutation, `cargo fmt --check`,
  strict all-target/all-feature Clippy, all-feature wasm32 check, and
  `doctor --full` passed on the remote builder. The wasm check emitted only the
  two pre-existing target-specific dead-code warnings for
  `gpu_supersample_frame` and `has_surface`.
- `contract`: glTF triangles without authored normals use deterministic flat
  face normals and split shared vertices; irrecoverable faces fail. The loader
  accepts at most `JOINTS_0/1` plus `WEIGHTS_0/1`, retains four normalized
  influences, and emits `SkinInfluencesTruncated` only when nonzero influences
  are discarded. Computed normals emit `ComputedFlatNormals`. Successful
  computation/degradation is represented in `asset_load_report.v1`; rejected
  UV, malformed normal, joint, and morph data remain structured `AssetError`s
  because no asset transaction is committed.
- `doctor`: `C14-GLTF-SEMANTIC-HANDLING` pins the new normal, UV, skin,
  node-morph, report, test, and documentation ownership. Its mutation disabling
  ordinary texture-slot discovery is rejected. Existing C04 skin pins now
  follow the extracted strongest-four implementation rather than a stale
  single-set normalization detail.
- `docs`: updated the asset guide, API/report contract, error remedies, README
  support boundary, changelog, post-release erratum, and stable JSON golden.
- `bootstrap`: canonical source `/home/johannes/projects/scena`; isolated
  destination `/home/johannes/.cache/codex-worktrees/scena-full-review-v18-checklist`;
  branch `main`; base HEAD `7b4fc9ca77e12fd12a69fab92650e1e46ee10354`;
  `AGENTS.md` hash
  `d0ed47595f6b6a6ec733651dfde650b7893aa2939d6137dc2f7eeade7528ac7a`
  and skills aggregate
  `ad212b6090c76bfaaf80fdb94eddd0be1703dd55d05d13732031de6944e5b111`
  matched after every explicit bootstrap.
- `counts`: about 100 minutes investigation; zero release-candidate pushes;
  zero full-matrix runs; zero user-required actions. Corrections comprised one
  public-test API typo, one zero-test filter invocation, one warning-consumer
  wiring omission, one doctor-test wiring omission, two strict-Clippy findings,
  and two doctor ownership/size findings resolved by extraction rather than a
  threshold increase. No identical product failure reached two failed
  remediation attempts.
- `full`: intentionally deferred to Checkpoint D because this slice's focused
  and scoped gates are green and the repository-wide remediation batch remains
  active.

### C15 — Validate anchor and connector transform extras strictly (smaller and
missed broader finding)

Owner: shared glTF anchor/connector extras parser.

- [x] Add nonfinite, zero-length, parallel, unpaired forward/up, malformed
  matrix, nondecomposable matrix, and zero-scale TRS fixtures.
- [x] Centralize anchor and connector transform validation instead of allowing
  their rules to diverge.
- [x] Require finite vectors, nonzero bases, nonparallel forward/up, finite
  16-value matrices, valid decomposition, finite quaternions, and valid scale.
- [x] Fail the asset transaction with a path-qualified diagnostic; never degrade
  invalid authored orientation to identity.
- [x] Keep the existing import-unit composition contract intact.
- [x] Update authoring guides and JSON examples.
- [x] Acceptance: invalid authored metadata cannot silently change orientation
  or partially instantiate a scene.

Validation ledger (2026-07-20):

- `classification`: product defect. The old shared parser returned identity for
  zero, parallel, or unpaired forward/up bases and malformed matrices; anchor
  zero scale was retained as deferred `invalid_reason`, while connector TRS was
  not transform-validated at all. The first executable product proof passed
  compilation and failed 0/4 exactly on those load-time expectations.
- `focused`: `c15_gltf_marker_transform_contracts` now passes 5/5. It covers
  f64-to-f32 overflow in translation, quaternion, forward, and matrix values;
  zero and parallel bases; unpaired forward/up; zero scale; zero quaternion;
  short, projective, sheared, and overflow matrices; valid basis units; and a
  valid nonuniform-scale matrix preserving translation, normalized rotation,
  and scale.
- `contract`: `parse_marker_transform` is the single anchor/connector transform
  owner. It accepts either finite TRS with normalized quaternion, a paired
  finite nonzero nonparallel forward/up basis, or a finite column-major affine
  16-value matrix that round-trips through a nonzero-scale TRS decomposition.
  Matrix plus TRS/basis fields is rejected as ambiguous. Every rejection is an
  `AssetError::Parse` containing the asset path and exact
  `nodes[n].extras.scena.{anchors|connectors}[n].field` path before the local
  asset transaction can be committed.
- `scoped`: the unchanged C05 import-unit suite passes 4/4; the renamed legacy
  invalid-anchor load test and asset-catalog readiness test each pass. This
  proves explicit marker units, inherited import units, connector composition,
  and catalog failure reporting remain intact across the stricter boundary.
- `scoped`: `cargo fmt --all -- --check`, strict all-target/all-feature Clippy,
  all-feature wasm32 check, and `doctor --full` passed on the remote builder.
  The wasm check emitted only the same pre-existing target-specific dead-code
  warnings for `gpu_supersample_frame` and `has_surface`.
- `doctor`: `C15-GLTF-MARKER-TRANSFORMS` pins common parser ownership, both
  callers' transactional `Result` flow, exact test cases, authoring/error docs,
  README, changelog, and the v1.8.0 erratum. Its mutation replacing paired-basis
  validation with `false` is rejected. The older M3A ownership and evidence
  pins were moved to the new strict parser and load-time test rather than
  weakened.
- `docs`: expanded the anchor/connector authoring guide with quaternion,
  forward/up, matrix, exclusivity, units, and JSON-path rules plus a valid
  matrix example; updated asset/error/troubleshooting docs, README security
  behavior, changelog, and the v1.8.0 post-release erratum.
- `bootstrap`: canonical source `/home/johannes/projects/scena`; isolated
  destination `/home/johannes/.cache/codex-worktrees/scena-full-review-v18-checklist`;
  branch `main`; base HEAD `7b4fc9ca77e12fd12a69fab92650e1e46ee10354`;
  `AGENTS.md` hash
  `d0ed47595f6b6a6ec733651dfde650b7893aa2939d6137dc2f7eeade7528ac7a`
  and skills aggregate
  `ad212b6090c76bfaaf80fdb94eddd0be1703dd55d05d13732031de6944e5b111`
  matched after every full-tree bootstrap.
- `counts`: about 55 minutes investigation; one production remediation
  attempt; one initial test-harness import correction; one zero-test doctor
  filter invocation corrected; one stale doctor evidence pin corrected; one
  mechanical rustfmt pass. An absolute-prefix rsync created seven duplicate
  formatted files under a local generated `home/` subtree; all seven were
  enumerated and that exact subtree was removed before explicit-destination
  copies. Zero release-candidate pushes, zero full-matrix runs, and zero
  user-required actions. No identical product failure reached two remediation
  attempts.
- `full`: intentionally deferred to Checkpoint D because C15's focused and
  scoped gates are green and no cross-backend renderer behavior changed.

## 4. Public API, primitives, browser controls, and capability behavior

### C16 — Resolve `Transform::scale_by` semantic inconsistency

- [x] Decide publicly whether `scale_by` composes multiplicatively or whether
  it is a setter with a corrected name such as `set_scale`.
- [x] Add order-sensitive tests alongside `rotate_*_deg` and translation
  composition.
- [x] Prefer a deprecation/migration path if changing existing public behavior.
- [x] Update API docs, examples, migration guide, and public API freeze.
- [x] Acceptance: method names, docs, and composition behavior teach one
  unambiguous mental model.

Validation ledger (2026-07-20):

- `decision`: `Transform::scale_by(f32)` is multiplicative. Explicit
  replacement is owned by `with_scale(Vec3)` and
  `with_uniform_scale(f32)`. This aligns `scale_by` with the compositional
  `rotate_*_deg` vocabulary while keeping replacement concise and discoverable.
  A deprecation cannot preserve the ambiguous method name and simultaneously
  correct its semantics, so the migration is source-explicit: v1.8.0 code that
  depended on replacement changes `scale_by(x)` to `with_uniform_scale(x)`.
- `compatibility`: every pre-existing repository `scale_by` call starts from
  identity, `default`, or `at`, so its numeric result is unchanged. The one
  user-facing example that intends direct replacement now calls
  `with_uniform_scale`. SceneHost's private `TransformScaleExt` shim was removed
  after strict Clippy proved the new inherent `with_scale` method made it dead;
  recipe transform output is unchanged.
- `focused`: the pre-change C16 test compiled far enough to prove the explicit
  setters were absent, then failed on all three missing-method sites. After the
  implementation, `c16_transform_scale_semantics` passes 3/3: repeated scale
  multiplication, nonuniform-to-uniform replacement order, translation
  preservation, and rotation-call-order preservation are all exact.
- `scoped`: the M5 public-API freeze test passes with the verified base HEAD
  supplied through `SCENA_RELEASE_COMMIT`; its generated artifact includes
  `Transform::with_scale`, `Transform::with_uniform_scale`, and
  `Transform::scale_by`. The first run correctly failed provenance because the
  isolated rsync snapshot has no `.git`, then passed through the documented
  explicit-provenance path.
- `scoped`: `cargo fmt --all -- --check`, strict all-target/all-feature Clippy,
  `cargo check --examples --all-features`, rustdoc with `-D warnings`, and
  `doctor --full` passed on the remote builder. A separate wasm rebuild was not
  repeated: C15's all-feature wasm check was green immediately before this
  target-independent const math/API slice, and no platform module changed.
- `doctor`: `C16-TRANSFORM-SCALE-SEMANTICS` pins the three method bodies, call
  order tests, migration/API/public-freeze docs, README, example, changelog, and
  v1.8.0 erratum. Its mutation replacing the X-axis multiplication with direct
  assignment is rejected.
- `docs`: updated API reference, Three.js migration guide, active public API
  index, README happy path, runnable layers example, changelog, and the v1.8.0
  post-release correction note.
- `bootstrap`: canonical source `/home/johannes/projects/scena`; isolated
  destination `/home/johannes/.cache/codex-worktrees/scena-full-review-v18-checklist`;
  branch `main`; base HEAD `7b4fc9ca77e12fd12a69fab92650e1e46ee10354`;
  `AGENTS.md` hash
  `d0ed47595f6b6a6ec733651dfde650b7893aa2939d6137dc2f7eeade7528ac7a`
  and skills aggregate
  `ad212b6090c76bfaaf80fdb94eddd0be1703dd55d05d13732031de6944e5b111`
  matched after the full-tree bootstrap.
- `counts`: about 35 minutes investigation; one production remediation; one
  doctor-fixture copy correction; one expected provenance-environment
  correction; one rustfmt layout correction; and one strict-Clippy duplicate
  owner removal. Zero release-candidate pushes, zero full-matrix runs, and zero
  user-required actions. No identical product failure reached two remediation
  attempts.
- `full`: intentionally deferred to Checkpoint D. The public API received its
  focused freeze/doc/example gates, but the repository-wide test chain remains
  a one-time final integration action.

### C17 — Make centering/framing operate on visible bounds and target aspect

- [x] Add scenes whose origin is far from visible bounds, multiple imports,
  hidden helpers, and portrait/landscape targets unlike the camera's old
  aspect.
- [x] Define `center_on` as origin alignment or visible-bounds centering; if
  behavior changes, introduce a precise new method and deprecate ambiguity.
- [x] Route legacy `frame_all`/`frame_import` through existing
  `FramingOptions` instead of creating another framing API.
- [x] Use target output aspect for capture framing and expose view direction,
  padding, clipping, and helper inclusion.
- [x] Add projection-based rendered assertions, not only camera-number tests.
- [x] Update README viewer setup, API docs, framing guide, and examples.
- [x] Acceptance: visible content is centered/fitted for the actual target and
  callers can select non-dead-front views.

Validation ledger (2026-07-20):

- `classification`: product defect. Origin-only centering, hidden/helper
  aggregate inclusion, camera-aspect reuse, and forced front framing were
  independently reproduced. Scoped integration then exposed two related
  product defects: interactive controls discarded the initial framing angles,
  and SceneHost aggregate framing retained a stale depth range after
  single-node framing. One legacy no-light test also had a harness defect: it
  inferred darkness from projected nonzero-pixel area even though the CPU
  diagnostic path intentionally preserves PBR base color without direct
  lighting.
- `focused`: `c17_visible_bounds_framing` passes 4/4. The deterministic cases
  cover an origin 110 units from visible content, two independently placed
  imports, hidden and tagged-helper exclusion with explicit helper opt-in, a
  portrait 240x720 target replacing a 16:9 camera aspect, a selectable
  three-quarter view, pixel margin, tightened clipping, projected-rectangle
  containment, and centered/unclipped CPU pixels.
- `contract`: `move_origin_to` is the explicit origin operation;
  `center_visible_bounds_on` centers visible non-helper subtree geometry; the
  ambiguous `center_on` spelling is deprecated. `frame_all`,
  `frame_all_with_assets`, and `frame_import` now delegate to the existing
  `FramingOptions` model. Option-bearing variants expose the actual viewport,
  view preset/direction, fill, margin, depth tightening, and helper inclusion.
  Hidden nodes remain excluded. Bounds aggregation moved to
  `scene/view_bounds.rs` so both scene owners remain below the 500-line KISS
  limit.
- `viewer/host`: headless viewers use their configured width/height;
  interactive viewers use `PlatformSurface::size`; SceneHost uses its logical
  viewport and tightens near/far for each aggregate solve. Interactive
  `OrbitControls` now receives the same `FramingOutcome`, preserving 45-degree
  yaw and 30-degree pitch instead of snapping to front on first input.
- `scoped`: the orbit-framing test failed before the control handoff and then
  passed; the full `m7_interactive_viewer` file passes 15/15. The SceneHost
  frame-all proof failed with `framed bounds project outside the camera depth
  range` before depth tightening and then passed; the instanced frame/pick
  proof also passes. In `first_render_api`, 10/11 tests passed immediately;
  the stale pixel-area assertion was replaced with the existing structured
  `MissingLightingOrEnvironment` plus `fallback_applied == false` contract,
  after which its focused test passed. Unchanged passing tests were not rerun.
- `public/docs`: the M5 public API freeze passes with explicit base-commit
  provenance and now records the aggregate/import option methods, centering
  methods, and helper option. README, API reference, rendering guide,
  Three.js migration guide, public API contract, camera framing example,
  changelog, and v1.8.0 erratum all describe actual-target framing and the
  centering split. All-feature examples and rustdoc with `-D warnings` pass.
- `platform`: strict all-target/all-feature Clippy and the all-feature
  `wasm32-unknown-unknown` check pass. The WASM check reports only the same two
  pre-existing target-specific dead-code warnings for
  `gpu_supersample_frame` and `has_surface`.
- `doctor`: `C17-VISIBLE-BOUNDS-FRAMING` pins API delegation, bounds ownership,
  helper exclusion, real target dimensions, orbit-control handoff, SceneHost
  depth tightening, executable tests, docs, example, public freeze, changelog,
  and erratum; its helper-default mutation is rejected. The C06 presentable
  viewer rule now pins the structured opt-out diagnostic rather than the stale
  near-black-area wording. Both mutation tests and `doctor --full` pass.
- `bootstrap`: canonical source `/home/johannes/projects/scena`; isolated
  destination `/home/johannes/.cache/codex-worktrees/scena-full-review-v18-checklist`;
  branch `main`; base HEAD `7b4fc9ca77e12fd12a69fab92650e1e46ee10354`;
  `AGENTS.md` hash
  `d0ed47595f6b6a6ec733651dfde650b7893aa2939d6137dc2f7eeade7528ac7a`
  and skills aggregate
  `ad212b6090c76bfaaf80fdb94eddd0be1703dd55d05d13732031de6944e5b111`
  matched after each full-tree bootstrap.
- `counts`: about 70 minutes investigation; three product remediation
  signatures with one implementation attempt each; one stale harness
  assertion with one failed threshold remedy, one discriminating intensity
  probe, and one successful contract correction; two zero-test command-filter
  corrections; two doctor/public-doc needle corrections; two mechanical
  rustfmt passes. Zero release-candidate pushes, zero full-matrix runs, and
  zero user-required actions. No product signature reached the two-failed-
  remediation circuit breaker.
- `full`: intentionally deferred to Checkpoint D. C17 changed public and
  cross-platform viewer behavior, so it received focused render, viewer,
  SceneHost, WASM, Clippy, docs, example, rustdoc, freeze, and doctor proof;
  the repository-wide release chain remains a one-time final integration run.

### C18 — Remove the public polyline panic

- [x] Add direct API no-unwind tests for zero and one point.
- [x] Make the preferred constructor return `Result<_, GeometryError>` and
  deprecate or clearly scope any panicking convenience constructor.
- [x] Keep recipe validation as defense in depth.
- [x] Add Rustdoc examples for valid and invalid construction.
- [x] Acceptance: untrusted/public construction has a structured error path.

Validation ledger (2026-07-20):

- `classification`: partially stale review finding plus public-contract gap.
  The v1.7.2 C01 remediation had already added
  `GeometryDesc::try_polyline -> Result`, zero/one-point catch-unwind proof,
  recipe validation/build rejection, and fallible SceneHost construction. The
  remaining defect was that the adjacent panicking `polyline` wrapper was
  still presented as an endorsed, undocumented peer.
- `decision`: preserve the infallible wrapper only for semver compatibility and
  deprecate it with an exact migration to `GeometryDesc::try_polyline` for
  runtime or untrusted input. Repository code no longer calls the wrapper.
  Changing `polyline`'s return type in place would be a larger source-breaking
  change without improving the already-available structured path.
- `test-first`: `C18-FALLIBLE-POLYLINE` was added before the deprecation/docs
  change and failed on the absent attribute, API guidance, public freeze,
  README, changelog, and erratum. After implementation its mutation removing
  the deprecation is rejected.
- `focused`: the existing direct catch-unwind proof passes for both zero and
  one point and returns exact `GeometryError::PolylineTooShort` values. The
  valid built-in geometry test passes after migrating its only wrapper call to
  `try_polyline`. Scene-recipe validation plus SceneHost build rejection passes
  with `scene-host` enabled for both invalid lengths.
- `docs/API`: `try_polyline` now has compiled Rustdoc for valid construction
  and the exact invalid result; its focused doctest passes 1/1 with
  `RUSTDOCFLAGS=-D warnings`. README, API reference, active public API contract,
  changelog, and the v1.8.0 erratum identify it as the preferred constructor.
  The M5 public-API freeze passes with explicit base-commit provenance and
  records `GeometryDesc::try_polyline`.
- `scoped`: `cargo fmt --all`, strict all-target/all-feature Clippy, the C18
  doctor mutation, and `doctor --full` pass on the remote builder. No WASM
  rebuild was repeated: the preceding C17 all-feature WASM check was green and
  C18 only adds target-independent deprecation/Rustdoc around an existing
  constructor.
- `bootstrap`: canonical source `/home/johannes/projects/scena`; isolated
  destination `/home/johannes/.cache/codex-worktrees/scena-full-review-v18-checklist`;
  branch `main`; base HEAD `7b4fc9ca77e12fd12a69fab92650e1e46ee10354`;
  `AGENTS.md` hash
  `d0ed47595f6b6a6ec733651dfde650b7893aa2939d6137dc2f7eeade7528ac7a`
  and skills aggregate
  `ad212b6090c76bfaaf80fdb94eddd0be1703dd55d05d13732031de6944e5b111`
  remain matched in the isolated checkout.
- `counts`: about 30 minutes investigation; one source-contract remediation;
  one expected test-first doctor failure; one mechanical rustfmt pass; zero
  release-candidate pushes, zero full-matrix runs, and zero user-required
  actions. No repeated failure reached a circuit breaker.
- `full`: intentionally deferred to Checkpoint D. The focused runtime,
  recipe, API-freeze, Rustdoc, strict-Clippy, and full-doctor surfaces are
  green; the repository-wide release chain remains a one-time final action.

### C19 — Add duplicated cylinder/cone UV seam vertices

- [x] Add a UV oracle and checkerboard render showing the last side quad.
- [x] Emit a duplicated seam column at `u=1` while preserving indexed topology,
  normals, tangents, caps, and expected vertex counts.
- [x] Cover cone tip behavior separately from the cylindrical seam.
- [x] Update primitive golden images/count fixtures as needed.
- [x] Acceptance: no side quad interpolates backward across the entire texture.

Validation ledger (2026-07-20):

- `classification`: confirmed product defect. Cylinder side rows contained only
  `segments` vertices, so the closing indexed quad reused the `u=0` vertex;
  cone faces duplicated positions but assigned the last base vertex `u=0`.
  Both paths interpolated the closing face backward across almost the entire
  texture.
- `test-first`: the C19 structural and checkerboard integration test was added
  before the production patch. The old cylinder failed the required 52-vertex
  seam column and the old cone failed the required `u=1` closing base. The
  initial rendered threshold was structure-blind, so a discriminating probe
  compared corrected UVs with an explicit old-behavior mutation; the stable
  oracle requires fewer checker transitions from the corrected mapping.
- `implementation`: cylinder sides now emit `segments + 1` vertices per row,
  with an exact duplicate position/normal at `u=1`; side indices advance into
  that seam while cap indices and topology remain independent. Cone faces keep
  face-local tips and assign their closing base to `(segment + 1) / segments`.
  The deterministic 12-segment cylinder fixture changes from 50 to 52 vertices
  while retaining 144 indices; cone remains 49 vertices and 72 indices.
- `focused`: `tests/c19_primitive_uv_seams.rs` passes 3/3, including per-side-
  triangle UV spans, separate cone-tip behavior, and the mutation-backed CPU
  checker render. The primitive mesh unit module passes 5/5 for counts,
  bounds, computed outward winding/normals, and signed-dimension invariance.
  The new production MikkTSpace proof confirms finite generated cylinder/cone
  tangents and matching tangent frames across both duplicated cylinder rows.
- `harness`: one focused rerun exposed that the structural test had treated
  interleaved cylinder side/cap indices as contiguous side-only blocks. It was
  classified as a harness defect and corrected to use each 12-index segment
  block; no production code changed for that signature. A later tangent test
  compile failure was the missing test-only `GeometryDesc` import and passed
  after that single harness correction.
- `docs/doctor`: source Rustdoc, README, rendering/API references, changelog,
  and v1.8.0 erratum now document seam-safe generated UVs and the changed
  cylinder count. `C19-PRIMITIVE-UV-SEAMS` pins implementation, count,
  structural/render/tangent proofs, and public documentation; its mutation
  restoring `side_row = segments` is rejected.
- `scoped`: remote `cargo fmt --all -- --check`, strict all-target/all-feature
  Clippy, the C19 doctor mutation, and `doctor --full` pass. No WASM or physical
  GPU lane was added because the geometry and deterministic checker proof are
  backend-independent and execute through the CPU renderer.
- `bootstrap`: canonical source `/home/johannes/projects/scena`; isolated
  destination `/home/johannes/.cache/codex-worktrees/scena-full-review-v18-checklist`;
  branch `main`; base HEAD `7b4fc9ca77e12fd12a69fab92650e1e46ee10354`;
  `AGENTS.md` hash
  `d0ed47595f6b6a6ec733651dfde650b7893aa2939d6137dc2f7eeade7528ac7a`
  and skills aggregate
  `ad212b6090c76bfaaf80fdb94eddd0be1703dd55d05d13732031de6944e5b111`
  remain matched in the isolated checkout.
- `counts`: about 50 minutes investigation; two product corrections (cylinder
  and cone) with one exact-seam refinement; one visual-oracle refinement backed
  by a known-bad mutation; one index-layout harness correction; one test-only
  import correction; one zero-test command-filter correction; two mechanical
  rustfmt corrections. Zero release-candidate pushes, zero full-matrix runs,
  and zero user-required actions. No repeated signature reached the circuit
  breaker.
- `full`: intentionally deferred to Checkpoint D. Focused geometry, rendered
  UV, tangent, strict-Clippy, and doctor surfaces are green; the repository-
  wide release chain remains a one-time final integration action.

### C20 — Finish pointer and WASM capability ergonomics

#### C20a — Pointer capture

- [x] Add a browser interaction test: pointer-down inside `<scena-viewer>`,
  move and release outside, then start a new orbit.
- [x] Use pointer capture/release plus `pointercancel` and lost-capture cleanup.
- [x] Preserve multi-pointer/touch behavior and remove listeners on disconnect.
- [x] Update browser controls/accessibility docs.
- [x] Acceptance: orbit state never sticks after outside release/cancellation.

#### C20b — MSAA degradation on WASM

- [x] Add capability matrices for WebGPU and WebGL2 sample counts/formats.
- [x] Let automatic/profile-selected MSAA degrade to a supported mode with a
  structured capability report.
- [x] Keep an explicit unsupported sample-count request as an actionable error
  when silently changing user intent would be wrong.
- [x] Update capabilities/browser/rendering docs and JSON fixtures.
- [x] Acceptance: portable defaults run, explicit demands fail honestly.

#### C20c — Environment-driven GPU execution

- [x] Pin that `SCENA_USE_GPU` remains test/proof metadata and cannot change CLI
  renderer execution.
- [x] Make the explicit `--gpu` flag the sole CLI backend selector and document
  its precedence and fallback contract.
- [x] Report selected backend and selection source in every result envelope.
- [x] Update CLI help, environment-variable docs, CLAUDE/agent instructions,
  and troubleshooting.
- [x] Acceptance: no undocumented environment state silently changes execution.

Validation ledger — C20:

- `focused`: the pre-fix Playwright pointer sequence reported
  `pointer_capture_outside_release=false` and `reentry_clean=false`; after a
  forced task-local WASM rebuild the viewer-element-only sequence passed
  outside release, clean re-entry, zero active pointers, and a second 18/11
  orbit delta. The `msaa-capability` browser workflow passed on both WebGL2 and
  WebGPU, including automatic FXAA fallback and exact `Msaa4` rejection.
  `c20_wasm_capability_contracts` passes two matrix/backward-shape tests; the
  default and explicit-GPU CLI render tests pass with `inspection,scene-host`;
  capture, CAD, and `--help` envelope assertions pass.
- `scoped`: `cargo fmt --all -- --check`, strict
  `cargo clippy --all-targets --all-features -- -D warnings`, capability and
  capture stable-contract goldens, the M5 public-API freeze test, C20 doctor
  mutation, and `xtask doctor --full` all pass on the isolated builder.
- `doctor`: `C20-BROWSER-EXECUTION-ERGONOMICS` pins pointer lifecycle, live
  sample matrices, automatic-versus-explicit MSAA behavior, CLI selection
  source/fallback reporting, help/docs, and the registered force-rebuild flag.
- `provenance`: canonical source `/home/johannes/projects/scena`; isolated
  checkout `/home/johannes/.cache/codex-worktrees/scena-full-review-v18-checklist`;
  branch `main`; base HEAD `7b4fc9ca77e12fd12a69fab92650e1e46ee10354`;
  `AGENTS.md` hash
  `d0ed47595f6b6a6ec733651dfde650b7893aa2939d6137dc2f7eeade7528ac7a`
  and skills aggregate
  `ad212b6090c76bfaaf80fdb94eddd0be1703dd55d05d13732031de6944e5b111`
  matched after every sync.
- `counts`: about 65 minutes investigation; one stale-browser-package
  provenance correction, one doctor-needle remediation attempt, and one
  source-ownership split after the capability owner crossed its enforced size
  ceiling. Zero release-candidate pushes, zero full-matrix runs, and zero
  user-required actions. No product signature crossed the circuit breaker.
- `full`: intentionally deferred to Checkpoint D. The focused browser, CLI,
  schema, strict-Clippy, and doctor surfaces are green; no physical-hardware GPU
  claim is inferred from the builder browser run.

### C21 — Orient generated CAD inspection lighting

Additional defect discovered by the C20 CAD-envelope integration proof: three
directional light presets without transforms are co-directional. The generated
broad-face inspection could therefore be almost black and fail its own quality
verification even though the CAD geometry and framing were valid.

- [x] Preserve the failing `recipe inspect-cad` broad-face result with
  `severe_black_crush` as the test-first proof.
- [x] Generate the existing oriented `studio_rig` instead of three identity-
  transform directional lights, with the smallest robust presentation exposure
  correction proven by the focused probe.
- [x] Prove broad-face, top-feature, and overview renders all pass verification
  and retain their backend-selection envelopes.
- [x] Add a doctor mutation that rejects removal of the oriented rig.
- [x] Update CAD agent guidance, changelog, and v1.8.0 errata.
- [x] Acceptance: the CAD inspection command passes its own quality proof for
  the canonical terminal asset.

Validation ledger — C21:

- `focused`: the canonical CAD CLI test first failed with
  `severe_black_crush` at low-clip fraction 0.862. The oriented rig alone
  improved it to 0.827 but retained the signature. At the two-attempt circuit
  breaker, production/test edits stopped. Smaller probes showed the opposite
  face (0.864) and no-edge-overlay variant (0.830) still failed, while +0.1 EV
  passed at 0.710; +0.25 EV passed at 0.630 and was selected for margin. The
  final canonical three-view CAD test passes, including every nested and
  top-level backend-selection envelope.
- `scoped`: `C21-CAD-INSPECTION-LIGHTING` mutation proof, formatting, strict
  all-feature Clippy, and `xtask doctor --full` pass remotely.
- `docs`: CAD agent guidance, changelog, and v1.8.0 post-release errata explain
  that the oriented studio rig and +0.25 EV are presentation-only.
- `counts`: about 25 minutes investigation; two failed remediation attempts
  with one signature, one circuit-breaker diagnostic batch, and one informed
  correction after the probes distinguished illumination from face direction,
  overlays, and oracle failure. Zero release-candidate pushes, zero full-matrix
  runs, and zero user-required actions.
- `full`: intentionally deferred to Checkpoint D because the exact CLI visual
  proof, strict-Clippy surface, and doctor mutation are green.

## 5. Agent and CLI contract remediation

### A01 — Make validation and execution resolve exactly the same recipe inputs

- [x] Create recipes referencing imports, `scene.environment.uri`, fonts,
  textures, builtin presets, and nested external glTF dependencies.
- [x] Share one resource enumeration/resolution plan between
  `validate-recipe`, `recipe build`, and render-time execution.
- [x] Allow validation to choose syntax-only versus full-resolution mode
  explicitly; never label syntax-only validation as execution equivalence.
- [x] Include policy root, normalized URI, required/optional state, and remedy
  in diagnostics.
- [x] Acceptance: full validation success implies the same resolver can start
  the build without discovering a new missing/forbidden authored resource.

Validation ledger — A01 (2026-07-20):

- `focused red`: the new all-resource CLI fixture failed against the old path:
  missing environment, font, and texture resources passed validation; the CLI
  rejected explicit `--syntax-only`; and the report had no resource inventory.
  The nested-glTF fixture pinned discovery of a missing external buffer, while
  the scene-host fixture required one full-validation success to proceed through
  `recipe build` without a newly discovered authored-resource policy failure.
- `classification`: product/contract defect. Structural validation and build
  independently enumerated only subsets of recipe resources, so validation
  success did not imply that execution could resolve the same document.
- `implementation`: `RecipeResourcePlan` is now the shared, policy-owned
  inventory for imports, environment URIs and builtins, fonts, and every
  authored material texture slot. Full validation resolves and loads that plan,
  including nested glTF dependencies; recipe build runs the same plan as its
  authoritative preflight and consumes its resolved import URI. The default CLI
  mode is full resolution; `--syntax-only` is explicit and reports
  `execution_equivalent:false`. Reports include the effective policy and
  path-qualified resource status, normalized URI, roots, required state, and
  remedies. The optional policy payload is boxed internally to keep the public
  validation error below the strict Clippy size ceiling without changing JSON.
- `focused green`: `cargo test --all-features --test a01_recipe_resolution --
  --nocapture` passes 4/4; the default-feature variant passes its applicable
  3/3 tests. The same all-feature focused result was reused after formatting-only
  edits because the behavior surface did not change.
- `scoped`: `cargo fmt --all -- --check`; the A01 doctor mutation; both
  `scene_recipe_validation` golden/backward-shape tests; the M5 public API freeze
  with `SCENA_RELEASE_COMMIT=7b4fc9ca77e12fd12a69fab92650e1e46ee10354`;
  `cargo clippy --all-features --all-targets -- -D warnings`; and
  `cargo run -p xtask -- doctor --full` all pass on `scena-builder`.
- `gate findings`: the first doctor mutation run found two brittle source-text
  needles after rustfmt; that test-harness defect was fixed with stable semantic
  tokens and passed on remediation attempt 1. The first strict-Clippy run found
  `result_large_err` after the new inline policy report enlarged the error; the
  product/API-layout defect was fixed by boxing only the new optional field and
  passed on remediation attempt 1. Neither signature reached the circuit
  breaker.
- `documentation`: README, getting-started, API/schema, LLM-builder,
  troubleshooting, changelog, v1.8.0 errata, and the public app-builder skill
  now distinguish full resolution from syntax-only validation and use the full
  mode in execution-equivalent agent loops.
- `provenance/process`: canonical checkout
  `/home/johannes/projects/scena`, isolated validation checkout
  `/home/johannes/.cache/codex-worktrees/scena-full-review-v18-checklist`, branch
  `main`, base HEAD `7b4fc9ca77e12fd12a69fab92650e1e46ee10354`; AGENTS hash
  `d0ed47595f6b6a6ec733651dfde650b7893aa2939d6137dc2f7eeade7528ac7a`
  and skills hash
  `4620da681326be370174c5a04b67e954612e81d26911a6a7306bf892bfafff17`
  matched remotely. A01-specific elapsed time was not separately captured from
  the continuous goal clock (process-recording gap); remediation-attempt maxima
  were 1 per distinct signature, with zero release-candidate pushes, zero full-
  matrix runs, and zero user-required actions.
- `full`: intentionally deferred to Checkpoint D. A01 changed public schema/API
  and CLI behavior, but its focused and scoped surfaces are green and the batch
  policy reserves the complete matrix for the final stable diff.

### A02 — Add explicit, inspectable sandbox roots

- [x] Add repeatable `--allow-root PATH` and/or a policy-file option rather than
  a global sandbox-disable switch.
- [x] Canonicalize roots and reject traversal/symlink escapes under a documented
  policy.
- [x] Emit the effective policy in build/render results and a discovery command.
- [x] Use identical policy handling across validation, build, render, inspect,
  diagnose, and repair.
- [x] Update security, recipe, CLI, agent, and troubleshooting docs.
- [x] Acceptance: an operator can authorize an external model library narrowly
  and reproduce the exact policy from output metadata.

Validation ledger — A02 (2026-07-20):

- `focused red`: after one fixture-only compile correction, the three-test A02
  contract reached the old CLI and failed 0/3: `policy recipe` with options was
  an unknown command, every recipe-aware command rejected `--allow-root`, and
  the escape test therefore exited at argument parsing instead of exercising
  canonical containment.
- `classification`: product/contract defect. The core policy already failed
  closed on canonical resource paths, but CLI operators had no narrow way to
  extend its current-directory root and could not reproduce an effective
  override from command metadata.
- `implementation`: repeatable `--allow-root <directory>` is parsed through one
  CLI helper, requires an existing directory, canonicalizes before policy
  construction, and appends without removing compiled defaults. Per-root
  provenance distinguishes `compiled_default` from `operator_override`.
  Resource canonicalization and containment remain independent, so `..` and
  symlinks cannot escape an authorized library. The same policy object now
  flows through full validation, explicit recipe build/render, and
  asset-or-recipe render, inspect, diagnose, doctor, and repair. Recipe results
  expose the effective top-level `policy`; `scena policy recipe` previews it.
  Direct asset inputs reject this recipe-only option, and no global disable
  switch exists.
- `focused green`: `cargo test --all-features --test a02_recipe_policy_cli --
  --nocapture` passes 3/3. It covers two repeatable discovery roots plus a
  missing-root argument error; default denial followed by authorized success
  through validation, recipe build/render, legacy render, inspect, diagnose,
  doctor, and repair; and canonical parent-traversal plus symlink escape
  rejection.
- `scoped`: `cargo fmt --all -- --check`; the A02 doctor mutation;
  `scena_cli_help`; the FR04 command-contract check; default policy discovery;
  strict `cargo clippy --all-features --all-targets -- -D warnings`; strict
  default-feature `cargo clippy --all-targets -- -D warnings`; and
  `cargo run -p xtask -- doctor --full` all pass on `scena-builder`.
- `gate findings`: the initial test file omitted its `path_str` helper, and the
  next run assumed discovery nested its policy even though discovery is the
  policy document; both were distinct test-harness signatures corrected before
  acceptance. Default-feature Clippy then found the inspection-only direct-
  asset guard as dead code; matching its cfg to the owning command surface fixed
  that configuration defect on remediation attempt 1. No signature reached the
  two-attempt circuit breaker.
- `documentation`: README, getting-started, API/schema and asset security docs,
  LLM-builder guide and skill, troubleshooting, CLI help, changelog, and v1.8.0
  errata now describe the narrow-root workflow, canonical containment, policy
  metadata, direct-asset boundary, and absence of an unsandboxed mode.
- `provenance/process`: canonical checkout
  `/home/johannes/projects/scena`, isolated validation checkout
  `/home/johannes/.cache/codex-worktrees/scena-full-review-v18-checklist`, branch
  `main`, base HEAD `7b4fc9ca77e12fd12a69fab92650e1e46ee10354`; AGENTS hash
  `d0ed47595f6b6a6ec733651dfde650b7893aa2939d6137dc2f7eeade7528ac7a`
  and skills hash
  `61f0ec6d13aecddf11f6c556cdd2f5d62e11164a56589ae9d6c516f7430b953e`
  matched remotely. A02-specific elapsed time was not separately captured from
  the continuous goal clock (process-recording gap); maximum remediation count
  was 1 per distinct product/configuration signature, with zero release-
  candidate pushes, zero full-matrix runs, and zero user-required actions.
- `full`: intentionally deferred to Checkpoint D. The policy/API/CLI changes
  have focused command-matrix, both feature configurations, source mutation,
  help, and doctor coverage; the complete batch suite remains final-only.

### A03 — Add `scena capabilities [--json]` with live provenance

- [x] Add static/no-device and live-adapter modes with distinct status.
- [x] Probe adapter/device limits, formats, sample counts, features, backend,
  adapter identity, and actual readback/presentation constraints where safe.
- [x] Never hardcode `Rgba8UnormSrgb` when the selected target differs.
- [x] Emit `scena.capability_report.v1` with probe source/time/backend and
  structured reasons for unavailable probes.
- [x] Make `cli_version` report every compiled feature relevant to command
  availability.
- [x] Add schema fixtures, catalog entries if new, CLI help, examples, and
  doctor pins.
- [x] Acceptance: agents can plan before rendering without receiving static
  fiction about unusual hardware.

Validation ledger — A03 (2026-07-20):

- `focused red`: the A03 CLI/schema tests were written before production code.
  The first remote compile failed at `CapabilityReportV1.probe` because the
  additive provenance contract did not exist; this stopped before the unknown
  `capabilities` command could run, but precisely pinned the missing typed v1
  surface rather than accepting an untyped CLI-only field.
- `classification`: product/contract defect. Factory capability tables and
  renderer-time adapter reports existed, but there was no pre-render command,
  no static-versus-measured status, no requested-device report, no format/
  sample provenance, no structured unavailable result, and CLI version output
  exposed only two of fifteen availability-affecting Cargo features.
- `implementation`: `scena capabilities [--live] [--json]` now emits the
  existing `scena.capability_report.v1`. The deterministic default reports
  `static_no_device` and labels target facts unmeasured. Strict `--live`
  creates a headless GPU renderer and records timestamp, requested/selected
  backend, adapter identity/features/limits, requested-device features/limits,
  actual selected color format, Depth32Float format features, usable sample
  counts, and readback/presentation constraints. Adapter/device failure stays
  on stdout as the same schema with exit 1 and a structured `unavailable`
  reason; it never substitutes the static row as measured. The reusable live
  probe is renderer-owned, while CLI argument and failure policy remain in the
  binary adapter.
- `focused green`: default-feature and all-feature executions of
  `tests/a03_capabilities_cli.rs` pass 3/3. They prove byte-identical static and
  explicit-`--json` output, live measured-or-structured-unavailable behavior,
  target-format agreement with the constructed renderer, nonempty measured
  color/depth sample counts, device metadata, and the exact fifteen-feature
  version key set. The builder exercised the measured branch; this is adapter
  discovery evidence, not physical-GPU rendering or presentation proof.
- `schema compatibility`: the stable fixture now demonstrates the additive
  static `probe` shape. All three capability contract/round-trip/golden tests
  pass, including explicit removal of both additive `post_processing` and
  `probe` fields to prove old v1 reports still deserialize.
- `doctor red/green`: the A03 mutation first failed to compile because the new
  checker did not exist. The completed `A03-LIVE-CAPABILITY-DISCOVERY` rule
  rejects replacing `live_wgpu_adapter` provenance with the compiled table and
  pins CLI dispatch/help, strict unavailable output, adapter/device/format
  measurement, stable fixture, compatibility tests, docs, and changelog. Its
  mutation passes 1/1. Full doctor initially found the new schema definitions
  had pushed `diagnostics/capabilities.rs` above the 500-line KISS limit; the
  first remedy moved them into the owned `capability_probe` child module, after
  which `doctor --full` reports `mode=Full status=pass`.
- `scoped`: remote `cargo fmt --all -- --check`, strict default and all-feature
  Clippy, the 6/6 FR04 real-CLI schema matrix, focused stable contracts, and
  full doctor pass. FR04 first exposed an older A01 help/evidence identifier
  mismatch for the added validation modes; aligning the test evidence key with
  the already-public help contract fixed that distinct harness defect once.
  README, API/capability/schema/troubleshooting docs, LLM app-builder guide and
  skill, CLI help/examples, `[Unreleased]` changelog, and the dated v1.8.0
  erratum now distinguish static planning, measured headless probing,
  unavailable hardware, and unprobed presentation. No new schema ID/catalog
  row was needed because this is an additive field on the existing v1 schema.
- `provenance/process`: canonical checkout `/home/johannes/projects/scena`,
  isolated validation checkout
  `/home/johannes/.cache/codex-worktrees/scena-full-review-v18-checklist`, branch
  `main`, base HEAD `7b4fc9ca77e12fd12a69fab92650e1e46ee10354`; shared checkout
  absent. `AGENTS.md` hash
  `d0ed47595f6b6a6ec733651dfde650b7893aa2939d6137dc2f7eeade7528ac7a`
  and skills aggregate
  `4b0574c6e9964a288f91b76d8c54ab09cbe95864d55817d20968517ee6e3caa9`
  matched after explicit bootstrap. Elapsed A03 work was about 25 minutes;
  maximum remediation count was one for each distinct signature; zero release-
  candidate pushes, zero full-matrix runs, and zero user-required actions.
- `full`: intentionally deferred to Checkpoint D. No workspace-wide tests,
  browser/native presentation lanes, package/publish proof, or release matrix
  was run for this focused discovery contract.

### A04 — Normalize help, listing, names, and exit semantics

- [x] Make global and per-command `--help` exit 0 and write help to stdout.
- [x] Add `scena examples agent list` with stable machine-readable output.
- [x] Choose one canonical spelling for every template; support old hyphen/
  underscore names as aliases with migration diagnostics.
- [x] Define `diff` exit behavior: command success versus inequality, with an
  explicit `--exit-code`/CI mode if needed.
- [x] Test stdout/stderr separation, JSON mode, EPIPE, and unknown commands.
- [x] Update CLI reference, examples, README, and agent docs.
- [x] Acceptance: callers never need to scrape error messages to discover
  templates or interpret a normal help request as failure.

A04 validation ledger (2026-07-20):

- `focused`: the new `tests/a04_cli_ergonomics.rs` first ran 5 tests with 2
  passing and 3 expected failures: per-command help exited as an invalid
  command, template discovery returned an unknown-template error, and diff had
  no declared exit policy. After the implementation, the scene-host form is
  5/5 green and the default-feature form is 3/3 green. The proof covers global
  and every declared per-command help path, the stable template catalog and
  deprecated aliases, report-only versus `--exit-code` diff behavior, clean
  stdout/stderr separation, structured unknown-command errors, and EPIPE.
- `scoped`: `fr04_cli_schema_matrix` is 6/6 green; the affected agent-template
  suite is 7/7 green; the recipe-diff suite is 5/5 green; the catalog stable-
  contract golden, schema-catalog golden, and schema CLI golden are each green.
  `cargo fmt --all -- --check`, default strict clippy, and all-feature strict
  clippy pass on the isolated builder. These previously green contract tests
  were not rerun after a final unused-import-only edit because their behavior
  surface was unchanged; strict clippy and doctor were rerun instead.
- `doctor`: the A04 mutation test first failed to compile because the doctor
  rule did not exist, then passed after `A04-CLI-ERGONOMICS` was added.
  `doctor --full` initially rejected the unpinned catalog fixture and two files
  over the 500-significant-line limit. The fixture is now pinned, template
  catalog ownership moved to `examples_agent/catalog.rs`, schema report
  construction moved to `schema_catalog/reports.rs`, the mutation is 1/1
  green, and `doctor --full` passes.
- `docs/contracts`: help now has a stable command-scoped JSON result; template
  names are canonical kebab-case with explicit legacy aliases; the mature
  imported `product-configurator` and authored
  `product-configurator-starter` are unambiguous; diff reports inequality as
  data unless CI mode is requested. README, examples, schema contracts,
  troubleshooting, API/getting-started/LLM agent guidance, app-builder skill,
  changelog, and v1.8.0 errata were updated. The new
  `scena.agent_template_catalog.v1` schema, fixture, catalog row, and doctor pin
  are stable-contract evidence.
- `provenance/process`: canonical checkout `/home/johannes/projects/scena`,
  isolated validation checkout
  `/home/johannes/.cache/codex-worktrees/scena-full-review-v18-checklist`, branch
  `main`, base HEAD `7b4fc9ca77e12fd12a69fab92650e1e46ee10354`; shared checkout
  absent. `AGENTS.md` hash
  `d0ed47595f6b6a6ec733651dfde650b7893aa2939d6137dc2f7eeade7528ac7a`
  and skills aggregate
  `e044c75b13fba491f50bbb5aa28d68b2aad1ce100da059636a997907db81de23`
  matched after explicit bootstrap. Elapsed A04 work was about 32 minutes;
  one product-contract remediation, one policy/architecture remediation, zero
  repeated same-signature attempts, zero release-candidate pushes, zero full-
  matrix runs, and zero user-required actions.
- `full`: intentionally deferred to Checkpoint D. No workspace-wide test,
  browser/native presentation lane, package/publish proof, or release matrix
  was run for this focused CLI-contract slice.

### A05 — Put `scena-convert` under the stable JSON-envelope contract

- [x] Inventory every success, warning, progress, and error output.
- [x] Use the shared serde/envelope writer and stable schema IDs for machine
  mode.
- [x] Preserve a human mode explicitly; never mix progress text into JSON.
- [x] Align exit codes and BrokenPipe behavior with `scena`.
- [x] Add schema catalog/fixtures/doctor pins and conversion CLI docs.
- [x] Acceptance: every machine-mode outcome is one valid documented envelope.

A05 validation ledger (2026-07-20):

- `focused`: `tests/a05_scena_convert_contracts.rs` first ran 0/3 because
  `--json` and `--human` were unknown and all failures were prose on stderr.
  It now runs 4/4 and covers JSON help, planned conversion, invalid request,
  captured tool stdout progress and stderr warning, successful conversion,
  nonzero tool failure with exit-code provenance, unavailable-tool remedy, and
  explicit non-JSON human output. Machine outcomes are exactly one stdout JSON
  line with empty stderr.
- `implementation`: `scena.asset_conversion.v1` owns typed statuses
  (`planned`, `converted`, `invalid_request`, `tool_unavailable`, and
  `conversion_failed`), the exact command, optional tool exit code, and typed
  per-line diagnostics. Machine mode uses captured child output; human mode
  alone inherits live child output. Invalid requests exit 2, tool startup or
  conversion failures exit 1, success exits 0, BrokenPipe remains quiet, and
  non-Broken stdout I/O remains `scena.cli_io_error.v1`/74.
- `scoped`: the A05 suite is 4/4; the typed conversion fixture golden, schema-
  catalog golden, schema CLI golden, and legacy M5 converter plan test are each
  green. The shared CLI output contract is 3/3, including both binaries'
  BrokenPipe and structured non-Broken I/O behavior. `cargo fmt --all --
  --check`, default strict clippy, and all-feature strict clippy pass. One
  initial strict-lint finding (`collapsible_if`) was repaired mechanically.
- `doctor`: the A05 mutation test first failed to compile because the source-
  derived rule did not exist. `A05-SCENA-CONVERT-CONTRACT` now rejects replacing
  captured child output with inherited/status-only execution; its mutation is
  1/1 green. The stable fixture is pinned by doctor and `doctor --full` passes.
- `docs/contracts`: README, getting started, assets, schema contracts,
  troubleshooting, `[Unreleased]` changelog, and the v1.8.0 erratum document
  the two output modes, single-document guarantee, statuses, diagnostic
  capture, and exit semantics. The public schema catalog, fixture map, stable
  fixture, schema-list golden, and public Rust exports all include the new v1
  contract.
- `provenance/process`: canonical checkout `/home/johannes/projects/scena`,
  isolated validation checkout
  `/home/johannes/.cache/codex-worktrees/scena-full-review-v18-checklist`, branch
  `main`, base HEAD `7b4fc9ca77e12fd12a69fab92650e1e46ee10354`; shared checkout
  absent. `AGENTS.md` hash
  `d0ed47595f6b6a6ec733651dfde650b7893aa2939d6137dc2f7eeade7528ac7a`
  and skills aggregate
  `e044c75b13fba491f50bbb5aa28d68b2aad1ce100da059636a997907db81de23`
  matched after explicit bootstrap. Elapsed A05 work was about 16 minutes;
  one product-contract remediation, one lint remediation, zero repeated same-
  signature attempts, zero release-candidate pushes, zero full-matrix runs,
  and zero user-required actions. One combined-gate command was stopped before
  evidence acceptance when its second command lacked the scoped target-dir
  environment; the builder preflight remained healthy and both gates were
  rerun independently.
- `full`: intentionally deferred to Checkpoint D. No workspace-wide tests,
  browser/native lanes, package/publish proof, or release matrix were run for
  this focused conversion-transport slice.

### A06 — Fix ignored and misrouted positional inputs

- [x] Prove `repair <asset-or-recipe>` accepts but ignores its positional.
- [x] Either use the positional as the repair target or remove it through a
  compatible deprecation; reject conflicting `--from`/target combinations.
- [x] Dispatch `doctor <authored-recipe>` through the recipe path, not glTF.
- [x] Add asset, recipe, missing, malformed, and policy-rejected tests.
- [x] Update command help and troubleshooting.
- [x] Acceptance: every accepted positional changes or constrains the operation
  exactly as help states.

A06 validation ledger (2026-07-20):

- `claim audit`: the original claim was still true for raw assets but stale for
  recipes. C03 had already routed authored recipes through
  `scene_host_manifest_from_resolved_recipe`, so recipe doctor and recipe repair
  targets used every import and effective policy. A raw repair positional was
  still reduced to a string and never loaded before planning from `--from`.
- `focused`: `tests/a06_repair_doctor_inputs.rs` first ran 2/3. Duplicate
  positional rejection and the valid/missing/malformed/policy-rejected recipe
  doctor matrix were already green; a nonexistent raw target incorrectly
  returned a successful visual repair plan. The final suite is 4/4: valid raw
  asset and recipe targets reach planning, missing/malformed raw assets return
  `scena.asset_doctor.v1`, valid/missing/malformed/policy-rejected recipe doctor
  inputs retain their correct structured owner reports, a second target is an
  argument error, and command help describes target validation.
- `implementation`: repair now calls `Assets::doctor_asset_path` for raw
  targets and stops on its non-OK report. Recipe targets retain the complete
  policy-aware manifest build. The positional therefore constrains execution
  without claiming that an older diagnosis is cryptographically source-bound.
  The existing raw repair-plan integration test remains green.
- `scoped`: A06 is 4/4 and the established raw visual-repair integration test
  is 1/1. `cargo fmt --all -- --check`, default strict clippy, and all-feature
  strict clippy pass. The C03 two-import recipe proof was not rerun because the
  A06 matrix directly covers the same doctor dispatch and no C03 owner changed.
- `doctor`: the A06 mutation test first failed to compile because its rule did
  not exist. `A06-REPAIR-DOCTOR-INPUTS` now rejects bypassing raw target
  validation and pins recipe doctor routing, help, tests, docs, changelog, and
  agent skill guidance. One Markdown line-wrap mismatch in the new checker was
  corrected without production changes. `doctor --full` then caught the
  central doctor runner at 601 significant lines; A03-A06 dispatch now has a
  focused `agent_contracts` owner and the runner is below its 600-line limit.
  The mutation is 1/1 green and `doctor --full` passes.
- `docs/contracts`: command-scoped help has machine-readable repair notes.
  README, schema contracts, troubleshooting, LLM app-builder guide and skill,
  changelog, and v1.8.0 errata distinguish target validation from report
  planning and identify each structured pre-plan failure family.
- `provenance/process`: canonical checkout `/home/johannes/projects/scena`,
  isolated validation checkout
  `/home/johannes/.cache/codex-worktrees/scena-full-review-v18-checklist`, branch
  `main`, base HEAD `7b4fc9ca77e12fd12a69fab92650e1e46ee10354`; shared checkout
  absent. `AGENTS.md` hash
  `d0ed47595f6b6a6ec733651dfde650b7893aa2939d6137dc2f7eeade7528ac7a`
  and skills aggregate
  `847d5cde03dfe7aea2acbfb71a5e82cbca42f4c6b2b28ac8830ff60fb21d843b`
  matched after explicit bootstrap. Elapsed A06 work was about 16 minutes;
  one product remediation, one checker-text remediation, one architecture-
  limit remediation, zero repeated same-signature attempts, zero release-
  candidate pushes, zero full-matrix runs, and zero user-required actions.
- `full`: intentionally deferred to Checkpoint D. No workspace-wide tests,
  browser/native lanes, package/publish proof, or release matrix were run for
  this focused input-routing slice.

### A07 — Provide nearest candidates and preserve remedies

- [x] Add shared candidate suggestions for node, mesh, material, animation,
  variant, anchor, connector, template, preset, and schema lookups.
- [x] Use deterministic normalized distance and cap result count.
- [x] Include candidates as structured data, not prose only.
- [x] Preserve `NoActiveCamera` remedy text through `Display`, JSON, CLI, and
  render outcome conversions.
- [x] Acceptance: an agent can correct a misspelled name or missing camera from
  the first error response.

A07 validation ledger (2026-07-20):

- `focused`: `tests/a07_name_candidates.rs` first failed to compile because the
  shared ranker, candidate-bearing lookup variants, recipe diagnostic field,
  and remedy-preserving conversions did not exist. The final all-feature suite
  is 5/5. It proves case/separator normalization, deterministic distance and
  tie ordering, duplicate removal, a three-result cap, node/clip/animation/
  variant/anchor/connector candidates, recipe node/geometry/material/preset
  candidates, schema/template CLI candidates, and the serialized
  `NoActiveCamera` SceneHost code and remedy.
- `implementation`: `nearest_name_candidates` is the single ranking owner and
  schema discovery no longer keeps a second edit-distance implementation.
  Candidate vectors survive `LookupError`, `AnimationError`, recipe validation,
  `SceneHostError`, host recipe diagnostics, and `scena.cli_error.v1`. Missing-
  camera Display names both `Scene::add_default_camera` and
  `Scene::set_active_camera`; CLI render errors continue to format the same
  typed error without replacing its remedy. Empty additive candidate fields
  remain serde-defaulted and omitted, preserving existing v1 golden shapes.
- `scoped`: the affected M5 public-error display, M7 recovery snapshot, M8
  material-variant lookup, schema CLI, and two recipe-validation serde tests
  pass. The M7 snapshot initially exposed its stale pre-remedy expectation and
  was updated. An initial schema-test filter executed zero tests and was
  rejected as evidence; the actual schema test was rerun 1/1. `cargo fmt --all
  -- --check`, default strict clippy, and all-feature strict clippy pass. The
  default clippy pass first exposed that the new integration test lacked its
  required `scene-host` cfg and passed after that harness-only correction.
- `doctor`: the checker mutation test first failed to compile because its rule
  did not exist. `A07-NAME-CANDIDATES-REMEDIES` now pins the shared algorithm,
  every producer/conversion, structured CLI and recipe fields, camera remedy,
  focused tests, public docs, changelog, release erratum, and app-builder skill.
  Its first green attempt required replacing two rustfmt-sensitive full-line
  needles with semantic fragments. `doctor --full` then reported
  `src/diagnostics.rs` at 502 significant lines; `AnimationError` now has a
  focused diagnostics owner module, restoring the 500-line policy. The
  mutation is 1/1 green and final `doctor --full` passes.
- `docs/contracts`: README, API/errors, schema contracts, troubleshooting, LLM
  app-builder guide and debugging skill, changelog, and v1.8.0 errata document
  the typed candidate field, supported lookup families, ambiguity rule, cap,
  normalization, and both camera remedies.
- `provenance/process`: canonical checkout `/home/johannes/projects/scena`,
  isolated validation checkout
  `/home/johannes/.cache/codex-worktrees/scena-full-review-v18-checklist`, branch
  `main`, base HEAD `7b4fc9ca77e12fd12a69fab92650e1e46ee10354`; shared checkout
  absent. `AGENTS.md` hash
  `d0ed47595f6b6a6ec733651dfde650b7893aa2939d6137dc2f7eeade7528ac7a`
  and skills aggregate
  `013b0f02852521fec71d76cb7c413daf6c5407018e9e95dcb677450e8df6d1b2`
  matched after explicit bootstrap. Elapsed A07 work was about 34 minutes;
  two product-contract remediations, three harness/checker remediations, one
  architecture-limit remediation, no repeated same-signature attempt beyond
  one correction, zero release-candidate pushes, zero full-matrix runs, and
  zero user-required actions.
- `full`: intentionally deferred to Checkpoint D. No workspace-wide tests,
  browser/native presentation lanes, package/publish proof, or release matrix
  were run for this focused error-contract slice.

### A08 — Unify transform grammar

- [x] Inventory node/import/placement transform shapes and precedence.
- [x] Select one canonical transform schema with finite-number and composition
  rules.
- [x] Add migration aliases/versioning rather than silently reinterpreting old
  recipes.
- [x] Update field-model/schema examples, recipes, docs, and golden fixtures.
- [x] Acceptance: equivalent transforms use the same grammar everywhere.

A08 validation ledger (2026-07-20):

- `focused`: the initial remote
  `cargo test --all-features --test a08_transform_grammar` compile failed on
  the reviewed split grammar (`imports[].transform` was `Transform` while
  authored nodes used `SceneRecipeTransformV1`), which pinned the expected
  test-first defect. The final five-test target passed and proves shared tagged
  `raw`/`trs` input, finite conversion, exact X-then-Y-then-Z degree
  composition, explicit-tag precedence, zero-quaternion rejection, the narrow
  published-v1.8.0 legacy import reader plus migration warning, canonical
  serialization, and legacy placement-result migration.
- `focused`: the A08 doctor mutation first failed to compile because
  `check_a08_transform_grammar` did not exist. After implementation,
  `cargo test -p xtask
  a08_doctor_rejects_restoring_the_untagged_import_transform_type` passed and
  executed a known-bad mutation that restores the old import field type.
- `scoped`: exact remote tests passed for
  `placement_and_recipe_patch_goldens_match_live_schema_serialization`,
  `scena_place_cli_stdout_matches_golden_fixture`,
  `fr03_place_apply_emits_persistent_recipe_and_rejects_stale_source`, and
  `fr01_schema_get_emits_authoritative_recipe_field_model`. Placement and
  recipe-patch writers now serialize through `SceneRecipeTransformV1`; their
  exact legacy readers remain source-compatible, quaternion normalization and
  three-decimal stability are pinned, and recipe apply output exactly matches
  the preview grammar including canonical default omission.
- `scoped`: `cargo fmt --check`, default-feature
  `cargo clippy --all-targets -- -D warnings`, all-feature
  `cargo clippy --all-features --all-targets -- -D warnings`, and
  `cargo run -p xtask -- doctor --full` passed on the isolated remote builder.
  The doctor architecture limit initially rejected the enlarged
  `src/scene_host/recipe.rs`; import-transform application was moved into the
  existing recipe transform owner and the final full doctor run passed.
- `docs`: updated README, API inventory, schema/placement contracts, the LLM
  app-builder guide and skill, changelog, v1.8.0 errata, authoritative field
  model, stable placement/patch fixtures, and CLI golden output. New output is
  always tagged; old untagged recipe imports produce the structured
  `legacy_transform_shape` warning and auto-fix.
- `bootstrap`: canonical source `/home/johannes/projects/scena`, isolated
  destination
  `/home/johannes/.cache/codex-worktrees/scena-full-review-v18-checklist`,
  branch `main`, HEAD `7b4fc9ca77e12fd12a69fab92650e1e46ee10354`.
  Root `AGENTS.md` hash
  `d0ed47595f6b6a6ec733651dfde650b7893aa2939d6137dc2f7eeade7528ac7a`
  and skills aggregate
  `ec27641ab91bb80ed48c1aaabffde660f73c44712ff1e53620fbff979985b20a`
  matched after explicit bootstrap. Shared checkout was missing and validation
  stayed isolated.
- `process`: elapsed A08 work was about 55 minutes. One test-first product
  compile failure and one test-first checker compile failure were expected.
  Subsequent investigation found six distinct one-attempt signatures: one
  escaped checker needle, one stale CLI golden, one duplicate serializer
  default mismatch, one normalized/rounded stable fixture mismatch, one
  architecture-size finding, and one helper visibility/root-handle compile
  error. No failure signature received two failed remediation attempts. There
  were zero release-candidate pushes, zero full-matrix runs, and zero
  user-required actions.
- `full`: intentionally deferred to Checkpoint D. No workspace-wide tests,
  browser/native presentation lanes, package/publish proof, or release matrix
  were run for this focused schema/serialization slice.

### A09 — Resolve feature discoverability without bloating default builds

- [x] Preserve the corrected fact that `scene-host` already implies
  `inspection`; do not add a false two-feature error.
- [x] Decide whether an opt-in `agent` umbrella feature should enable the full
  self-verification surface.
- [x] Keep default-feature policy explicit and measure compile/package impact
  before enabling anything by default.
- [x] Make unavailable commands/APIs report the exact one-step feature remedy.
- [x] Update README install snippets, getting started, feature-flags docs, and
  cargo examples.
- [x] Acceptance: a fresh user can intentionally select the agent surface from
  public documentation without trial-and-error cfg failures.

A09 validation ledger (2026-07-20):

- `decision`: added opt-in `agent = ["scene-host"]`; `scene-host =
  ["inspection"]` remains the single lower-level dependency edge and
  `default = []` remains exact. Nothing was enabled by default, and no false
  `scene-host,inspection` composition was introduced.
- `focused`: the initial default-feature
  `cargo test --test a09_feature_discoverability` failed both expected
  test-first assertions because the alias and executable CLI remedy did not
  exist. The final default run passed 2/2, proving empty defaults, the exact
  feature graph, and a structured default-build command error containing
  `cargo install scena --features agent`. The final
  `cargo test --features agent --test a09_feature_discoverability` passed 2/2,
  compiling both `SceneHostCore` and inspection types and proving version JSON
  reports `agent`, `scene_host`, and `inspection` as active.
- `focused`: the A09 doctor mutation initially failed to compile because
  `check_a09_feature_discoverability` did not exist. The final
  `cargo test -p xtask
  a09_doctor_rejects_a_redundant_or_default_agent_feature` passed and executed
  a mutation that redundantly changed the alias to
  `agent = ["scene-host", "inspection"]`.
- `measurement`: isolated remote `cargo tree --no-default-features --features
  agent --edges features --prefix none` and the equivalent `--features
  scene-host` output were byte-identical: 1,214 lines, SHA-256
  `a8627e04f06433862edf212daf14bb0ecab2704fb67e086ecc7375b33aafb6fe`.
  Thus the alias adds no dependency or compile surface beyond the already
  documented SceneHost selection. `cargo package --allow-dirty --no-verify`
  succeeded with 1,058 files and a 6,185,710-byte compressed crate
  (`0a09a40ba61e1fe27ad88cea4327ba096804eec0dae6bf7687c12ae2fb480e31`);
  feature selection adds no conditional package files.
- `scoped`: `cargo fmt --check`, default-feature
  `cargo clippy --all-targets -- -D warnings`, agent-feature
  `cargo clippy --features agent --all-targets -- -D warnings`, and
  `cargo run -p xtask -- doctor --full` passed on the isolated builder. The
  only remediation after functional green was one test-harness clippy finding
  for constant cfg assertions; compile-time const assertions replaced them and
  the same lane passed.
- `docs`: README, getting started, feature flags, API, examples, the public LLM
  guide, app-builder skill and recipe-loop reference, feature ownership JSON,
  changelog, and v1.8.0 errata now use `--features agent` for the complete
  workflow. Lower-level `inspection` and `scene-host` choices remain documented
  for deliberate narrow builds, and runtime feature errors provide exact
  install and source-run commands.
- `bootstrap`: canonical source `/home/johannes/projects/scena`, isolated
  destination
  `/home/johannes/.cache/codex-worktrees/scena-full-review-v18-checklist`,
  branch `main`, HEAD `7b4fc9ca77e12fd12a69fab92650e1e46ee10354`.
  Root `AGENTS.md` hash
  `d0ed47595f6b6a6ec733651dfde650b7893aa2939d6137dc2f7eeade7528ac7a`
  and skills aggregate
  `a333a1ac0f97feaa5abf4512d2eac8b2ec77b0f4b3b59f24a608331c48216fa3`
  matched after explicit bootstrap; the shared checkout remained missing and
  validation remained isolated.
- `process`: elapsed A09 work was about 28 minutes. Two expected test-first
  red contracts and one expected checker compile failure preceded the patch;
  one distinct test-lint remediation was required. No signature received two
  failed remediation attempts. There were zero release-candidate pushes, zero
  full-matrix runs, and zero user-required actions.
- `full`: intentionally deferred to Checkpoint D. No workspace-wide test,
  browser/native presentation matrix, docs build, or release matrix was run
  for this focused feature-discovery slice.

## 6. Proof and release-quality remediation

### Q01 — Replace the required WebGPU smoke test with real pixel parity
(review B13)

- [x] Keep adapter identity, submissions, draw calls, and nonblack counts as
  diagnostics, not parity criteria.
- [x] Compare renderer output against a source-bound reference or trusted
  cross-backend oracle with declared color space, tolerance, crop/mask, and
  resolution.
- [x] Reuse the q01-style metrics where appropriate: per-channel error,
  RMSE/SSIM or equivalent, high-percentile threshold, and rejected known-bad
  mutations.
- [x] Include wrong colors, geometry shift, missing object, vertical flip,
  linear-as-sRGB, and stale-reference mutations.
- [x] Require physical hardware evidence for the lane named hardware parity;
  software adapters remain conformance only.
- [x] Emit diff heatmap, worst-region bounding box, metric summary, adapter,
  commit, command, and artifact hashes.
- [x] Update browser/release-gate docs and doctor pins.
- [ ] Acceptance: a hardware GPU rendering a materially wrong image fails.

Implementation status (2026-07-20): complete. Physical-hardware execution of
the acceptance lane remains deferred to the final integration checkpoint; the
release consumer and evaluator reject synthetic hardware artifacts with wrong
pixels, but the CPU-only builder cannot supply physical-GPU provenance.

Validation ledger (2026-07-20):

- `focused red`: the original `required_gpu_parity_test.js` contract accepted
  draw/submission/nonblack hardware smoke with no pixel evaluation. A new
  missing-parity fixture failed first because the old evaluator did not emit
  `PIXEL_PARITY_MISSING`; the release-consumer regression independently showed
  that a smoke-only hardware artifact passed the old readiness predicate.
- `classification`: product/policy defect in the parity contract and release
  consumer. Two later browser failures were test-harness defects with distinct
  signatures: full-frame IoU counted expected raster-boundary differences, and
  an initially weak stale-reference translation remained inside that boundary
  tolerance. A minimized synthetic probe distinguished each before the harness
  was changed.
- `implementation`: `tests/browser/required_gpu_parity.js` now evaluates the
  exact renderer readback against the CPU oracle at RGBA8/sRGB, with a
  CPU-reference-derived two-pixel gradient mask, RGB tolerance 4, at least
  99.5% in tolerance, RMSE at most 2, p99.5 delta at most 4, and foreground IoU
  at least 0.995. The same evaluator rejects all six named mutations. The Rust
  browser probe emits source-bound CPU/WebGPU frames; the JS runner writes PNG
  reference/candidate/heatmap artifacts, raw worst-region diagnostics,
  metrics, adapter/command/commit provenance, and source/artifact hashes.
  `hardware-gpu.yml` runs the dedicated parity command, while the xtask release
  consumer independently validates metrics, mask policy, source binding,
  mutations, and physical-adapter provenance. A stable
  `scena.q01.required_webgpu_pixel_parity.v1` contract, docs, README,
  changelog, release errata, and `Q01-REQUIRED-WEBGPU-PIXEL-PARITY` doctor rule
  pin the behavior.
- `focused green`: `npm run test:required-gpu-parity` passed the exact success,
  missing evidence, wrong color, detached readback, software conformance,
  boundary-noise, and six-mutation cases. The two focused xtask release tests
  passed, proving that smoke-only/wrong-pixel hardware evidence is rejected and
  correct software output remains diagnostic only. The Q01 doctor mutation
  tests passed 2/2.
- `rendered output`: on `scena-builder`,
  `SCENA_BROWSER_BACKENDS=webgpu SCENA_GPU_EVIDENCE_CLASS=software-conformance
  npm run browser:q01-parity` rendered the actual Rust/WASM triangle and passed
  conformance: renderer hash `963cc7b7752a6b05`, 3,230 compared pixels, 866
  reference-edge pixels excluded, tolerance fraction 1.0, RGB RMSE 0, p99.5
  delta 0, foreground IoU 1.0, and all six mutations rejected. This is scoped
  dirty-tree evidence bound to baseline HEAD plus source checksums, not release
  or physical-hardware evidence. An earlier full M6 probe passed Q01 and then
  stopped at the separate Q02 anisotropy proof, motivating the dedicated
  focused command rather than weakening either lane.
- `scoped`: remote `cargo test --test stable_contracts` passed 27/27; the
  WebAssembly browser-probe check had already passed for the unchanged Rust
  producer; `cargo fmt --all --check`,
  `cargo clippy -p xtask --all-targets -- -D warnings`, and
  `cargo run -p xtask -- doctor --full` passed. Existing CLI schema and FR04
  contract suites also passed after repairing a discovered stale command-help
  join for the already-shipped `--allow-root` option.
- `skipped`: no physical GPU was available on the CPU builder, so the final
  acceptance checkbox remains open. Workspace-wide tests, native GPU lanes,
  docs/package/publish gates, and the full release matrix remain deferred to
  Checkpoint D.
- `bootstrap`: canonical source `/home/johannes/projects/scena`, isolated
  destination
  `/home/johannes/.cache/codex-worktrees/scena-full-review-v18-checklist`,
  branch `main`, HEAD `7b4fc9ca77e12fd12a69fab92650e1e46ee10354`.
  Root `AGENTS.md` hash
  `d0ed47595f6b6a6ec733651dfde650b7893aa2939d6137dc2f7eeade7528ac7a`
  and skills aggregate
  `a333a1ac0f97feaa5abf4512d2eac8b2ec77b0f4b3b59f24a608331c48216fa3`
  matched after explicit bootstrap; the shared checkout remained missing and
  validation remained isolated.
- `counts`: about 150 minutes investigation; one production-contract
  implementation and three distinct harness refinements, with no repeated
  signature patched twice without a smaller discriminator; zero
  release-candidate pushes, zero full-matrix runs, and zero user-required
  actions.
- `full`: deferred to Checkpoint D. Existing green scoped proof is reusable
  unless the browser parity producer, evaluator, schema, consumer, or workflow
  changes again.

### Q02 — Strengthen KHR material visual proofs

- [x] Replace any-nonzero/1-LSB directional checks with feature-specific regions
  and minimum meaningful effect sizes.
- [x] Add positive, disabled-control, and known-wrong-direction mutations for
  anisotropy and every covered KHR material feature.
- [x] Separate numerical implementation tolerance from visible acceptance.
- [x] Test more than one view/light direction where the material is directional.
- [x] Acceptance: disabling or misdirecting the feature fails reliably while
  harmless noise does not create a pass.

Validation ledger (2026-07-20):

- `focused red`: the new exact M8 contract initially failed to compile because
  the old suite had no `evaluate_khr_material_feature_mutations` evaluator. The
  prior implementation used whole-half max/average checks, including
  anisotropy `on > off`, clearcoat `+1`, and sheen `+2`; it had no disabled,
  wrong-direction, subvisible, or harmless-noise mutation contract.
- `classification`: test-policy defect. Rendering code already produced
  material responses, but the proof could certify imperceptible changes. Two
  doctor-only harness corrections followed with distinct evidence: a prose
  guard joined words across an intervening phrase, and a mutation removed only
  one of three matching source tokens. The latter was reduced to an exact
  three-occurrence count before changing the mutation to remove all copies.
- `implementation`: `tests/m8_visual_proof.rs` now evaluates fixed
  feature-specific foreground regions for clearcoat, sheen, anisotropy,
  iridescence, dispersion, and transmission/volume. Visible acceptance uses a
  minimum four-code-value channel delta plus per-feature effect RMSE,
  changed-pixel fraction, and semantic direction floors. Reference RMSE <= 1.1
  and effect alignment >= 0.9 form a separate numerical-tolerance layer.
  Anisotropy renders at -35 and +35 degree light yaws. The same evaluator must
  reject the disabled frame, a two-LSB fake effect, and an inverted-effect
  mutation, while accepting deterministic one-LSB noise around valid output.
  The old epsilon assertions were removed rather than left as competing proof.
- `rendered output`: the focused source-frame run passed all seven rows. Measured
  enabled-effect RMSE was 4.877 for clearcoat, 119.367 for sheen, 33.773 and
  33.616 for the two anisotropy directions, 15.596 for iridescence, 46.201 for
  dispersion, and 183.065 for transmission/volume. Every disabled,
  two-LSB-nudge, and inverted-effect row failed; every valid one-LSB-noise row
  passed. The evaluator writes
  `target/gate-artifacts/m8-visual/khr-material-feature-proof.json` with floors,
  metrics, directions, and mutation outcomes.
- `doctor red/green`: the Q02 mutation test first showed that removing the
  subvisible-mutation contract produced no finding. After extending
  `Q02-ROUND-E-MATERIALS`, it passed and now pins the regional evaluator,
  thresholds, two light directions, artifact, rendering docs, README,
  changelog, and v1.8.0 errata. The older `ASSETS-M8` rule was changed from
  pinning removed max helpers to pinning the mutation-tested oracle.
- `focused green`: remote exact
  `m8_khr_material_visual_oracle_rejects_disabled_and_wrong_direction_mutations`
  passed 1/1 in 38.05 seconds. The Q02 doctor mutation passed 1/1.
- `scoped`: the complete remote `m8_visual_proof` file passed 3/3 in 68.96
  seconds; `cargo fmt --all --check`,
  `cargo clippy --test m8_visual_proof -- -D warnings`,
  `cargo clippy -p xtask --all-targets -- -D warnings`, and
  `cargo run -p xtask -- doctor --full` passed. `docs/rendering.md`, README,
  CHANGELOG, and v1.8.0 release-note errata describe the stronger evidence and
  its CPU-versus-required-GPU boundary.
- `skipped`: no browser/GPU rerun was required because this slice changes the
  deterministic CPU/reference proof and source guards, not shader or browser
  production code. Workspace-wide tests, docs build, packaging/publish checks,
  and the full release matrix remain deferred to Checkpoint D.
- `bootstrap`: canonical source `/home/johannes/projects/scena`, isolated
  destination
  `/home/johannes/.cache/codex-worktrees/scena-full-review-v18-checklist`,
  branch `main`, HEAD `7b4fc9ca77e12fd12a69fab92650e1e46ee10354`.
  Root `AGENTS.md` hash
  `d0ed47595f6b6a6ec733651dfde650b7893aa2939d6137dc2f7eeade7528ac7a`
  and skills aggregate
  `a333a1ac0f97feaa5abf4512d2eac8b2ec77b0f4b3b59f24a608331c48216fa3`
  matched after each explicit bootstrap; the shared checkout remained missing
  and validation remained isolated.
- `counts`: about 82 minutes investigation; one proof implementation and two
  distinct doctor-harness refinements; zero release-candidate pushes, zero
  full-matrix runs, and zero user-required actions.
- `full`: deferred to Checkpoint D. Reuse this scoped evidence unless the M8
  evaluator, material render path, Q02 doctor rule, or pinned docs change.

### Q03 — Replace quadrant-mean-only structure checks

- [x] Add local structural metrics: windowed SSIM, edges/features, region masks,
  silhouette/coverage, or source-specific landmarks.
- [x] Keep broad means as debugging rows only.
- [x] Add mutations that preserve quadrant means while moving/removing
  structure.
- [x] Emit heatmaps and worst-region boxes.
- [x] Acceptance: structure-preserving mean tricks are rejected.

Validation ledger (2026-07-21):

- `focused red`: the new exact
  `q03_structure_oracle_rejects_quadrant_mean_preserving_mutations` test first
  failed to compile because the old M2 suite had no quadrant-preserving image
  mutations or structural evaluator. The old gate used four quadrant means and
  occupancy counts as acceptance, so it could not distinguish rearranged
  pixels with the same per-quadrant histogram.
- `classification`: proof-policy defect. The renderer output itself was not
  implicated. One subsequent harness compile failure was classified separately:
  this png crate version returns an optional decoded-buffer size; unwrapping the
  validated PNG size resolved it without changing metrics or thresholds.
- `implementation`: `tests/m2_visual_proof.rs` now compares every M2 fixture to
  a committed source frame using 8x8 blurred-luminance SSIM windows at stride
  four, a dilated Sobel-edge IoU floor of 0.85, and a dilated foreground IoU
  floor of 0.95. Mean window SSIM must be at least 0.97 and the worst window at
  least 0.70. Nine compact stripped PNG references are source-bound in
  `m2-headless-core-frames.toml`; fixture/reference metadata declares
  `local-structure-v2`. Broad quadrant values remain serialized under explicit
  `debug-only` labeling and are no longer acceptance inputs.
- `mutations`: rotating each quadrant 180 degrees and sorting each quadrant's
  exact pixel multiset into luminance stripes preserve all legacy quadrant
  means and nonblack counts. The moved frame failed with mean SSIM 0.9344,
  worst-window SSIM 0.00003, and edge/foreground IoU 0. The collapsed-stripe
  frame failed with mean SSIM 0.8953, worst-window SSIM -0.0430, edge IoU
  0.0276, and foreground IoU 0. Each mutation writes its own diff heatmap,
  metric JSON, and worst 8x8 region; each valid fixture writes the same artifact
  family.
- `doctor`: `Q03-M2-LOCAL-STRUCTURE` pins the evaluator, floors, source-frame
  catalog, both mean-preserving mutations, heatmap/worst-region outputs,
  debug-only quadrant role, README, changelog, M2 checklist, and v1.8.0 errata.
  Its mutation test passed. The existing effect-footprint and M2 fixture rules
  were migrated to `local-structure-v2` without weakening paired masks.
- `focused green`: the exact mean-preserving mutation test passed 1/1; the
  integrated source-reference test passed all nine fixtures with baseline
  SSIM/IoU rows at 1.0. The complete M2 visual file passed 5/5 in 6.65 seconds.
- `scoped`: the Q03 doctor mutation, legacy effect-footprint doctor regression,
  and M2 fixture-metadata regression each passed 1/1; remote
  `cargo fmt --all --check`, `cargo clippy --test m2_visual_proof -- -D
  warnings`, `cargo clippy -p xtask --all-targets -- -D warnings`, and
  `cargo run -p xtask -- doctor --full` passed.
- `skipped`: no browser/GPU rerun was warranted because the changed surface is
  the deterministic CPU reference oracle and its source metadata, not renderer
  or browser production code. Workspace-wide tests, docs build,
  package/publish checks, and the complete release matrix remain deferred to
  Checkpoint D.
- `bootstrap`: canonical source `/home/johannes/projects/scena`, isolated
  destination
  `/home/johannes/.cache/codex-worktrees/scena-full-review-v18-checklist`,
  branch `main`, HEAD `7b4fc9ca77e12fd12a69fab92650e1e46ee10354`.
  Root `AGENTS.md` hash
  `d0ed47595f6b6a6ec733651dfde650b7893aa2939d6137dc2f7eeade7528ac7a`
  and skills aggregate
  `a333a1ac0f97feaa5abf4512d2eac8b2ec77b0f4b3b59f24a608331c48216fa3`
  matched after each explicit bootstrap; the shared checkout remained missing
  and validation remained isolated.
- `counts`: about 74 minutes investigation; one proof implementation and one
  png-decoder API harness correction; zero release-candidate pushes, zero
  full-matrix runs, and zero user-required actions.
- `full`: deferred to Checkpoint D. Reuse this evidence unless the M2 render
  fixtures, source frames, evaluator thresholds, or doctor/docs contract change.

### Q04 — Split optional GPU smoke from required resource-lifecycle proof

- [x] Replace `let Ok(..) else { return }` in required leak tests with an
  explicit skip artifact or failure according to lane policy.
- [x] Keep adapter-optional developer smoke tests clearly named non-gating.
- [x] Add a strict hardware lane that records allocation/destruction counters,
  adapter provenance, and at least one complete lifecycle.
- [x] Add known leak and missing-adapter mutations.
- [ ] Acceptance: a green required lifecycle lane always executed assertions on
  an accepted adapter.

Validation ledger (2026-07-21):

- `focused red`: the exact
  `required_lifecycle_evaluator_rejects_known_leak_and_missing_adapter` test
  first failed to compile because no required lifecycle evidence type or
  evaluator existed. The independent xtask mutation then failed to compile
  because release readiness had no lifecycle-artifact consumer.
- `classification`: proof-policy defect. Five adapter-sensitive C09 tests used
  `let Ok(Renderer::headless_gpu(..)) else { return; }`, so a zero-assertion run
  was indistinguishable from success. The renderer resource lifecycle itself
  was not implicated. The first scoped doctor failure exposed missing schema,
  module-size, and renamed-test integrations; the second isolated the catalog
  module-size boundary. Both were contract-integration defects with distinct
  signatures, not repeated renderer remediation attempts.
- `implementation`: the five existing checks are explicitly suffixed
  `optional_gpu_smoke` and write typed `status:"skipped"`,
  `proof_class:"optional-developer-smoke"` artifacts when construction is
  unavailable. The separate
  `required_hardware_gpu_resource_lifecycle_executes_complete_cycle` test is
  enabled only by `SCENA_REQUIRE_GPU_RESOURCE_LIFECYCLE=1`; under that policy,
  missing/CPU/software adapters panic rather than skip. It records baseline,
  expanded prepared, released, and confirmed-poll counters plus complete
  adapter, command, commit, timestamp, and assertion provenance in
  `scena.q04.required_gpu_resource_lifecycle.v1`.
- `release/doctor`: the self-hosted Linux GPU workflow runs the exact strict
  test. The Linux native release lane and staged bundle require
  `c09-gpu-resource-lifecycle/required-result.json`; the independent consumer
  rechecks accepted adapter identity, at least ten assertions, resource growth,
  return to baseline shape, exact destroyed count, `Confirmed`, and zero
  pending work. `RENDER-C09` pins the producer/workflow/consumer/docs chain and
  rejects restoration of the silent `let Ok` form. The new stable schema and
  fixture are available through `scena schema`; catalog rows were split into
  their operational owner module to remain below the 500-line KISS limit.
- `mutations`: both the producer evaluator and independent release consumer
  accept a valid synthetic evidence row, then reject
  `poll_pending_after = 1` and `adapter = null`. The doctor mutation disables
  `SCENA_REQUIRE_GPU_RESOURCE_LIFECYCLE` in the required hardware workflow and
  is rejected.
- `focused green`: the producer evaluator passed 1/1, the independent release
  consumer passed 1/1, and the C09 doctor mutation passed 1/1. Normal CPU-builder
  execution passed the complete C09 file 8/8 and emitted an explicit
  `diagnostic-without-required-hardware-policy` skip for the strict test. The
  same exact strict test with its requirement flag deliberately failed on the
  builder's `device_type:"Cpu"`, proving unavailable hardware cannot be green.
- `scoped`: stable contracts passed 27/27 and the schema CLI/golden suite passed
  8/8. Remote `cargo fmt --all --check`, `cargo clippy --lib -- -D warnings`,
  `cargo clippy --test c09_gpu_resource_lifecycle -- -D warnings`,
  `cargo clippy -p xtask --all-targets -- -D warnings`, and
  `cargo run -p xtask -- doctor --full` passed.
- `skipped`: the Hetzner builder is CPU-only, so it cannot produce the required
  physical-hardware artifact and the acceptance checkbox remains open until
  the self-hosted GPU lane runs. Workspace-wide tests, docs build,
  package/publish checks, and the complete release matrix remain deferred to
  Checkpoint D.
- `bootstrap`: canonical source `/home/johannes/projects/scena`, isolated
  destination
  `/home/johannes/.cache/codex-worktrees/scena-full-review-v18-checklist`,
  branch `main`, HEAD `7b4fc9ca77e12fd12a69fab92650e1e46ee10354`.
  Root `AGENTS.md` hash
  `d0ed47595f6b6a6ec733651dfde650b7893aa2939d6137dc2f7eeade7528ac7a`
  and skills aggregate
  `a333a1ac0f97feaa5abf4512d2eac8b2ec77b0f4b3b59f24a608331c48216fa3`
  matched after every explicit bootstrap; the shared checkout remained missing
  and validation remained isolated.
- `counts`: about 58 minutes investigation; two proof implementations, two
  doctor contract-integration refinements, zero release-candidate pushes, zero
  full-matrix runs, and zero user-required actions.
- `full`: deferred to Checkpoint D. Reuse this scoped evidence unless the C09
  test/evaluator, release consumer, schema catalog, workflow, or pinned docs
  change.

### Q05 — Make reported shadow filtering match implementation

- [x] Pin the mismatch between public 3x3 PCF wording/stats and the current
  single hardware comparison/2x2 bilinear footprint.
- [x] Either implement and prove the documented 3x3 kernel or report the actual
  2x2 behavior consistently.
- [x] Update shader tests, stats/capabilities, README/rendering docs, and goldens.
- [x] Acceptance: implementation, metrics, capability JSON, and public wording
  name the same filter.

Validation ledger (2026-07-21):

- `focused red`: the new
  `triangle_shaders_use_nine_comparison_taps_for_reported_pcf3x3` test failed
  against the reviewed implementation: each GPU shader contained one
  `textureSampleCompareLevel` call while capabilities, frame stats, stable JSON,
  and historical docs reported kernel size 3.
- `classification`: product correctness and public-contract mismatch. The one
  linearly filtered comparison produced an implicit 2×2 hardware footprint; it
  was not the documented 3×3 PCF sample grid. Implementing the already-public
  contract preserved the stable field meaning and improved the shipped shadow
  path instead of renaming every report to the weaker behavior.
- `implementation`: both texture-array and WebGL2 texture-2D fragment shaders
  derive one texel from `textureDimensions(shadow_map)`, issue the center and
  eight neighboring depth comparisons, and average them by 9. The comparison
  sampler now uses nearest min/mag filtering, so each grid position is one
  comparison and the effective contract is exactly nine texels rather than
  nine overlapping 2×2 filtered footprints. Receiver frustum gating and
  clamp-to-edge behavior are unchanged.
- `mutations/doctor`: the lib oracle counts exactly nine comparison calls in
  each live shader and rejects a source-derived mutation that removes eight.
  The independent `Q05-SHADOW-PCF3X3` doctor evaluator rejects the same
  one-tap mutation, pins nearest sampling, shader/runtime tests, kernel constant,
  capability/stat goldens, exact directional-only scope, README, rendering and
  capability docs, changelog, and v1.8.0 errata.
- `focused green`: the exact nine-tap oracle passed 1/1; the builder's software
  Vulkan GPU compiled the texture-array pipeline and passed
  `headless_gpu_directional_shadow_visibility_darkens_receiver_when_available`
  1/1 in 29.03 seconds. The WebGL2 texture-2D WGSL passed Naga parsing and
  validation 1/1, and `single_shadow_map_records_pcf3x3_prepare_stats` passed
  1/1. The Q05 doctor mutation and current-source shadow-map contract each
  passed 1/1.
- `scoped`: all GPU output shader unit tests passed 30/30 and the complete
  affected `m2_lighting_depth_clipping` integration file passed 30/30 in 49.82
  seconds. Remote `cargo fmt --all --check`, `cargo clippy --lib -- -D
  warnings`, `cargo clippy -p xtask --all-targets -- -D warnings`, and
  `cargo run -p xtask -- doctor --full` passed. The Q04 stable-contract run
  remains valid because Q05 verified but did not alter the existing kernel-3
  fixtures.
- `skipped`: the CPU builder cannot provide physical native/WebGPU/WebGL2
  hardware screenshots. Those cross-backend artifacts and the full release
  matrix remain deferred to Checkpoint D; they are required before release
  readiness, not replaced by the software Vulkan proof.
- `bootstrap`: canonical source `/home/johannes/projects/scena`, isolated
  destination
  `/home/johannes/.cache/codex-worktrees/scena-full-review-v18-checklist`,
  branch `main`, HEAD `7b4fc9ca77e12fd12a69fab92650e1e46ee10354`.
  Root `AGENTS.md` hash
  `d0ed47595f6b6a6ec733651dfde650b7893aa2939d6137dc2f7eeade7528ac7a`
  and skills aggregate
  `a333a1ac0f97feaa5abf4512d2eac8b2ec77b0f4b3b59f24a608331c48216fa3`
  matched after every explicit bootstrap; the shared checkout remained missing
  and validation remained isolated.
- `counts`: about 34 minutes investigation; one implementation attempt, one
  documentation-needle refinement, zero release-candidate pushes, zero
  full-matrix runs, and zero user-required actions.
- `full`: deferred to Checkpoint D. Reuse this scoped evidence unless either
  output shader, shadow sampler, reported kernel fields/goldens, Q05 doctor
  evaluator, or pinned public docs change.

### Q06 — Enforce silent-failure patterns with doctor where mechanically
detectable

- [x] Add source guards for side-effectful short-circuit parallel consumers,
  recipe assembly outside its owner, required GPU tests returning on missing
  adapters, stale package versions, and required artifact-list drift.
- [x] Pin tests/functions as active CI targets, not substring comments.
- [x] Add known-bad executed fixtures for every new rule.
- [x] Keep runtime-only correctness in tests; do not pretend static doctor pins
  replace execution.
- [x] Acceptance: deleting or bypassing the test/docs/gate owner produces a
  deterministic doctor finding.

Validation ledger (2026-07-21):

- `focused red`: two new xtask mutations failed against the old doctor: an
  ignored required test still satisfied `require_rust_test_functions`, and a
  file-level `// fail_closed release_evidence` comment suppressed an otherwise
  unregistered `return;`. The cross-owner test then failed to compile until
  the Q06 guard owner existed.
- `classification`: doctor policy defects. Required-test discovery inspected
  only `#[test]`, while control-flow policy granted a whole-file exemption on
  two unrestricted substrings. The existing C01, C03, Q04, and C04 guards were
  valid but had no one active, mutation-tested owner tying all five
  silent-failure families together.
- `implementation`: required Rust test pins now accept only active,
  non-ignored test items. Early-return exemptions use exact paths plus owner
  and rationale; marker words no longer bypass the scan. The new
  `FULL-REVIEW-Q06-SILENT-FAILURE-GUARDS` rule protects non-short-circuit Rayon
  completion (including the split owner module), canonical SceneHost recipe
  assembly, typed required-GPU skip evidence, Linux native rendered-output
  existence, canonical public versions, active mutation tests, and the
  documentation boundary between source wiring and runtime proof.
- `known-bad fixtures`: the aggregate fixture passes in its complete form and
  independently restores `.any`, first-import assembly, a bare required-GPU
  return, the missing Linux artifact inventory row, and a stale public version;
  all five mutations are rejected. Separate mutations prove ignored tests and
  comment-word control-flow bypasses fail.
- `integration finding`: rebuilding the previously stale bundle exposed that
  the first C02 portability implementation embedded the complete 1K HDR into
  WASM: public raw/brotli size rose to 6,585,959/2,521,223 bytes and proof to
  7,330,151/2,713,114, correctly failing existing budgets. This was a product
  defect, not a reason to widen the gate. Studio now embeds a licensed,
  SHA-pinned 128x64 Lanczos derivative (27,666 bytes) while retaining the 1K
  source and provenance. The focused preset catalog/load/render suite passed
  3/3.
- `generated proof`: the documented builder validates both generated package
  versions as 1.8.0, stamps 1.8.0 titles/cache busters, and compiles WASM with
  size optimization before `wasm-opt`. Rebuilt public output is 3,880,185 raw
  / 1,149,523 Brotli bytes; proof output is 4,435,555 raw / 1,304,041 Brotli
  bytes, both below the unchanged budgets. Generated package directories stay
  ignored; tracked HTML/JS cache-buster and version text changed through the
  builder only.
- `focused`: the complete `q06_` xtask filter passed 7/7 before the final
  version-text extension; the aggregate five-family mutation test passed again
  afterward. Runtime behavior remains owned by the focused C01/C03/Q04/C04
  tests and rendered/hardware lanes, not by this static meta-rule.
- `scoped`: `cargo fmt --all --check`, strict xtask Clippy, the complete Q06
  filter (7/7), and `doctor --full` passed. The physical-GPU evidence required
  by Q01/Q04 remains deferred to its real hardware lane.
- `bootstrap`: canonical source `/home/johannes/projects/scena`, isolated
  destination
  `/home/johannes/.cache/codex-worktrees/scena-full-review-v18-checklist`,
  branch `main`, HEAD `7b4fc9ca77e12fd12a69fab92650e1e46ee10354`;
  shared checkout absent. Root `AGENTS.md` hash
  `d0ed47595f6b6a6ec733651dfde650b7893aa2939d6137dc2f7eeade7528ac7a`
  and skills aggregate
  `a333a1ac0f97feaa5abf4512d2eac8b2ec77b0f4b3b59f24a608331c48216fa3`
  matched after every explicit sync/bootstrap.
- `counts`: about 85 minutes investigation; one Q06 policy implementation,
  two fixture refinements, one C02 payload correction, and one measured build
  profile correction; zero release-candidate pushes, zero full-matrix runs,
  and zero user-required actions. No repeated product failure signature
  reached the circuit breaker.
- `full`: deferred to Checkpoint D. Q06 does not claim physical GPU, browser
  parity, workspace-wide tests, rustdoc, publish, or release readiness from
  static source guards.

## 7. Performance remediation: measure before optimizing

No percentage or ranking below is a release claim until a controlled benchmark
distribution proves it. For each item, capture cold/warm state, scene size,
backend/adapter, sample count, p50/p95, allocations, commit, and command. Close
an item as `measured-no-change` if the proposed complexity is not justified.

### P01 — Cache WGSL shader modules and compatible pipeline state

- [x] Benchmark full prepare after structural edits across representative
  binding modes/features, recording shader-module and pipeline creation counts.
- [x] Create one shader module per Device/source variant or texture-binding mode
  and share it across compatible pipelines.
- [x] Key caches by every source/define/layout/format/sample-count dependency;
  invalidate on device loss.
- [x] Do not hide first-use shader compilation inside `render()`.
- [x] Add cache-hit/miss/allocation counters and correctness parity.
- [ ] Acceptance: module compilation count drops without stale pipelines or
  device-lifetime leaks, and controlled p95 materially improves.

### P02 — Render native PresentOnly once and restore surface MSAA

- [x] Benchmark pass count and GPU submissions with post off/on, capture off/on,
  and AA off/MSAA.
- [x] In PresentOnly/no-post mode, render directly to the surface or render once
  and blit; do not redraw the full scene offscreen and on-surface.
- [x] Build/use a multisampled surface path with resolve when MSAA is requested
  and supported.
- [x] Preserve readback and post paths without introducing hidden allocations.
- [ ] Add rendered parity and physical native-surface evidence.
- [ ] Acceptance: one scene pass for PresentOnly/no-post and observable MSAA on
  the presented window.

### P03 — Remove unconditional native prepare synchronization

- [x] Measure `prepare()` CPU wall time, wait count/duration, pending
  destructions, and memory under steady rendering and rapid structural edits.
- [x] Replace `wait_indefinitely()` with nonblocking polling for routine
  destruction bookkeeping.
- [x] Keep explicit blocking only for APIs whose contract requires completion,
  shutdown, readback, or resource-pressure recovery.
- [x] Bound pending destructions and prove no use-after-free/leak.
- [x] Acceptance: steady retained rendering no longer serializes CPU and GPU at
  every prepare.

### P04 — Stop cloning complete animation clips every tick

- [x] Benchmark allocations/bytes and tick time for multiple mixers and large
  clips.
- [x] Store immutable clips through `Arc<AnimationClip>` or an equivalent stable
  owner and borrow channels during evaluation.
- [x] Preserve clip replacement/rebinding and thread-safety semantics.
- [x] Add zero-per-tick clip-data allocation assertions after warmup.
- [x] Acceptance: keyframe vectors are not reallocated per mixer per frame.

### P05 — Remove measured CPU raster hot-loop waste

- [x] Profile before selecting subitems; keep linear intermediate buffers
  correct for transparency/transmission/reflection.
- [x] Defer tone mapping and sRGB encoding to one final resolve where semantics
  permit, rather than per covered fragment under overdraw.
- [x] Replace divisions by precomputed inverse-area multiplication.
- [x] Compute camera/quaternion/tangent invariants once and avoid duplicate
  vertex projection between binning/rasterization.
- [x] Use a proven bit-identical 256-entry u8-to-linear LUT where applicable.
- [x] Add q01/reference parity plus controlled distributions after each
  independently measurable subitem.
- [x] Acceptance: only changes with measured benefit and unchanged reference
  truth remain.

### P06 — Reuse joint maps and precompute skin normal matrices

- [x] Measure joint-resolution complexity and per-vertex skinning cost across
  joint/node/influence sizes.
- [x] Reuse the existing `SourceNodeIndex` mapping for joint resolution.
- [x] Compute position and normal matrices once per joint/update, not per
  vertex-influence.
- [x] Preserve nonuniform-scale normal correctness and singular-transform
  diagnostics.
- [x] Add animated/skinned rendered parity and allocation counters.
- [x] Acceptance: resolution scales with nodes+joints rather than their product,
  and inverse/cofactor work scales with joints.

### P07 — Memoize doctor source scanning

- [x] Instrument files opened, bytes read, directory walks, cache hits, and total
  elapsed time for `doctor --full`; use deterministic cache/I/O counters rather
  than noisy per-rule wall time on the shared CPU builder.
- [x] Build one immutable repo file index and content cache per doctor run.
- [x] Share needle searches without changing missing-file/fail-closed behavior.
- [x] Add cache-equivalence and mutation tests.
- [x] Report measured improvement; do not publish the review's unmeasured
  `>10x` claim unless the controlled distribution proves it.
- [x] Acceptance: identical findings/order with materially less repeated I/O.

### P08 — Remove smaller proven clones/rescans

- [x] Store label glyph bitmaps as shared immutable bytes such as `Arc<[u8]>`
  and prove raster output unchanged.
- [x] Compute primitive-list flags once instead of rescanning in every band
  worker.
- [x] Measure allocations and worker time before/after each change.
- [x] Keep only subitems with a meaningful signal and no complexity regression.
- [x] Acceptance: counters and output parity prove the retained changes.

Validation ledger (2026-07-21):

- `P01 focused`: `p01_shader_module_cache` proved one device-owned shader
  module per texture-binding mode, prepare-only creation, hit/miss/creation
  counters, and pixel parity. The controlled llvmpipe sample recorded cold p95
  4840.878 ms and warm p95 5122.931 ms (-5.826%) and was classified
  `inconclusive-software-adapter`; the p95 acceptance box remains open pending
  physical controlled-GPU evidence.
- `P02 focused`: the native target planner proves one scene-color pass and one
  queue submission for present-only/no-post, preserves offscreen readback/post
  plans, and selects a multisampled surface resolve target when requested.
  `native_surface_hardware_proof` compiles with the new counters. Physical
  native-window parity and visibly effective MSAA remain open.
- `P03-P04 focused`: prepare polling tests prove routine nonblocking operation
  and the bounded pressure fallback; PF00 mixer metrics prove zero cloned clip
  bytes after warmup with shared `Arc<AnimationClip>` ownership.
- `P05 focused`: `p05_cpu_hot_loop`, q01, depth clipping, OIT, strict
  transmission, and LUT tests passed. Retained row bins already carried
  projected primitives, so the review's duplicate-projection subclaim required
  no second implementation. Final encoding, reciprocal-area, and shared-LUT
  changes remained only after reference parity.
- `P06 focused`: shared source-node-index and skin matrix tests prove O(nodes +
  joints) resolution, once-per-joint normal matrices, inverse-transpose truth
  under nonuniform scale, C14 CPU/GPU semantics, and M3B animated deformation.
- `P07 focused/scoped`: cache mutation/equivalence tests passed; the complete
  371-test xtask suite passed; `doctor --full` preserved its finding/order
  contract and reported `file_opens=1107`, `bytes_read=11602180`,
  `source_tree_walks=14`, `directory_reads=163`, `cache_hits=11769`, and
  `elapsed_ms=67348`. Per-rule timing was intentionally not added because it
  does not isolate cache work on a shared CPU; deterministic I/O/cache counters
  replace that part of the requested instrumentation. No `>10x` claim is made.
- `P08 focused`: glyph buffers stay shared as `Arc<[u8]>`, primitive flags scan
  once per frame, allocation/work counters pass, and the label integration
  image is pixel-identical.
- `scoped`: the combined P01/P03/P05/P08 integration tests passed 4/4; focused
  skin, animation, clipping, transparency, transmission, q01, and label suites
  passed. `cargo fmt` and full doctor passed after the source-owner splits.
- `full`: deferred to Checkpoint D. No physical-GPU shader latency or native
  surface/MSAA acceptance is inferred from the Hetzner CPU/llvmpipe builder.

## 8. Documentation, README, release, and roadmap completion

Documentation changes happen with the behavior they describe, not as a single
unreviewed rewrite at the end. D01-D06 are the final consistency sweep after
implementation.

### D01 — Repair version pins and demo package drift

- [x] Align `demo/pkg/package.json` with the crate/release version policy.
- [x] Include `scripts/build_demo_wasm.js`, package manifests, lockfiles,
  docs.rs links, examples, and generated bundle metadata in the version-bump
  sweep.
- [x] Add a doctor rule that derives and compares user-visible versions from
  one canonical source, with an explicit allowlist for historical docs.
- [x] Rebuild generated demo assets only through the documented build, then
  verify no stale `1.7.1` runtime/package metadata remains.
- [x] Record generated-file provenance and do not hand-edit compiled bundles.

Validation ledger (2026-07-21):

- `claim correction`: `demo/pkg/package.json` and `demo/proof/pkg/package.json`
  are generated and ignored, not committed as the review claimed. Both local
  generated packages were genuinely stale at 1.7.1, while the committed
  `demo/index.html`, `demo/main.js`, `demo/proof/index.html`, and
  `demo/proof.js` also carried stale 1.7.1 cache busters; the public title and
  proof subtitle were older still at 1.5.
- `focused red`: the D01 mutation fixture failed to compile until a dedicated
  version owner existed. It now independently rejects a mismatched scena row
  in `Cargo.lock`, a publishable/versioned release-gate Node package, stale
  generated package metadata, stale cache busters/title, a numeric current
  docs.rs link, a numeric example dependency, and a builder that stops
  validating generated package versions.
- `implementation`: `D01-PUBLIC-VERSION-ALIGNMENT` derives 1.8.0 only from the
  root `[package]`, compares the lockfile, optional generated package and WASM
  size manifests, tracked public/proof text and cache busters, current docs.rs
  links, version-agnostic onboarding/example dependencies, and the documented
  build owner. Root Node release-gate manifests stay private and versionless.
  Historical numeric evidence is allowed only under the explicit
  `HISTORICAL_VERSION_PATH_PREFIXES` paths for changelog, release notes,
  reviews, checklists, and decisions.
- `generated provenance`: only `npm run demo:build` and `npm run proof:build`
  changed generated outputs. Each ignored size manifest records
  `crate_version`, feature set, raw/Brotli sizes, and optimized WASM SHA-256;
  tracked cache busters are derived from that hash plus normalized JS/HTML.
  The final public/proof hashes are
  `64867028852d42548970064edf22e0637babc29a584c637ce91ce20741efdbdd`
  and
  `4013822e6fda5b1ff8601a993bdf5cdeb0f038fbe74df918351153bbb36d7fc9`.
- `focused/scoped`: the eight-surface D01 mutation test passed 1/1 after two
  fixture-only expectation corrections. Final formatting, strict xtask lint,
  D01 mutation rerun, and full doctor are recorded after this ledger. Browser
  runtime behavior was unchanged, so no Playwright lane was run for this
  metadata/build-policy slice.
- `bootstrap`: canonical source `/home/johannes/projects/scena`, isolated
  destination
  `/home/johannes/.cache/codex-worktrees/scena-full-review-v18-checklist`,
  branch `main`, HEAD `7b4fc9ca77e12fd12a69fab92650e1e46ee10354`;
  shared checkout absent. Root `AGENTS.md` hash
  `d0ed47595f6b6a6ec733651dfde650b7893aa2939d6137dc2f7eeade7528ac7a`
  and skills aggregate
  `a333a1ac0f97feaa5abf4512d2eac8b2ec77b0f4b3b59f24a608331c48216fa3`
  matched after every explicit sync/bootstrap.
- `counts`: about 40 minutes after Q06 bundle diagnosis; one D01 implementation
  and two fixture-only refinements; zero release-candidate pushes, zero full
  matrices, and zero user-required actions.
- `full`: deferred to Checkpoint D. D01 does not claim runtime browser,
  physical-GPU, workspace-wide, rustdoc, publish, or release-readiness proof.

### D02 — Update README and onboarding claims

- [x] Make the quick start use a PBR-visible default or explicitly add lighting
  and environment before render.
- [x] Qualify `shadows` as the exact shipped directional-caster/filter scope
  until point/spot/cascaded work is proven.
- [x] Keep low-level explicit defaults distinct from high-level presentable
  defaults.
- [x] Make every `cargo install scena` command portable outside the repo.
- [x] Document agent feature selection, capability discovery, policy roots,
  recipe validation/build equivalence, and backend selection.
- [x] Compile/test every Rust and shell/CLI snippet touched.
- [x] Replace stale hardcoded schema counts with generated discovery or wording
  that does not drift.

### D03 — Update user documentation with each affected contract

- [x] `docs/getting-started.md`: portable install, visible first render,
  feature flags, and first diagnostic step.
- [x] `docs/troubleshooting.md`: blank/black output, `diagnose`, `repair`,
  capability checks, policy violations, missing cameras, and surface recovery.
- [x] `docs/rendering.md`: defaults, transfer functions, clipping, real shadow
  scope/filter, MSAA degradation, and high-level/low-level distinction.
- [x] `docs/assets.md` and glTF support tables: cache/reload policy, UV sets,
  computed normals, influence limits, morph overrides, coordinate animation,
  anchors/connectors, and unsupported behavior.
- [x] `docs/browser.md`, `docs/capabilities.md`, `docs/lifecycle.md`, and
  `docs/errors.md`: pointer capture, live/static capability provenance, device
  and surface recovery, transfer/readback, and structured remedies.
- [x] `docs/api.md`, `docs/specs/public-api.md`, and Rustdoc: scale/framing/
  polyline migrations and any new result types.
- [x] `docs/examples.md` and agent-builder guide/skill: list command, template
  aliases, policy roots, build/validate/render loop, and portable assets.
- [x] `docs/guides/llm-app-builder.md` and
  `.codex/skills/scena-app-builder/**`: keep the public agent workflow aligned;
  do not repurpose root `AGENTS.md`, which is contributor governance rather
  than an installed-user guide.
- [x] `docs/schema-contracts.md` and fixtures: every additive/changed machine
  field, ordering/default/compatibility, and schema catalog entry.
- [x] Run link/snippet/schema fixture checks as focused/scoped proof.

### D04 — Keep changelog and release notes historically honest

- [x] Put fixes in `CHANGELOG.md` under `[Unreleased]` until their actual release
  is chosen.
- [x] Create next-version release notes only when a version is selected.
- [x] If v1.8.0 documentation made a false shipped claim, add an explicit dated
  erratum; do not rewrite history to imply the remediation shipped there.
- [x] Include breaking/deprecation notes for scale/framing/polyline/recipe
  grammar or newly strict importer behavior.
- [x] Link every headline visual/performance claim to its exact proof artifact
  and source commit.

### D05 — Consolidate roadmap truth

- [x] Choose one active open-backlog document; keep the RFC as the scope charter
  and completed checklists as historical evidence.
- [x] Remove duplicate `[shipped]`, `[deferred]`, and `[reopened]` statuses from
  overlapping active roadmaps or replace them with links to the canonical row.
- [x] Reconcile `next-release-easy-use-and-state-of-the-art.md` section 10: its
  verdict and shipped gate tags must not contradict each other.
- [x] Refresh the RFC's current execution tracks to point to the actual active
  checklist(s).
- [x] Add doctor reverse-drift checks so shipped implementation cannot remain
  marked deferred and unproven work cannot remain marked shipped.
- [x] Archive rather than delete useful completed evidence.

### D06 — Decide recipe persistence language before promising round trips

- [x] Through RFC governance, decide whether `SceneRecipeV1` is a canonical
  persisted document, an interchange/build input, or a transient authoring
  contract.
- [x] Before promising persistence, specify canonicalization, unknown fields,
  migrations, stable IDs, URI policy, extension points, and version-to-version
  round-trip behavior.
- [x] Keep host/domain ownership outside renderer scope.
- [x] Update README, recipe/schema docs, CLI wording, examples, and release notes
  only after the decision.
- [x] Add round-trip and forward/backward compatibility fixtures if persistence
  is ratified. N/A: persistence was not ratified; existing v1 compatibility
  fixtures remain the interchange-contract proof.

Validation ledger (2026-07-21):

- `D02-D03 content`: README, getting-started, troubleshooting, rendering,
  assets, browser/capability/lifecycle/error, API/Rustdoc, examples, agent
  guide/skill, and schema fixtures now describe the implemented defaults,
  remedies, policy, portability, import, output, and performance contracts.
  The final example/doc/CLI compile checks remain consolidated in Checkpoint D.
- `D04 history`: all remediation entries remain under `[Unreleased]`; the
  v1.8.0 notes contain explicit post-release errata. No next version was
  invented. No new headline timing claim was made: the llvmpipe P01 sample is
  recorded as inconclusive, and the checklist binds deterministic evidence to
  base HEAD `7b4fc9ca77e12fd12a69fab92650e1e46ee10354` plus the uncommitted diff.
  A final source commit cannot be cited because commit authorization was not
  given.
- `D05-D06 decision`: this file is the sole active implementation backlog; the
  RFC remains the scope charter and older plans are archived evidence. The RFC
  defines `SceneRecipeV1` as a versioned interchange/build input, not canonical
  application persistence. Same-version canonicalization, unknown-field loss,
  migrations, recipe-local IDs, URI policy, extensions, and host ownership are
  explicit. Persistence was not ratified, so new persisted-document round-trip
  fixtures are N/A.
- `focused red/green`: the D05-D06 mutation test first failed because the new
  governance owner did not exist, then passed 1/1 after implementation. One
  fixture/source-phrase correction was required after the first compiled run.
  It rejects both multiple-active-backlog drift and a persistence overclaim.
- `scoped`: `cargo fmt` completed on the remote builder and
  `cargo run -p xtask -- doctor --full` passed with the new docs/source pins.
  The doctor run includes Markdown links, required docs, schema fixtures,
  reverse shipped/deferred drift, and D01/D05/D06 governance checks.
- `bootstrap`: canonical `/home/johannes/projects/scena`; isolated destination
  `/home/johannes/.cache/codex-worktrees/scena-full-review-v18-checklist`;
  branch `main`; base HEAD `7b4fc9ca77e12fd12a69fab92650e1e46ee10354`;
  shared checkout missing; agent hash
  `d0ed47595f6b6a6ec733651dfde650b7893aa2939d6137dc2f7eeade7528ac7a`
  and skills hash
  `a333a1ac0f97feaa5abf4512d2eac8b2ec77b0f4b3b59f24a608331c48216fa3`
  matched after explicit bootstrap.
- `counts`: one implementation remediation and one proof-fixture refinement;
  zero release-candidate pushes, zero full matrices, zero user-required
  actions. Full release-level proof remains Checkpoint D.

## 9. Feature projects: governed backlog, not current defects

These items do not block Checkpoint D for the remediation batch. Each must be
split into its own implementation checklist and gets one full integration run
at the end of that feature batch, not after every subfix.

### F01 — Shadow completeness

- [ ] Ratify exact scope and order: spot shadows, point cubemap shadows, then
  cascaded directional shadows.
- [ ] Define authoring APIs, capability degradation, atlas/resource lifetime,
  bias/filtering, culling, transparency, and backend limits.
- [ ] Add feature-specific CPU/reference scenes and real GPU/browser proof.
- [ ] Update README's shadow claim only as each scope is proven.

### F02 — Linear HDR post chain

- [ ] Define `Rgba16Float` scene/post targets for native/WebGPU with explicit
  WebGL2/unsupported fallbacks.
- [ ] Keep lighting, bloom, blur, and compositing linear with one final transfer.
- [ ] Specify exposure/headroom, readback/capture format, memory/performance,
  and capability reporting.
- [ ] Add highlight/headroom and gamma-space mutation tests.

### F03 — Animation blending and crossfade

- [ ] Define layer/weight/additive/mask/conflict semantics and an explicit host
  clock.
- [ ] Avoid simulation/gameplay concepts; this remains renderer presentation.
- [ ] Add deterministic transform/morph blend oracles and allocation budgets.

### F04 — glTF/GLB export decision

- [ ] Require a concrete Blender/model-viewer/external-validator round-trip
  consumer before adding exporter scope.
- [ ] Ratify RFC ownership and assess a companion crate/tool.
- [ ] Define the supported subset and fail closed on data that cannot be
  represented; never silently drop scena-only state.
- [ ] Add import-export-import structural and visual comparisons.

### F05 — Draco and `KHR_animation_pointer`

- [ ] Gather failed real-asset telemetry and prioritize each extension
  independently.
- [ ] Review dependency license, security, maintenance, native/WASM size, and
  deterministic failure behavior.
- [ ] Keep compressed decode in assets and animation-pointer binding in import
  rebinding owners.
- [ ] Do not claim Sketchfab/general prevalence without dated evidence.

### F06 — API ergonomics

- [ ] Evaluate `scena::prelude` against name collisions and public API cost.
- [ ] Design a one-call `render_gltf_to_png` helper that composes existing
  Assets/Scene/Renderer owners, portable studio setup, framing, capture, and
  diagnostics without hiding async asset work or prepare.
- [ ] Wire existing placement functions as methods only where it reduces real
  friction.
- [ ] Extend legacy framing through existing `FramingOptions`, not another
  options type.

### F07 — Sampling and text polish

- [ ] Add anisotropic filtering only after live device-limit probing and sampler
  capability reporting; measure texture-quality and sampling cost.
- [ ] Treat international text as a complete font fallback/shaping/bidi/
  breaking/atlas project, not merely enabling a vendored font.
- [ ] Add script-specific native/browser rendered proof and accessibility
  expectations.

### F08 — Proof tooling productization

- [ ] Provide reusable per-pixel heatmaps, worst-region bounding boxes,
  windowed SSIM, masks, and mutation helpers.
- [ ] Record color space, alpha, crop, resolution, thresholds, reference digest,
  command, commit, and backend in every comparison.
- [ ] Migrate weak lanes only when old known-bad examples demonstrably fail.

## 10. Claim-to-work completeness crosswalk

Use this table before splitting the work into issues or pull requests.

| Review claim | Disposition |
|---|---|
| B1 parallel CPU bands | C01 |
| B2 agent template portability | C02 |
| B3 cone/wedge winding | C05 |
| B4 successful black defaults | C06 |
| B5 WebGPU no-post transfer | C07 |
| B6 Z-up rotation animation | C08 |
| B7 external texture reload | C09 |
| B8 device-loss recovery | C11 |
| B9 CPU near-plane clipping | C13 |
| B10 glTF normals/UV1/influences/morph overrides | C14 |
| B11 recipe first-import/sandbox bypass | C03, A01-A02 |
| B12 release-readiness/Linux Vulkan artifact fail-open | C04 |
| B13 browser/KHR/m2 proof weakness | Q01-Q03 |
| `scale_by` semantics | C16 |
| centering/framing | C17 |
| polyline panic | C18 |
| cylinder/cone UV seam | C19 |
| viewer pointer capture | C20a |
| stale demo version | D01 |
| path-only scene cache | C10 |
| WASM MSAA hard error | C20b |
| surface Outdated/Lost | C12 |
| `SCENA_USE_GPU` | C20c |
| repair positional/doctor routing | A06 |
| anchor forward/up degradation | C15 |
| GPU leak tests fail open | Q04 |
| shader recompilation | P01 |
| double native render/missing surface MSAA | P02 |
| blocking prepare poll | P03 |
| animation clip clone | P04 |
| CPU raster hot loop | P05 |
| skin joint/normal work | P06 |
| doctor repeated I/O | P07 |
| label clone/band rescan | P08 |
| capabilities command/live probe | A03 |
| validation vs render resolution | A01 |
| sandbox operator escape | A02 |
| help/list/diff/template names | A04 |
| `scena-convert` envelope | A05 |
| name suggestions/remedies | A07 |
| transform grammar | A08 |
| feature discoverability | A09 |
| recipe persistence wording | D06 |
| shadow/HDR/export/blending/Draco/ergonomics/text/proof features | F01-F08 |
| missed ordinary `textureInfo.texCoord` fallback | C14b |
| missed broad anchor/connector validation | C15 |
| missed PCF claim mismatch | Q05 |
| missed swallowed surface validation | C12 |

The review's feature-gate subclaim that two independent features are required
is corrected: `scene-host` already implies `inspection`. A09 may simplify
feature discovery but must not encode the false dependency. The requested
feature rankings and performance wins remain proposals until governed and
measured respectively.

## 11. Single final integration checkpoint

Run this section only after Checkpoints A-C are closed and the complete
remediation diff is stable. Do not run it after each item.

### 11.1 Final source and focused-proof audit

- [x] Freshly bootstrap one isolated remote snapshot from the final local tree
  and record path, target directory, branch, HEAD, and agent-file hashes.
- [x] Confirm every completed C/A/Q/P/D item has focused red and focused green
  evidence or a documented test-first exception.
- [ ] Confirm every performance item has a controlled distribution or is closed
  `measured-no-change` without an unsupported speed claim.
- [x] Confirm every public claim points to current proof and no optional F item
  is presented as shipped.
- [x] Confirm `git diff --check` and generated-file provenance.

### 11.2 Full CPU/native builder gates — run once

- [x] `cargo fmt --all --check`.
- [x] `cargo clippy --workspace --all-targets -- -D warnings`.
- [x] `cargo test -p xtask`.
- [x] `cargo test --workspace`.
- [x] `cargo test --doc`.
- [x] `cargo check --examples`.
- [x] Run the feature-specific scene-host/inspection/KTX2 integration targets
  present in `.github/workflows/ci.yml` so nondefault public surfaces are not
  skipped by the base workspace test.
- [x] `cargo run -p xtask -- doctor --full` from a fresh source/artifact state.
- [x] `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features`.
- [x] Produce source-bound M5 public API/benchmark artifacts with the canonical
  release command and validate their digests/provenance.

### 11.3 Full rendered/backend proof — run once on the proper hosts

- [x] Headless CPU q01 reference and all touched CPU visual proofs, including
  known-bad mutation rejection.
- [x] Linux browser WebGL2 complete lane.
- [ ] Linux browser WebGPU conformance lane plus required physical-hardware
  parity evidence from an accepted adapter.
- [ ] Native Vulkan, Windows DX12, and macOS Metal platform lanes for touched
  lifecycle/render behavior.
- [x] WASM package/all-feature compile and browser SceneHost proofs.
- [ ] Native attached-surface resize/loss, PresentOnly, MSAA, and device-loss
  proofs where supported.
- [ ] Strict controlled performance distributions and baseline comparison for
  P01-P08. GitHub-hosted timing uses
  `SCENA_M9_TIMING_POLICY=report-only-hosted`.
- [ ] Every lane artifact records exact commit, command, backend/adapter,
  timestamps, hashes, metrics, and accepted/rejected mutations.

### 11.4 Release-readiness and package proof — run once

- [ ] Stage the complete source-provenance-bearing lane bundle from an explicit
  artifact root.
- [x] Prove C04's missing-root, zero-evidence, omitted Linux Vulkan output, and
  substitution mutations still fail.
- [ ] Run release-readiness and require a positive validated-artifact count.
- [ ] Run the clean, locked `cargo publish --dry-run` flow.
- [ ] Run semver/public-API checks required by the selected next release.
- [x] Verify packaged builtin environments, licenses, demos, docs, examples,
  schema fixtures, and version metadata are present and consistent.
- [x] Do not tag, publish, push, merge, or create a release unless separately
  authorized.

### 11.5 Final completion conditions

- [x] All C01-C21 correctness items are implemented and proven.
- [x] All A01-A09 agent/CLI contracts are coherent and documented.
- [ ] All Q01-Q06 required evidence fails on known-bad output and fails closed
  when prerequisites are missing.
- [ ] All P01-P08 changes are measured, output-correct, and free of unsupported
  ranking claims.
- [x] D01-D06 align README, user docs, API/schema docs, examples, changelog,
  release notes, version pins, and roadmaps with shipped reality.
- [x] The one final full checkpoint is green, or any remaining hardware/
  environment blocker is recorded honestly without converting it into success.
- [x] Local checkout, remote-builder proof, GitHub workflow proof, hardware
  proof, and published release state are reported as separate facts.
- [x] Optional F01-F08 items remain deferred or have separately ratified
  project checklists; they are not silently counted as remediation closure.

### 11.6 Final integration ledger (2026-07-21)

- `bootstrap`: canonical source `/home/johannes/projects/scena`; isolated
  destination
  `/home/johannes/.cache/codex-worktrees/scena-full-review-v18-checklist`;
  external target
  `/home/johannes/.cache/codex-targets/scena-full-review-v18-checklist`;
  branch `main`; source HEAD
  `7b4fc9ca77e12fd12a69fab92650e1e46ee10354`; shared builder checkout
  absent. The final full-tree mirror excluded only `.git` and `target`, then
  explicitly recopied `AGENTS.md` and `.codex/skills/**`. Canonical and remote
  hashes matched: `AGENTS.md`
  `d0ed47595f6b6a6ec733651dfde650b7893aa2939d6137dc2f7eeade7528ac7a`;
  skills aggregate
  `0d43e5362bbec042d3056cd4bd3663f2b772a69f23b6f59d3414cd62b8d9dd75`.
- `full CPU/native`: the one decisive `cargo test --workspace` run passed the
  library, every integration target, 372 xtask tests, 65 doctests, and four
  compile-fail doctests. Formatting, all-target/all-feature strict clippy,
  examples, feature-specific scene-host/inspection/KTX2 targets, full doctor,
  all-feature rustdoc, M5 release artifacts, claim audit, q01 CPU release
  oracle, and all-feature wasm check passed. After the final brushed-steel
  preset, browser assertion, and checklist changes, only affected scoped proof
  was repeated: the preset unit test, WebGPU material/source-material probes,
  formatting, all-target/all-feature clippy, 65 doctests plus four
  compile-fail doctests, and full doctor all passed. The workspace suite was
  not rerun because no other workspace-test risk surface changed.
- `rendered`: the complete Linux WebGL2 workflow, SceneHost WebGL2 proof, and
  semantic-AOV proof passed on Chromium/SwiftShader. The required WebGPU parity
  evaluator, Q02 WebGPU material proof, and focused source-glTF-material proof
  passed as software conformance. The complete WebGPU workflow then hit one
  fixture-fetch environment failure followed by repeated Chrome 147
  pre-navigation `SIGTRAP` exits. A smaller direct Playwright launch with the
  same executable and WebGPU flags succeeded, but the exact lane remained
  reproducibly unavailable; investigation stopped at the circuit breaker.
  Software conformance is not substituted for Q01/Q04 physical-hardware
  acceptance.
- `package`: the release browser-probe wasm package built and the size gate
  passed at 1,520,877 Brotli bytes against 2,097,152. `cargo package --list`
  recorded 1,068 packaged files. `cargo publish --dry-run --locked
  --allow-dirty` packaged 15.4 MiB/6.0 MiB compressed and compiled the package;
  the required clean detached-tree proof remains open because this exact diff
  is intentionally uncommitted and no commit was authorized.
- `release readiness`: an explicit `target/gate-artifacts` run discovered
  2,397 files and validated 32 of 81 required artifacts, then failed closed on
  the unstaged Linux Vulkan, physical WebGPU, macOS, Windows, lifecycle,
  benchmark, and visual-proof evidence. No staging metadata or platform
  artifact was synthesized. Missing-root, zero-evidence, omitted Linux Vulkan,
  and substitution mutations passed their negative tests.
- `physical Linux probe`: the Raspberry Pi V3D adapter was accepted only after
  the repository's explicit unstable-headless opt-in. The focused Q04 lifecycle
  test, P01 controlled distribution, and attached native surface proof each
  reached real driver work but failed to complete or emit an artifact within
  the 30-minute circuit breaker. Each process was terminated without a code or
  threshold change. This is environment/capability evidence, not a pass; the
  final physical acceptance remains assigned to the Windows one-shot lane.
- `Windows final-run hardening`: the prior one-shot runner did not execute the
  newly required Q01 live-pixel, Q04 strict-lifecycle, or P01 controlled-p95
  proofs and did not validate attached resize/loss. A test-first validator
  extension failed on the missing coverage, then passed while rejecting a
  non-rejected Q01 mutation, a Q04 pending destruction, sub-threshold P01 p95,
  and incomplete resize handling. The native proof now performs an attached
  resize/reprepare/render/restore cycle and latches structured surface loss.
  `scripts/build_windows_complete_hardware_bundle.sh` now requires a clean
  exact commit and pinned wasm-pack, cross-compiles all four Windows
  executables, packages both browser WASM variants, and writes a complete
  SHA-256 manifest plus source-commit binding. `run-proof.ps1` verifies that
  binding before running the complete proof set and uploading one archive.
- `Windows bundle scoped proof`: the updated JS validator passes locally;
  remote `cargo check --example native_surface_hardware_proof`, the focused
  P01 test, strict example/P01 and xtask Clippy, formatting, the Windows-bundle
  doctor mutation, and full doctor pass. The native example plus FR06, Q04,
  and P01 optimized `x86_64-pc-windows-gnu` executables all cross-compile. The
  first P01 replay failed before compilation on `/tmp` quota and passed after
  using the task-local `TMPDIR`, with no production change.
- `provenance correction`: four provisional lane JSONs generated while the
  tree was dirty passed their lane checks but cannot be attributed to base
  commit `7b4fc9ca77e12fd12a69fab92650e1e46ee10354`. The complete 52 MiB artifact
  directory was preserved outside the canonical staging path as
  `target/gate-artifacts-uncommitted-evidence-20260721`; none is counted as
  release evidence. A clean exact-commit rebuild remains mandatory.
- `physical lifecycle producer correction`: a focused xtask contract first
  failed because release staging required Q04 physical evidence but neither
  dependency matrix produced it. Both CI and release macOS Metal lanes now run
  the strict lifecycle test through the lane command recorder before artifact
  upload. The focused contract, formatting, strict xtask Clippy, and full
  doctor pass. This makes the staged bundle satisfiable without weakening the
  missing/software-adapter rejection; execution still awaits the exact-commit
  GitHub lane.
- `selected release version`: v1.9.0 was selected for the complete
  compatibility-preserving correctness, workflow, proof, and performance
  batch. The M5 version/file tests first failed against the old v1.8.0
  manifest and missing note, then passed after aligning Cargo, lockfile,
  generated demo metadata, cache-busters, current doctor references,
  changelog, README indexes, and `docs/release-notes/v1.9.0.md`. The D01 drift
  mutation test, formatting, strict xtask Clippy, and full doctor also pass.
  The already-green workspace integration checkpoint was not repeated for
  version/documentation-only changes.
- `first exact-commit hosted matrix`: GitHub Actions run `29818164424` at
  `3969753549be6b329439a1ec81885cc31acbfbe4` passed both browser lanes and the
  wasm package lane, then exposed four batched gate defects. Linux and Windows
  hit the 600-significant-line module guard after the current release-note
  list grew by one line; the Q01 doctor mutation fixture omitted the two new
  Windows proof scripts; macOS used GNU-only `head -c 0`; and the 4K lane
  treated a periodic Rayon scheduler allocation as a product regression.
  `scripts/collect_ci_failure_evidence.sh 29818164424` captured all four failed
  job logs and six artifact sets before any remediation edit.
- `hosted-matrix remediation`: the current release-readiness scan no longer
  rescans the obsolete v1.7.1 note, the Q01 fixture copies both required
  Windows scripts, and the broken-pipe proof no longer depends on GNU `head`.
  Allocation diagnostics recorded 16 steady allocations and a periodic 17th
  1,520-byte Rayon scheduler refill. A red/green contract now gates the p95
  allocation count against the unchanged stored budget, keeps maximum allocated
  bytes blocking, and reports the observed maximum count. The exact dedicated
  4K replay passed in 604.05 seconds with every feature row at or below p95 16;
  observed parallel-row maxima remained 17 and were retained in the artifact.
- `second exact-commit hosted matrix`: GitHub Actions run `29824678879` at
  `d801ce62c103d7ae415a822d00af6a0ce0c88f14` passed both browser lanes, the
  wasm package lane, and Headless 4K. Windows found one stale expected metric
  name in the benchmark-row test. macOS proved that BSD `head` rejects both
  zero-byte and zero-line forms. Linux native caught two M2 fullscreen-edge
  reference regressions introduced by the reciprocal-multiply barycentric
  optimization. `scripts/collect_ci_failure_evidence.sh 29824678879` captured
  all three failed job logs and seven artifact sets before remediation.
- `second-matrix remediation`: the Windows benchmark assertion now expects the
  p95 allocation metric that the contract emits. The Unix broken-pipe proof
  uses POSIX `dd` with a zero input count and suppressed transfer statistics.
  The exact M2 oracle failed before the opaque raster path restored division
  by triangle area, then passed with the committed images unchanged. The
  reciprocal optimization remains only on paths whose reference proofs stayed
  green; preserving fullscreen edge coverage takes precedence over that
  micro-optimization. A focused projection-bit test separately confirms that
  the near/far clipping projection does not perturb fully visible triangles.
- `third exact-commit hosted matrix`: GitHub Actions run `29827472523` at
  `fae3cddddd4ae04c4483e7d6e723b1ee0ebcddb4` passed WebGL2, WebGPU software
  conformance, wasm packaging, Headless 4K, macOS Metal, and hosted Windows
  DX12. Linux native alone failed the M2 structure oracle. The collected seven
  artifact sets prove that macOS and Windows matched the committed references
  while Linux omitted the exact fullscreen boundary pixels `(15,0)` and
  `(31,0)`, plus four pixels reconstructed on the `x = 0` clipping boundary.
  This classifies the failure as platform-dependent floating-point boundary
  handling, not reference drift. `scripts/collect_ci_failure_evidence.sh
  29827472523` preserved the sole failed log and all artifacts before editing.
- `third-matrix remediation`: CPU triangle inclusion now admits only an
  eight-ULP normalized barycentric boundary tolerance, rejects non-finite and
  meaningfully exterior samples, and uses a scale-aware sixteen-ULP tolerance
  for clipping-plane boundary reconstruction. Opaque, OIT, transmission, and
  semantic-AOV raster paths share the rule. The focused tolerance unit and the
  unchanged exact M2 reference oracle pass on the isolated builder. This is the
  second and final remediation attempt for the repeated M2 signature; another
  same-signature failure trips the circuit breaker and forbids another patch
  without a smaller reproducer from the failing host.
- `fourth exact-commit hosted matrix`: GitHub Actions run `29830583399` at
  `97816d97d94fdc15e0bcf14bc9caf794696385c3` passed WebGL2, WebGPU software
  conformance, wasm packaging, and Headless 4K. Linux, macOS, and hosted
  Windows advanced through the M2 renderer/reference suites, proving the
  boundary correction closed the repeated signature, then all failed on the
  same four stale doctor source pins. Those pins still required clipping
  implementation text in `cpu.rs` and `semantic_aov.rs` after the shared
  implementation moved to `cpu_geometry.rs`. The collected evidence under
  `target/ci-failure-evidence/run-29830583399` contains all three failed logs,
  annotations, and six artifact sets. This is a policy/test-harness ownership
  defect, not a third production-renderer remediation attempt.
- `fourth-matrix remediation`: production rendering code remained frozen.
  `ARCH-CLIPPING` and `FR06-SEMANTIC-AOV` now pin the shared owner implementation
  in `cpu_geometry.rs` and the delegating calls in the CPU and semantic-AOV
  consumers. New mutations prove doctor rejects removal of either consumer
  delegation and removal of the shared boundary tolerance. Both focused xtask
  tests, formatting, xtask-only strict Clippy, and `doctor --full` pass on the
  isolated builder. The workspace suite was intentionally not repeated because
  this correction changes only source-derived policy and its mutation tests.
- `fifth exact-commit hosted matrix`: GitHub Actions run `29833841396` at
  `a8b8d2d8d689d59a46899384e098ba3324c1213d` passed hosted Windows DX12,
  macOS Metal including the strict physical lifecycle proof, WebGPU software
  conformance, WebGL2, wasm packaging, and Headless 4K. Linux native/headless
  passed the all-feature 345-test library run and subsequent focused targets,
  then failed only in `scena_agent_cli_stdout_matches_golden_fixtures`: a
  camera-frustum coordinate rounded to three decimals was one emitted unit
  different (`-999.265` versus `-999.264`). The complete failed log and all
  seven artifact sets were collected under
  `target/ci-failure-evidence/run-29833841396` before editing.
- `fifth-matrix remediation`: the CLI output remains unchanged. The inspection
  golden now keeps schema and every non-frustum field exact while accepting at
  most one three-decimal output unit in camera-frustum floats. A focused replay
  then exposed the same harness family in animation rendered observations:
  Linux included one boundary pixel at the middle sample, changing its payload
  hash, coverage by one pixel, and centroid by less than 0.04 CSS px. The
  animation golden keeps all schema, clip, transform, revision, reason, and
  summary fields exact; hashes must be valid and preserve the same sample-change
  pattern; rendered centroids allow 0.05 px and coverage allows one pixel.
  Mutations beyond each threshold and integer identity drift are rejected. The
  complete 19-test `scena_cli_agent` target, focused mutation proof, formatting,
  and changed-target strict Clippy pass on the isolated builder. An attempted
  all-targets `inspection`-only Clippy invocation reached a pre-existing
  conditional unused import in `a09_feature_discoverability`; it is recorded as
  an invalid over-broad feature combination, not substituted for the green
  changed-target gate.
- `sixth exact-commit hosted matrix`: GitHub Actions run `29838343514` at
  `0516f5a4fe921e8cc3ceab3432ab7f3b802caa1d` passed hosted Windows DX12,
  macOS Metal including strict physical lifecycle, WebGPU software
  conformance, WebGL2, wasm packaging, and Headless 4K. Linux native/headless
  passed 373 xtask tests, all 345 all-feature library tests, and 98 recipe CLI
  tests before failing only in the MSAA8 diagnostic assertion. The product
  correctly rejected the explicit request with `supports at most 4 samples`
  and `explicit prepare requested 8`; the test accepted only the older
  equivalent wording. The complete failed log and all seven artifact sets were
  collected under `target/ci-failure-evidence/run-29838343514` before editing.
- `sixth-matrix remediation`: production code remains frozen. The recipe CLI
  test now accepts both supported diagnostic forms only when they name the
  expected maximum and requested sample counts. Focused mutations reject a
  missing maximum, a missing request, and incorrect counts. This is a distinct
  test-harness contract defect and the first remediation attempt for this
  signature. The focused mutation test, the exact real-adapter MSAA overlay
  test, formatting, changed-target all-feature strict Clippy, and full doctor
  pass on the isolated builder. The workspace suite was intentionally not
  repeated because this patch changes only the stale test assertion and its
  checklist evidence; the next exact-commit hosted matrix is the deciding full
  checkpoint.
- `seventh exact-commit hosted matrix`: GitHub Actions run `29843462107` at
  `fbe66e09697c99fee9df46d98c4e679f3aaa9e44` passed all seven implementation
  lanes: Linux native/headless, hosted Windows DX12, macOS Metal including
  strict physical lifecycle, WebGPU software conformance, WebGL2, wasm
  packaging, and Headless 4K. Linux ran beyond the prior recipe assertion and
  completed its full cargo and documentation gates. The dependent pre-merge
  release-evidence integrity job then failed closed because the physical Q04
  `required-result.json` lacked the source-checksum provenance required of
  every staged release artifact. The sole failed log and all eight artifact
  sets were collected under
  `target/ci-failure-evidence/run-29843462107` before editing.
- `seventh-matrix remediation`: renderer and lifecycle behavior remain frozen.
  The Q04 producer now hashes `Cargo.lock` and its own test source into the
  required artifact, validates non-empty well-formed checksums before writing,
  and rejects a missing-checksum mutation. The independent Windows one-shot
  validator rejects the same mutation, the stable fixture carries the field,
  and doctor pins the producer so a future removal fails source policy. Release
  gate documentation, release notes, and the changelog describe the binding.
  This is a provenance/test-harness defect, distinct from all renderer and CLI
  signatures. The C09 evaluator mutation, independent Windows validator
  mutation, doctor producer-removal mutation, canonical release-staging test,
  stable-fixture test, formatting, C09 and xtask strict Clippy, and full doctor
  pass on the isolated builder. The first JS replay hit the known constrained
  system-temp quota (`write`, errno `-122`) and passed unchanged with the
  task-scoped `TMPDIR`.
- `latest isolated bootstrap`: canonical source
  `/home/johannes/projects/scena`; destination
  `/home/johannes/.cache/codex-worktrees/scena-full-review-v18-checklist`;
  target `/home/johannes/.cache/codex-targets/scena-full-review-v18-checklist`;
  branch `codex/full-review-remediation-1.9`; pre-remediation source HEAD
  `fbe66e09697c99fee9df46d98c4e679f3aaa9e44`; shared checkout absent. The
  explicit post-mirror bootstrap matched `AGENTS.md` SHA-256
  `d0ed47595f6b6a6ec733651dfde650b7893aa2939d6137dc2f7eeade7528ac7a`
  and relative skills aggregate
  `8b5c11eb955253fe651e9b084c9d4bf5139afb565dc53d4223da1dc2e4771a0e`.
- `open acceptance`: physical WebGPU parity (Q01), physical GPU resource
  lifecycle (Q04), controlled physical-GPU p95 for shader caching (P01),
  attached native PresentOnly/MSAA proof (P02), Linux Vulkan, macOS Metal,
  Windows DX12, complete staged release readiness, selected-release semver
  review, and a clean-tree publish dry-run. Optional F01-F08 remain deferred.
- `process counts through the seventh hosted matrix`: seven release-candidate
  pushes, seven GitHub full-matrix runs, zero user-required hardware actions,
  seven branch commits/pushes, and no tag/merge/publish. One workspace-wide CPU
  test checkpoint was run. The hosted 4K investigation took approximately 45
  minutes, used two
  unsuccessful unpushed aggregation/projection experiments, then froze changes
  for per-sample and per-allocation-size probes before the benchmark-contract
  fix. The WebGPU browser launch circuit breaker separately tripped after two
  same-signature remediation-free replays; no third production or harness patch
  was made there.
