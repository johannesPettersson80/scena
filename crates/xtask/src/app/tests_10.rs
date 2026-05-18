use crate::app::prelude::*;

#[test]
pub(crate) fn binary_render_asset_contracts_are_source_enforced() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let mut findings = Vec::new();

    check_binary_render_asset_contracts(&root, &mut findings);

    assert_eq!(findings, Vec::new());
}

#[test]
pub(crate) fn binary_render_asset_contracts_reject_text_fixtures_with_binary_extensions() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-binary-asset-contract-test");
    let fixture_dir = fixture_root.join("tests/assets/environment/generated");
    fs::create_dir_all(&fixture_dir).expect("fixture dir");
    fs::write(
        fixture_dir.join("fake.ktx2"),
        b"SCENA_CUBEMAP_V1\nencoding = rgba16f-text-fixture\n",
    )
    .expect("fixture write");
    let mut findings = Vec::new();

    check_binary_render_asset_contracts(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "BINARY-ASSET-TRUTH-P9"
                && finding.message.contains("fake.ktx2")
                && finding.message.contains("text fixture data")
        }),
        "text fixtures must not be allowed to masquerade as binary render assets: {findings:?}",
    );
}

#[test]
pub(crate) fn m7_ergonomics_contracts_are_source_enforced() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let mut findings = Vec::new();

    check_m7_ergonomics_contracts(&root, &mut findings);

    assert_eq!(findings, Vec::new());
}

#[test]
pub(crate) fn easy_scene_setup_contracts_are_source_enforced() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let mut findings = Vec::new();

    check_easy_scene_setup_contracts(&root, &mut findings);

    assert_eq!(findings, Vec::new());
}

#[test]
pub(crate) fn easy_scene_setup_contracts_reject_inline_look_from_literal_regression() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/easy-scene-inline-look-from");
    write_minimal_easy_scene_fixture(
        &fixture_root,
        "frame_bounds(()) bounds_for_transforms add_grid_floor FramingOptions::new().look_from(Vec3::new(-0.4398, 0.3051, 0.8447))",
    );
    let mut findings = Vec::new();

    check_easy_scene_setup_contracts(&fixture_root, &mut findings);

    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "DEMO-CAMERA-VIEWS-NAMED"),
        "doctor must reject inline Vec3 literal look_from camera views in the demo: {findings:?}",
    );
}

#[test]
pub(crate) fn easy_scene_setup_contracts_allow_azimuth_elevation_camera_view() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root =
        root.join("target/xtask-doctor-regressions/easy-scene-azimuth-elevation-view");
    write_minimal_easy_scene_fixture(
        &fixture_root,
        "frame_bounds(()) bounds_for_transforms add_grid_floor FramingOptions::new().azimuth_elevation(-27.5, 17.8)",
    );
    let mut findings = Vec::new();

    check_easy_scene_setup_contracts(&fixture_root, &mut findings);

    assert_eq!(findings, Vec::new());
}

#[test]
pub(crate) fn easy_scene_setup_contracts_reject_post_framing_angle_patch() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/easy-scene-angle-patch");
    write_minimal_easy_scene_fixture(
        &fixture_root,
        "frame_bounds(()) bounds_for_transforms add_grid_floor .focus_on_framing(framing).with_angles(-0.4, 0.3)",
    );
    let mut findings = Vec::new();

    check_easy_scene_setup_contracts(&fixture_root, &mut findings);

    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "DEMO-CAMERA-VIEWS-NAMED"),
        "doctor must reject .with_angles() pose patches after focus_on_framing(): {findings:?}",
    );
}

#[test]
pub(crate) fn easy_scene_setup_contracts_reject_open_diagnostics_and_public_frame_text() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/easy-scene-diagnostics");
    write_minimal_easy_scene_fixture(
        &fixture_root,
        "frame_bounds(()) bounds_for_transforms add_grid_floor",
    );
    fs::write(
        fixture_root.join("demo/index.html"),
        r#"<details id="diagnostics" class="diagnostics" open><strong id="metric-frame">0</strong></details>"#,
    )
    .expect("demo html fixture");
    fs::write(
        fixture_root.join("demo/main.js"),
        "setStatus(activeAsset.label, `frame ${frameCount}`);",
    )
    .expect("demo js fixture");
    let mut findings = Vec::new();

    check_easy_scene_setup_contracts(&fixture_root, &mut findings);

    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "DEMO-DIAGNOSTICS"),
        "doctor must reject public frame-counter diagnostics regressions: {findings:?}",
    );
}

fn write_minimal_easy_scene_fixture(fixture_root: &Path, demo_page_rs: &str) {
    let _ = fs::remove_dir_all(fixture_root);
    fs::create_dir_all(fixture_root.join("src/demo_page")).expect("fixture demo_page");
    fs::create_dir_all(fixture_root.join("docs/guides")).expect("fixture guides");
    fs::create_dir_all(fixture_root.join("docs/release-notes")).expect("fixture release notes");
    fs::create_dir_all(fixture_root.join("demo")).expect("fixture demo");
    fs::create_dir_all(fixture_root.join("src/scene")).expect("fixture scene");
    fs::create_dir_all(fixture_root.join("src/geometry")).expect("fixture geometry");
    fs::create_dir_all(fixture_root.join("tests")).expect("fixture tests");
    fs::write(
        fixture_root.join("docs/guides/easy-scene-setup.md"),
        "frame_bounds add_studio_lighting add_grid_floor set_auto_exposure scene.mate project_world_point Camera views azimuth_elevation three_quarter_front_right\n```rust\nlet mut scene = Scene::new();\nscene.add_studio_lighting()?;\nscene.add_grid_floor(&assets, GridFloorOptions::new())?;\nscene.frame_bounds(camera, bounds, FramingOptions::new().azimuth_elevation(-27.5, 17.8))?;\n```",
    )
    .expect("guide fixture");
    fs::write(
        fixture_root.join("docs/guides/migrating-from-threejs.md"),
        "new THREE.Box3 controls.target.copy OrbitControls::from_framing spherical.theta spherical.phi azimuth_elevation",
    )
    .expect("migration fixture");
    fs::write(
        fixture_root.join("docs/release-notes/v1.3.0.md"),
        "Status: ready OrbitControls::from_framing Aabb::union ScreenRect ProjectedPoint GridFloorHandles LookupError::InvalidBounds LookupError::UnsupportedCameraType FramingOptions::azimuth_elevation FramingOptions::front FramingOptions::back FramingOptions::left FramingOptions::right FramingOptions::top FramingOptions::bottom FramingOptions::three_quarter_front_left FramingOptions::three_quarter_front_right FramingOptions::three_quarter_back_left FramingOptions::three_quarter_back_right",
    )
    .expect("release notes fixture");
    fs::write(
        fixture_root.join("docs/README.md"),
        "Easy scene setup guides/easy-scene-setup.md",
    )
    .expect("docs readme fixture");
    fs::write(
        fixture_root.join("README.md"),
        "## Easy Scene Setup\ndocs/guides/easy-scene-setup.md docs/release-notes/v1.3.0.md",
    )
    .expect("readme fixture");
    fs::write(fixture_root.join("src/demo_page.rs"), demo_page_rs).expect("demo fixture");
    fs::write(
        fixture_root.join("src/demo_page/connectors.rs"),
        "project_world_point",
    )
    .expect("connector projection fixture");
    fs::write(
        fixture_root.join("src/diagnostics.rs"),
        "InvalidBounds InvalidFramingOption UnsupportedCameraType A viewport width or height was zero Bounds were empty A named framing option failed validation does not support the camera type",
    )
    .expect("diagnostics fixture");
    fs::write(
        fixture_root.join("src/scene/framing.rs"),
        "pre-existing aspect # Examples # Errors LookupError::UnsupportedCameraType LookupError::InvalidFramingOption",
    )
    .expect("framing fixture");
    fs::write(fixture_root.join("src/scene/lights.rs"), "studio docs").expect("lights fixture");
    fs::write(fixture_root.join("src/lib.rs"), "").expect("lib fixture");
    fs::write(fixture_root.join("src/geometry.rs"), "").expect("geometry fixture");
    fs::write(fixture_root.join("src/geometry/bounds.rs"), "").expect("bounds fixture");
    fs::write(
        fixture_root.join("tests/examples_visual_proof.rs"),
        "frame_bounds_rendered_output_proves_fill_center_and_unclipped_object frame-bounds-rendered-output computed_distance projected_rect nonblack_pixel_rect",
    )
    .expect("visual proof fixture");
    fs::write(
        fixture_root.join("demo/index.html"),
        r#"<details id="diagnostics" class="diagnostics"><strong id="metric-frame">0</strong></details>"#,
    )
    .expect("demo html fixture");
    fs::write(
        fixture_root.join("demo/main.js"),
        "setStatus('demo', 'rendered');",
    )
    .expect("demo js fixture");
}

#[test]
pub(crate) fn demo_build_heartbeat_contract_is_source_enforced() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let mut findings = Vec::new();

    check_demo_build_heartbeat_contract(&root, &mut findings);

    assert_eq!(findings, Vec::new());
}

#[test]
pub(crate) fn demo_build_heartbeat_contract_rejects_direct_wasm_pack_script() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/direct-demo-wasm-pack");
    fs::create_dir_all(fixture_root.join("scripts")).expect("fixture scripts");
    fs::write(
        fixture_root.join("package.json"),
        r#"{"scripts":{"demo:build":"wasm-pack build --release --target web --out-dir demo/pkg . --features demo-page"}}"#,
    )
    .expect("package fixture");
    fs::write(
        fixture_root.join("scripts/build_demo_wasm.js"),
        "wasm-pack\n",
    )
    .expect("script fixture");
    let mut findings = Vec::new();

    check_demo_build_heartbeat_contract(&fixture_root, &mut findings);

    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "DEMO-BUILD-HEARTBEAT"),
        "doctor must reject a silent direct wasm-pack demo build script: {findings:?}",
    );
}

#[test]
pub(crate) fn public_fields_in_struct_detects_material_desc_visibility_regressions() {
    let source = r#"
        pub struct MaterialDesc {
            kind: MaterialKind,
            pub base_color: Color,
            pub(crate) roughness_factor: f32,
        }
    "#;

    assert_eq!(
        public_fields_in_struct(source, "MaterialDesc"),
        vec!["pub base_color: Color", "pub(crate) roughness_factor: f32"]
    );
}

/// Plan line 1588 / Phase 4: doctor rule regression coverage. Each of the
/// following tests writes a fixture whose required documentation
/// substring is missing and asserts the matching DOCS-* rule fires.
/// Closes the regression gap for the documentation contracts that were
/// previously enforced only by the live tree.
#[test]
pub(crate) fn doctor_rejects_render_lifecycle_doc_missing_substring_regression() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/lifecycle-doc-stub");
    let doc_path = fixture_root.join("docs/lifecycle.md");
    fs::create_dir_all(doc_path.parent().expect("lifecycle parent")).expect("fixture dir");
    fs::write(
        &doc_path,
        "# Render lifecycle\n\nThis stub deliberately omits the contract substrings.\n",
    )
    .expect("lifecycle fixture");
    let mut findings = Vec::new();

    check_required_doc_contracts(&fixture_root, &mut findings);

    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "DOCS-LIFECYCLE"),
        "doctor must reject docs/lifecycle.md when the contract substrings \
         are missing: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_asset_gltf_doc_missing_substring_regression() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/gltf-doc-stub");
    let doc_path = fixture_root.join("docs/assets.md");
    fs::create_dir_all(doc_path.parent().expect("gltf parent")).expect("fixture dir");
    fs::write(
        &doc_path,
        "# glTF contract\n\nStub that omits the connector and stale-import contract terms.\n",
    )
    .expect("gltf fixture");
    let mut findings = Vec::new();

    check_required_doc_contracts(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| finding.rule == "DOCS-GLTF"),
        "doctor must reject docs/assets.md when its required \
         substrings are missing: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_visual_quality_doc_missing_substring_regression() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/visual-doc-stub");
    let doc_path = fixture_root.join("docs/headless-rendering.md");
    fs::create_dir_all(doc_path.parent().expect("visual parent")).expect("fixture dir");
    fs::write(
        &doc_path,
        "# Visual quality\n\nStub without color management or determinism clauses.\n",
    )
    .expect("visual fixture");
    let mut findings = Vec::new();

    check_required_doc_contracts(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| finding.rule == "DOCS-VISUAL"),
        "doctor must reject docs/headless-rendering.md when its required \
         substrings are missing: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_platform_doc_missing_substring_regression() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/platform-doc-stub");
    let doc_path = fixture_root.join("docs/platforms.md");
    fs::create_dir_all(doc_path.parent().expect("platform parent")).expect("fixture dir");
    fs::write(
        &doc_path,
        "# Platforms\n\nStub without the required browser backend substrings.\n",
    )
    .expect("platform fixture");
    let mut findings = Vec::new();

    check_required_doc_contracts(&fixture_root, &mut findings);

    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "DOCS-PLATFORM"),
        "doctor must reject docs/platforms.md when its required platform \
         substrings are missing: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_errors_doc_missing_substring_regression() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/errors-doc-stub");
    let doc_path = fixture_root.join("docs/errors.md");
    fs::create_dir_all(doc_path.parent().expect("errors parent")).expect("fixture dir");
    fs::write(
        &doc_path,
        "# Errors\n\nStub without the renderer error contract terms.\n",
    )
    .expect("errors fixture");
    let mut findings = Vec::new();

    check_required_doc_contracts(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| finding.rule == "DOCS-ERRORS"),
        "doctor must reject docs/errors.md when its required substrings \
         are missing: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_public_api_doc_missing_substring_regression() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/public-api-doc-stub");
    let doc_path = fixture_root.join("docs/api.md");
    fs::create_dir_all(doc_path.parent().expect("public-api parent")).expect("fixture dir");
    fs::write(
        &doc_path,
        "# Public API\n\nStub without the public-API surface contract terms.\n",
    )
    .expect("public-api fixture");
    let mut findings = Vec::new();

    check_required_doc_contracts(&fixture_root, &mut findings);

    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "DOCS-PUBLIC-API"),
        "doctor must reject docs/api.md when the public-API surface \
         contract substrings are missing: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_required_docs_missing_file_regression() {
    // DOCS-REQUIRED: the doctor walks `REQUIRED_DOCS` and asserts every
    // file is present + non-empty. A fixture missing one such doc
    // regresses the rule. Picking `docs/decisions/ADR-0005-local-release-candidate-deferrals.md`
    // because it is part of the canonical required set and silent
    // deletion would breach the local-release-candidate paperwork.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/required-doc-missing");
    // Empty fixture root → every required doc is missing.
    fs::create_dir_all(&fixture_root).expect("fixture dir");
    let mut findings = Vec::new();

    require_files(&fixture_root, &mut findings, "DOCS-REQUIRED", REQUIRED_DOCS);

    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "DOCS-REQUIRED"),
        "doctor must reject the repo when REQUIRED_DOCS files are missing: \
         {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_m5_release_cargo_missing_metadata_regression() {
    // ARCH-M5-RELEASE: Cargo.toml must keep the rust-version, docs.rs
    // documentation pointer, keywords, categories, include list, and
    // hybrid `["rlib", "cdylib"]` crate type. A stub manifest without
    // those entries regresses the v1 release contract.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/m5-release-cargo-stub");
    let manifest_path = fixture_root.join("Cargo.toml");
    fs::create_dir_all(manifest_path.parent().expect("manifest parent")).expect("fixture dir");
    fs::write(
        &manifest_path,
        "[package]\nname = \"scena\"\nversion = \"0.0.0\"\n",
    )
    .expect("manifest fixture");
    let mut findings = Vec::new();

    require_contains(
        &fixture_root,
        &mut findings,
        "ARCH-M5-RELEASE",
        "Cargo.toml",
        &[
            "version = \"1.3.0\"",
            "rust-version = ",
            "documentation = \"https://docs.rs/scena\"",
            "keywords = [",
            "categories = [",
            "include = [",
            "crate-type = [\"rlib\", \"cdylib\"]",
        ],
    );

    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "ARCH-M5-RELEASE"),
        "doctor must reject Cargo.toml stubs that drop the v1 release-metadata \
         surface: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_visual_browser_m1_missing_artifact_regression() {
    // VISUAL-BROWSER-M1: each browser-probe workflow must declare its
    // visual artifact under `target/gate-artifacts/m6-browser-visual/`
    // with a renderer/color/tolerance/source contract; absence regresses
    // the M6 browser parity gate.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/visual-browser-m1-stub");
    let stub_path = fixture_root.join("src/browser_probe/workflows/pbr.rs");
    fs::create_dir_all(stub_path.parent().expect("workflow parent")).expect("fixture dir");
    fs::write(
        &stub_path,
        "// Stub workflow without the visual-artifact declarations.\n",
    )
    .expect("workflow fixture");
    let mut findings = Vec::new();

    require_contains(
        &fixture_root,
        &mut findings,
        "VISUAL-BROWSER-M1",
        "src/browser_probe/workflows/pbr.rs",
        &["pbr-environment-lit", "renderer", "tolerance"],
    );

    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "VISUAL-BROWSER-M1"),
        "doctor must reject browser-probe workflows that drop their visual \
         artifact declarations: {findings:?}",
    );
}
