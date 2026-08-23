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
4. Keep README, RFC, specs, examples, and milestone checklists aligned with shipped behavior.
5. For public API changes, update or add examples and API-diff evidence once the M5 baseline
   exists.
6. For rendering, browser, visual, glTF, or platform changes, require the proof named in
   `docs/specs/release-gates.md`; unit tests alone are not release evidence.
7. Before starting a release matrix, prove every configured publication prerequisite is
   satisfiable by the repository's actual lanes, secrets, and governance.
8. Do not publish or tag unless the user asks.

## Versioning Defaults

- `0.0.x`: foundation, scaffolding, docs, and internal tooling before real renderer API.
- `0.x.0`: backward-compatible public renderer capability after implementation starts.
- `1.0.0`: only after the acceptance index and release gates are complete.

Breaking public API changes are allowed before `1.0.0`, but they must update examples, docs,
and migration notes when users can reasonably have adopted the previous API.

## Required Remote Gates

Run on `scena-builder` before release-ready handoff. These are release-checkpoint gates, not
the default inner loop. During implementation, use the `scena-remote-builder` ladder:
focused proof first, scoped gates second, then the exact full release workflow below once.

An unrun required gate is not a pass. Record the exact blocker when a gate cannot run. Use
`scena-remote-builder` to sync local uncommitted work before focused/scoped gates, but freeze
and commit the final candidate before the exact rehearsal.

## Frozen Candidate Rehearsal

Before tagging, freeze one candidate SHA and keep a concise ledger for that SHA. The ledger
must name every required command, artifact, result, and covered source surface. Do not
reconstruct release commands from memory: execute the exact workflow in this skill and
compare it with the current CI and Release workflows first.

For an isolated builder snapshot without `.git`, set `SCENA_RELEASE_COMMIT` to the frozen
local SHA for every provenance-sensitive command. Before tagging, the same frozen snapshot
must have:

- focused regression proof and scoped gates green;
- the full native/WASM/browser checkpoint green once;
- each locally reproducible `scripts/release_lane_command.sh` command recorded;
- honest release-lane artifacts (never synthesized macOS/Windows evidence on Linux);
- `scripts/verify_packaged_agent_install.sh` green;
- `cargo publish --dry-run --locked` green; and
- no source, harness, workflow, lockfile, agent-guidance, or release-contract edit after the
  rehearsal.

Any such edit invalidates affected ledger entries. Return to focused proof, rerun only
affected scoped lanes, refreeze once, and perform the required final checkpoint. Do not tag
an incompletely replayed candidate and use GitHub as the first complete test environment.

Before patching a failed release run, execute
`scripts/collect_ci_failure_evidence.sh <run-id>` for every failed run and classify every
failed job. Batch all known corrections into one release candidate. Two failed remedies with
the same signature trip the investigation circuit breaker; no third push is allowed without
a smaller discriminating proof.

Shared GitHub-hosted machines are not controlled performance hardware. Their M9 wall-clock
measurements use `SCENA_M9_TIMING_POLICY=report-only-hosted`; sample validity and allocation
budgets still block. Strict timing evidence must come from controlled hardware and must never
be replaced by a widened hosted-runner baseline.

The packaged artifact is a separate product surface. Repository all-feature builds and
default-feature `cargo publish --dry-run --locked` do not prove documented packaged feature
installations. Create/extract the `.crate`, install every documented consumer feature set,
and execute its assertion. The current mandatory gate is
`scripts/verify_packaged_agent_install.sh` for `--features agent`; repeat this against the
actual crates.io archive after publication.

Do not hand-chain remembered commands. Use `scripts/release_lane_command.sh` exactly and
preserve its ledger. Every isolated invocation receives `SCENA_RELEASE_COMMIT`; every focused
browser rerun repeats `SCENA_BROWSER_BACKENDS` and `SCENA_GPU_EVIDENCE_CLASS` because a new
SSH/shell invocation does not inherit an earlier command's environment.

For multi-fix work, release hygiene is one exact full release run at the final integration
checkpoint plus focused/scoped evidence per logical fix. Do not rerun publish/doc/browser
gates after each small patch unless that patch changes their release-artifact surface.

## Exact Scena Release Workflow

Use this exact workflow for a version bump, tag, or publish. Read the current
`.github/workflows/ci.yml` and `.github/workflows/release.yml` first. Those files are
authoritative; if their commands or versions differ from this section, update this skill in
the candidate before proceeding.

## Failure history this contract closes

The v1.10.x release history exposed these avoidable process failures. Treat recurrence as a
workflow defect, not as normal release iteration:

- a tag was pushed before the complete locally reproducible matrix was green;
- actionable work was incorrectly reported as blocked/stopped instead of continued;
- duplicate Cargo/doctor jobs competed for one cache, and broad gates were restarted instead
  of narrowing the reproducer;
- commands that selected zero tests were initially mistaken for evidence;
- isolated snapshots omitted `SCENA_RELEASE_COMMIT` in native, browser, size, and artifact
  commands, producing false visual/provenance failures;
- remote snapshots were stale despite matching size/mtime, because checksums/final sync were
  not used consistently;
- `rsync --delete` removed `node_modules` after an earlier `npm ci`, and a corrupted Binaryen
  install was trusted before checksum comparison;
- full-debug/incremental target trees and `/tmp` quota exhausted the builder because release
  environment/disk requirements were applied only after failure;
- browser and ChromeDriver majors differed, while wasm-pack ignored the `CHROMEDRIVER`
  environment override; the explicit `--chromedriver` option was discovered late;
- a focused FR06 rerun omitted `SCENA_BROWSER_BACKENDS=webgl2` and accidentally tested
  unavailable WebGPU;
- a stale HTTP server from a deleted worktree owned the deployment-probe port and served 404s;
- a malformed combined shell wrapper lost the checkout directory, and a wasm-pack argument
  ordering error turned `--out-dir` into a Cargo option;
- unsupported builder Chromium produced changing failures that were retried too broadly
  before the supported GitHub browser lane was allowed to decide;
- the published v1.10.5 archive was never installed with its documented `agent` feature, so
  repository builds and default-feature publish dry-run missed an omitted `include_str!`
  asset;
- a concurrent Cardine/Campo task wrote new WIP directly into canonical Scena `main` after a
  release was declared clean; and
- cleanup commands were repeatedly attempted with disallowed broad recursive forms instead
  of prevalidated exact-path deletion.

The procedure below is designed to prevent those exact classes rather than merely remind an
agent to “test more.”

## 1. Freeze and final sync

1. Inventory all Scena worktrees and preserve/move every dirty source change.
2. Commit all intended source, tests, workflows, metadata, release notes, and agent guidance.
3. Record `release_sha="$(git rev-parse HEAD)"`; require a clean checkout and an unused tag.
4. Run the builder preflight, final `rsync --delete`, explicit `AGENTS.md` and skill-tree copy,
   and checksum verification.
5. After that final sync, run `npm ci`. Do not sync again unless willing to reinstall npm
   dependencies and regenerate affected lane artifacts.

For every remote command, pass this environment explicitly (substitute the task slug/SHA):

```bash
env \
  CARGO_TARGET_DIR="$HOME/.cache/codex-targets/scena-<task-slug>" \
  CARGO_INCREMENTAL=0 \
  CARGO_PROFILE_DEV_DEBUG=0 \
  CARGO_PROFILE_TEST_DEBUG=0 \
  SCENA_RELEASE_COMMIT=<frozen-sha> \
  <command>
```

Verify the workflow-pinned Rust, Node, npm, wasm-pack, Playwright, Binaryen, and Brotli
versions. Verify `node -e 'require("playwright")'`, `node_modules/.bin/wasm-opt --version`,
the browser version, and a matching browser-driver major version. For a system Chrome,
invoke wasm-pack tests with `--chromedriver <matching-path>`.

## 2. Linux native/headless lane

Export the release environment above, install/point `VK_ICD_FILENAMES` at Lavapipe, set
`LIBGL_ALWAYS_SOFTWARE=1` and `SCENA_REQUIRE_PARITY=1`, then run the current Release workflow
commands in this order:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p xtask
cargo test -p xtask -- --list > target/xtask-test-list.txt
grep -Fqx 'app::tests_08::release_readiness_rejects_constant_ppm_visual_artifact: test' target/xtask-test-list.txt
cargo test -p xtask app::tests_08::release_readiness_rejects_constant_ppm_visual_artifact -- --exact
cargo test
cargo test --features inspection --test measurement_visual_proof
cargo test --lib --features scene-host scene_host::core_tests::c13_
SCENA_RELEASE_PROFILE=test-unoptimized cargo test --test m5_release
bash scripts/release_lane_command.sh linux-native-vulkan cargo test --test m9_platform_release
SCENA_M9_TIMING_POLICY=report-only-hosted SCENA_RUN_M9_PLATFORM_BENCHMARK=1 bash scripts/release_lane_command.sh linux-native-vulkan cargo test --test m9_platform_release m9_platform_benchmark_writes_release_artifact -- --exact --test-threads=1
bash scripts/release_lane_command.sh headless-cpu cargo test --test m9_platform_release
bash scripts/release_lane_command.sh headless-cpu cargo test --test m9_platform_release -- --ignored --test-threads=1
bash scripts/release_lane_command.sh headless-cpu cargo test --test q01_waterbottle_cpu_reference q01_default_cpu_waterbottle_matches_reference_and_rejects_known_bad_renders -- --exact
bash scripts/release_lane_command.sh headless-cpu cargo test --test q01_waterbottle_cpu_reference q11_waterbottle_cpu_is_byte_deterministic_before_reference_comparison -- --exact
bash scripts/release_lane_command.sh headless-cpu cargo test --test examples_visual_proof q02_live_cpu_round_e_showcase_emits_shared_evaluator_frame -- --exact
bash scripts/release_lane_command.sh headless-cpu node scripts/evaluate_round_e_cpu_materials.cjs
bash scripts/release_lane_command.sh linux-native-vulkan cargo check --examples --all-features
RUSTDOCFLAGS='-D warnings' cargo doc --no-deps --all-features
cargo run -p xtask -- doctor --full
cargo run -p xtask -- claim-audit
cargo run -p xtask -- release-lane-artifact linux-native-vulkan
cargo run -p xtask -- release-lane-artifact headless-cpu
```

Also run the CI-only native commands that are not present in the Release native job, followed
by the feature-contract lane. Do not treat the broader all-feature command as a reason to
skip these named contracts; the exact selectors also prove that the workflow still reaches
the intended tests.

```bash
cargo test --doc
cargo test --lib --features scene-host,inspection
cargo test --features scene-host,inspection --test scene_recipe_contracts --test label_text
cargo test --features scene-host,inspection --test scena_cli_agent_templates
cargo test --features scene-host,inspection --test scena_cli_recipe --test scena_cli_agent
cargo test --features inspection --test m7_threejs_ergonomics m7_scene_inspection
cargo test --features ktx2 --test m8_assets_materials_ecosystem m8_ktx2_basisu_feature_decodes_basisu_ktx2_rgba_pixels -- --exact
cargo check --example scene_inspection --features inspection
scripts/verify_packaged_agent_install.sh
cargo test --workspace --all-features --tests
```

## 3. WebGL2 lane

Pass `SCENA_BROWSER_BACKENDS=webgl2` on every command or enclosing SSH invocation. Confirm the
effective value before running. Use the matching explicit ChromeDriver option for the four
wasm-pack tests when the builder browser is not Playwright's bundled browser.

```bash
npm ci
cargo test --lib render::gpu::shader_manifest::tests -- --nocapture
wasm-pack test --headless --chrome --chromedriver <matching-path> --test m1_browser_rendered_output
wasm-pack test --headless --chrome --chromedriver <matching-path> --test m3a_browser_rendered_output
wasm-pack test --headless --chrome --chromedriver <matching-path> --test m3b_browser_rendered_output
wasm-pack test --headless --chrome --chromedriver <matching-path> --test m6_browser_renderer_parity --features browser-probe
bash scripts/release_lane_command.sh linux-webgl2-chromium wasm-pack build --dev --target web --out-dir target/m6-browser-pkg . --features browser-probe
bash scripts/release_lane_command.sh linux-webgl2-chromium npm run browser:m6
bash scripts/release_lane_command.sh linux-webgl2-chromium npm run browser:scene-host-proof
bash scripts/release_lane_command.sh linux-webgl2-chromium npm run browser:fr06-semantic-aov
bash scripts/release_lane_command.sh linux-webgl2-chromium npm run demo:build
bash scripts/release_lane_command.sh linux-webgl2-chromium npm run proof:build
cargo test --features agent --test photo_render_cli photo_render_camera_behavior_is_easy_path_for_imported_asset -- --exact
SCENA_DOCTOR_REQUIRE_GENERATED_ARTIFACTS=1 cargo run -p xtask -- doctor --full
```

Before the deployment probes, prove port 18104 is unused or stop only its verified stale
Scena listener. Start the server, curl the exact index, run both wrapped probes, stop it, and
finalize the lane:

```bash
python3 -m http.server 18104 --directory demo > target/gate-artifacts/cloudflare-demo-server.log 2>&1 &
demo_server_pid=$!
trap 'kill "$demo_server_pid" 2>/dev/null || true' EXIT
curl --fail --silent --show-error --retry 10 --retry-connrefused --retry-delay 1 http://127.0.0.1:18104/index.html >/dev/null
bash scripts/release_lane_command.sh linux-webgl2-chromium npm run cloudflare:demo -- http://127.0.0.1:18104/index.html
bash scripts/release_lane_command.sh linux-webgl2-chromium npm run cloudflare:materials -- 'http://127.0.0.1:18104/proof/?sample=material-presets'
kill "$demo_server_pid" 2>/dev/null || true
trap - EXIT
cargo run -p xtask -- release-lane-artifact linux-webgl2-chromium
```

## 4. WebGPU software-conformance lane

Pass both `SCENA_BROWSER_BACKENDS=webgpu` and
`SCENA_GPU_EVIDENCE_CLASS=software-conformance` on every invocation. First verify the selected
browser actually exposes software WebGPU.

```bash
npm ci
cargo test --lib render::gpu::shader_manifest::tests -- --nocapture
npm run test:required-gpu-parity
bash scripts/release_lane_command.sh linux-webgpu-chromium wasm-pack build --dev --target web --out-dir target/m6-browser-pkg . --features browser-probe
bash scripts/release_lane_command.sh linux-webgpu-chromium npm run browser:q02-materials
bash scripts/release_lane_command.sh linux-webgpu-chromium npm run browser:m6
cargo run -p xtask -- release-lane-artifact linux-webgpu-chromium
```

## 5. WASM and 4K lanes

```bash
npm ci
cargo check --target wasm32-unknown-unknown --all-features
bash scripts/release_lane_command.sh wasm32-unknown-unknown wasm-pack build --release --target web --out-dir target/m9-browser-pkg . --features browser-probe
bash scripts/release_lane_command.sh wasm32-unknown-unknown npm run wasm:size
cargo run -p xtask -- release-lane-artifact wasm32-unknown-unknown
SCENA_RUN_DEDICATED_4K_BENCHMARK=1 SCENA_M9_TIMING_POLICY=report-only-hosted SCENA_BENCHMARK_PROFILE=perf-test SCENA_BENCHMARK_COMMAND='cargo test --profile perf-test --test m9_platform_release m9_dedicated_headless_4k_benchmark_writes_release_blocker_artifact' cargo test --profile perf-test --test m9_platform_release m9_dedicated_headless_4k_benchmark_writes_release_blocker_artifact
```

## 6. Package/publish proof

From the frozen candidate, run:

```bash
scripts/verify_packaged_agent_install.sh
cargo publish --dry-run --locked
```

The first command must create/extract the `.crate`, install it with `--features agent`, and
execute the installed binary. A repository build is not a substitute. Run doctor/claim audit
after generated artifacts are final. Local release-readiness can only accept artifacts that
actually exist; do not fabricate macOS/Windows evidence on Linux.

## 7. Push, tag, monitor, and public proof

Set `release_branch`, `release_sha`, `version`, and `tag` once. Recheck topology/triggers and
that the version/tag are unused, then use this sequence:

```bash
release_branch="$(git branch --show-current)"
release_sha="$(git rev-parse HEAD)"
version="$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -n1)"
tag="v${version}"

test -z "$(git status --porcelain)"
git fetch origin --prune
test "$(git rev-parse HEAD)" = "$release_sha"
test -z "$(git tag -l "$tag")"
test -z "$(git ls-remote --tags origin "refs/tags/$tag")"
curl --fail --silent --show-error \
  --header 'User-Agent: scena-release-preflight (https://github.com/johannesPettersson80/scena)' \
  "https://crates.io/api/v1/crates/scena/$version" >/tmp/scena-existing-version.json && exit 1 || test "$?" = 22

git push --set-upstream origin "$release_branch"
test "$(git ls-remote --heads origin "refs/heads/$release_branch" | cut -f1)" = "$release_sha"
git tag -a "$tag" "$release_sha" -m "Release scena $version"
git push origin "refs/tags/$tag"
test "$(git ls-remote --tags origin "refs/tags/$tag^{}" | cut -f1)" = "$release_sha"
```

The tag starts both CI and Release. Discover and record the exact run IDs by immutable SHA:

```bash
gh run list --commit "$release_sha" --limit 20 \
  --json databaseId,workflowName,event,status,conclusion,headSha,url
```

Poll those exact run IDs until every job in both runs is terminal. Do not push a fix while
sibling jobs from the same run are still active. For each poll use concise structured output:

```bash
gh run view <run-id> --json status,conclusion,url,jobs
```

For any failed run:

```bash
scripts/collect_ci_failure_evidence.sh <run-id>
```

Collect every failed run/job, classify all failures, make one consolidated candidate, and
replay only affected local lanes plus one final frozen checkpoint before the next push.

Completion requires all of the following on the same SHA: tag, successful CI, successful
Release workflow, public/latest GitHub release, non-yanked crates.io version, docs.rs result,
and installation of the downloaded public `.crate` with `--features agent`. Verify those
surfaces explicitly:

```bash
gh release view "$tag" --json tagName,isDraft,isPrerelease,isLatest,publishedAt,targetCommitish,url
curl --fail --silent --show-error \
  --header 'User-Agent: scena-release-verification (https://github.com/johannesPettersson80/scena)' \
  "https://crates.io/api/v1/crates/scena/$version" > /tmp/scena-crates-io-version.json
curl --fail --silent --show-error --location \
  "https://docs.rs/scena/$version/scena/" >/dev/null

public_root="$(mktemp -d)"
curl --fail --silent --show-error --location \
  --header 'User-Agent: scena-release-verification (https://github.com/johannesPettersson80/scena)' \
  "https://crates.io/api/v1/crates/scena/$version/download" \
  --output "$public_root/scena-$version.crate"
tar -xzf "$public_root/scena-$version.crate" -C "$public_root"
test -f "$public_root/scena-$version/tests/assets/photo/final/photo_final_policy_v1.json"
mkdir -p "$public_root/install"
CARGO_INSTALL_ROOT="$public_root/install" cargo install \
  --path "$public_root/scena-$version" --features agent --locked
"$public_root/install/bin/scena" --version --format json | tee "$public_root/version.json"
rg -q '"agent"[[:space:]]*:[[:space:]]*true' "$public_root/version.json"
```

Only afterward fast-forward the intended default branch if authorized. When the release
branch is a clean fast-forward of `origin/main`, update and delete it atomically, then monitor
the new main CI to terminal:

```bash
git fetch origin --prune
test "$(git merge-base origin/main "$release_sha")" = "$(git rev-parse origin/main)"
git push --atomic origin "$release_sha:refs/heads/main" ":refs/heads/$release_branch"
gh run list --branch main --commit "$release_sha" --limit 10 \
  --json databaseId,workflowName,status,conclusion,headSha,url
```

After main CI passes, switch the local checkout to `main`, fast-forward, re-verify agent files,
delete the contained local release branch, and remove only the task-scoped builder
checkout/target and generated release artifacts.
