# Release gates

Status: active release contract

This document names the durable release evidence and commands that the release
workflow and `xtask` validate. A local green command is evidence only for the
exact source commit recorded by its artifact; it is not publication proof.

Static doctor guards enforce ownership and wiring; runtime correctness remains
owned by executed focused tests and rendered evidence. A source pin cannot
replace the focused behavior, mutation, browser, or physical-hardware proof
named by the corresponding release contract.

## Proof-level vocabulary

These labels are ordered by claim strength and are not interchangeable:

| Level | What it proves | What it cannot claim |
|---|---|---|
| smoke | The process/backend initialized, submitted work, and produced nonblank output. | Correct pixels, parity, or release readiness. |
| conformance | A named API/lifecycle/format contract and its negative cases executed on the stated backend. Software adapters may provide this level. | Physical-hardware behavior or broad image equivalence. |
| deterministic reference | One renderer path matches a committed source-bound full-frame oracle and rejects named mutations. | Another backend or adapter matches it. |
| cross-backend parity | Two named renderer paths match over a declared full-frame normalization, thresholds, and mutation set. | Unnamed fixtures, materials, or platforms. |
| hardware evidence | A conformance/reference/parity proof executed on an identified physical adapter with source and artifact provenance. | Publication authority by itself. |
| release evidence | Hardware or deterministic evidence required by policy, issued by trusted CI/operator workflow, provenance-verified, complete, and accepted by staging. | Any claim beyond its exact fixture and declared scope. |

Local commands and artifacts state their level. GitHub-hosted CPU/software GPU
results are local/CI conformance unless a stricter section below explicitly
promotes them. Physical macOS/Windows artifacts become release evidence only
after Q03 provenance and staging validation; a `release_evidence:true` field is
not self-authenticating. The enforcing workflows and schemas are named in each
required section below: Q01 browser parity, native m8 WaterBottle, Q08 physical
CPU/GPU parity, Q04 lifecycle, Q07 antialiasing effect, and Q11 reference
stability.

`Cargo.toml` is the canonical public version source. The D01 doctor sweep
compares `Cargo.lock`, generated package and WASM-size metadata when present,
tracked demo/proof titles and cache busters, current docs.rs links, public
dependency instructions, examples, and the documented bundle builder. Numeric
historical evidence is exempt only under the source-owned
`HISTORICAL_VERSION_PATH_PREFIXES` list (`CHANGELOG.md`, versioned release
notes, reviews, checklists, and decisions); current onboarding/API surfaces are
never exempt.

## Required command families

- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `cargo check --examples --all-features`
- `cargo run -p xtask -- doctor --full`
- `cargo publish --dry-run`

Browser, GPU, rendered-output, and performance lanes add their own typed proof
artifacts. Required lanes fail closed when their renderer or hardware work is
unavailable; optional diagnostic lanes must say that they are non-release
evidence.

## CI-issued release provenance

Downloaded lane artifacts do not become release evidence merely because a
local process copied them into the canonical bundle. The CI and release
workflows first generate `ci-provenance.json` from trusted GitHub context. The
versioned record binds repository, workflow/ref, workflow SHA, source commit,
run ID and attempt, job, timestamp, and a SHA-256 digest over every downloaded
artifact file. The unsigned record remains `release_evidence: false` and names
`CI_ATTESTATION_NOT_YET_VERIFIED`.

The workflows then use the commit-pinned `actions/attest` action to issue a
SLSA v1 provenance attestation for that exact manifest. Strict staging runs
with `SCENA_REQUIRE_CI_PROVENANCE=1`, recomputes the complete artifact-tree
digest, checks the source commit is reachable, requires the current workflow
context to match every recorded identity field, and invokes
`gh attestation verify` against the exact repository, signer workflow, source
digest, and source ref. Wrong repositories, replayed runs, changed refs,
missing jobs, stale manifests, and post-manifest artifact changes fail closed.

Local staging remains available for development, but its
`staging-metadata.json` explicitly records `release_evidence: false` with
`CI_PROVENANCE_UNVERIFIED`; it cannot satisfy release readiness. Only staging
metadata carrying a verified attestation receipt can pass the release gate.
Readiness does not trust that receipt as a self-reported field: the staged
bundle retains the signed `ci-provenance.json`, and readiness independently
runs the same constrained signature verification before it can return success.
Retain `ci-provenance.json`, `staging-metadata.json`, and the staged artifact
bundle with the release. GitHub's attestation is the cryptographic identity
record; ordinary workflow artifact retention is only transport and must not be
treated as the trust anchor.

## Required WebGPU hardware parity

`SCENA_REQUIRE_PARITY=1 SCENA_BROWSER_BACKENDS=webgpu npm run browser:q01-parity`
is the required physical-hardware image-parity producer. It compares the live
renderer-owned WebGPU readback with the CPU oracle for the exact
`m6-identical-unlit-triangle-v1` fixture under a declared top-left opaque-sRGB8
normalization, exact dimensions, and a declared CPU-reference-derived two-pixel
gradient-edge mask for both channel errors and foreground IoU. Candidate edges
never expand the exclusion mask.

The evaluator requires Chebyshev-4 coverage of at least 99.5%, RGB RMSE no
greater than 2, a 99.5th-percentile channel delta no greater than 4, and
foreground IoU of at least 0.995. It also requires rejection of wrong-color,
geometry-shift, missing-object, vertical-flip, linear-as-sRGB, and
stale-reference mutations. Draw calls, submissions, nonblack pixels, and
adapter identity remain necessary diagnostics, but none is an image-parity
criterion.

The dedicated `scena.q01.required_webgpu_pixel_parity.v1` artifact contains
the CPU/GPU/diff PNG hashes, metric summary, worst-region bounding box,
adapter, command, commit, and source checksums. The aggregate
`scena.m6.rust_wasm_renderer_probe.v1` artifact embeds the evaluated result.
The release consumer independently requires that pixel result and its six
mutation rejections; setting a top-level status string on a smoke-only artifact
does not pass. Software adapters remain `software-conformance` evidence and
cannot satisfy a `hardware-release` lane.
When Chromium redacts the WebGPU adapter identity, the Q01 producer supplements
it with `SystemInfo.getInfo` evidence collected from that exact browser process.
The active renderer must match a physical device in the sanitized report;
SwiftShader or an empty/unmatched report still fails closed.

The final Windows physical proof is built from a clean exact commit with
`scripts/build_windows_complete_hardware_bundle.sh <output.zip>`. The bundle
contains pinned browser packages and cross-compiled proof executables plus a
SHA-256 manifest. Run its `run-proof.ps1` once with the same 40-hex
`-SourceCommit` and an operator-provided `-UploadUrl`. That one command covers
live Q01 WebGPU parity, combined WebGPU/WebGL2 output and semantic-AOV proofs,
attached-surface resize/loss/PresentOnly/MSAA, Q04 resource retirement, and the
controlled P01 shader-cache distribution. A dirty checkout or commit mismatch
is rejected before packaging, and every downloaded and installed file is
verified against the manifest.
The native MSAA phase must complete with matching multisampled surface-color
and scene-depth attachments; resolved overlays remain single-sample. Any
uncaptured native wgpu validation message is retained in the uploaded run log.
The install step copies every manifest-listed root file, including
`source-commit.txt`, before validating the installed workspace.
The independent validator evaluates privacy-redacted Q01 WebGPU adapters with
the same-browser Chromium GPU inventory, rejects software renderers, and
normalizes path separators before requiring canonical Q01 artifact locations.
It also runs the native DX12 WaterBottle headline with
`SCENA_REFERENCE_DIFF=1` and requires the complete 512x512 comparison, pinned
reference hash, render and diff hashes, worst-region bounding box, structured
adapter key, fixed thresholds, and rejection of a horizontal-mirror mutation.

The macOS Metal CI and release lanes run that same WaterBottle full-frame
oracle. Sparse point samples and color-family histograms remain useful
diagnostics, but they cannot set `release_evidence:true` without the reference
diff and mirror rejection. Release staging independently rejects an incomplete
result and requires `m8-real-asset/waterbottle_gpu.png`,
`waterbottle_diff.png`, and their source-provenance bindings.
WaterBottle region checks use `scena.m8.waterbottle_adapter_expectation.v1`.
The portable and GitHub macOS profiles both retain Chebyshev-25 region
tolerances; the macOS profile instead pins the separately measured samples by
backend/vendor/device/device-type/driver fields, with owner, review date,
expiry, workflow run, and image hash in `reference_metadata.toml`. Adapter
display names are diagnostic only and cannot select an acceptance profile.

## Required physical CPU/GPU parity

Transmission, near-plane clipping, authored and imported dynamic transforms,
Z-up rotation animation, the core PBR sweep, and PF08 texture baking each run
as an exact test with `SCENA_REQUIRE_GPU_PARITY=1` on the macOS Metal release
lane and in the one-shot Windows DX12 proof. The strict variable changes adapter
absence and software fallback into failures. A passing
`scena.q08.required_cpu_gpu_parity.v1` result must identify the exact test,
physical adapter and backend, source commit and checksum, and a nonzero executed
assertion count. Release staging and the Windows archive validator re-check
those fields and reject skip artifacts or diagnostic runs.

Without the strict variable, an installed lavapipe adapter may exercise the GPU
code path as `diagnostic-gpu-conformance`; if no adapter is configured, the
ordinary all-target test emits a structured `skipped` result and returns. Both
forms deliberately set `release_evidence:false`. Lavapipe therefore remains
valuable CPU-hosted conformance coverage but never substitutes for Metal or
DX12 release evidence.

## Rendered WaterBottle mutations

The always-on Q01 CPU WaterBottle oracle retains one cheap post-hoc flattened
pixel mutation, but wrong-material and wrong-camera evidence must be fresh scene
renders. Each starts from a new glTF import, changes the mesh material or active
camera before `prepare`, renders through the CPU material/camera path,
PBR-neutral tonemapping, and sRGB8 output, then compares that frame with the same
committed reference. The result binds each mutation PNG hash, failing metrics,
mutation stage, render count, and pipeline coverage. Release finalization and
staging reject material/camera mutations labeled as pixel edits or carrying no
render execution.

## Q11 reference stability and regeneration

The Q01 CPU oracle compares two independently constructed in-process renders
byte-for-byte before reading the committed reference. Linux x86_64, macOS
arm64, and Windows x86_64 lanes run the exact Q11 test and retain separate
`scena.q11.reference_stability.v1` artifacts with the source commit, asset and
reference hashes, both render hashes, and both fixed-oracle metric records.
Release readiness requires all three records. The shared Chebyshev-4,
99.5%-within-tolerance, RMSE-2.0 policy is not widened to accommodate a host;
per-architecture references require measured stable differences and a separate
reviewed policy change.

Reference regeneration is intentionally a two-person/two-step process.
`scripts/stage_q01_waterbottle_reference_candidate.sh` runs only from a clean
checkout and writes a non-release candidate, diff heatmap, generator/source
provenance, and external Blender-anchor binding under `target/`; it never
overwrites the oracle. Promotion uses
`scripts/promote_q01_waterbottle_reference.cjs` with a separately authored
`scena.q11.reference_approval.v1` document naming the reviewer and binding the
candidate, prior reference, diff, generator commit, and external anchor. The
approval command refuses a tolerance change. Fresh three-architecture evidence
is required after promotion.

## Required GPU resource lifecycle

The self-hosted Linux hardware workflow runs
`SCENA_REQUIRE_GPU_RESOURCE_LIFECYCLE=1 cargo test --test
c09_gpu_resource_lifecycle
required_hardware_gpu_resource_lifecycle_executes_complete_cycle -- --exact
--nocapture`. An unavailable or software adapter fails instead of passing by
returning before assertions.
The macOS Metal CI and release lanes run the same strict test on their physical
host before uploading the artifact set consumed by staging; this keeps the
release matrix satisfiable even when the optional self-hosted Linux runner is
offline.

The producer writes
`c09-gpu-resource-lifecycle/required-result.json` with schema
`scena.q04.required_gpu_resource_lifecycle.v1`. Release readiness independently
requires physical adapter provenance, at least ten executed lifecycle
assertions, a larger prepared resource set, return to the baseline retained
shape, `Confirmed` device polling, and exact destruction of every queued
resource with zero remaining pending work. The producer also binds `Cargo.lock`
and its lifecycle test source with SHA-256 checksums, and both release staging
and the Windows bundle validator reject missing or malformed source provenance.
The artifact is required for the physical macOS Metal release lane. The hosted
Linux native Vulkan lane uses software rendering and does not claim this
physical-hardware proof. Files written by the clearly named optional
developer smoke tests have `status:"skipped"` and
`proof_class:"optional-developer-smoke"`; they cannot satisfy this gate.
The checksum-verified Windows complete-hardware bundle runs the same strict
Q04 test executable; its independent validator rejects a missing adapter,
incomplete assertions, a retained-shape mismatch, or any pending destruction.

## Anti-aliasing pixel effect

The native Metal and exact-candidate Windows hardware lanes render one pinned
high-contrast asymmetric diagonal with None, FXAA, MSAA4, and MSAA8 when the
adapter supports it. FXAA/MSAA must add intermediate-luma boundary coverage,
reduce hard transitions and normalized squared edge energy, preserve global
contrast, and stay within an edge-local coverage bound. Unsupported MSAA8 must
be recorded as structured `UNSUPPORTED_SAMPLE_COUNT` degradation; silent skip
is not accepted. The same evaluator rejects no-op AA and whole-frame blur
mutations.

Required WebGPU/WebGL2 PF01 browser output now records edge metrics from the
renderer-owned capture. Its validator requires FXAA to reduce relative hard
transitions and normalized edge energy while preserving contrast and bounded
coverage. Hash inequality alone is not anti-aliasing evidence.

## Performance timing policy

M9 always requires a valid distribution with the configured minimum sample
count and always blocks deterministic allocation-count or allocation-byte
regressions. Wall-clock timing enforcement depends on the measurement host:

- `strict-controlled` is the default and enforces the stored p95 frame/prepare
  thresholds. Use it on the isolated Hetzner builder or another stable,
  controlled performance host.
- `report-only-hosted` is mandatory on shared GitHub-hosted runners. The
  artifact retains the observed pass/fail result and regression percentage,
  but variable wall-clock timing alone does not fail the lane.

The policy is selected by `SCENA_M9_TIMING_POLICY`, recorded in every benchmark
artifact and baseline-comparison row, and doctor-enforced in hosted workflows.
A hosted regression must be reproduced under `strict-controlled` before it is
called a product performance defect. Allocation regressions remain blocking in
both modes; the hosted policy is not permission to widen stored baselines.

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

Generated demo/proof WASM bundles follow the same split. The ordinary source doctor
treats their joint absence as unavailable, not as evidence. After both
bundles are built, the browser release lane runs
`SCENA_DOCTOR_REQUIRE_GENERATED_ARTIFACTS=1 cargo run -p xtask -- doctor --full`;
that explicit mode fails if either bundle or its size manifest is absent or
outside budget. It never creates a placeholder artifact.

Shader health is compiled evidence, not a collection of source substrings.
The production-derived WGSL manifest owns every shader-module creation and all
texture-binding/output variants. CI runs its Naga parse/validation, required
entry-point, binding/location, capability, and omitted-variant mutation tests.
Material-uniform safety likewise compares the WGSL struct span, Rust encoder
length, and the actual bind-layout/bind-group size helper; doctor pins those
semantic tests instead of the numeric layout literal.

## Readiness invocation and result

Readiness consumes one explicit staged bundle. Invoke it with either:

```bash
cargo run -p xtask -- release-readiness --artifact-root target/gate-artifacts
```

or a non-empty `SCENA_RELEASE_ARTIFACT_ROOT`. The CLI argument takes
precedence. A missing, empty, nonexistent, unreadable, or incomplete root is a
policy failure; discovery of zero files can never be reported as ready.

The command prints a `scena.release_readiness.v1` JSON result and exits nonzero
on failure. The result includes `artifact_root`, `artifact_root_source`,
`discovered_artifact_count`, `required_artifact_count`, and
`validated_artifact_count`. A passing result requires the validated count to be
positive and equal to the canonical required-artifact count, with every schema,
lane/backend, source-commit, timestamp, digest, and file-binding check green.
The required inventory and its specialized validators are owned together by
`crates/xtask/src/app/release/review_artifacts.rs`; in particular,
`m9-platform/linux-native-vulkan/rendered-output.json` and
`c09-gpu-resource-lifecycle/required-result.json` are required to exist.

`stage-release-artifacts <downloaded-root> <canonical-output-root>` already
names both its input and output explicitly. Staging success does not replace
the readiness command and does not make a partial bundle acceptable.

Human review remains normal repository governance but is not a machine release
artifact or publication prerequisite. Optional supplementary review evidence
is described in `docs/specs/release-reviews.md`. GitHub workflow status, a Git
tag, a GitHub release object, and registry publication are separate downstream
facts.

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
