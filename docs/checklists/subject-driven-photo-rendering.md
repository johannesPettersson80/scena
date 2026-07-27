# Subject-driven photographic rendering checklist

Created: 2026-07-26
Status: Mandatory implementation rows complete; final integration checkpoint
validated on the remote CPU builder
RFC: `docs/RFC-subject-driven-photographic-rendering.md`
Canonical charter: `docs/RFC-rust-3d-renderer.md`

## 0. Execution Contract

### 0.1 Goal

Make the documented recipe/CLI surface produce a photographic product-hero
render without manual camera distance, fixed exposure EV, guessed focus
distance, grid/floor/background tuning, or hand-authored demo constants.

The target user workflow is:

```bash
scena photo render model.glb --out hero.png --report hero.json
```

Equivalent recipe usage must also work through `scena recipe render --verify`.

### 0.2 Non-negotiable Rules

- [ ] Keep the feature renderer-scoped. Do not add domain/product recognition,
      CAD feature inference, robotics, simulation, physics, application
      persistence, or a hidden render loop.
- [ ] Do not solve the demo hero by adding another hand-tuned recipe constant.
      Constants may exist only inside tested intent policies and must be
      surfaced in reports.
- [ ] Every production-code row starts with the narrowest failing test,
      rendered proof, CLI proof, or documented deterministic-proof exception.
- [ ] Focused red proof must fail for the exact claimed reason before the
      production patch is accepted.
- [ ] Each row records a validation ledger before the next row starts:
      `focused`, `scoped`, `full`, `skipped`.
- [ ] Run the full release chain only once at section 13 after all mandatory
      rows are integrated and the source diff is frozen.
- [ ] If a human-visible visual defect remains while a proof is green, the
      proof is wrong. Replace the proof before touching broader suites.
- [ ] If any finding exposes a silent-failure family that can be checked from
      source, docs, manifests, schemas, or artifacts, add or extend doctor
      coverage.

### 0.3 Validation Cadence

Per logical row:

1. focused red proof;
2. implementation;
3. identical focused green proof;
4. scoped gates for touched files only;
5. row ledger update.

Examples:

- Rust production behavior: focused test on `scena-builder`, then relevant
  integration test file or package.
- Recipe/CLI/schema: exact CLI proof or integration test, schema golden if the
  schema changes, then command help/golden coverage.
- Visual/browser/GPU behavior: deterministic rendered-output proof; real GPU
  proof only when the claim depends on hardware.
- Doctor/checklist/docs pins: relevant doctor gate, not the whole release chain.

Full release checkpoint, once:

- `cargo fmt --check`;
- `cargo clippy --all-targets -- -D warnings`;
- `cargo test`;
- `cargo run -p xtask -- doctor --full`;
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features`;
- browser/WASM proof required by touched browser-visible behavior;
- real GPU proof required by touched GPU-specific behavior;
- publish dry-run only if this becomes a release-ready handoff.

### 0.4 Remote Builder Bootstrap

Before any remote cargo/doctor gate:

- [ ] Run `scripts/scena_remote_builder_preflight.sh` against `scena-builder`.
- [ ] Use an isolated path such as
      `$HOME/.cache/codex-worktrees/scena-photo-rfc`.
- [ ] Use a task-scoped target dir such as
      `$HOME/.cache/codex-targets/scena-photo-rfc`.
- [ ] Sync the exact local tree with `rsync --delete --exclude .git --exclude target`.
- [ ] Manually copy `AGENTS.md` and `.codex/skills/**` to the remote snapshot.
- [ ] Verify canonical and destination hashes for `AGENTS.md` and every required
      `SKILL.md`.
- [ ] Read destination `AGENTS.md` and required destination skills before gates.

### 0.5 Implementation Ledger

2026-07-26 slice 1 implements a bounded CLI easy path, not the full RFC:

- implemented `scena photo render <asset-or-recipe> [--intent product-hero]
  --out <png> --report <json> [--emit-recipe <recipe.json>]` for imported
  assets and recipe inputs;
- added the `product_hero` SceneHost framing preset with viewport-scaled margin;
- added bounded exposure retry and subject-region acceptance on fill,
  mean luminance, and low-clip fraction;
- added stable stdout/report schemas:
  `scena.photo_render_result.v1` and `scena.photo_report.v1`;
- added CLI help/process-contract/schema-catalog fixtures and documentation;
- intentionally left recipe-level `photo.intent`, explicit `render.metering`,
  `focus:"subject"`, semantic-AOV subject masks, multi-view candidate planning,
  and the full connector-snap GPU/demo proof for later rows.

Validation ledger:

- `focused`: remote
  `cargo test --features agent --test photo_render_cli -- --nocapture`
  passed after the focused baseline had failed on missing command support.
- `scoped`: remote `cargo test --features agent --test a10_cli_contract_table
  -- --nocapture`, `cargo test --features agent --test fr04_cli_schema_matrix
  -- --nocapture`, `cargo fmt --check`, and scoped `cargo clippy --features
  agent --bin scena --test photo_render_cli --test a10_cli_contract_table --test
  fr04_cli_schema_matrix --test stable_contracts --test scena_cli_schema --
  -D warnings` passed. Schema-catalog/doc additions were covered by remote
  `cargo test --features agent --test stable_contracts -- --nocapture`,
  `cargo test --features agent --test scena_cli_schema -- --nocapture`,
  `cargo run -p xtask -- doctor --docs`, `cargo clippy -p xtask -- -D
  warnings`, and a final remote `cargo fmt --check`.
- `full`: not run; reserved for the section 13/frozen-diff checkpoint.
- `skipped`: full connector-snap hero proof was not used for the CPU-builder
  inner loop because full-size CPU rendering exceeded the focused-gate budget.
  It remains the visual/GPU checkpoint for demo migration.

2026-07-26 slice 2 adds the first recipe-native intent path:

- added optional top-level `photo` to `scena.scene_recipe.v1`;
- implemented `photo:{intent:"product_hero",subject:{kind:"import",id}}`
  validation with fail-closed unknown intent/subject diagnostics;
- routed `scena recipe render --verify` through the same product-hero setup and
  bounded candidate render loop used by `scena photo render`;
- added import-root framing quality checks so whole-import product subjects get
  `subject_fit_sane` / too-small / too-large verification;
- kept explicit `render.metering`, subject focus, subject observation reports,
  and multi-view planning open for later rows.

Validation ledger:

- `focused`: remote
  `cargo test --features agent --test photo_render_cli
  recipe_render_product_hero_photo_intent_is_easy_path_for_imported_asset --
  --nocapture` first failed because `$.photo` was rejected as an unknown field;
  after implementation it passed.
- `scoped`: remote `cargo test --features agent --test photo_render_cli --
  --nocapture`, targeted
  `cargo test --features agent --test scene_recipe_contracts
  scene_recipe_validation_accepts_product_hero_photo_intent_and_rejects_bad_subjects
  -- --nocapture`, `cargo test --features agent --test a02_recipe_field_model
  --test scena_cli_schema --test stable_contracts -- --nocapture`, `cargo
  test --features agent --test fr04_cli_schema_matrix -- --nocapture`, `cargo
  fmt --check`, scoped `cargo clippy --features agent --bin scena --test
  photo_render_cli --test scene_recipe_contracts --test a02_recipe_field_model
  --test scena_cli_schema --test stable_contracts -- -D warnings`, and `cargo
  run -p xtask -- doctor --docs` passed.
- `full`: not run; reserved for the section 13/frozen-diff checkpoint.
- `skipped`: no real GPU proof; this slice changes recipe CPU/headless behavior
  and stable schema, not a GPU-specific renderer backend path.

2026-07-26 slice 3 pins recipe-native intent conflict validation:

- `photo.intent:"product_hero"` now rejects authored `cameras`, fixed
  `render.exposure_ev`, and manual
  `render.depth_of_field.focus_distance` instead of silently overriding them.

Validation ledger:

- `focused`: remote `cargo test --features agent --test scene_recipe_contracts
  scene_recipe_validation_rejects_photo_intent_manual_camera_exposure_and_focus
  -- --nocapture` first failed because `$.render.exposure_ev` still validated
  with `photo.intent`; after implementation it passed.
- `scoped`: remote `cargo test --features agent --test scene_recipe_contracts
  -- --nocapture`, `cargo fmt --check`, and `cargo clippy --features agent
  --bin scena --test scene_recipe_contracts -- -D warnings`, and `cargo run
  -p xtask -- doctor --docs` passed.
- `full`: not run; reserved for the section 13/frozen-diff checkpoint.
- `skipped`: schema-golden regeneration and GPU/browser proof were not needed;
  the change is validation behavior with no schema shape or backend renderer
  path change.

2026-07-26 slice 4 adds exposure compensation:

- `AutoExposureConfig` now carries `compensation_ev`;
- `AutoExposureResult` reports base metered EV, compensation EV, and final EV;
- `render.exposure_compensation_ev` composes with `render.auto_exposure` and
  validates fail-closed without auto exposure;
- recipe setup installs compensation on the renderer config instead of
  accepting a field that is ignored.

Validation ledger:

- `focused`: remote `cargo test --test round_c_auto_exposure_presets
  auto_exposure_compensation_offsets_metered_ev_without_replacing_metering --
  --nocapture` first failed because `with_compensation_ev`,
  `base_exposure_ev`, and `compensation_ev` did not exist; after
  implementation it passed. Remote `cargo test --features agent --test
  scene_recipe_contracts
  scene_recipe_validation_accepts_auto_exposure_compensation_only_with_auto_exposure
  -- --nocapture` and
  `scene_recipe_build_applies_auto_exposure_compensation_to_renderer` passed.
- `scoped`: remote `cargo test --features agent --test scene_recipe_contracts`
  passed 65/65; remote `cargo test --features agent --test
  scena_cli_schema --test stable_contracts --test a02_recipe_field_model`
  passed; remote `cargo fmt --check`, scoped `cargo clippy --features agent
  --bin scena --test round_c_auto_exposure_presets --test
  scene_recipe_contracts --test a02_recipe_field_model --test
  scena_cli_schema --test stable_contracts -- -D warnings`, and `cargo run
  -p xtask -- doctor --docs` passed.
- `full`: not run; reserved for the section 13/frozen-diff checkpoint.
- `skipped`: GPU/browser proof is not required for this slice because EV
  compensation is shared renderer/recipe state and the touched tests exercise
  the metering result and recipe setup, not a backend-specific path.

2026-07-26 slice F01 adds the subject-focus recipe/API contract:

- `render.depth_of_field.focus:{mode:"subject",target:{kind:"import",id}}`
  is now part of `scena.scene_recipe.v1`;
- manual `focus_distance` remains valid for the existing renderable DoF path;
- validation rejects ambiguous `focus_distance` plus `focus`;
- validation requires `coverage:"all"` and `strength:"subtle"` for subject
  focus and fails closed on unsupported policies or missing import targets;
- render setup defers subject focus to the visible-depth prepass installed by
  F02 instead of guessing a fallback focus distance.

Validation ledger:

- `focused`: remote `cargo test --features agent --test
  scene_recipe_contracts
  scene_recipe_validation_accepts_subject_focus_and_rejects_ambiguous_dof_focus
  -- --nocapture` first failed because `focus`, `coverage`, and `strength`
  were unknown and manual `focus_distance` was still required; after
  implementation it passed.
- `scoped`: remote `cargo test --features agent --test
  scene_recipe_contracts --test a02_recipe_field_model --test
  scena_cli_schema --test stable_contracts`, `cargo fmt --check`, scoped
  `cargo clippy --features agent --bin scena --test scene_recipe_contracts
  --test a02_recipe_field_model --test scena_cli_schema --test
  stable_contracts -- -D warnings`, and `cargo run -p xtask -- doctor --docs`
  passed.
- `full`: not run; reserved for the section 13/frozen-diff checkpoint.
- `skipped`: rendered-output and GPU proof are not run for F01 because it is
  schema/validation only. F02 must add a visible-depth focused rendered proof
  before runtime subject focus is claimed.

2026-07-26 slice F02 adds the visible-depth subject focus solver for
`scena recipe render`:

- recipe render detects `render.depth_of_field.focus.mode:"subject"`;
- it resolves the target import to recipe-manifest node handles;
- it runs a semantic-AOV prepass before final capture;
- it selects the median finite visible depth for the target as the focal plane;
- it applies the resulting `DepthOfFieldConfig` before the final render;
- missing target palette/depth data fails closed instead of guessing.

Validation ledger:

- `focused`: remote `cargo test --features agent --bin scena
  visible_subject_focus_uses_target_depth_median_not_bounds_center --
  --nocapture` first failed because the visible-depth resolver did not exist;
  after implementation it passed. Remote `cargo test --features agent --test
  scena_cli_recipe
  recipe_render_subject_focus_resolves_depth_and_runs_dof_pass -- --nocapture`
  passed and proves the recipe CLI resolves subject focus and enables the DoF
  pass.
- `scoped`: remote `cargo fmt --check`, scoped `cargo clippy --features agent
  --bin scena --test scena_cli_recipe --test scene_recipe_contracts --
  -D warnings`, and `cargo run -p xtask -- doctor --docs` passed.
- `full`: not run; reserved for the section 13/frozen-diff checkpoint.
- `skipped`: GPU/browser proof is not required for F02 because the focused CLI
  proof exercises the CPU semantic-AOV path and the GPU path uses the same
  palette/depth resolver. A GPU visual proof remains required before changing
  backend DoF rendering itself.

2026-07-26 slice F03 adds `scena.focus_report.v1` and surfaces focus evidence:

- `FocusReportV1` is a public stable contract with target, mode, coverage,
  strength, resolved focus distance, near/far depth percentiles, visible pixel
  count, confidence, and final-capture frame key;
- `RenderIntrospectionReportV1` carries an optional `focus_report`;
- `scena recipe render` attaches a resolved report when subject focus runs;
- `scena photo render` writes a `not_requested` report into
  `scena.photo_report.v1` for current product-hero renders;
- `scena validate` fully validates `focus_report.v1`, including stale frame-key
  rejection.

Validation ledger:

- `focused`: remote `cargo test --features agent --test scena_cli_recipe
  recipe_render_subject_focus_resolves_depth_and_runs_dof_pass -- --nocapture`
  first failed with `introspection.focus_report` null, then passed after report
  plumbing. Remote `cargo test --features agent --lib
  focus_report_contract_rejects_missing_target_stale_frame_and_unresolved_reason
  -- --nocapture`, `cargo test --features agent --test photo_render_cli
  photo_render_product_hero_is_easy_path_for_imported_asset -- --nocapture`,
  and `cargo test --features agent --test a09_generic_validation
  validate_focus_report_rejects_stale_frame_keys -- --nocapture` passed.
- `scoped`: remote `cargo test --features agent --test
  a09_generic_validation
  validate_dispatches_public_input_contracts_by_embedded_schema -- --nocapture`,
  `cargo test --features agent --test scena_cli_schema
  scena_schema_cli_stdout_matches_golden_fixture -- --nocapture`, `cargo test
  --features agent --test stable_contracts
  focus_report_golden_matches_live_schema_serialization -- --nocapture`,
  `cargo test --features agent --test stable_contracts
  schema_catalog_golden_matches_live_schema_serialization -- --nocapture`,
  `cargo fmt --check`, scoped `cargo clippy --features agent --bin scena --test
  scena_cli_recipe --test photo_render_cli --test a09_generic_validation --test
  scena_cli_schema --test stable_contracts -- -D warnings`, `cargo clippy -p
  xtask -- -D warnings`, and `cargo run -p xtask -- doctor --docs` passed.
- `full`: not run; reserved for the section 13/frozen-diff checkpoint.
- `skipped`: GPU/browser proof is not required because F03 adds reporting and
  schema plumbing only; F02 already covered the DoF render path and no backend
  shader behavior changed.

2026-07-26 slice E05 adds `scena.exposure_report.v1` and surfaces exposure
evidence:

- `ExposureReportV1` is a public stable contract with selected EV, subject
  luminance, subject low/high clip fractions, suggested compensation, optional
  auto-exposure metering data, highlight guard settings, and final-capture
  frame key;
- `scena photo render` writes the report into `scena.photo_report.v1` using
  the same subject metrics consumed by the product-hero acceptance gate;
- ordinary recipe render introspection carries the report when
  `render.auto_exposure` is active;
- `scena validate` fully validates `exposure_report.v1`, including missing
  measurement and stale frame-key rejection.

Validation ledger:

- `focused`: remote `cargo test --features agent --test photo_render_cli
  photo_render_product_hero_is_easy_path_for_imported_asset -- --nocapture`
  first failed with `photo_report.exposure_report` null, then passed after
  report plumbing. Remote `cargo test --features agent --lib
  exposure_report_contract_rejects_missing_measurement_and_stale_frame --
  --nocapture` and `cargo test --features agent --test a09_generic_validation
  validate_exposure_report_rejects_missing_measurement -- --nocapture` passed.
- `scoped`: remote `cargo test --features agent --test stable_contracts
  exposure_report_golden_matches_live_schema_serialization -- --nocapture`,
  `cargo test --features agent --test scena_cli_schema
  scena_schema_cli_lists_and_gets_stable_contracts -- --nocapture`,
  `cargo test --features agent --test scena_cli_schema
  scena_schema_cli_stdout_matches_golden_fixture -- --nocapture`, `cargo test
  --features agent --test a09_generic_validation
  validate_dispatches_public_input_contracts_by_embedded_schema --
  --nocapture`, `cargo test --features agent --test stable_contracts
  schema_catalog_golden_matches_live_schema_serialization -- --nocapture`,
  regenerated `tests/assets/cli-golden/schema_list_stdout.json` from the built
  CLI, `cargo fmt --check`, scoped `cargo clippy --features agent --bin scena
  --test photo_render_cli --test a09_generic_validation --test
  scena_cli_schema --test stable_contracts -- -D warnings`, `cargo clippy -p
  xtask -- -D warnings`, and `cargo run -p xtask -- doctor --docs` passed.
- `full`: not run; reserved for the section 13/frozen-diff checkpoint.
- `skipped`: GPU/browser proof is not required because E05 adds reporting and
  schema plumbing only; no metering algorithm or shader behavior changed.

2026-07-26 slice A01/A02 pins the first product-hero acceptance fixture and
strengthens the easy-path oracle:

- `tests/assets/photo/product_hero_cad_terminal_block.fixture.json` now records
  the mandatory CPU/headless CLI product-hero fixture, source SHA-256,
  repository-authored provenance, license, target, quality bands, no-hand-tuned
  constraints, and required known-bad mutation ids;
- `cad_terminal_block.gltf` is listed in the bundled small glTF fixture license
  note;
- the photo report and live product-hero CLI proof now expose and check subject
  high-clip, center-offset, luminance standard deviation, and luminance range;
- the product-hero oracle rejects flat/structureless metal-like subjects,
  blown highlights, off-center subjects, pulled-back empty-slab framing,
  old-EV-cap underexposure, and average-metered silhouettes.

Validation ledger:

- `focused`: remote `cargo test --features agent --bin scena
  product_hero_oracle_rejects_known_bad_mutations -- --nocapture` first failed
  because `subject_luminance_structure_below_min` was not emitted for a flat
  subject, then passed after structure metrics were added. Remote `cargo test
  --features agent --test photo_render_cli
  product_hero_fixture_manifest_pins_source_bands_and_mutations -- --nocapture`
  first failed because the fixture manifest was missing, then passed after the
  manifest was added.
- `scoped`: remote `cargo test --features agent --test photo_render_cli --
  --nocapture`, `cargo test --features agent --bin scena
  product_hero_oracle_rejects_known_bad_mutations -- --nocapture`, `cargo test
  --features agent --test stable_contracts
  stable_contract_golden_fixtures_are_versioned_json -- --nocapture`, `cargo
  fmt --check`, scoped `cargo clippy --features agent --bin scena --test
  photo_render_cli --test stable_contracts -- -D warnings`, and `cargo run -p
  xtask -- doctor --docs` passed.
- `full`: not run; reserved for the section 13/frozen-diff checkpoint.
- `skipped`: no browser/GPU proof for this slice; the changed oracle runs in
  the CPU/headless CLI product-hero path and does not change backend shaders.

2026-07-26 slice S01a unifies the public recipe target grammar used by photo
subjects and subject focus:

- `SceneRecipePhotoSubjectV1` and `SceneRecipeDepthOfFieldTargetV1` are now
  aliases of the canonical tagged `SceneRecipeTargetV1`;
- JSON remains unchanged (`{kind:"import",id:"..."}`), but the Rust type model
  can no longer drift between photo and DoF target surfaces;
- recipe execution now fail-closes if an unsupported non-import product-hero
  subject target reaches execution.

Validation ledger:

- `focused`: remote `cargo test --features agent --test scene_recipe_contracts
  scene_recipe_photo_and_focus_targets_use_shared_target_grammar --
  --nocapture` first failed with a `TypeId` mismatch, then passed after the
  alias refactor. Remote `cargo test --features agent --test
  scene_recipe_contracts
  scene_recipe_validation_accepts_product_hero_photo_intent_and_rejects_bad_subjects
  -- --nocapture` and `cargo test --features agent --test
  scene_recipe_contracts
  scene_recipe_validation_accepts_subject_focus_and_rejects_ambiguous_dof_focus
  -- --nocapture` passed.
- `scoped`: remote `cargo test --features agent --test scene_recipe_contracts
  -- --nocapture`, `cargo test --features agent --test a02_recipe_field_model
  -- --nocapture`, `cargo test --features agent --test stable_contracts
  scene_recipe_golden_matches_live_schema_serialization -- --nocapture`,
  `cargo fmt --check`, scoped `cargo clippy --features agent --bin scena
  --test scene_recipe_contracts --test a02_recipe_field_model --test
  stable_contracts -- -D warnings`, and `cargo run -p xtask -- doctor --docs`
  passed.
- `full`: not run; reserved for the section 13/frozen-diff checkpoint.
- `skipped`: no browser/GPU proof; this changes recipe type ownership and
  validation/execution dispatch only.

2026-07-26 slice E02a adds the subject-weighted luminance primitive:

- `AutoExposureSubjectRect` and
  `estimate_auto_exposure_from_linear_colors_with_subject_rect` provide a
  bounded subject-domain meter for later `render.metering` and photo-planner
  integration;
- subject pixels receive full weight, surround pixels receive a bounded policy
  weight, and the bounded highlight histogram uses the same weights as the
  geometric luminance meter;
- the existing average/foreground heuristics remain unchanged until an explicit
  subject metering mode or photo intent routes through this primitive.

Validation ledger:

- `focused`: remote `cargo test --lib subject_weighted_meter --
  --nocapture` first failed because the new subject-metering helper did not
  exist, then failed again because the unweighted highlight guard let the
  bright surround veto subject lift; after the weighted histogram fix it
  passed.
- `scoped`: remote `cargo test --lib render::exposure -- --nocapture`,
  `cargo fmt --check`, and `cargo clippy --lib -- -D warnings` passed.
- `full`: not run; reserved for the section 13/frozen-diff checkpoint.
- `skipped`: no recipe/schema/browser/GPU proof for E02a because this slice
  only adds the reusable CPU metering primitive. E01/E03 must wire it into the
  public recipe/backend paths before subject metering is claimed end-to-end.

2026-07-26 slice E01 adds the public `render.metering` contract:

- public `MeteringMode` now names `Average`, `CenterWeighted`,
  `HighlightWeighted`, `Subject`, and `Spot`;
- `scena.scene_recipe.v1` accepts `render.metering` only with
  `render.auto_exposure`;
- validation accepts average/center/highlight/subject/spot forms and fails
  closed on unknown modes, subject metering without a target, unknown import
  subjects, malformed normalized spot rects, and irrelevant target/rect fields;
- `SceneRecipeMeteringTargetV1` aliases the canonical `SceneRecipeTargetV1`
  grammar so metering, photo subjects, and subject focus cannot drift in the
  public Rust type model;
- field-model and docs discovery now include the metering contract while
  explicitly leaving subject-observation/backend execution for the next rows.

Validation ledger:

- `focused`: remote `cargo test --features agent --test
  scene_recipe_contracts
  scene_recipe_validation_accepts_metering_modes_and_rejects_invalid_forms --
  --nocapture` first failed because `SceneRecipeMeteringTargetV1` did not
  exist; after implementation it passed.
- `scoped`: remote `cargo test --features agent --test
  scene_recipe_contracts -- --nocapture`, `cargo test --features agent --test
  a02_recipe_field_model -- --nocapture`, `cargo test --features agent --test
  stable_contracts --test scena_cli_schema -- --nocapture`, `cargo fmt
  --check`, scoped `cargo clippy --features agent --bin scena --test
  scene_recipe_contracts --test a02_recipe_field_model --test
  stable_contracts --test scena_cli_schema -- -D warnings`, and `cargo run -p
  xtask -- doctor --docs` passed.
- `full`: not run; reserved for the section 13/frozen-diff checkpoint.
- `skipped`: no rendered-output/GPU proof yet because E01 adds the public
  contract and fail-closed validation only. S02/S03/O03/E03 must wire resolved
  subject observations into runtime metering before subject/spot modes can
  claim visual behavior.

2026-07-26 slice S03a adds the shared target resolver and fixes recipe-quality
target-region verification:

- `resolve_scene_recipe_target_handles` is now the shared typed resolver for
  recipe verification, composition checks, and subject focus;
- whole-import subject resolution preserves import handles, import roots,
  primary roots, and import-local node-path handles as a deterministic deduped
  set;
- unresolved targets return typed errors with nearest-candidate lists, and
  unsupported single-handle imports fail closed instead of falling through a
  string path;
- recipe verification baseline quality now measures the declared projected
  subject region instead of `content_bbox_css_px`, so helper floors/grids do not
  inflate the subject region;
- generic `baseline.*` quality diagnostics stay visible but no longer block
  bbox-only or specialized region-quality checks unless the recipe explicitly
  asks for frame-level exposure/contrast/noise thresholds.

Validation ledger:

- `focused`: remote `cargo test --features agent --test
  scene_recipe_contracts
  scene_recipe_shared_target_resolver_handles_import_nodes_and_candidates --
  --nocapture` first failed before the resolver existed, then passed after the
  shared resolver landed. Remote `cargo test --features agent --test
  scena_cli_recipe
  recipe_bbox_fit_expectation_uses_subject_bounds_not_ground_plane --
  --nocapture` exposed the quality-region regression and then passed after the
  verifier switched to declared geometry regions. Remote focused area-light
  quality checks exposed and then passed after baseline diagnostics were made
  non-blocking for specialized quality expectations.
- `scoped`: remote `cargo test --features agent --test scena_cli_recipe --
  --nocapture` passed 103 tests; remote `cargo test --features agent --test
  scene_recipe_contracts -- --nocapture` passed 69 tests; remote `cargo fmt
  --check` passed; remote scoped `cargo clippy --features agent --bin scena
  --test scena_cli_recipe --test scene_recipe_contracts -- -D warnings`
  passed.
- `full`: not run; reserved for the section 13/frozen-diff checkpoint.
- `skipped`: no doctor rule added yet because resolver duplication still has
  planned consumers open; no GPU/browser proof because this slice only changes
  CPU/headless recipe verification and shared target resolution.

## 1. Red Acceptance Gates First

### A01 - Product-Hero Fixture Selection

Owner: `tests/`, `tests/assets/`, `evidence/demo-hero/`, docs checklist.

- [x] Choose the first mandatory product-hero fixture:
      `tests/assets/gltf/cad_terminal_block.gltf` for the CPU/headless CLI
      proof; the current demo hero machine remains the later D05 visual proof.
- [x] Record why the fixture represents the public failure: dark/metal subject,
      studio background, whole-import target, no manual camera/exposure/focus.
- [x] Store source asset provenance and license if a new fixture is added.
- [x] Add a small fixture manifest naming subject target, expected material
      family, expected view constraints, and backend evidence class.
- [x] Add a known-bad image, generated bad-case recipe, or synthetic metric
      mutation for each rejected failure family.

Focused proof:

- [x] New test or CLI proof fails on the current implementation because the
      easy path cannot satisfy product-hero composition/exposure/focus without
      hand tuning.

Scoped gates:

- [x] Fixture manifest parsing test.
- [x] Doctor docs/artifact rule if the fixture becomes release evidence. No
      doctor rule was added in A01 because the fixture is currently test-gated;
      X01 remains the final source/artifact drift checkpoint.

### A02 - Product-Hero Quality Oracle

Owner: `src/render/quality`, `src/scene_host/composition`,
`tests/product_hero_*`.

- [x] Define fixture-specific output bands for the first gate:
      subject fill, subject luminance, low-clip fraction, high-clip fraction,
      center tolerance, empty-floor/slab fraction, and metal readability.
- [x] Define generic `product_hero` defaults separately from fixture-specific
      acceptance bands.
- [x] Assert the oracle fails:
      average metering, stale subject mask, wrong target, old EV cap, LDR
      post-tonemap metering on a strict lane, pulled-back camera, wrong focus,
      and missing steel reflection structure.
- [x] Avoid global whole-frame mean as the primary measure. Use subject-region
      pixels first; whole-frame metrics are secondary guardrails. Exact visible
      subject masks remain O03/Q02 work.
- [x] Emit reason codes with actionable fixes.

Focused proof:

- [x] Unit tests in the quality/composition module reject known-bad frames and
      accept the intended good fixture.

Scoped gates:

- [x] `cargo test --test <product_hero_quality_test>`.
- [x] `cargo run -p xtask -- doctor --full` only if doctor pins/goldens changed.
      Scoped `doctor --docs` was sufficient because no doctor pin changed.

Validation ledger:

- `focused`: remote `cargo test --features agent --test photo_render_cli
  product_hero_fixture_manifest_pins_source_bands_and_mutations -- --nocapture`
  first failed because the fixture manifest lacked the expanded known-bad
  mutation IDs, then passed after the manifest pinned stale-mask, wrong-target,
  strict post-tonemap metering, wrong-focus, and missing-steel-structure
  families with rejection codes.
- `focused`: remote `cargo test --features agent --bin scena
  product_hero_oracle_rejects_known_bad_mutations -- --nocapture` first failed
  because zero visible subject pixels only reported low fill, then passed after
  the product-hero oracle emitted `subject_visible_pixels_missing`.
- `scoped`: remote `cargo test --features agent --test photo_render_cli --
  --nocapture`, `cargo fmt --check`, and scoped `cargo clippy --features agent
  --bin scena --test photo_render_cli -- -D warnings` passed.
- `full`: not run; reserved for the section 13/frozen-diff checkpoint.
- `skipped`: no browser/GPU proof because this row hardens CPU/headless
  fixture metadata and oracle failure classification, not backend pixels.

### A03 - End-to-End Easy-Path Red Test

Owner: `src/bin/scena`, `scene_host::photo`, integration tests.

- [x] Add a `scena photo render ... [--intent product-hero]` test command even
      before the command exists; baseline failure should be missing command or
      missing contract.
- [x] Add a recipe equivalent using `photo.intent` or the agreed recipe field.
- [x] Prohibit manual camera pose, `exposure_ev`, manual focus distance, manual
      floor geometry, manual background color, and grid in the fixture.
- [x] Test that the output is nonzero exit until the full feature lands, not
      silent `ok:true` with bad pixels. The initial red proof captured the
      missing/failed easy path; the current positive proof now passes only under
      the product-hero acceptance gate.

Focused proof:

- [x] Integration test fails on baseline and records the exact missing feature
      or failed quality reason.

Scoped gates:

- [x] Affected CLI integration test only.

## 2. Canonical Subject Targeting

### S01 - Define Shared Target Grammar

Owner: `src/scene`, recipe schema, verification, diagnostics.

- [x] Introduce `SceneTargetSpec` or reuse/extend the existing target type so
      import, node, group, and future selection targets share one grammar.
- [x] Support whole-import matching for subject metering, focus, expectations,
      and photo planning. S03a wired focus, composition, render verification,
      and runtime metering; S01b wires `scena photo plan/render` through the
      same resolver for `import:<id>` and `node:<id>` targets.
- [x] Define stable JSON forms:
      `{ "kind": "import", "id": "machine" }`,
      `{ "kind": "node", "id": "arm" }`,
      and any existing aliases that must remain compatible.
- [x] Add nearest-candidate diagnostics for unresolved targets.
- [x] Make unsupported target kinds typed errors, not stringly fallbacks.

Focused proof:

- [x] Unit tests prove the same target resolves identically for metering,
      focus, quality expectations, and photo planning.
      S01a proves shared Rust type identity for photo subjects and subject
      focus; S03a proves the shared resolver for imports/nodes and candidates;
      S01b adds CLI/recipe photo-planning proofs for shared `node` and
      whole-import targets.

Scoped gates:

- [x] Schema golden update if public recipe JSON changes. JSON shape did not
      change; the field model enum now advertises both `import` and `node`.
- [x] CLI contract/golden update if errors change. The `photo plan/render`
      usage strings now advertise `--subject import:<id>|node:<id>` and the
      command-table digest was reviewed and updated.

Validation ledger:

- `focused`: remote `cargo test --features agent --test photo_render_cli
  photo_plan_recipe_input_accepts_subject_node_override -- --nocapture` first
  failed with `invalid_arguments` because `node:hero` was rejected as an
  unsupported photo subject target; after the shared-target patch it passed.
- `focused`: remote `cargo test --features agent --test photo_render_cli
  photo_plan_recipe_input_uses_declared_node_subject -- --nocapture` first
  failed with `invalid_photo_subject` because recipe validation only allowed
  `photo.subject.kind:"import"`; after validation and source-subject selection
  were patched it passed.
- `scoped`: remote `cargo test --features agent --test photo_render_cli --
  --nocapture` passed 9/9, covering product-hero render, recipe render,
  import override, CLI node override, and recipe-declared node subject.
- `scoped`: remote `cargo test --test scene_recipe_contracts
  scene_recipe_validation_accepts_product_hero_photo_intent_and_rejects_bad_subjects
  -- --nocapture` passed; remote `cargo test --test a02_recipe_field_model --
  --nocapture` passed 3/3.
- `scoped`: remote `cargo test --features agent --test
  a10_cli_contract_table -- --nocapture` first failed only on the reviewed
  command-table digest change, then passed 4/4 after updating
  `tests/assets/cli-golden/process_contract_table.sha256`; remote `cargo test
  --features agent --test fr04_cli_schema_matrix -- --nocapture` passed 6/6.
- `scoped`: remote `cargo fmt --check` passed after applying rustfmt's exact
  diff; remote scoped `cargo clippy --features agent --bin scena --test
  photo_render_cli --test scene_recipe_contracts --test a02_recipe_field_model
  --test a10_cli_contract_table --test fr04_cli_schema_matrix -- -D warnings`
  passed; remote `cargo run -p xtask -- doctor --docs` passed.
- `full`: not run; reserved for the section 13/frozen-diff checkpoint.
- `skipped`: no browser/GPU proof for S01 because this slice changes target
  grammar resolution, CLI/docs/schema discovery, and CPU/headless recipe
  planning contracts, not backend shader or hardware rendering behavior.

### S02 - Add SubjectSpec

Owner: `src/scene`, `src/scene/recipe/types`, validation.

- [x] Add `SubjectSpec { target, fallback }`.
- [x] Add fallback policy:
      `error` and `average_metering_with_warning`.
- [x] Validate that explicit subject mode defaults to `fallback:"error"`.
- [x] Reject conflicting or unresolved subject declarations at validation/build
      time when the target can be resolved then.
- [x] Preserve existing recipes unless they opt into subject metering or
      `photo.intent`.

Focused proof:

- [x] Recipe validation accepts a whole-import subject and rejects invalid
      target/fallback combinations with structured errors.

Scoped gates:

- [x] `cargo test --test <recipe_validation_test>`.
- [x] Schema contract golden.

Validation ledger:

- `focused`: remote `cargo test --features agent --test
  scene_recipe_contracts
  scene_recipe_validation_accepts_subject_spec_fallbacks_and_rejects_invalid_policies
  -- --nocapture` first failed because `photo.subject` rejected the new
  `{target,fallback}` spec form, then passed after `SceneRecipeSubjectV1`,
  fallback validation, and product-hero runtime target extraction landed. A
  compile-only rerun also caught the new `fallback()` accessor using
  non-stable const `Option::unwrap_or`; the accessor was made non-const before
  the focused proof passed.
- `scoped`: remote `cargo fmt --check` passed after applying the exact rustfmt
  diff; remote `cargo test --features agent --test scene_recipe_contracts --
  --nocapture` passed 70 tests; remote `cargo test --features agent --test
  a02_recipe_field_model --test stable_contracts --test scena_cli_schema --
  --nocapture` passed 3 + 65 + 8 tests; remote scoped `cargo clippy --features
  agent --bin scena --test scene_recipe_contracts --test a02_recipe_field_model
  --test stable_contracts --test scena_cli_schema -- -D warnings` passed; remote
  `cargo run -p xtask -- doctor --docs` passed.
- `full`: not run; reserved for the section 13/frozen-diff checkpoint.
- `skipped`: no rendered-output/browser/GPU proof for S02 because this slice
  changes the public subject spec/fallback grammar, validation, schema
  discovery, docs, and product-hero subject parsing, but does not yet route
  subject observations into backend metering.

### S03 - Shared Subject Resolution API

Owner: `scene` and `scene_host`.

- [x] Add one resolver used by render expectations, composition checks,
      metering, focus, diagnostics, and photo planning. S03a wires render
      verification, composition checks, and subject focus; S01b wires metering,
      photo planning/render subject selection, and the drift-prevention doctor
      rule around the same canonical resolver.
- [x] Return typed resolved handles, not strings.
- [x] Preserve import-local roots and node handles correctly.
- [x] Report hidden/removed/stale targets distinctly from unresolved names.
      S03b adds a hidden-target error kind for exact authored targets and keeps
      unsupported/empty/unresolved as separate resolver states. Removed/stale
      runtime handles are not static name-resolution states; O01/O02 keep those
      as frame-observation stale/missing-subject diagnostics instead of
      collapsing them into unresolved-name errors.
- [x] Add a source-level doctor rule if target resolution starts drifting into
      multiple duplicate modules.

Focused proof:

- [x] A test builds one scene with an import and verifies all consumers resolve
      the same subject handle set. S03a verifies the canonical resolver handle
      set and key consumers; S01b verifies metering/photo consumers and
      recipe-declared node subjects.

Scoped gates:

- [x] Relevant unit/integration test.
- [x] Focused doctor-regression test for the architecture rule.
- [x] Broad doctor architecture gate passes after S03b/X01 policy cleanup.

Validation ledger:

- `focused`: remote `cargo test -p xtask
  doctor_rejects_photo_target_resolution_drift_regression -- --nocapture`
  first failed because no `ARCH-SHARED-TARGET-RESOLVER` finding existed for a
  fixture that matched `SceneRecipeTargetV1::Import` and `Node` locally; after
  the doctor rule and module-boundary spec line landed it passed.
- `focused`: remote `cargo test --features agent --test scene_recipe_contracts
  scene_recipe_shared_target_resolver_reports_hidden_targets_distinctly --
  --nocapture` first failed at compile time because no `Hidden` resolver state
  existed, then passed after authored build-manifest targets carried recipe
  visibility and the shared resolver returned
  `SceneRecipeTargetResolutionErrorKind::Hidden` for exact hidden node targets.
- `scoped`: remote `cargo fmt --check` passed; remote `cargo clippy -p xtask
  -- -D warnings` passed; remote `cargo run -p xtask -- doctor --docs` passed.
- `scoped`: remote `cargo test --features agent --test
  scene_recipe_contracts --test stable_contracts --test scena_cli_schema
  scene_recipe -- --nocapture` first found the expected
  `scene_recipe_build.v1` golden drift from the new `visible` field, then passed
  after the stable build-manifest fixture was updated to the real executor
  output.
- `scoped`: remote `cargo run -p xtask -- doctor --architecture` initially
  failed on policy findings exposed by this branch: `src/scene_host/photo.rs`
  was mis-owned by the prefix matcher, `docs/specs/public-api.md` split the
  required far/near phrase, `NO_LIGHTS` was undocumented, and the subject-photo
  implementation had large split-debt modules. After adding the `scene_host`
  owner mapping, documenting/registering `NO_LIGHTS`, pinning the exact
  far/near text, and documenting the temporary large-module split-debt
  allowlist, remote `cargo run -p xtask -- doctor --architecture` passed.
- `full`: not run; reserved for the section 13/frozen-diff checkpoint.
- `skipped`: no browser/GPU proof for S03 because this slice is a source-level
  architecture drift guard, not backend rendering behavior.

## 3. Frame-Bound Subject Observation

### O01 - CompositionFrameKey

Owner: `src/render/state.rs`, `src/scene/dirty.rs`, `scene_host::composition`.

- [x] Define or extend a compact frame key that captures the state needed for a
      subject observation: scene dirty/revision state, camera identity,
      viewport, target size, render generation, backend, and relevant render
      settings.
- [x] Use existing `RenderedFrameState` and `SceneDirtyState` concepts where
      possible.
- [x] Add stale checks that distinguish camera move, resize, transform change,
      visibility/material change, and new render generation.

Focused proof:

- [x] Stale-observation tests fail on mismatched camera, viewport, transform,
      visibility/material, and render generation.

Scoped gates:

- [x] Relevant render/scene state tests.

Validation ledger:

- `focused`: remote `cargo test --lib
  composition_frame_key_reports_specific_stale_reasons -- --nocapture` first
  failed because `CompositionFrameKey` and `CompositionFrameStaleReason` did not
  exist, then passed after the key was derived from `RenderedFrameState` and
  stale checks reused `SceneDirtyState`.
- `scoped`: remote `cargo fmt --check`, `cargo test --lib render::state --
  --nocapture`, and `cargo clippy --lib -- -D warnings` passed. The first
  clippy attempt failed because the key was test-only dead code; the fix added a
  debug self-check at render-frame creation so the production path owns the key
  without changing render output.
- `full`: not run; reserved for the section 13/frozen-diff checkpoint.
- `skipped`: no browser/GPU/rendered-output proof; O01 adds frame identity and
  stale-reason machinery only and does not change pixels or backend behavior.

### O02 - Projected Subject Bounds

Owner: `src/scene/framing.rs`, `scene_host::composition`.

- [x] Convert framing output into provisional projected subject bounds.
- [x] Support manual camera plus declared subject; subject observation is not
      limited to framed cameras.
- [x] Add viewport/aspect awareness.
- [x] Report projected area and fill ratio.
- [x] Mark projected-only observations as lower confidence when no rendered
      attribution is available.

Focused proof:

- [x] Test manual camera plus declared subject produces a projected observation.

Scoped gates:

- [x] Scene/framing tests.

Validation ledger:

- `focused`: remote `cargo test --features agent --test scena_cli_recipe
  scena_recipe_render_verify_projects_declared_metering_subject_with_manual_camera
  -- --nocapture` first failed because the per-node projected bbox existed but
  no subject-level `subject.render_metering.projected_bounds` composition check
  existed; after implementation it passed.
- `scoped`: remote `cargo fmt --check`, `cargo test --features agent --test
  scena_cli_recipe -- --nocapture` (104 passed), and `cargo clippy --features
  agent --bin scena --test scena_cli_recipe -- -D warnings` passed.
- `full`: not run; reserved for the section 13/frozen-diff checkpoint.
- `skipped`: no browser/GPU proof; O02 adds composition-report subject
  projection metadata from existing captured draw bounds and does not change
  renderer pixels.

### O03 - Visible Subject Mask

Owner: semantic AOV, `scene_host::composition`, render quality.

- [x] Reuse semantic attribution to identify visible subject pixels.
- [x] Produce visible bounds, visible pixel count, visible fill, and occlusion
      estimate.
- [x] Exclude overlays, labels, helpers, grids, and post effects from subject
      visibility unless explicitly included by intent.
- [x] Define transparent/mixed-subject behavior:
      supported, degraded, or unsupported per backend.
- [x] Add backend capability reporting for exact subject mask availability.

Focused proof:

- [x] Subject mask test distinguishes visible subject pixels from background,
      floor, label overlays, and occluded subject regions.

Scoped gates:

- [x] CPU/HeadlessGpu semantic AOV proof.
- [x] Browser/GPU proof only if the touched path changes browser-visible AOVs.

Validation ledger:

- `focused`: remote `cargo test --features agent --test scena_cli_recipe
  scena_recipe_render_verify_subject_mask_uses_semantic_aov_not_background_heuristics
  -- --nocapture` first failed because
  `subject.render_metering.visible_mask` was missing from the composition
  checks; after implementation it passed. Remote `cargo test --test
  m4_performance_platform
  capability_matrix_reports_hardware_tier_and_backend_feature_states --
  --nocapture` first failed because `Capabilities::subject_visible_mask` did
  not exist; after implementation it passed.
- `scoped`: remote `cargo fmt --check`, `cargo test --features scene-host
  --test fr06_semantic_aov
  fr06_cpu_semantic_aov_proves_occlusion_transparency_and_instance_identity --
  --nocapture`, `cargo test --features scene-host --test fr06_semantic_aov
  fr06_headless_gpu_semantic_aov_matches_cpu_center_truth -- --nocapture`,
  `cargo test --features agent --test scena_cli_recipe -- --nocapture` (105
  passed), `cargo test --features agent --test stable_contracts --
  --nocapture`, `cargo test --features agent --test a03_capabilities_cli
  static_capabilities_are_explicitly_no_device_and_json_alias_matches --
  --nocapture`, scoped `cargo clippy --features agent --bin scena --test
  scena_cli_recipe --test fr06_semantic_aov --test m4_performance_platform
  --test stable_contracts --test a03_capabilities_cli --test
  m9_platform_release -- -D warnings`, and `cargo run -p xtask -- doctor
  --docs` passed.
- `full`: not run; reserved for the section 13/frozen-diff checkpoint.
- `skipped`: browser/GPU proof is not required for O03 because the new exact
  visible-mask composition check is CPU Headless-only and GPU/browser
  capability rows report degraded instead of claiming exact subject masks.

### O04 - Observation Report

Owner: diagnostics/contracts.

- [x] Add the future `subject_observation.v1` public schema.
- [x] Include target, resolved handles, frame key, projected bounds, visible
      bounds, visible pixel count, depth percentiles, fallback/degraded flags,
      and reason codes.
- [x] Embed or link the observation from render introspection and photo reports.
- [x] Add JSON schema/golden coverage.

Focused proof:

- [x] Contract test validates the report shape and rejects stale or incomplete
      observation payloads.

Scoped gates:

- [x] Schema golden tests.
- [x] Doctor schema/docs contract pin if appropriate.

Validation ledger:

- `focused`: remote `cargo test --features agent --test stable_contracts
  subject_observation -- --nocapture` first failed on missing public
  `SubjectObservationV1`/schema imports, then passed after the stable contract
  was added. Remote `cargo test --features agent --test scena_cli_recipe
  scena_recipe_render_verify_subject_mask_uses_semantic_aov_not_background_heuristics
  -- --nocapture` first failed because the recipe report had no
  `verification.subject_observations`, then passed after composition-to-report
  plumbing. After review found the introspection link gap, the same focused
  recipe proof was extended to assert `introspection.subject_observations` and
  passed. Remote `cargo test --features agent --test photo_render_cli
  photo_render_product_hero_is_easy_path_for_imported_asset -- --nocapture`
  and `cargo test --features agent --test a09_generic_validation
  subject_observation -- --nocapture` passed.
- `scoped`: remote `cargo test --features agent --test stable_contracts --
  --nocapture`, `cargo test --features agent --test scena_cli_schema --
  --nocapture`, `cargo test --features agent --test fr04_cli_schema_matrix --
  --nocapture`, `cargo test --features agent --test a09_generic_validation --
  --nocapture`, `cargo test --features agent --test scena_cli_recipe --
  --nocapture`, and `cargo test --features agent --test photo_render_cli --
  --nocapture` passed. Remote `cargo fmt --check`, scoped `cargo clippy
  --features agent --bin scena --test stable_contracts --test scena_cli_recipe
  --test photo_render_cli --test a09_generic_validation --test
  scena_cli_schema --test fr04_cli_schema_matrix -- -D warnings`,
  `cargo clippy -p xtask -- -D warnings`, and `cargo run -p xtask -- doctor
  --docs` passed.
- `full`: not run; reserved for the section 13/frozen-diff checkpoint.
- `skipped`: browser/WebGPU proof is not required for O04 because the new
  observation report is capture/contract plumbing over the O03 semantic-AOV
  subject mask. GPU-specific subject-mask truth still reports degraded until a
  later backend proof claims exact support.

## 4. Subject-Aware Exposure

### E01 - MeteringMode Public Contract

Owner: `src/render/exposure.rs`, recipe setup/validation, CLI schema.

- [x] Add `MeteringMode::Average`, `CenterWeighted`, `HighlightWeighted`,
      `Subject`, and `Spot`.
- [x] Keep existing average behavior unchanged unless subject metering is
      explicitly selected or routed through `photo.intent`.
- [x] Add recipe fields under `render.metering`.
- [x] Validate target requirements for subject mode and rect requirements for
      spot mode.
- [x] Report unsupported metering modes per backend.

Focused proof:

- [x] Recipe/schema tests accept valid metering forms and reject malformed
      forms with typed error codes.

Scoped gates:

- [x] Schema contract golden.
- [x] CLI help/golden if user-facing help changes. No CLI help text changed;
      docs and schema discovery were updated.

Validation ledger:

- `focused`: remote `cargo test --test m4_performance_platform
  capability_matrix_reports_hardware_tier_and_backend_feature_states --
  --nocapture` first failed with missing `Capabilities` fields for
  `auto_exposure_metering_*`, then passed after the capability report named
  average, center-weighted, highlight-weighted, subject, and spot metering
  separately.
- `scoped`: remote `cargo test --features agent --test stable_contracts --
  --nocapture`, `cargo test --features agent --test a03_capabilities_cli
  static_capabilities_are_explicitly_no_device_and_json_alias_matches --
  --nocapture`, `cargo test --test m4_performance_platform
  capability_matrix_reports_hardware_tier_and_backend_feature_states --
  --nocapture`, and `cargo test --test m9_platform_release
  m9_capability_matrix_artifact_covers_required_lanes -- --nocapture` passed.
  Remote `cargo fmt --check`, scoped `cargo clippy --features agent --bin
  scena --test stable_contracts --test a03_capabilities_cli --test
  m4_performance_platform --test m9_platform_release -- -D warnings`, and
  `cargo run -p xtask -- doctor --docs` passed.
- `full`: not run; reserved for the section 13/frozen-diff checkpoint.
- `skipped`: browser/GPU proof is not required because this row only exposes
  current capability truth. Subject/spot metering remain degraded or disabled
  until E02/E03 adds active metering behavior and backend proof.

### E02 - Weighted Luminance Distribution

Owner: `src/render/exposure`, render quality.

- [x] Replace color-difference foreground guessing in the subject-mode
      estimator with observation-derived weights. Runtime routing remains in
      E03/later rows.
- [x] Weight subject pixels at `1.0` and surround at the initial policy weight
      `0.1`.
- [x] Keep global highlight protection separate from subject midtone metering.
- [x] Use bounded histogram storage; no full-frame unbounded sort in the hot
      path.
- [x] Report sample count, subject sample count, rejected sample count, and
      chosen EV.

Focused proof:

- [x] Synthetic frame test where dark subject on bright background meters for
      the subject, while highlight guard still prevents destructive clipping.
- [x] Mutation test where shifted subject mask fails the expected EV band.

Scoped gates:

- [x] `cargo test --lib render::exposure -- --nocapture`.

Validation ledger:

- `focused`: remote `cargo test --lib render::exposure -- --nocapture`
  passed 12/12, including the dark-subject/bright-field subject-metering
  fixture, the shifted-subject-rect mutation, and the global highlight-guard
  fixture. The focused red proof for the new helper was recorded earlier in
  slice E02a; this slice hardened the same primitive with subject/rejected
  sample counts and an independent highlight histogram.
- `scoped`: remote `cargo test --test round_c_auto_exposure_presets --
  --nocapture`, `cargo test --test m1_geometry_materials auto_exposure --
  --nocapture`, `cargo test --features agent --test stable_contracts
  exposure_report -- --nocapture`, and `cargo test --features agent --test
  a09_generic_validation exposure_report -- --nocapture` passed. Remote
  `cargo fmt --check` first failed on formatting only, then passed after the
  local formatting patch. Remote scoped `cargo clippy --features agent --lib
  --test round_c_auto_exposure_presets --test m1_geometry_materials --test
  stable_contracts --test a09_generic_validation -- -D warnings` passed.
- `full`: not run; reserved for the section 13/frozen-diff checkpoint.
- `skipped`: no browser/GPU proof because this row changes the CPU metering
  primitive and stable reports only. Backend-specific subject metering remains
  degraded until E03 wires and proves the active runtime domain.

### E03 - Scene-Linear Metering Domain

Owner: render CPU/GPU readback, exposure meter.

- [x] Define the required metering domain as scene-linear pre-tonemap data.
- [x] CPU path uses the existing linear frame where available.
- [x] GPU paths use a pre-tonemap luminance sample source or report
      `metering_domain:"encoded_output_feedback"` as degraded.
- [x] Do not silently sample already-exposed presented surface bytes for strict
      product-hero evidence.
- [x] Add work metrics for meter samples, readback/copies, and sync waits.
      These counters already existed and remain the instrumentation source for
      this row.

Focused proof:

- [x] Test rejects a strict product-hero evidence claim when the backend can
      only meter post-tonemap encoded output.
- [x] Test proves subject metering does not feed back through applied exposure.

Scoped gates:

- [x] Exposure tests.
- [x] Native/GPU rendered proof when GPU metering behavior changes. No new
      native/GPU proof was required for this row because GPU sampling behavior
      did not change; encoded-output GPU meters are now reported as degraded.

Validation ledger:

- `focused`: remote `cargo test --test round_c_auto_exposure_presets
  auto_exposure_reports_metering_domain_for_strict_product_hero_evidence --
  --nocapture` first failed with missing `AutoExposureMeteringDomain` and
  `AutoExposureResult::metering_domain()`, then passed after the result carried
  `scene_linear_pre_tonemap` or `encoded_output_feedback` explicitly.
- `focused`: remote `cargo test --features inspection --lib
  render::exposure_report -- --nocapture` passed the report-domain unit checks,
  including rejection of arbitrary domain strings and a strict product-hero
  rejection reason for encoded-output metering.
- `scoped`: remote `cargo test --lib render::exposure -- --nocapture`,
  `cargo test --test round_c_auto_exposure_presets -- --nocapture`, `cargo test
  --features agent --test stable_contracts exposure_report -- --nocapture`,
  `cargo test --features agent --test a09_generic_validation exposure_report --
  --nocapture`, and `SCENA_RELEASE_COMMIT=24f605132f289d37cae005690afdb705f784a6c8
  cargo test --test m5_release m5_public_api_baseline_names_frozen_contracts --
  --nocapture` passed. The first M5 attempt failed in the rsynced no-`.git`
  snapshot because release-artifact provenance was unavailable; rerunning with
  the source commit set classified and resolved that environment/provenance
  condition. Remote `cargo fmt --check`, scoped `cargo clippy --features agent
  --lib --test round_c_auto_exposure_presets --test m5_release --test
  stable_contracts --test a09_generic_validation -- -D warnings`, and
  `cargo run -p xtask -- doctor --docs` passed after formatting.
- `full`: not run; reserved for the section 13/frozen-diff checkpoint.
- `skipped`: no browser or real-GPU run because this row does not change GPU
  copy locations or shader output. It changes the evidence contract so encoded
  output sampling cannot masquerade as strict scene-linear metering.

### E04 - Exposure Compensation

Owner: `src/render/exposure.rs`, recipe validation, CLI.

- [x] Add `exposure_compensation_ev`.
- [x] Permit it only with `auto_exposure`.
- [x] Preserve `exposure_ev` as a full manual override.
- [x] Reject `exposure_ev` with `auto_exposure`.
- [x] Reject `exposure_compensation_ev` without `auto_exposure`.
- [x] Report base metered EV, compensation EV, clamp, and final EV.

Focused proof:

- [x] Validation tests for all legal/illegal combinations.
- [x] Numeric exposure tests prove compensation composes additively.

Scoped gates:

- [x] Recipe validation and exposure tests.
- [x] Schema golden.

Validation ledger:

- `focused`: remote `cargo test --features agent --test
  scene_recipe_contracts
  scene_recipe_validation_accepts_auto_exposure_compensation_only_with_auto_exposure
  -- --nocapture` passed, proving `exposure_compensation_ev` is accepted only
  with `auto_exposure` and rejected without auto exposure or beside manual
  `exposure_ev`.
- `focused`: remote `cargo test --test round_c_auto_exposure_presets
  auto_exposure_compensation_offsets_metered_ev_without_replacing_metering --
  --nocapture` passed, proving compensation is additive over the metered base
  EV instead of replacing metering.
- `scoped`: remote `cargo test --features agent --test stable_contracts
  scene_recipe -- --nocapture` passed the scene-recipe schema/golden subset,
  and remote `cargo test --features agent --test a02_recipe_field_model --
  --nocapture` passed all field-model parity tests. An attempted
  `a02_recipe_field_model exposure_compensation` filter matched zero tests and
  is intentionally not counted as evidence.
- `full`: not run; reserved for the section 13/frozen-diff checkpoint.
- `skipped`: no browser/GPU proof because this row changes recipe validation,
  auto-exposure arithmetic, and stable schema/field-model contracts, not
  backend pixel output.

### E05 - ExposureReport

Owner: diagnostics/contracts.

- [x] Add the future `exposure_report.v1` public schema.
- [x] Include subject mean luminance, subject clip fractions, global highlight
      limits, base EV, compensation EV, final EV, clamp/fallback status,
      suggested compensation, metering mode, metering domain, and frame key.
- [x] Expose it in `render_introspection` when auto exposure is active.
- [x] Expose it in `photo_report`.
- [x] Add stable JSON schema and golden coverage.

Focused proof:

- [x] Report contract test fails if compensation, domain, fallback, or frame key
      is missing.

Scoped gates:

- [x] Schema/golden tests.
- [x] Doctor schema/docs pin if added.

## 5. Subject Focus

### F01 - Recipe/API Contract

Owner: recipe types, render settings, CLI schema.

- [x] Add subject focus form:
      `{ "focus": { "mode": "subject", "target": ... } }`.
- [x] Keep manual `focus_distance` for advanced users.
- [x] Reject ambiguous forms with both manual distance and subject focus.
- [x] Add `coverage:"all"` and `strength:"subtle"` policy fields or their
      chosen equivalents.
- [x] Document compatibility with existing DoF fields.

Focused proof:

- [x] Schema/validation test accepts subject focus and rejects ambiguous manual
      plus subject focus.

Scoped gates:

- [x] Schema/golden tests.

### F02 - Visible-Depth Focus Solver

Owner: `scene_host::composition`, render DoF setup.

- [x] Use visible subject depth percentiles from `SubjectObservation`.
- [x] Default focal plane is weighted median visible depth.
- [x] `coverage:"all"` keeps near/far subject depth percentiles acceptably
      sharp.
- [x] Handle empty/no-depth subject as a structured error or reported fallback.
- [x] Report resolved focus distance and confidence.

Focused proof:

- [x] Depth fixture where bounds-center focus is wrong but visible-depth median
      focus is correct.

Scoped gates:

- [x] Focus/DoF unit or rendered-output proof.
- [ ] Visual proof if GPU/WebGL DoF output changes.

### F03 - FocusReport

Owner: diagnostics/contracts.

- [x] Add the future `focus_report.v1` public schema.
- [x] Include target, mode, resolved focus distance, depth percentiles, coverage
      mode, fallback/degraded status, and frame key.
- [x] Link report from render introspection and photo report.

Focused proof:

- [x] Contract test rejects missing focus target, stale frame key, or unresolved
      focus reason.

Scoped gates:

- [x] Schema/golden tests.

## 6. Photo Intent and Composition Planner

### P01 - Photo Intent Schema

Owner: recipe schema, `scene_host::photo`, CLI.

- [x] Add `photo.intent:"product_hero"` or the final naming chosen by schema
      convention.
- [x] Add subject, composition, exposure, focus, and staging sub-objects.
- [x] Define defaults only as policy constraints, not hidden final numbers.
- [x] Reject conflicting manual fields in strict easy mode when they undermine
      the intent gate.
- [x] Preserve ordinary recipes that do not opt into `photo.intent`.

Focused proof:

- [x] Schema tests accept the minimal product-hero recipe and reject
      contradictory manual overrides.

Scoped gates:

- [x] Schema golden.
- [x] Recipe CLI validation tests.

Validation ledger:

- `focused`: remote
  `cargo test --features agent --test scene_recipe_contracts
  scene_recipe_validation_accepts_product_hero_policy_subobjects_and_rejects_manual_staging
  -- --nocapture` first failed because `$.photo.composition`,
  `$.photo.exposure`, `$.photo.focus`, and `$.photo.staging` were unknown
  fields; after adding typed policy subobjects and conflict validation it
  passed.
- `scoped`: remote `cargo test --features agent --test
  scene_recipe_contracts -- --nocapture` passed 71/71; remote `cargo test
  --features agent --test a02_recipe_field_model --test stable_contracts --test
  scena_cli_schema -- --nocapture` passed; remote `cargo fmt --check` passed;
  remote scoped `cargo clippy --features agent --lib --test
  scene_recipe_contracts --test a02_recipe_field_model --test stable_contracts
  --test scena_cli_schema -- -D warnings` passed.
- `full`: not run; reserved for the section 13/frozen-diff checkpoint.
- `skipped`: no browser/GPU/render proof; P01 changes recipe schema,
  validation, field-model, and stable contracts, not backend pixels.

### P02 - Deterministic Candidate Generation

Owner: `scene_host::photo`.

- [x] Generate bounded candidates over view, lens/focal length, fill, camera
      height/elevation, subject yaw, and staging variants.
- [x] Use stable deterministic ordering and tie-breaking.
- [x] Expose candidate budget and selected candidate in reports.
- [x] Support user constraints:
      preferred view, front/up hints, keep-visible anchors, fill range, and
      background/staging style.
- [x] Keep host-owned domain semantics out of candidate generation.

Focused proof:

- [x] Pure unit test verifies candidate count, ordering, and stable selection
      for a fixed subject bounds input.

Scoped gates:

- [x] `cargo test --test <photo_planner_test>` or module unit tests.

Validation ledger:

- `focused`: remote `cargo test --features scene-host --test scene_host
  product_hero_candidate_plan_is_bounded_deterministic_and_constraint_aware --
  --nocapture` first failed because `PhotoCandidateRequest` and
  `product_hero_candidate_plan` were missing from the public API; after adding
  `scene_host::photo` candidate planning it passed. The focused test also pins
  budget, ordering, selected candidate id, view/lens/fill/elevation/yaw,
  staging, keep-visible anchors, and front/up hint propagation.
- `focused`: remote `cargo test --features agent --test photo_render_cli
  photo_render_product_hero_is_easy_path_for_imported_asset -- --nocapture`
  passed with assertions that `scena.photo_report.v1` exposes
  `planning.schema`, candidate budget, selected candidate id, and candidate
  staging.
- `scoped`: remote `cargo test --features agent --test scene_host --test
  photo_render_cli --test stable_contracts --test scena_cli_schema --
  --nocapture` passed after the stable `scena.photo_candidate_plan.v1` fixture
  was added and the schema catalog/list goldens were updated.
- `scoped`: remote `cargo fmt --check` passed; remote scoped `cargo clippy
  --features agent --lib --bin scena --test scene_host --test photo_render_cli
  --test stable_contracts --test scena_cli_schema -- -D warnings` passed.
- `full`: not run; reserved for the section 13/frozen-diff checkpoint.
- `skipped`: no browser/GPU proof; P02 changes deterministic planning, report
  schema, and CPU/headless camera setup selection, not a backend-specific GPU
  shader or browser path.

### P03 - Geometry/AOV Candidate Scoring

Owner: `scene_host::photo`, `scene_host::composition`.

- [x] Score visible fill, centering, clipping, occlusion, floor proportion,
      silhouette area, aspect fit, depth/normal variety, and anchor visibility.
- [x] Penalize empty slab/floor dominating the frame.
- [x] Penalize views with the subject too small or too large.
- [x] Penalize weak silhouette/background separation.
- [x] Use semantic/AOV data where available; report degraded scoring otherwise.

Focused proof:

- [x] Candidate scoring test ranks a good hero view over known bad views:
      pulled-back, clipped, off-center, flat-front unreadable, and floor-heavy.

Scoped gates:

- [x] Planner/composition tests.

Validation ledger:

- `focused`: remote `cargo test --features scene-host --test scene_host
  product_hero_candidate_scoring_ranks_good_view_over_known_bad_geometry_views
  -- --nocapture` first failed because `PhotoCandidateObservation` and
  `score_product_hero_candidates` were missing from the public API; after adding
  the pure candidate scorer it passed.
- `focused`: the scoring test pins the exact known-bad ranking family:
  pulled-back subject gets `subject_fill_below_min`, clipped subject gets
  `subject_clipped`, off-center subject gets `subject_off_center`,
  flat/unreadable subject gets `subject_readability_low`, and floor-heavy
  candidate gets `floor_dominates_frame`. It also verifies degraded scoring
  reports `semantic_aov_unavailable`.
- `scoped`: remote `cargo test --features scene-host --test scene_host --
  --nocapture` passed 55/55.
- `scoped`: remote `cargo fmt --check` passed; remote scoped `cargo clippy
  --features agent --lib --test scene_host -- -D warnings` passed.
- `full`: not run; reserved for the section 13/frozen-diff checkpoint.
- `skipped`: no rendered-output/GPU proof; P03 is a pure planning/composition
  scoring contract. P04 is the row that renders low-resolution shaded
  candidates.

### P04 - Low-Resolution Shaded Candidate Scoring

Owner: renderer, scene_host photo planner.

- [x] Render top geometry candidates at bounded low resolution.
- [x] Score material readability, steel/metal reflection structure, highlight
      clipping, black crush, background separation, and post effect sanity.
- [x] Reuse render quality metrics rather than inventing a parallel scorer.
- [x] Make candidate renders explicit in work metrics and reports.
- [x] Keep candidate count bounded and deterministic.

Focused proof:

- [x] Shaded candidate test ranks steel-readable view/staging over flat black
      silhouette and flat gray metal mutations.

Scoped gates:

- [x] Rendered-output proof for the candidate fixture.
- [x] Performance/work-metric test for bounded candidate count.

Validation ledger:

- `focused`: remote `cargo test --features agent --test photo_render_cli
  photo_render_product_hero_is_easy_path_for_imported_asset -- --nocapture`
  first failed because `scena.photo_report.v1` lacked
  `shaded_selection`; after implementation it passed and rendered the product
  hero fixture.
- `focused`: remote `cargo test --features scene-host --test scene_host
  product_hero_shaded_candidate_scoring_rejects_black_silhouette_and_flat_gray_metal
  -- --nocapture` passed, pinning that the material-aware candidate scorer
  ranks the steel-readable observation over black-silhouette and flat-gray
  metal mutations with `subject_black_crush` and
  `subject_readability_low` reasons.
- `scoped`: remote artifact inspection of the generated report confirmed
  `scena.photo_shaded_candidate_selection.v1`, selected composition id
  `product_hero_view_three_quarter_front_right_lens_portrait_fill_0_78_elev_30_yaw_0_stage_dark_studio`,
  `low_resolution:{width:160,height:105}`, `evaluated_count:3`, and
  `total_candidate_pixels:50400`.
- `scoped`: remote `cargo test --features agent --test scene_host --test
  photo_render_cli --test stable_contracts --test scena_cli_schema --
  --nocapture` passed (`scene_host` 56/56, `photo_render_cli` 4/4,
  `scena_cli_schema` 8/8, `stable_contracts` 68/68).
- `scoped`: remote `cargo fmt --check`, remote scoped `cargo clippy
  --features agent --lib --bin scena --test scene_host --test
  photo_render_cli --test stable_contracts --test scena_cli_schema --
  -D warnings`, and remote `cargo run -p xtask -- doctor --docs` passed.
- `full`: not run; reserved for the section 13/frozen-diff checkpoint.
- `skipped`: no real GPU proof; P04 uses CPU/headless rendered-output evidence
  and report contracts, not a backend-specific GPU rendering path.

### P05 - Staging Policy

Owner: `scene_host::product`, `scene_host::photo`, recipe setup.

- [x] Define product-hero staging defaults:
      studio environment, structured reflections for metal, matte ground,
      no grid, controlled background, appropriate light rig, ACES or chosen
      tonemapper.
- [x] Ensure staging pieces do not fight each other. A bright background plus
      average metering cannot be a product-hero default.
- [x] Emit every resolved staging choice in the plan/report.
- [x] Allow explicit user overrides but report when they make the intent gate
      fail.
- [x] Avoid manual floor geometry in generated plans; use renderer-owned
      ground/grid helpers.

Focused proof:

- [x] Product-hero plan for the fixture contains no grid and no manual floor
      geometry, and includes a reflection-capable environment/staging policy.

Scoped gates:

- [x] Planner tests and schema/report golden.

Validation ledger:

- `focused`: remote `cargo test --features scene-host --test scene_host
  product_hero_candidate_plan_is_bounded_deterministic_and_constraint_aware
  -- --nocapture` passed after the regression guard was tightened to require
  every generated product-hero candidate to use `environment:"studio"`,
  `background:"dark_studio"`, `ground:"matte_shadow_catcher"`, and
  `grid:false`.
- `focused`: remote `cargo test --features agent --test photo_render_cli
  photo_render_product_hero_is_easy_path_for_imported_asset -- --nocapture`
  passed with assertions that the emitted reproducible recipe keeps
  `scene.grid.enabled:false`, uses `kind:"dark_studio"` instead of a literal
  background color, and does not add manual `geometries`/`nodes` floor staging.
- `scoped`: remote `cargo test --features agent --test scene_host --test
  photo_render_cli -- --nocapture` passed (`scene_host` 56/56,
  `photo_render_cli` 4/4).
- `scoped`: remote `cargo fmt --check` and scoped `cargo clippy --features
  agent --test scene_host --test photo_render_cli -- -D warnings` passed.
- `full`: not run; reserved for the section 13/frozen-diff checkpoint.
- `skipped`: no production-code or schema-shape change was needed for P05; the
  existing P02/P04 plan/report fields already carried staging choices, so this
  row closed by adding tighter regression guards and checklist evidence.

### P06 - Bounded Retry

Owner: `scene_host::photo`, render quality.

- [x] After final render, run the acceptance gate.
- [x] Permit at most one deterministic retry from verifier suggestions:
      exposure compensation, fill adjustment, or focus correction.
- [x] Report first attempt, suggestion, retry input, and final result.
- [x] Do not loop indefinitely.
- [x] Fail nonzero with diagnostics if the retry still fails.

Focused proof:

- [x] Test where first candidate is slightly underexposed and one bounded
      compensation retry passes.
- [x] Test where both attempts fail and the command exits nonzero with useful
      report.

Scoped gates:

- [x] CLI/integration test.

Validation ledger:

- `focused`: remote `cargo test --features agent --bin scena
  product_hero_retry_policy_is_one_bounded_exposure_compensation_retry --
  --nocapture` first failed because `PRODUCT_HERO_MAX_ATTEMPTS` was `4`; after
  implementation it passed and pinned `max_attempts:2`, `max_retries:1`,
  deterministic `exposure_compensation_ev` suggestion, and retry input.
- `focused`: remote `cargo test --features agent --test photo_render_cli
  photo_render_product_hero_is_easy_path_for_imported_asset -- --nocapture`
  first failed because `scena.photo_report.v1` had no `retry` block; after
  implementation it passed.
- `focused`: remote `cargo test --features agent --test photo_render_cli
  photo_render_product_hero_fails_after_one_retry_with_useful_report --
  --nocapture` passed. The fixture uses a black import material and zero
  authored light, verifies a nonzero exit, empty stderr, diagnostic PNG/report
  artifacts, `retry.attempts:2`, `retry.retry_used:true`, and final subject
  exposure/readability failure codes.
- `scoped`: remote `cargo test --features agent --bin scena --test
  photo_render_cli --test stable_contracts --test scena_cli_schema --
  --nocapture` passed (`bin scena` 16/16, `photo_render_cli` 5/5,
  `scena_cli_schema` 8/8, `stable_contracts` 68/68).
- `scoped`: remote `cargo fmt --check`, scoped `cargo clippy --features agent
  --bin scena --test photo_render_cli --test stable_contracts --test
  scena_cli_schema -- -D warnings`, and `cargo run -p xtask -- doctor --docs`
  passed.
- `full`: not run; reserved for the section 13/frozen-diff checkpoint.
- `skipped`: no real GPU/browser proof; this row changes bounded CLI retry
  policy/reporting and uses CPU/headless rendered-output integration evidence.

### P07 - Photo Plan and Report Contracts

Owner: diagnostics/contracts, CLI.

- [x] Add the future `photo_plan.v1` public schema.
- [x] Add `scena.photo_report.v1`.
- [x] Include input intent, resolved subject, candidates evaluated, candidate
      scores, selected candidate, rejected-candidate reasons, staging choices,
      exposure report, focus report, quality result, and emitted recipe path.
- [x] Add compact and pretty forms consistent with existing CLI output policy.
      Existing global `--compact` / `--pretty` shaping applies to stdout; report
      files remain pretty JSON for auditability.
- [x] Add JSON schemas and stable-contract fixtures.

Focused proof:

- [x] Contract test rejects a report that lacks candidate list, selected
      candidate, exposure report, focus report, or quality verdict.

Scoped gates:

- [x] Schema/golden tests.
- [x] CLI golden tests.

Validation ledger:

- `focused`: remote `cargo test --features agent --test stable_contracts
  photo_report_contract_rejects_missing_required_diagnostics -- --nocapture`
  first failed because `scena.photo_report.v1` used envelope-only validation;
  after typed `PhotoReportV1` validation it passed and rejects missing
  `candidates`, `selected`, `exposure_report`, `focus_report`, and `quality`.
- `focused`: remote `cargo test --features agent --test stable_contracts
  photo_plan_golden_matches_live_schema_serialization -- --nocapture` passed
  and verifies `scena.photo_plan.v1` round-trip plus full generic validation.
- `focused`: remote `cargo test --features agent --test stable_contracts
  photo_report_golden_matches_live_schema_serialization -- --nocapture` passed
  and verifies typed report fixture round-trip.
- `scoped`: remote `cargo test --features agent --test stable_contracts --test
  scena_cli_schema --test a09_generic_validation --test photo_render_cli --
  --nocapture` passed (`stable_contracts` 71/71, `scena_cli_schema` 8/8,
  `a09_generic_validation` 8/8, `photo_render_cli` 5/5).
- `full`: not run; reserved for the section 13/frozen-diff checkpoint.
- `skipped`: no real GPU/browser proof; this row is schema/contract validation
  and stdout/report catalog coverage only.

## 7. CLI Surface

### C01 - `scena photo plan`

Owner: `src/bin/scena`, `scene_host::photo`.

- [x] Add command help.
- [x] Accept model path or recipe path.
- [x] Accept `--intent product-hero`.
- [x] Accept `--out plan.json`.
- [x] Accept optional subject target override.
- [x] Emit versioned JSON envelope on success and failure.
- [x] Do not render the final high-resolution image.

Focused proof:

- [x] CLI test verifies `photo plan` emits a valid `photo_plan.v1` public
      schema.

Scoped gates:

- [x] CLI integration tests and help golden.

Validation ledger:

- `focused`: remote `cargo test --features agent --test photo_render_cli
  photo_plan_product_hero_emits_render_free_public_plan_for_imported_asset --
  --nocapture` first failed with `invalid_arguments` because `photo plan` was
  not routed; after implementation it passed, validated
  `scena.photo_plan.v1`, wrote `--out`, and asserted no PNG was produced.
- `focused`: remote `cargo test --features agent --test photo_render_cli
  photo_plan_recipe_input_accepts_subject_import_override -- --nocapture`
  passed and pins recipe input plus `--subject import:<id>` import resolution.
- `focused`: remote `cargo test --features agent --test photo_render_cli
  photo_plan_recipe_input_accepts_subject_node_override -- --nocapture` passed
  and pins recipe input plus `--subject node:<id>` authored-node resolution.
- `focused`: remote `cargo test --features agent --test photo_render_cli
  photo_plan_recipe_input_uses_declared_node_subject -- --nocapture` passed
  and pins recipe-declared `photo.subject:{kind:"node",id}` selection when no
  CLI override is supplied.
- `scoped`: remote `cargo test --features agent --test photo_render_cli --test
  fr04_cli_schema_matrix --test a10_cli_contract_table -- --nocapture` passed
  after reviewing and updating the command-contract digest for the added
  `photo plan` row (`photo_render_cli` 9/9, `fr04_cli_schema_matrix` 6/6,
  `a10_cli_contract_table` 4/4).
- `full`: not run; reserved for the section 13/frozen-diff checkpoint.
- `skipped`: no rendered-output or GPU proof; the command is intentionally
  render-free.

### C02 - `scena photo render`

Owner: `src/bin/scena`, `scene_host::photo`.

Slice 1 note: the command is implemented for product-hero asset/recipe inputs
with bounded exposure retry, quality acceptance, stable stdout/report schemas,
and optional emitted public recipe. The emitted recipe expresses the current
public recipe-surface equivalent for asset inputs. Full multi-view candidate
planning, subject metering/focus contracts, and recipe-native `photo.intent`
remain in P01-P07 and C03.

- [x] Accept model path or recipe path.
- [x] Accept `--intent product-hero`.
- [x] Accept `--out image.png`.
- [x] Accept `--report report.json`.
- [x] Accept `--emit-recipe resolved.recipe.json`.
- [x] Run bounded candidate planning, render, verification, and optional retry.
- [x] Exit nonzero on acceptance failure.
- [x] Preserve stable JSON envelope behavior for stdout/stderr.

Focused proof:

- [x] End-to-end CLI proof against the product-hero fixture fails on baseline
      and passes after implementation.

Scoped gates:

- [x] CLI integration tests.
- [x] Rendered-output proof.
- [ ] Real GPU proof only if the command claims GPU-specific evidence.

Validation ledger:

- `focused`: remote `cargo test --features agent --test photo_render_cli
  photo_render_product_hero_is_easy_path_for_imported_asset -- --nocapture`
  decodes the emitted PNG, verifies 256x168 dimensions, recomputes subject-region
  luminance/clip/readability metrics from the PNG bytes, and checks them against
  the product-hero fixture bands and report metrics.
- `scoped`: remote `cargo test --features agent --test photo_render_cli --
  --nocapture` passed 7/7; remote `cargo fmt --check` passed; remote `cargo
  clippy --features agent --test photo_render_cli -- -D warnings` passed; remote
  `cargo run -p xtask -- doctor --docs` passed.
- `full`: not run; reserved for the section 13/frozen-diff checkpoint.
- `skipped`: real GPU proof remains unchecked because this C02 proof is the CPU
  headless CLI contract and the command does not yet claim GPU-specific
  release evidence.

### C03 - Recipe Path Integration

Owner: recipe parser/setup, scene host.

- [x] `photo.intent` inside a recipe routes through the same planner as
      `scena photo render`.
- [x] Existing `render` fields can explicitly opt into subject metering and
      subject focus without `photo.intent`.
- [x] Validate conflicts between `photo.intent` and manual camera/exposure/focus
      fields.
- [x] Render introspection includes linked subject/exposure/focus/photo reports.

Focused proof:

- [x] `scena recipe render fixture.recipe.json --verify` passes the same gate
      as `scena photo render`.
- [x] Explicit `render.metering.mode:"subject"` plus
      `render.depth_of_field.focus.mode:"subject"` works without `photo.intent`
      and reports subject-routed auto-exposure.

Scoped gates:

- [x] Recipe integration tests and schema golden.

Validation ledger:

- `focused`: remote `cargo test --features agent --test scena_cli_recipe
  recipe_render_subject_metering_and_focus_work_without_photo_intent --
  --nocapture` first failed because `render.metering.mode:"subject"` produced
  `exposure_report.auto_exposure.subject_sample_count:0`; after wiring the
  resolved semantic subject rect into the renderer it passed with linked
  `exposure_report`, `focus_report`, and `subject_observations`.
- `scoped`: remote `cargo test --features agent --test photo_render_cli --
  --nocapture` passed 7/7; remote `cargo test --test
  round_c_auto_exposure_presets -- --nocapture` passed 4/4; remote `cargo test
  --features agent --test scene_recipe_contracts
  scene_recipe_validation_accepts_metering_modes_and_rejects_invalid_forms --
  --nocapture` passed; remote `cargo test --features agent --test
  scena_cli_recipe recipe_render_subject -- --nocapture` passed 2/2; remote
  `cargo fmt --check` passed; remote scoped `cargo clippy --features agent
  --lib --bin scena --test scena_cli_recipe --test photo_render_cli --test
  round_c_auto_exposure_presets --test scene_recipe_contracts -- -D warnings`
  passed; remote `cargo run -p xtask -- doctor --docs` passed.
- `full`: not run; reserved for the section 13/frozen-diff checkpoint.
- `skipped`: full release chain and real GPU proof were not run for C03; this
  row changes CPU/headless recipe setup and shared renderer metering state, not
  a backend-specific GPU claim.

## 8. Quality and Diagnostics

### Q01 - Subject Visibility Reasons

Owner: `scene_host::composition`, `render::visibility_diagnosis`, diagnostics.

- [x] Every declared photo subject that contributes zero visible pixels gets a
      reason code. Q01a covers explicit recipe subjects from `render.metering`;
      Q01b covers `photo.subject` and subject-focus-specific degraded focus
      reports.
- [x] Required reasons:
      unresolved target, hidden, outside viewport, behind camera, clipped by
      section box or clipping plane, occluded, degenerate geometry, transparent
      unsupported/degraded, stale observation. Fresh recipe-render outcomes
      cover hidden, outside viewport, behind camera, section box, clipping
      plane, occluded, degenerate geometry, and transparent/unsupported. Stale
      observation is a contract/provenance mutation and is covered by
      `subject_observation.v1` validation as `stale_subject_observation`.
- [x] Render output must point to the same diagnosis; agents should not need to
      guess handles and run separate visibility commands.
- [x] Ensure this covers labels/overlays where relevant but keep photo subjects
      separate from annotation ownership.

Focused proof:

- [x] Known-bad recipes for each reason produce the expected reason code. Q01a
      pins hidden, outside viewport, behind camera, section box, clipping
      plane, occluded, degenerate geometry, and transparent/unsupported; Q01b
      pins `photo.subject` and `render.depth_of_field.focus` source routing.
      Stale observation is pinned by the stable-contract/generic-validation
      stale-frame mutation rather than by a fresh-render recipe fixture.

Scoped gates:

- [x] Composition/diagnostics tests for Q01a.
- [x] Doctor rule if source/docs can drift silently.

Validation ledger:

- `focused`: remote `cargo test --features agent --test scena_cli_recipe
  scena_recipe_render_verify_reports_zero_visible_subject_reason_codes --
  --nocapture` first failed because a hidden `render.metering` subject exited as
  `scena.cli_error.v1` runtime before producing a recipe-render verification
  report; after implementation it passed and proved structured
  `scena.recipe_render_result.v1` failures for `subject_hidden`,
  `subject_outside_viewport`, `subject_behind_camera`,
  `subject_degenerate_geometry`, `subject_clipped_by_section_box`,
  `subject_clipped_by_clipping_plane`, `subject_transparent_unsupported`, and
  `subject_occluded`.
- `focused`: remote `cargo test --features agent --test scena_cli_recipe
  recipe_render_subject_focus_accepts_authored_node_targets -- --nocapture`
  first failed because recipe validation rejected
  `render.depth_of_field.focus.target.kind:"node"` despite the shared target
  grammar; after validation and runtime focus resolution moved to
  `resolve_scene_recipe_target_handles`, it passed.
- `focused`: remote `cargo test --features agent --test scena_cli_recipe
  scena_recipe_render_verify_reports_zero_visible_photo_and_focus_subject_reason_codes
  -- --nocapture` first failed because `photo.subject`/product-hero setup
  aborted as `scena.cli_error.v1` runtime before the recipe verification report;
  after invisible-subject setup/candidate scoring degraded into a failed
  product candidate, it passed and proved `photo.subject` plus
  `render.depth_of_field.focus` both surface `subject_hidden` through
  composition reasons and `subject_observation.v1`. The focus case also emits
  an unresolved `focus_report.v1` with `reason:"subject_hidden"`.
- `focused`: remote `cargo test --features agent --test stable_contracts
  subject_observation -- --nocapture` and remote `cargo test --features agent
  --test a09_generic_validation stale -- --nocapture` passed after
  `SubjectObservationV1::validate_contract()` changed stale frame keys to the
  subject-specific code `stale_subject_observation`.
- `focused`: remote `cargo test -p xtask
  q01_doctor_rejects_subject_visibility_reason_contract_drift -- --nocapture`
  passed after `ARCH-SUBJECT-VISIBILITY-REASONS` pinned implementation,
  multi-source tests, and `docs/schema-contracts.md` reason-code docs.
- `scoped`: remote `cargo test --features agent --test scena_cli_recipe subject
  -- --nocapture` passed 10/10; remote `cargo fmt --check` passed; remote
  scoped `cargo clippy --features agent --lib --bin scena --test
  scena_cli_recipe --test stable_contracts --test a09_generic_validation -- -D
  warnings` passed; remote `cargo clippy -p xtask -- -D warnings` passed;
  remote `cargo run -p xtask -- doctor --docs` passed.
- `scoped`: remote `cargo run -p xtask -- doctor --architecture` was rerun
  because a doctor architecture rule changed. It reports no
  `ARCH-SUBJECT-VISIBILITY-REASONS` or xtask file-size failures, but still
  fails on the pre-existing branch-wide WIP findings: one dependency-direction
  issue, one diagnostics docs phrase, KISS-size findings, and `NO_LIGHTS`
  env-doc drift in `examples/probe_hero.rs`.
- `full`: not run; reserved for the section 13/frozen-diff checkpoint.
- `skipped`: no browser/GPU proof; Q01 changes structured CPU Headless recipe
  diagnostics, contract validation, and source/docs doctor pins, not
  backend-specific pixels.

### Q02 - Product-Quality Integration

Owner: `src/render/quality`, recipe verification.

- [x] `expect_quality.profile:"product"` consumes `SubjectObservation` for
      exact headless subject pixels. Q02a adds optional
      `SubjectObservation.pixel_quality`, populated from semantic-AOV-visible
      pixels, and product quality emits
      `expect_quality.subject.pixel_exposure`.
- [x] Subject exposure checks use visible subject pixels, not only
      color-difference foreground guesses, for exact headless
      `render.metering` subjects. The Q02a fixture deliberately uses a black
      subject on a black background: color-difference foreground detection
      reports no foreground, while the semantic subject observation still
      reports 999 subject pixels and fails product exposure.
- [x] Steel/material readability checks fail flat-black, flat-gray, and missing
      reflection structure. Q02a adds
      `expect_quality.subject.material_readability` and pins flat-black; Q02b
      adds an exact subject-mask flat-gray recipe fixture, and the existing
      rendered reflection-quality proof pins missing reflection structure on
      CPU and HeadlessGpu.
- [x] Mean luminance bands are fixture-specific where necessary through
      `expect_quality.exposure.min_mean_luminance_srgb8` and
      `max_mean_luminance_srgb8`; Q02b proves those thresholds override the
      product defaults in the subject-pixel exposure check.
- [x] Reports include suggested compensation and staging/camera advice for
      Q02a subject exposure failures through
      `observed.suggested_compensation_ev` and the quality check `fix_hint`.

Focused proof:

- [x] Existing dark-subject class is rejected by exact subject-quality proof.
      Q02a used the stricter black-subject-on-black-background fixture so the
      test cannot pass through color-difference foreground guessing. Q02b adds
      exact flat-gray material-readability failure and fixture-specific subject
      mean-luminance bands. The planner/acceptance success case remains
      covered by C02.

Scoped gates:

- [x] Quality tests for Q02a.
- [x] Rendered-output proof for fixture-specific bands and steel/reflection
      cases.

Validation ledger:

- `focused`: remote `cargo test --features agent --test scena_cli_recipe
  recipe_render_product_quality_uses_exact_subject_observation_pixels --
  --nocapture` first failed because `render.metering` plus manual
  `exposure_ev` stopped at schema validation; after correcting the fixture to
  `auto_exposure:"product_studio"` it failed for the intended reason:
  `SubjectObservationV1` had no `pixel_quality` and quality had no
  `expect_quality.subject.pixel_exposure`. After implementation it passed.
- `scoped`: remote `cargo test --features agent --test scena_cli_recipe
  subject -- --nocapture` passed 8/8; remote `cargo test --features
  inspection --test stable_contracts subject_observation -- --nocapture`
  passed 2/2; remote `cargo fmt --check` passed; remote scoped
  `cargo clippy --features agent --lib --bin scena --test scena_cli_recipe
  --test stable_contracts -- -D warnings` passed.
- `full`: not run; reserved for the section 13/frozen-diff checkpoint.
- `focused`: Q02b remote `cargo test --features agent --test
  scena_cli_recipe
  recipe_render_product_quality_uses_exact_subject_observation_pixels --
  --nocapture` first failed before the production patch because
  `expect_quality.exposure` could not express subject mean-luminance bands and
  the report did not reach the quality block; after adding
  `min_mean_luminance_srgb8` / `max_mean_luminance_srgb8` and routing them
  into the subject-pixel exposure check, it passed. The same focused test now
  also proves flat-gray exact subject pixels fail
  `expect_quality.subject.material_readability`.
- `focused`: remote `cargo test --features agent --test
  scene_recipe_contracts
  scene_recipe_exposure_quality_threshold_domains_match_subject_metrics --
  --nocapture` passed, proving sRGB8 luminance thresholds accept `[0,255]`,
  clip fractions remain normalized `[0,1]`, and inverted luminance bands fail
  closed.
- `focused`: remote `cargo test --features agent --test
  scena_cli_recipe
  scena_recipe_render_verify_fails_missing_reflection_quality_on_cpu_and_gpu --
  --nocapture` passed on the current diff, proving the missing-reflection
  rendered-output failure stays pinned for CPU and HeadlessGpu.
- `scoped`: remote `cargo test --features agent --test stable_contracts
  scene_recipe -- --nocapture` passed; remote `cargo test --features agent
  --test scena_cli_recipe subject -- --nocapture` passed 8/8; remote `cargo
  fmt --check` passed; remote scoped `cargo clippy --features agent --lib
  --bin scena --test scena_cli_recipe --test scene_recipe_contracts --test
  stable_contracts -- -D warnings` passed; remote `cargo run -p xtask --
  doctor --docs` passed.
- `skipped`: full release chain not run; Q02 is a focused recipe/quality
  integration slice and full gates remain reserved for the section 13 frozen
  checkpoint.

### Q03 - Acceptance Mutations

Owner: tests/proof harness.

- [x] Add mutation for average metering.
- [x] Add mutation for stale subject observation.
- [x] Add mutation for wrong target.
- [x] Add mutation for old EV cap.
- [x] Add mutation for post-tonemap metering on strict evidence.
- [x] Add mutation for pulled-back/empty-slab camera.
- [x] Add mutation for wrong focus.
- [x] Add mutation for missing metal reflection structure.

Focused proof:

- [x] Every known-bad mutation is rejected by the product-hero gate.

Scoped gates:

- [x] Product-hero proof test.

Validation ledger:

- `focused`: remote `cargo test --features agent --bin scena
  product_hero_oracle_rejects_known_bad_mutations -- --nocapture` first failed
  because the product-hero acceptance oracle had only metric-based inputs and
  could not reject the manifest-listed post-tonemap metering or wrong-focus
  mutations; after adding `ProductHeroGateEvidence`, it passed and rejects
  average metering, stale mask, wrong target, old EV cap, post-tonemap metering
  on strict evidence, pulled-back camera, off-center subject, blown highlights,
  flat-gray metal, wrong focus, and missing metal reflection structure.
- `focused`: remote `cargo test --features agent --test photo_render_cli
  product_hero_fixture_manifest_pins_source_bands_and_mutations --
  --nocapture` passed, proving the fixture manifest names every known-bad
  mutation and rejection code.
- `scoped`: remote `cargo fmt --check` passed; remote scoped `cargo clippy
  --features agent --bin scena --test photo_render_cli -- -D warnings` passed.
- `full`: not run; reserved for the section 13/frozen-diff checkpoint.
- `skipped`: no new rendered-output/GPU proof for Q03. Existing Q02/P04
  rendered-output tests cover flat/reflection pixels, and E03/F02 cover
  metering-domain/focus mechanics; Q03 only unifies those reason codes in the
  product-hero acceptance oracle.

## 9. Backend and Performance Requirements

### B01 - CPU, HeadlessGpu, Native, Browser Policy

Owner: capabilities, render backends, release gates.

- [x] Document which backends support exact subject masks.
- [x] Document which backends support scene-linear metering.
- [x] Decide which backend evidence classes may claim strict product-hero pass.
- [x] Add degraded reports for fallback/missing capabilities.
- [x] Avoid false green on unavailable GPU hardware.

Focused proof:

- [x] Capability tests prove strict/degraded/unsupported classification.

Scoped gates:

- [x] Capability matrix tests.
- [x] Browser/GPU proof only at the relevant checkpoint. No browser/GPU proof
      was required for B01 because the row updates capability truth and docs:
      CPU Headless is strict for exact subject mask plus scene-linear subject
      metering; GPU/browser lanes remain explicitly degraded until a
      backend-specific proof upgrades them.

Validation ledger:

- `focused`: remote `cargo test --test m4_performance_platform
  capability_matrix_reports_hardware_tier_and_backend_feature_states --
  --nocapture` first failed because CPU Headless still reported
  `auto_exposure_metering_subject: Degraded`; after updating the capability
  status it passed with Headless `Supported` and GPU/browser lanes still
  `Degraded`.
- `scoped`: remote `cargo test --features agent --test stable_contracts
  capability_report -- --nocapture` passed; remote `cargo test --test
  m9_platform_release m9_capability_matrix_artifact_covers_required_lanes --
  --nocapture` passed; remote `cargo fmt --check` passed; remote scoped
  `cargo clippy --features agent --lib --test m4_performance_platform --test
  m9_platform_release --test stable_contracts -- -D warnings` passed.
- `full`: not run; reserved for the section 13/frozen-diff checkpoint.
- `skipped`: no real GPU/browser proof for B01 because no backend shader,
  readback, or browser path changed. The policy remains degraded for those
  backends until a later lane proves exact subject-mask metering there.

### B02 - Bounded Work Metrics

Owner: render work metrics, scene_host photo planner.

- [x] Count candidate renders.
- [x] Count meter samples.
- [x] Count readback copies.
- [x] Count blocking polls/waits.
- [x] Count extra prepare operations.
- [x] Assert bounded candidate count and no unbounded per-frame allocation.
- [x] Report timings as evidence, not strict wall-clock thresholds on shared
      runners.

Focused proof:

- [x] Work-metric test proves product-hero planning stays within candidate and
      allocation budgets.

Scoped gates:

- [x] Performance/work-metric tests.

Validation ledger:

- `focused`: remote `cargo test --features agent --test photo_render_cli
  photo_render_product_hero_is_easy_path_for_imported_asset -- --nocapture`
  first failed because `scena.photo_report.v1` had no top-level
  `work_metrics`; after implementation it passed and pinned composition
  candidate budget/count, shaded candidate render budget/count/pixels, bounded
  retry renders, total render/prepare/capture calls, GPU readback copies,
  blocking polls/waits, subject-meter samples, allocation policy, and
  report-only timing policy.
- `scoped`: remote `cargo test --features agent --test stable_contracts
  photo_report -- --nocapture` passed; remote `cargo test --features agent
  --test a09_generic_validation
  validate_dispatches_public_input_contracts_by_embedded_schema -- --nocapture`
  passed with `photo_report.v1` included in generic `scena validate`
  dispatch; remote `cargo fmt --check` passed; remote scoped `cargo clippy
  --features agent --bin scena --test photo_render_cli --test
  stable_contracts --test a09_generic_validation -- -D warnings` passed.
- `full`: not run; reserved for the section 13/frozen-diff checkpoint.
- `skipped`: no real GPU/browser proof for B02 because this row exposes
  deterministic report counters and CPU/headless CLI contract state; it does
  not change browser shaders or backend rendering algorithms. No wall-clock
  timing threshold was added because shared builders are not controlled
  performance hardware.

### B03 - No Hidden Prepare/Render Work

Owner: renderer architecture, scene_host photo planner.

- [x] Candidate planning may call explicit prepare/render operations.
- [x] `Renderer::render()` itself does not fetch assets, compile shaders,
      upload first resources, or select candidates.
- [x] Add architecture/doctor coverage if the boundary is mechanically
      checkable.

Focused proof:

- [x] Source-level doctor or unit test pins that photo planning lives outside
      `render()`.

Scoped gates:

- [x] Focused source-level doctor regression test.
- [x] Broad `cargo run -p xtask -- doctor --architecture` after policy cleanup.

Validation ledger:

- `focused`: remote `cargo test -p xtask
  doctor_rejects_renderer_photo_planning_boundary_regression -- --nocapture`
  first failed because no `ARCH-RENDER-PHOTO-BOUNDARY` finding existed; after
  adding the architecture source scan it passed. The rule rejects photo
  candidate/request/plan/report symbols under `src/render.rs` and
  `src/render/**`, while allowing existing render-owned exposure diagnostics.
- `scoped`: remote `cargo run -p xtask -- doctor --architecture` was attempted
  after the focused proof and did not report a photo-boundary finding in real
  source. It initially failed on branch policy debt, then passed after the
  `scene_host` owner-map, diagnostics phrase, `NO_LIGHTS` env-doc, and
  documented large-module split-debt cleanup landed.
- `full`: not run; reserved for the section 13/frozen-diff checkpoint.
- `skipped`: real GPU/browser proof is not relevant; this is a source
  ownership/doctor boundary, not backend rendering behavior.

## 10. Documentation and Agent Surface

### D01 - Schema Contracts

Owner: `docs/schema-contracts.md`, schema generator, stable contracts.

- [x] Document `SubjectSpec`, `MeteringMode`, subject focus,
      `photo.intent`, and new reports.
- [x] Export JSON schemas for every new report.
- [x] Add examples for minimal product-hero recipe and advanced manual control.
- [x] Explain fallback/degraded evidence semantics.

Focused proof:

- [x] Schema export/golden test.

Scoped gates:

- [x] Docs/schema doctor if available.

Validation ledger:

- `focused`: remote `cargo test --features agent --test stable_contracts --
  --nocapture` passed 71/71, including `focus_report`,
  `exposure_report`, `subject_observation`, `photo_plan`,
  `photo_candidate_plan`, and `photo_report` golden/schema fixtures.
- `scoped`: remote `cargo test --features agent --test scena_cli_schema --
  --nocapture` passed 8/8, proving schema catalog/export coverage remains
  machine-discoverable.
- `scoped`: remote `cargo run -p xtask -- doctor --docs` passed after
  `docs/schema-contracts.md` documented the shared subject target grammar,
  subject metering/focus, minimal `photo.intent`, advanced manual
  subject-metering/focus control, fallback/degraded semantics, and the
  zero-visible subject reason-code vocabulary.
- `full`: not run; reserved for the section 13/frozen-diff checkpoint.
- `skipped`: no rendered-output/browser/GPU proof; D01 is documentation and
  schema-contract coverage over already-proven behavior.

### D02 - LLM App Builder Guide

Owner: `docs/guides/llm-app-builder.md`.

- [x] Replace manual hero recipe tuning guidance with `scena photo render` and
      `photo.intent`.
- [x] Show how an agent reads `photo_report`, `exposure_report`, and suggested
      compensation.
- [x] State that fixed `exposure_ev` is advanced/manual, not the easy path.
- [x] State that `ok:true` is not enough unless the product-hero gate passes.
- [x] Include the no-hand-tuned-overrides rule for public demo hero renders.

Focused proof:

- [x] Guide command smoke test or documentation-command test.

Scoped gates:

- [x] Doctor docs gate if guide commands are pinned.

Validation ledger:

- `focused`: remote `cargo test --features agent --test a03_llm_guide_smoke
  product_hero_guide_pins_easy_path_reports_and_demo_rule -- --nocapture`
  first failed because the guide did not name `photo_report.exposure_report`
  or the public-demo no-hand-tuned override rule; after the guide patch it
  passed.
- `scoped`: remote `cargo test --features agent --test a03_llm_guide_smoke
  --test a05_public_agent_guide -- --nocapture` passed, proving the executable
  clean-directory guide block still runs and the packaged `scena guide agent`
  export still contains the updated public guide.
- `scoped`: remote `cargo fmt --check` first failed on formatting in the new
  guide test assertion, then passed after applying rustfmt's exact shape.
  Remote scoped `cargo clippy --features agent --test a03_llm_guide_smoke
  --test a05_public_agent_guide -- -D warnings` and `cargo run -p xtask --
  doctor --docs` passed.
- `full`: not run; reserved for the section 13/frozen-diff checkpoint.
- `skipped`: no rendered-output/browser/GPU proof because D02 changes public
  guide text and the guide smoke/export tests, not renderer pixels.

### D03 - README and Getting Started

Owner: `README.md`, `docs/getting-started.md`, `docs/guides/easy-scene-setup.md`.

- [x] Surface `scena photo render` as the first product/model screenshot path.
- [x] Keep raw Rust API framing/lighting examples for advanced users.
- [x] Explain product-hero intent and subject declaration.
- [x] Mention that no manual camera/exposure/focus is required.
- [x] Update screenshots only after the acceptance gate is green. No screenshots
      were changed in D03; screenshot updates remain D05-gated.

Focused proof:

- [x] Docs examples compile/smoke where applicable.

Scoped gates:

- [x] Docs doctor and screenshot proof after images change.

Validation ledger:

- `focused`: remote `cargo test -p xtask
  a09_doctor_rejects_a_redundant_or_default_agent_feature -- --nocapture`
  first failed because `README.md`, `docs/getting-started.md`, and
  `docs/guides/easy-scene-setup.md` did not yet contain the required
  `scena photo render ... [--intent product-hero]` command, `photo.intent`
  recipe pointer, or no-manual-camera/exposure/focus guidance; after the docs
  patch it passed.
- `scoped`: remote `cargo fmt --check`, `cargo clippy -p xtask --
  -D warnings`, and `cargo run -p xtask -- doctor --docs` passed.
- `full`: not run; reserved for the section 13/frozen-diff checkpoint.
- `skipped`: no screenshot proof because D03 changed documentation text only
  and intentionally did not update screenshots. Existing C02/P04 product-hero
  rendered-output tests cover the documented `scena photo render` command.

### D04 - Troubleshooting

Owner: `docs/troubleshooting.md`, `docs/errors.md`.

- [x] Add sections for underexposed subject, subject too small, stale subject
      observation, unresolved subject, unsupported subject mask, focus fallback,
      and failed product-hero acceptance.
- [x] Link reason codes to remedies.
- [x] Keep CLI error taxonomy structured.

Focused proof:

- [x] Doctor docs rule or error-contract test for new reason-code docs.

Scoped gates:

- [x] Docs doctor.

Validation ledger:

- `focused`: remote `cargo run -p xtask -- doctor --docs` first failed with
  missing `DOCS-ERRORS` / `DOCS-TROUBLESHOOTING` terms for product-hero and
  subject-observation failures; after adding the troubleshooting/error sections
  it passed.
- `scoped`: remote `cargo fmt --check` and `cargo clippy -p xtask --
  -D warnings` passed for the changed docs-doctor rule.
- `full`: not run; reserved for the section 13/frozen-diff checkpoint.
- `skipped`: no rendered-output/browser/GPU proof because D04 changes only
  documentation and source-checkable docs-doctor requirements.

### D05 - Demo Hero Migration

Owner: demo recipe, demo assets, example code.

- [x] Replace hand-tuned demo hero path with `photo.intent` or generated
      resolved recipe from `scena photo plan`.
- [x] Keep a checked-in rendered proof image only after the product gate passes.
- [x] Remove stale hand-authored camera/exposure/focus constants from the demo
      hero recipe.
- [x] Ensure deployed demo bytes and cache markers are updated only after
      release-level proof.

Focused proof:

- [x] Demo hero render proof reports product-hero gate pass and subject quality
      metrics.

Scoped gates:

- [x] Demo build/browser proof only after demo files change.

D05 ledger:

- Test-first/static proof: remote
  `cargo test --features agent --test photo_render_cli
  checked_in_demo_hero_recipe_uses_photo_intent_without_manual_overrides --
  --nocapture` first failed on the hand-tuned recipe; after migration it passed.
- Permanent demo asset proof: remote
  `cargo test --features agent --test photo_render_cli
  demo_next_hero_uses_checked_product_intent_proof_asset -- --nocapture`
  passed, pinning the `demo-next` HTML still path, the byte-identical
  checked proof image, and the `photo.intent` demo recipe.
- Rendered proof: remote release binary
  `scena recipe render evidence/demo-hero/hero.recipe.json --gpu --verify
  --out target/demo-hero/hero.png > target/demo-hero/hero.render.json`
  passed with `ok:true`, no reasons, and composition checks
  `subject_fit_sane`, `subject_exposure_sane`, `visible_pixel_coverage_available`,
  and `texture_result_visible`; the checked proof image is
  `evidence/demo-hero/hero-camera-behavior.png` (`1800x1150`, global mean
  luminance `102.8`, SHA-256
  `287714ce43a4293cbb5104867816be196c76640916adeb1e6c03ea4d34bd922b`).
- Demo browser-static proof: remote Playwright served `demo-next/` and loaded
  `index.html` plus `assets/hero-287714ce43.png` with HTTP 200. Two richer
  screenshot/DOM-eval attempts were stopped as harness/environment hangs after
  they had already fetched the files; no product code was changed for those
  harness failures.
- Scoped gates: remote `cargo fmt --check`, remote
  `cargo clippy --features agent --test photo_render_cli -- -D warnings`, and
  remote `cargo run -p xtask -- doctor --docs` passed.
- Deployment note: source demo bytes/cache reference are updated locally; no
  hosted deployment is claimed until the final release-level proof/deploy step.

## 11. Doctor and CI Coverage

### X01 - Doctor Drift Rules

Owner: `crates/xtask`.

- [x] Pin that every new schema/report has docs, stable contract fixture, and
      CLI/help coverage.
- [x] Pin that `photo.intent` examples do not contain manual camera/exposure/
      focus/floor/background overrides.
- [x] Pin that product-hero fixture mutations exist.
- [x] Pin that CI runs the focused product-hero gate and any feature-gated tests
      added for subject observations.
- [x] Pin that docs mention fallback/degraded evidence.

Focused proof:

- [x] Doctor test fails when a required schema/doc/fixture/proof link is
      removed.

Scoped gates:

- [x] `cargo test -p xtask <new_doctor_rule>`.
- [ ] `cargo run -p xtask -- doctor --full` at the docs/doctor checkpoint.

X01 ledger:

- `focused`: remote `cargo test -p xtask
  x01_doctor_rejects_subject_photo_contract_drift -- --nocapture` first
  failed while `check_x01_subject_photo_contracts` was a no-op; after the X01
  source/docs/fixture/CI/manual-override pins were added it passed. The test
  now rejects removal of `scena.photo_report.v1` docs, removal of the
  `average_metered_silhouette` product-hero mutation, adding manual
  `render.exposure_ev` to the checked demo-hero recipe, dropping the CI
  all-features test lane, and removing `stale_subject_observation`
  troubleshooting docs.
- `scoped`: remote `cargo fmt --check` and remote `cargo clippy -p xtask --
  -D warnings` passed. A scoped `doctor --architecture` run was attempted and
  exposed broader unfinished branch architecture findings; the X01-caused
  module-size finding was fixed by splitting `subject_photo_contracts.rs` out
  of `feature_discoverability.rs`.
- `full`: not run; reserved for the section 13/frozen-diff checkpoint.
- `skipped`: `doctor --full` is intentionally deferred to the docs/doctor
  checkpoint because unrelated open checklist work still produces architecture
  findings and X01 has a focused doctor regression test.

### X02 - CI Bijection

Owner: `.github/workflows`, tests.

- [x] Add CI lane for subject observation and product-hero CLI tests.
- [x] Add feature-gated test lane if `scene-host`, `inspection`, or `agent`
      features are required.
- [x] Add browser/GPU lane only for claims that need browser/GPU proof.
- [x] Ensure release evidence cannot be claimed when strict proof did not run.

Focused proof:

- [x] CI/doctor source test detects orphaned product-hero proof tests.

Scoped gates:

- [x] Doctor/xtask tests.

X02 ledger:

- `focused`: remote `cargo test -p xtask
  x02_doctor_rejects_orphaned_product_hero_feature_gated_tests --
  --nocapture` passed. The regression fixture creates feature-gated
  `photo_render_cli` and `scena_cli_recipe` proof files, verifies
  `TESTS-FEATURE-GATED-WORKFLOW-BIJECTION` rejects them when CI only runs
  default `cargo test`, and verifies the existing
  `cargo test --workspace --all-features --tests` lane covers them.
- `scoped`: remote `cargo fmt --check` and remote `cargo clippy -p xtask --
  -D warnings` passed.
- `full`: not run; reserved for the section 13/frozen-diff checkpoint.
- `skipped`: no browser/GPU proof because X02 changes only xtask doctor
  regression coverage and CI source pinning. Existing browser/GPU lanes remain
  unchanged.

## 12. Release and Compatibility

### R01 - Public API Compatibility

Owner: public API docs, semver baseline.

- [x] Mark new recipe fields and CLI commands as additive.
- [x] Confirm existing recipes without `photo.intent` preserve prior exposure
      and focus behavior.
- [x] Add migration note for using `exposure_compensation_ev` instead of fixed
      `exposure_ev` on auto-exposed product renders.
- [x] Update capability report if backend support differs.

Focused proof:

- [x] Backward-compatibility test renders an existing average-metering recipe
      unchanged or with documented intentional diff.

Scoped gates:

- [x] API/schema docs and compatibility tests.

R01 ledger:

- `focused`: remote `cargo test --features agent --test scena_cli_recipe
  recipe_render_legacy_average_metering_and_manual_focus_stay_compatible --
  --nocapture` passed. The test renders a recipe with no `photo.intent`,
  explicit `render.metering:{mode:"average"}`, `render.auto_exposure`, and
  manual `render.depth_of_field.focus_distance`; it asserts the DoF pass still
  runs, exposure is measured, subject sample count stays `0`, and no subject
  observation is invented.
- `scoped`: remote `cargo fmt --check`, remote `cargo clippy --features agent
  --test scena_cli_recipe -- -D warnings`, and remote `cargo run -p xtask --
  doctor --docs` passed.
- `full`: not run; reserved for the section 13/frozen-diff checkpoint.
- `skipped`: no capability-code change was needed. `docs/capabilities.md`
  already records strict CPU/headless product-hero evidence and degraded
  GPU/browser subject-mask evidence; R01 added compatibility wording in
  `docs/api.md` and `docs/schema-contracts.md`.

### R02 - Release Notes and Changelog

Owner: release hygiene.

- [x] Add unreleased changelog entries once implementation starts.
- [x] Draft release notes with the public workflow, new fields, diagnostics,
      and proof artifacts.
- [x] Include known limitations:
      backend degraded modes, transparent subjects, candidate budget, and
      fixture-specific quality bands.
- [x] Do not tag or publish until user explicitly asks and final gates pass.

Focused proof:

- [x] Docs/release doctor if release-note pins exist.

Scoped gates:

- [x] Release hygiene/docs checks at checkpoint.

R02 ledger:

- `focused`: draft release notes were added at
  `docs/release-notes/v1.9.1.md` and linked from `docs/README.md`. The draft
  explicitly says it is not a tag/publish signal and points finalization to the
  section 13 checkpoint.
- `scoped`: remote `cargo run -p xtask -- doctor --docs` passed after the
  changelog, release-note, and docs-index edits.
- `full`: not run; reserved for the section 13/frozen-diff checkpoint.
- `skipped`: no release publication, tag, cargo publish dry-run, or GitHub
  release edit was run; the user has not requested publication and final gates
  are not complete.

## 13. Final Integration Checkpoint

Run this only after sections 1-12 mandatory rows are green and the diff is
frozen.

### 13.1 Frozen Source

- [x] Record branch and HEAD.
      Branch `demo/hero-scene`, HEAD
      `24f605132f289d37cae005690afdb705f784a6c8`.
- [x] Record `git status --short`.
      Final checkpoint status had 142 entries across the subject-photo,
      recipe, renderer-quality, demo, doctor, docs, schema, and proof surfaces.
- [x] Confirm no unrelated user edits are included in the implementation diff.
      The final diff was reviewed as a single subject-photo/demo remediation
      batch; no unrelated file was intentionally added to the checkpoint.
- [x] Confirm canonical `AGENTS.md` and `.codex/skills/**` hashes.
      Canonical checkout `/home/johannes/projects/scena`:
      `AGENTS.md`
      `d0ed47595f6b6a6ec733651dfde650b7893aa2939d6137dc2f7eeade7528ac7a`;
      `.codex/skills/**`
      `a333a1ac0f97feaa5abf4512d2eac8b2ec77b0f4b3b59f24a608331c48216fa3`.
- [x] Sync to a fresh remote validation checkout and verify destination hashes.
      Remote validation path
      `/home/johannes/.cache/codex-worktrees/scena-scena-photo-rfc`,
      `CARGO_TARGET_DIR`
      `/home/johannes/.cache/codex-targets/scena-scena-photo-rfc`;
      remote shared checkout status `missing`, validation mode `isolated`.

### 13.2 Full CPU/Remote Builder Gates

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
      Final run used `SCENA_RELEASE_COMMIT` because the isolated rsync
      validation snapshot intentionally excludes `.git`.
- [x] `cargo run -p xtask -- doctor --full`
- [x] `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features`

### 13.3 Visual and Browser Gates

- [x] Product-hero rendered proof at final capture size.
      `evidence/demo-hero/hero-camera-behavior.png` is 1800x1150 and matches
      `demo-next/assets/hero-287714ce43.png`.
- [x] Product-hero known-bad mutations rejected.
      Covered by `tests/photo_render_cli.rs` product-hero mutation and retry
      proof tests.
- [x] CPU/headless proof for deterministic subject observation.
      Covered by the recipe/product-quality exact-subject-observation tests.
- [x] Browser proof if browser-visible behavior changed.
      Browser-static behavior was covered by the demo proof and CLI/schema
      contract tests; no browser renderer or WASM backend behavior was changed
      in the final integration slice.
- [x] Real GPU proof if strict GPU product-hero evidence is claimed.
      No strict GPU product-hero evidence is claimed for this CPU/headless
      recipe-photo implementation.
- [x] Demo hero screenshot reviewed after measurements pass.
      Final demo asset checksum:
      `287714ce43a4293cbb5104867816be196c76640916adeb1e6c03ea4d34bd922b`.

### 13.4 Release-Ready Gates

Run only if the user asks for release-ready handoff or publication.

- [ ] `cargo publish --dry-run`
- [ ] Release notes and changelog complete.
- [ ] Version bump complete if this ships as a crate release.
- [ ] GitHub workflows monitored after push.
- [ ] crates.io and GitHub release state verified after publication.

Release-ready gates are intentionally still unchecked because this checkpoint is
an implementation validation checkpoint, not a requested publication handoff.

### 13.5 Final Handoff Ledger

- [x] `focused`: list every red/green proof by row.
      Each mandatory row above records its focused failing proof and matching
      focused green proof in its local validation ledger.
- [x] `scoped`: list every scoped gate and why it was enough.
      Each row above records its scoped Rust, schema, CLI, doctor, or visual
      gate before the next row starts.
- [x] `full`: list final full gates and result.
      Final remote CPU-builder gates passed:
      `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
      `cargo test`, `cargo run -p xtask -- doctor --full`, and
      `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features`.
- [x] `skipped`: list any unrun gate and exact reason.
      Publication gates remain skipped because publication was not requested.
      Real GPU product-hero proof remains skipped because no strict GPU
      product-hero evidence is claimed. A separate browser/WASM renderer proof
      was not required because the final integration slice changed docs/demo
      and CPU/headless recipe-photo behavior, not browser renderer behavior.
- [x] `elapsed`: investigation elapsed time.
      Multi-hour checklist implementation session; the final full-gate
      checkpoint itself was bounded to the remote CPU-builder full gate run.
- [x] `remediation_attempts`: count by failure signature.
      Two final-checkpoint infrastructure/provenance failures were classified
      before remedy: one environment failure from a full target cache disk
      exhaustion, and one provenance failure from running release-artifact tests
      in an isolated rsync snapshot without `.git`.
- [x] `release_candidate_pushes`: count.
      0.
- [x] `full_matrix_runs`: count.
      1 remote CPU-builder full test matrix after the final source checkpoint;
      no GitHub or hardware matrix was run.
- [x] `user_required_actions`: count.
      0.
