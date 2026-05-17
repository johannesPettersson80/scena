use crate::app::prelude::*;

pub(crate) fn check_m3a_loading_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "Cargo.toml",
        &[
            "base64",
            "serde_json",
            "wasm-bindgen-futures",
            "Response",
            "obj = []",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/assets.rs",
        &[
            "mod fetch;",
            "mod gltf;",
            "mod obj;",
            "pub use fetch::{AssetFetcher, DefaultAssetFetcher}",
            "pub use gltf::{",
            "SceneAssetMesh",
            "scene_lookup: BTreeMap<AssetPath, SceneAsset>",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/assets/scene_loading.rs",
        &[
            "pub async fn load_scene",
            "pub async fn reload_scene",
            "RetainPolicy::Always",
            "ReloadRequiresRetain",
            "retained_source_bytes()",
            "with_retained_source_bytes",
            "SceneAsset::from_gltf_bytes",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/assets/fetch.rs",
        &[
            "pub trait AssetFetcher",
            "pub type DefaultAssetFetcher",
            "pub struct FileAssetFetcher",
            "pub struct BrowserAssetFetcher",
            "window.fetch_with_str",
            "wasm_bindgen_futures::JsFuture",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/assets/obj.rs",
        &[
            "pub async fn load_geometry",
            "parse_obj_geometry",
            "mtllib",
            "GeometryTopology::Triangles",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/assets/gltf.rs",
        &[
            "pub(super) fn from_gltf_bytes",
            "pub(super) fn from_gltf_bytes_with_external_resources",
            "pub(super) fn external_buffer_paths",
            "pub(super) fn external_image_paths",
            "open_gltf_with_massage",
            "Gltf::from_slice_without_validation",
            "parse_punctual_lights",
            "parse_gltf_clips",
            "parse_node_anchors",
            "parse_node_connectors",
            "from_gltf_transform",
            "UnsupportedRequiredExtension",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/assets/gltf/scene_asset.rs",
        &[
            "pub struct SceneAsset",
            "pub struct SceneAssetClip",
            "pub struct SceneAssetLight",
            "pub struct SceneAssetNode",
            "pub struct SceneAssetMesh",
            "pub fn mesh_count",
            "pub fn retained_source_bytes_len",
            "pub(in crate::assets) fn retained_source_bytes",
            "pub(in crate::assets) fn with_retained_source_bytes",
            "pub fn transform(&self)",
            "pub fn mesh(&self)",
            "pub fn meshes(&self)",
            "pub fn anchors(&self)",
            "pub fn connectors(&self)",
            "pub fn clips(&self)",
            "pub fn light(&self)",
            "pub const fn bounds",
            "pub const fn uses_vertex_colors",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/assets/gltf/anchors.rs",
        &[
            "pub struct SceneAssetAnchor",
            "pub(crate) fn invalid_reason",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/assets/gltf/connectors.rs",
        &[
            "pub struct SceneAssetConnector",
            "pub(crate) fn invalid_reason",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/assets/gltf/extensions.rs",
        &[
            "pub enum GltfExtensionStatus",
            "pub struct GltfExtensionDiagnostic",
            "pub(super) fn is_v1_required_gltf_extension",
            "pub(super) fn collect_extension_diagnostics",
            "KHR_lights_punctual",
            "KHR_materials_unlit",
            "KHR_materials_emissive_strength",
            "KHR_texture_transform",
            "KHR_mesh_quantization",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/assets/gltf/transform.rs",
        &[
            "from_gltf_transform",
            "GltfTransform",
            "\"matrix\"",
            "matrix_transform",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/assets/gltf/anchors.rs",
        &[
            "pub(super) fn parse_node_anchors",
            "validate_anchor_extras",
            "validate_number_array",
            "anchor rotation quaternion must be normalized",
            "anchor scale components must not be zero",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/assets/gltf/connectors.rs",
        &[
            "pub(super) fn parse_node_connectors",
            "\"connectors\"",
            "\"kind\"",
            "validate_connector_extras",
        ],
    );
}
