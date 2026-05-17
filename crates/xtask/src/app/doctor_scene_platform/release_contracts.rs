use crate::app::prelude::*;

pub(crate) fn check_m5_release_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_files(root, findings, "ARCH-M5-RELEASE", REQUIRED_EXAMPLES);
    require_files(
        root,
        findings,
        "ARCH-M5-RELEASE",
        REQUIRED_M5_GATE_ARTIFACTS,
    );
    require_contains(
        root,
        findings,
        "ARCH-M5-RELEASE",
        "Cargo.toml",
        &[
            "version = \"1.2.0\"",
            "rust-version = ",
            "documentation = \"https://docs.rs/scena\"",
            "keywords = [",
            "categories = [",
            "include = [",
            "\"/src/**\"",
            "\"/README.md\"",
            "\"/CHANGELOG.md\"",
            "\"/Cargo.toml\"",
            "crate-type = [\"rlib\", \"cdylib\"]",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M5-RELEASE",
        "src/diagnostics.rs",
        &[
            "pub enum DebugOverlay",
            "RendererChanged",
            "DebugOverlay",
            "pub struct RendererStats",
            "pub enum BuildError",
            "pub enum AssetError",
            "pub enum ImportError",
            "pub enum InstantiateError",
            "pub enum PrepareError",
            "pub enum RenderError",
            "pub enum LookupError",
            "pub enum AnimationError",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M5-RELEASE",
        "src/render/settings.rs",
        &[
            "pub fn debug_overlay",
            "pub fn set_debug",
            "pub fn set_debug_overlay",
            "debug_revision",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M5-RELEASE",
        "src/render.rs",
        &[
            "debug_revision",
            "NotPreparedReason::RendererChanged",
            "ChangeKind::DebugOverlay",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M5-RELEASE",
        "src/bin/scena-convert.rs",
        &[
            "scena-convert",
            "FBX to glTF",
            "FBX2glTF",
            "--dry-run",
            "planned",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M5-RELEASE",
        "docs/api.md",
        &[
            "Renderer::prepare",
            "Renderer::render",
            "Renderer::set_debug",
            "Renderer::set_debug_overlay",
            "Renderer::capability_report",
            "Renderer::gpu_adapter_report",
            "CapabilityReport",
            "AssetLoadOptions",
            "DebugOverlay",
            "RendererStats",
            "GpuAdapterReport",
            "AdapterLimitsReport",
            "BuildError",
            "RenderError",
            "SceneImport",
            "AnchorFrame",
            "ConnectorFrame",
            "ConnectorMetadata",
            "ConnectionAlignment",
            "ConnectionRoll",
            "ConnectionLineOverlay",
            "ConnectorRollPolicy",
            "ConnectorPolarity",
            "Scene::connect_import_connectors",
            "AnchorKey",
            "ConnectorKey",
            "InteractiveGltfViewer",
            "InteractiveGltfViewerBuilder",
            "interactive_gltf_viewer(path, surface)",
            "InteractiveGltfViewer::handle_surface_event",
            "Renderer::headless_default()",
            "Scene::with_default_camera()",
            "AssetStoreId",
            "Assets::store_id()",
            "Assets::load_scene_with_options()",
            "Assets::load_scene_with_report_options()",
            "Assets::contains_geometry",
            "Assets::contains_material",
            "Assets::contains_texture",
            "Assets::contains_environment",
            "Assets::release_unreferenced",
            "AssetEvictionStats",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M5-RELEASE",
        "docs/api.md",
        &[
            "Use this page as the conceptual map.",
            "Additive public API changes in 1.2.0:",
            "BuildError",
            "AnimationError",
            "MaterialTextureMissingDecodedPixels",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M5-RELEASE",
        "tests/m5_release.rs",
        &[
            "m5_debug_overlay_api_is_public_and_requires_prepare_after_change",
            "m5_public_api_baseline_names_frozen_contracts",
            "m5_benchmark_report_writes_required_scene_rows",
            "scena_convert_cli_reports_fbx_to_gltf_plan",
            "m5-benchmarks",
            "m5-public-api-freeze",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M5-RELEASE",
        "docs/specs/release-gates.md",
        &[
            "m5-benchmarks.json",
            "m5-public-api-freeze.json",
            "cargo check --examples",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M5-RELEASE",
        "docs/checklists/m5-v1-release.md",
        &[
            "m5_release",
            "m5-benchmarks.json",
            "m5-public-api-freeze.json",
            "cargo check --examples",
            "cargo publish --dry-run",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M5-RELEASE",
        "docs/checklists/acceptance-index.md",
        &[
            "m5-benchmarks.json",
            "m5-public-api-freeze.json",
            "cargo check --examples",
            "cargo publish --dry-run",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M5-RELEASE",
        "target/gate-artifacts/m5-benchmarks.json",
        &[
            "\"gate\": \"m5-benchmarks\"",
            "\"status\": \"passed\"",
            "static-viewer",
            "standard-model-viewer-gltf",
            "larger-industrial-gltf",
            "high-instance",
            "headless-4k",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M5-RELEASE",
        "target/gate-artifacts/m5-public-api-freeze.json",
        &[
            "\"gate\"",
            "m5-public-api-freeze",
            "\"status\"",
            "passed",
            "\"baseline\"",
        ],
    );
}

pub(crate) const REQUIRED_EXAMPLES: &[&str] = &[
    "examples/primitive_shapes.rs",
    "examples/glb_model_viewer.rs",
    "examples/picking_selection_hover.rs",
    "examples/instancing.rs",
    "examples/labels_helpers.rs",
    "examples/animation.rs",
    "examples/native_window.rs",
    "examples/browser_canvas.rs",
    "examples/headless_ci.rs",
    "examples/industrial_static_scene.rs",
    "examples/industrial_connector_assembly.rs",
    "examples/coordinate_connector_repair.rs",
];

pub(crate) const REQUIRED_M5_GATE_ARTIFACTS: &[&str] = &[
    "target/gate-artifacts/m5-benchmarks.json",
    "target/gate-artifacts/m5-public-api-freeze.json",
];
